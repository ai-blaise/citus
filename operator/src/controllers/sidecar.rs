//! `Sidecar` controller.

use super::boundary::ExecutionMode;
use super::{Context, ControllerError};
use crate::crds::sidecar::{ResourceRequirements, SidecarDeploymentSpec, SidecarDeploymentType};
use crate::reconcile::sidecar::SidecarReconcilePlan;
use futures::StreamExt;
use k8s_openapi::api::{apps::v1::Deployment, core::v1::Service};
use kube::{
    api::{Api, Patch, PatchParams},
    runtime::{controller::Action, watcher, Controller},
    CustomResource, Resource, ResourceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
    pub image: Option<String>,
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
            image: self.image.clone(),
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
    let resource_name = sidecar.name_any();
    let authoritative = sidecar.spec.to_authoritative();
    let plan = SidecarReconcilePlan::from_spec(&resource_name, &authoritative)
        .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
    let urls = plan.status_probe_urls();
    info!(
        sidecar = %resource_name,
        deployment = %plan.deployment_name,
        service = %plan.service_name,
        replicas = plan.replicas,
        image = %plan.image_ref(),
        readyz = %urls.readyz,
        metrics = %urls.metrics,
        mode = ctx.execution_mode.as_str(),
        "Sidecar reconcile plan built"
    );

    if matches!(ctx.execution_mode, ExecutionMode::Apply) {
        apply_sidecar_resources(&sidecar, &ctx, &plan).await?;
    }

    Ok(Action::requeue(ctx.default_requeue))
}

async fn apply_sidecar_resources(
    sidecar: &Sidecar,
    ctx: &Context,
    plan: &SidecarReconcilePlan,
) -> Result<(), ControllerError> {
    plan.validate_apply_ready()
        .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
    let namespace = sidecar.namespace().unwrap_or_else(|| "default".to_string());
    let owner_ref = sidecar.controller_owner_ref(&()).ok_or_else(|| {
        ControllerError::InvalidSpec("Sidecar owner reference requires metadata.uid".to_string())
    })?;
    let owner_ref = serde_json::to_value(owner_ref)
        .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
    let apply = PatchParams::apply("ai-blaise-citus-operator").force();

    let deployment_manifest = manifest_with_owner_namespace(
        &plan.deployment_manifest_yaml(),
        &namespace,
        owner_ref.clone(),
    )?;
    let deployments: Api<Deployment> = Api::namespaced(ctx.client.clone(), &namespace);
    deployments
        .patch(
            &plan.deployment_name,
            &apply,
            &Patch::Apply(&deployment_manifest),
        )
        .await?;

    let service_manifest =
        manifest_with_owner_namespace(&plan.service_manifest_yaml(), &namespace, owner_ref)?;
    let services: Api<Service> = Api::namespaced(ctx.client.clone(), &namespace);
    services
        .patch(&plan.service_name, &apply, &Patch::Apply(&service_manifest))
        .await?;

    patch_sidecar_status(sidecar, ctx, plan, &namespace, &apply).await
}

fn manifest_with_owner_namespace(
    yaml: &str,
    namespace: &str,
    owner_ref: Value,
) -> Result<Value, ControllerError> {
    let mut manifest: Value = serde_yaml::from_str(yaml)
        .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
    let metadata = manifest
        .get_mut("metadata")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| {
            ControllerError::InvalidSpec("manifest metadata must be an object".to_string())
        })?;
    metadata.insert("namespace".to_string(), json!(namespace));
    metadata.insert("ownerReferences".to_string(), json!([owner_ref]));
    Ok(manifest)
}

async fn patch_sidecar_status(
    sidecar: &Sidecar,
    ctx: &Context,
    plan: &SidecarReconcilePlan,
    namespace: &str,
    apply: &PatchParams,
) -> Result<(), ControllerError> {
    let urls = plan.status_probe_urls();
    let status = SidecarStatus {
        deployment_name: plan.deployment_name.clone(),
        service_name: plan.service_name.clone(),
        readyz: urls.readyz,
        metrics_url: urls.metrics,
    };
    let status_patch = json!({
        "apiVersion": "citus.ai-blaise.io/v2",
        "kind": "Sidecar",
        "metadata": {
            "name": sidecar.name_any(),
            "namespace": namespace,
        },
        "status": status,
    });
    let sidecars: Api<Sidecar> = Api::namespaced(ctx.client.clone(), namespace);
    sidecars
        .patch_status(&sidecar.name_any(), apply, &Patch::Apply(&status_patch))
        .await?;
    Ok(())
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
            image: None,
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
    fn manifest_apply_metadata_adds_namespace_and_owner_reference() {
        let manifest = manifest_with_owner_namespace(
            "apiVersion: v1\nkind: Service\nmetadata:\n  name: demo\nspec: {}\n",
            "prod",
            json!({"apiVersion":"citus.ai-blaise.io/v2","kind":"Sidecar","name":"demo","uid":"123","controller":true,"blockOwnerDeletion":true}),
        )
        .expect("manifest");
        assert_eq!(manifest["metadata"]["namespace"], json!("prod"));
        assert_eq!(
            manifest["metadata"]["ownerReferences"][0]["uid"],
            json!("123")
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
