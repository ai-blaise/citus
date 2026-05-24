//! `Webhook` controller.

use super::{Context, ControllerError};
use crate::crds::webhook::{WebhookEvent, WebhookRetryPolicy, WebhookSpec};
use crate::reconcile::webhook::WebhookReconcilePlan;
use futures::StreamExt;
use kube::{
    api::Api,
    runtime::{controller::Action, watcher, Controller},
    CustomResource,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};

/// Kube-rs typed resource for the Webhook CRD.
#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "citus.ai-blaise.io",
    version = "v2",
    kind = "Webhook",
    namespaced
)]
#[serde(rename_all = "camelCase")]
pub struct WebhookCrSpec {
    pub table: String,
    #[serde(default)]
    pub events: Vec<String>,
    pub url: String,
    #[serde(default)]
    pub headers_secret_ref: Option<String>,
    pub retry_policy: WebhookRetryPolicyCr,
    #[serde(default)]
    pub payload_template: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WebhookRetryPolicyCr {
    pub max_attempts: u32,
    pub backoff: String,
    #[serde(default)]
    pub dead_letter_table: Option<String>,
}

impl WebhookCrSpec {
    pub fn to_authoritative(&self) -> Result<WebhookSpec, String> {
        Ok(WebhookSpec {
            table: self.table.clone(),
            events: self
                .events
                .iter()
                .map(|event| parse_event(event))
                .collect::<Result<Vec<_>, String>>()?,
            url: self.url.clone(),
            headers_secret_ref: self.headers_secret_ref.clone(),
            retry_policy: WebhookRetryPolicy {
                max_attempts: self.retry_policy.max_attempts,
                backoff: self.retry_policy.backoff.clone(),
                dead_letter_table: self.retry_policy.dead_letter_table.clone(),
            },
            payload_template: self.payload_template.clone(),
        })
    }
}

fn parse_event(value: &str) -> Result<WebhookEvent, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "insert" => Ok(WebhookEvent::Insert),
        "update" => Ok(WebhookEvent::Update),
        "delete" => Ok(WebhookEvent::Delete),
        other => Err(format!("unsupported webhook event: {other}")),
    }
}

pub async fn run(ctx: Arc<Context>) -> Result<(), ControllerError> {
    let api: Api<Webhook> = Api::default_namespaced(ctx.client.clone());
    info!("Webhook controller starting");
    Controller::new(api, watcher::Config::default())
        .run(reconcile, error_policy, ctx)
        .for_each(|res| async move {
            match res {
                Ok((object, _action)) => debug!(?object, "reconciled Webhook"),
                Err(error) => error!(?error, "Webhook reconcile error"),
            }
        })
        .await;
    Ok(())
}

async fn reconcile(webhook: Arc<Webhook>, ctx: Arc<Context>) -> Result<Action, ControllerError> {
    let resource_name = webhook.metadata.name.as_deref().unwrap_or("webhook");
    let authoritative = webhook
        .spec
        .to_authoritative()
        .map_err(ControllerError::InvalidSpec)?;
    let plan = WebhookReconcilePlan::from_spec(resource_name, &authoritative)
        .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
    info!(
        webhook = ?webhook.metadata.name,
        events = plan.events.len(),
        apply_steps = plan.apply_plan().steps.len(),
        "Webhook reconciled"
    );
    Ok(Action::requeue(ctx.default_requeue))
}

fn error_policy(_webhook: Arc<Webhook>, error: &ControllerError, ctx: Arc<Context>) -> Action {
    error!(?error, "Webhook controller backoff");
    Action::requeue(ctx.default_requeue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cr_spec_round_trips_to_authoritative_spec() {
        let cr = WebhookCrSpec {
            table: "public.orders".to_string(),
            events: vec!["INSERT".to_string(), "UPDATE".to_string()],
            url: "https://hooks.example.com/orders".to_string(),
            headers_secret_ref: Some("orders-webhook".to_string()),
            retry_policy: WebhookRetryPolicyCr {
                max_attempts: 5,
                backoff: "exponential:1s:30s".to_string(),
                dead_letter_table: Some("companion.webhook_dead_letters".to_string()),
            },
            payload_template: Some("{\"table\":\"orders\"}".to_string()),
        };
        let spec = cr.to_authoritative().expect("valid webhook");
        spec.validate().expect("spec valid");
        let plan = WebhookReconcilePlan::from_spec("orders-hook", &spec).expect("plan valid");
        assert_eq!(plan.events.len(), 2);
        assert_eq!(plan.apply_plan().steps.len(), 6);
    }

    #[test]
    fn unsupported_event_is_rejected() {
        assert_eq!(
            parse_event("truncate"),
            Err("unsupported webhook event: truncate".to_string())
        );
    }
}
