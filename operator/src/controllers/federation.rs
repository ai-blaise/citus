//! `Federation` controller.

use super::{Context, ControllerError};
use crate::crds::federation::{FederationConnection, FederationSpec, FederationType};
use crate::reconcile::federation::FederationReconcilePlan;
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

/// Kube-rs typed resource for the Federation CRD.
#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "citus.ai-blaise.io",
    version = "v2",
    kind = "Federation",
    namespaced
)]
#[serde(rename_all = "camelCase")]
pub struct FederationCrSpec {
    #[serde(rename = "type")]
    pub federation_type: String,
    pub connection: FederationConnectionCr,
    pub foreign_schema_prefix: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FederationConnectionCr {
    pub secret_ref: String,
}

impl FederationCrSpec {
    pub fn to_authoritative(&self, resource_name: &str) -> Result<FederationSpec, String> {
        Ok(FederationSpec {
            name: resource_name.to_string(),
            federation_type: parse_federation_type(&self.federation_type)?,
            connection: FederationConnection {
                secret_ref: self.connection.secret_ref.clone(),
            },
            foreign_schema_prefix: self.foreign_schema_prefix.clone(),
        })
    }
}

fn parse_federation_type(value: &str) -> Result<FederationType, String> {
    match normalize_token(value).as_str() {
        "snowflake" => Ok(FederationType::Snowflake),
        "bigquery" => Ok(FederationType::BigQuery),
        "databricks" => Ok(FederationType::Databricks),
        "mysql" => Ok(FederationType::MySql),
        "mongo" | "mongodb" => Ok(FederationType::Mongo),
        "oracle" => Ok(FederationType::Oracle),
        other => Err(format!("unsupported federation type: {other}")),
    }
}

fn normalize_token(value: &str) -> String {
    value
        .trim()
        .chars()
        .filter(|character| !matches!(character, '-' | '_'))
        .collect::<String>()
        .to_ascii_lowercase()
}

pub async fn run(ctx: Arc<Context>) -> Result<(), ControllerError> {
    let api: Api<Federation> = Api::default_namespaced(ctx.client.clone());
    info!("Federation controller starting");
    Controller::new(api, watcher::Config::default())
        .run(reconcile, error_policy, ctx)
        .for_each(|res| async move {
            match res {
                Ok((object, _action)) => debug!(?object, "reconciled Federation"),
                Err(error) => error!(?error, "Federation reconcile error"),
            }
        })
        .await;
    Ok(())
}

async fn reconcile(
    federation: Arc<Federation>,
    ctx: Arc<Context>,
) -> Result<Action, ControllerError> {
    let resource_name = federation.metadata.name.as_deref().unwrap_or("federation");
    let authoritative = federation
        .spec
        .to_authoritative(resource_name)
        .map_err(ControllerError::InvalidSpec)?;
    let plan = FederationReconcilePlan::try_from(&authoritative)
        .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
    info!(
        federation = ?federation.metadata.name,
        backend = ?plan.backend,
        apply_steps = plan.apply_plan().steps.len(),
        "Federation reconciled"
    );
    Ok(Action::requeue(ctx.default_requeue))
}

fn error_policy(
    _federation: Arc<Federation>,
    error: &ControllerError,
    ctx: Arc<Context>,
) -> Action {
    error!(?error, "Federation controller backoff");
    Action::requeue(ctx.default_requeue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cr_spec_round_trips_to_authoritative_spec() {
        let cr = FederationCrSpec {
            federation_type: "snowflake".to_string(),
            connection: FederationConnectionCr {
                secret_ref: "snowflake-prod".to_string(),
            },
            foreign_schema_prefix: "snowflake_raw".to_string(),
        };
        let spec = cr.to_authoritative("warehouse").expect("valid federation");
        spec.validate().expect("spec valid");
        assert_eq!(spec.name, "warehouse");
        assert!(matches!(spec.federation_type, FederationType::Snowflake));
        let plan = FederationReconcilePlan::try_from(&spec).expect("plan valid");
        assert!(plan.backend.is_iceberg());
    }

    #[test]
    fn unsupported_type_is_rejected() {
        assert_eq!(
            parse_federation_type("sqlite"),
            Err("unsupported federation type: sqlite".to_string())
        );
    }
}
