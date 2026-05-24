//! `Sidecar` controller.

use super::{Context, ControllerError};
use crate::crds::sidecar::{ResourceRequirements, SidecarDeploymentSpec, SidecarDeploymentType};
use crate::reconcile::sidecar::SidecarReconcilePlan;
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

#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "citus.ai-blaise.io",
    version = "v2",
    kind = "Sidecar",
    namespaced,
    status = "SidecarStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct SidecarCrSpec {
    #[serde(rename = "type")]
    pub sidecar_type: String,
    #[serde(default = "default_one")]
    pub replicas: u32,
    #[serde(default = "default_cpu_millis")]
    pub cpu_millis: u32,
    #[serde(default = "default_memory_mib")]
    pub memory_mib: u32,
    #[serde(default)]
    pub config_yaml: Option<String>,
}

fn default_one() -> u32 {
    1
}

fn default_cpu_millis() -> u32 {
    250
}

fn default_memory_mib() -> u32 {
    512
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SidecarStatus {
    pub deployment_name: String,
    pub service_name: String,
    pub readyz: String,
    pub metrics_url: String,
}

impl SidecarCrSpec {
    pub fn to_authoritative(&self) -> SidecarDeploymentSpec {
        SidecarDeploymentSpec {
            sidecar_type: parse_sidecar_kind(&self.sidecar_type),
            replicas: self.replicas,
            resources: ResourceRequirements {
                cpu_millis: self.cpu_millis,
                memory_mib: self.memory_mib,
            },
            config_yaml: self.config_yaml.clone(),
        }
    }
}

fn parse_sidecar_kind(kind: &str) -> SidecarDeploymentType {
    match normalize_token(kind).as_str() {
        "analytical" => SidecarDeploymentType::Analytical,
        "vectorizer" => SidecarDeploymentType::Vectorizer,
        "cdc" => SidecarDeploymentType::Cdc,
        "coldtier" => SidecarDeploymentType::ColdTier,
        "raft" => SidecarDeploymentType::Raft,
        "hlc" => SidecarDeploymentType::Hlc,
        "txnstatus" => SidecarDeploymentType::TxnStatus,
        "schemajob" => SidecarDeploymentType::SchemaJob,
        "realtime" => SidecarDeploymentType::Realtime,
        "auth" => SidecarDeploymentType::Auth,
        "storage" => SidecarDeploymentType::Storage,
        "postgrest" => SidecarDeploymentType::Postgrest,
        "graphql" => SidecarDeploymentType::Graphql,
        "edgefunctions" => SidecarDeploymentType::EdgeFunctions,
        "backup" => SidecarDeploymentType::Backup,
        "repack" => SidecarDeploymentType::Repack,
        "mcp" => SidecarDeploymentType::Mcp,
        _ => SidecarDeploymentType::Custom(kind.trim().to_string()),
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
    let api: Api<Sidecar> = Api::default_namespaced(ctx.client.clone());
    info!("Sidecar controller starting");
    Controller::new(api, watcher::Config::default())
        .run(reconcile, error_policy, ctx)
        .for_each(|res| async move {
            match res {
                Ok((object, _action)) => debug!(?object, "reconciled Sidecar"),
                Err(error) => error!(?error, "Sidecar reconcile error"),
            }
        })
        .await;
    Ok(())
}

async fn reconcile(sidecar: Arc<Sidecar>, ctx: Arc<Context>) -> Result<Action, ControllerError> {
    let resource_name = sidecar.metadata.name.as_deref().unwrap_or("sidecar");
    let authoritative = sidecar.spec.to_authoritative();
    let plan = SidecarReconcilePlan::from_spec(resource_name, &authoritative)
        .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
    let urls = plan.status_probe_urls();
    info!(
        sidecar = ?sidecar.metadata.name,
        deployment = %plan.deployment_name,
        service = %plan.service_name,
        replicas = plan.replicas,
        readyz = %urls.readyz,
        metrics = %urls.metrics,
        "Sidecar reconcile plan built"
    );
    Ok(Action::requeue(ctx.default_requeue))
}

fn error_policy(_sidecar: Arc<Sidecar>, error: &ControllerError, ctx: Arc<Context>) -> Action {
    error!(?error, "Sidecar controller backoff");
    Action::requeue(ctx.default_requeue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cr_spec_round_trips_into_reconcile_plan() {
        let cr = SidecarCrSpec {
            sidecar_type: "Realtime".to_string(),
            replicas: 2,
            cpu_millis: 250,
            memory_mib: 512,
            config_yaml: Some("subscriptions:\n  max_per_tenant: 1000".to_string()),
        };
        let authoritative = cr.to_authoritative();
        let plan = SidecarReconcilePlan::from_spec("primary", &authoritative).expect("valid plan");
        assert_eq!(
            plan.deployment_name,
            "ai-blaise-citus-sidecar-primary-realtime"
        );
        assert_eq!(
            plan.status_probe_urls().readyz,
            "http://ai-blaise-citus-sidecar-primary-realtime:8080/readyz"
        );
    }

    #[test]
    fn custom_sidecar_type_is_preserved_for_reconcile_validation() {
        let sidecar_type = parse_sidecar_kind("Custom Analytics_v2");
        assert!(
            matches!(sidecar_type, SidecarDeploymentType::Custom(name) if name == "Custom Analytics_v2")
        );
    }
}
