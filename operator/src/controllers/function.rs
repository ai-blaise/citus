//! `Function` controller.

use super::{Context, ControllerError};
use crate::crds::function::{
    FunctionEvent, FunctionRuntime, FunctionSource, FunctionSpec, FunctionTrigger,
};
use crate::reconcile::function::FunctionReconcilePlan;
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

/// Kube-rs typed resource for the Function CRD.
#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "citus.ai-blaise.io",
    version = "v2",
    kind = "Function",
    namespaced
)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCrSpec {
    pub name: String,
    pub runtime: String,
    pub source: FunctionSourceCr,
    #[serde(default)]
    pub triggers: Vec<FunctionTriggerCr>,
    #[serde(default)]
    pub env_secrets: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FunctionSourceCr {
    #[serde(default)]
    pub git_ref: Option<FunctionGitRefCr>,
    #[serde(default)]
    pub inline: Option<FunctionInlineCr>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FunctionGitRefCr {
    pub repository: String,
    pub reference: String,
    pub path: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FunctionInlineCr {
    pub code: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FunctionTriggerCr {
    #[serde(default)]
    pub http: Option<FunctionHttpTriggerCr>,
    #[serde(default)]
    pub scheduled: Option<FunctionScheduledTriggerCr>,
    #[serde(default)]
    pub event: Option<FunctionEventTriggerCr>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FunctionHttpTriggerCr {
    pub path: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FunctionScheduledTriggerCr {
    pub schedule: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FunctionEventTriggerCr {
    pub table: String,
    pub event: String,
}

impl FunctionCrSpec {
    pub fn to_authoritative(&self) -> Result<FunctionSpec, String> {
        Ok(FunctionSpec {
            name: self.name.clone(),
            runtime: parse_runtime(&self.runtime)?,
            source: self.source.to_authoritative()?,
            triggers: self
                .triggers
                .iter()
                .map(FunctionTriggerCr::to_authoritative)
                .collect::<Result<Vec<_>, String>>()?,
            env_secrets: self.env_secrets.clone(),
        })
    }
}

impl FunctionSourceCr {
    fn to_authoritative(&self) -> Result<FunctionSource, String> {
        match (&self.git_ref, &self.inline) {
            (Some(git_ref), None) => Ok(FunctionSource::GitRef {
                repository: git_ref.repository.clone(),
                reference: git_ref.reference.clone(),
                path: git_ref.path.clone(),
            }),
            (None, Some(inline)) => Ok(FunctionSource::Inline {
                code: inline.code.clone(),
            }),
            (None, None) => Err("function source must set gitRef or inline".to_string()),
            (Some(_), Some(_)) => {
                Err("function source must not set both gitRef and inline".to_string())
            }
        }
    }
}

impl FunctionTriggerCr {
    fn to_authoritative(&self) -> Result<FunctionTrigger, String> {
        let set_count = usize::from(self.http.is_some())
            + usize::from(self.scheduled.is_some())
            + usize::from(self.event.is_some());
        if set_count != 1 {
            return Err(
                "function trigger must set exactly one of http, scheduled, or event".to_string(),
            );
        }
        if let Some(http) = &self.http {
            return Ok(FunctionTrigger::Http {
                path: http.path.clone(),
            });
        }
        if let Some(scheduled) = &self.scheduled {
            return Ok(FunctionTrigger::Scheduled {
                schedule: scheduled.schedule.clone(),
            });
        }
        let event = self.event.as_ref().expect("event checked above");
        Ok(FunctionTrigger::Event {
            table: event.table.clone(),
            event: parse_event(&event.event)?,
        })
    }
}

fn parse_runtime(value: &str) -> Result<FunctionRuntime, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "deno" => Ok(FunctionRuntime::Deno),
        "bun" => Ok(FunctionRuntime::Bun),
        other => Err(format!("unsupported function runtime: {other}")),
    }
}

fn parse_event(value: &str) -> Result<FunctionEvent, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "insert" => Ok(FunctionEvent::Insert),
        "update" => Ok(FunctionEvent::Update),
        "delete" => Ok(FunctionEvent::Delete),
        other => Err(format!("unsupported function event: {other}")),
    }
}

pub async fn run(ctx: Arc<Context>) -> Result<(), ControllerError> {
    let api: Api<Function> = Api::default_namespaced(ctx.client.clone());
    info!("Function controller starting");
    Controller::new(api, watcher::Config::default())
        .run(reconcile, error_policy, ctx)
        .for_each(|res| async move {
            match res {
                Ok((object, _action)) => debug!(?object, "reconciled Function"),
                Err(error) => error!(?error, "Function reconcile error"),
            }
        })
        .await;
    Ok(())
}

async fn reconcile(function: Arc<Function>, ctx: Arc<Context>) -> Result<Action, ControllerError> {
    let authoritative = function
        .spec
        .to_authoritative()
        .map_err(ControllerError::InvalidSpec)?;
    let plan = FunctionReconcilePlan::try_from(&authoritative)
        .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
    info!(
        function = ?function.metadata.name,
        http_triggers = plan.http_triggers.len(),
        scheduled_triggers = plan.scheduled_triggers.len(),
        event_triggers = plan.event_triggers.len(),
        apply_steps = plan.apply_plan().steps.len(),
        "Function reconciled"
    );
    Ok(Action::requeue(ctx.default_requeue))
}

fn error_policy(_function: Arc<Function>, error: &ControllerError, ctx: Arc<Context>) -> Action {
    error!(?error, "Function controller backoff");
    Action::requeue(ctx.default_requeue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cr_spec_round_trips_to_authoritative_spec() {
        let cr = FunctionCrSpec {
            name: "order-created".to_string(),
            runtime: "deno".to_string(),
            source: FunctionSourceCr {
                git_ref: Some(FunctionGitRefCr {
                    repository: "https://github.com/ai-blaise/functions".to_string(),
                    reference: "main".to_string(),
                    path: "orders/index.ts".to_string(),
                }),
                inline: None,
            },
            triggers: vec![
                FunctionTriggerCr {
                    http: Some(FunctionHttpTriggerCr {
                        path: "/orders".to_string(),
                    }),
                    scheduled: None,
                    event: None,
                },
                FunctionTriggerCr {
                    http: None,
                    scheduled: None,
                    event: Some(FunctionEventTriggerCr {
                        table: "public.orders".to_string(),
                        event: "INSERT".to_string(),
                    }),
                },
            ],
            env_secrets: vec!["orders-api-key".to_string()],
        };
        let spec = cr.to_authoritative().expect("valid function");
        spec.validate().expect("spec valid");
        let plan = FunctionReconcilePlan::try_from(&spec).expect("plan valid");
        assert_eq!(plan.http_triggers.len(), 1);
        assert_eq!(plan.event_triggers.len(), 1);
    }

    #[test]
    fn source_rejects_ambiguous_shape() {
        let source = FunctionSourceCr {
            git_ref: Some(FunctionGitRefCr {
                repository: "https://github.com/ai-blaise/functions".to_string(),
                reference: "main".to_string(),
                path: "orders/index.ts".to_string(),
            }),
            inline: Some(FunctionInlineCr {
                code: "export default {}".to_string(),
            }),
        };
        assert_eq!(
            source.to_authoritative(),
            Err("function source must not set both gitRef and inline".to_string())
        );
    }

    #[test]
    fn trigger_rejects_empty_shape() {
        assert_eq!(
            FunctionTriggerCr::default().to_authoritative(),
            Err("function trigger must set exactly one of http, scheduled, or event".to_string())
        );
    }
}
