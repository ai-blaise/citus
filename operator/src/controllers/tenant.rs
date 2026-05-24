//! `Tenant` controller.

use super::{
    boundary::{
        retry_class_for_error, BoundaryOperation, BoundaryOperationKind, ControllerBoundaryPlan,
    },
    Context, ControllerError,
};
use crate::crds::tenant::{TenantQuotas, TenantSpec};
use crate::reconcile::tenant::TenantReconcilePlan;
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

/// Kube-rs typed resource for the Tenant CRD.
#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "citus.ai-blaise.io",
    version = "v2",
    kind = "Tenant",
    namespaced
)]
#[serde(rename_all = "camelCase")]
pub struct TenantCrSpec {
    pub name: String,
    pub schema_name: String,
    pub max_connections: u32,
    pub max_qps: u32,
    pub max_storage_bytes: u64,
    #[serde(default)]
    pub region_affinity: Option<String>,
}

impl TenantCrSpec {
    pub fn to_authoritative(&self) -> TenantSpec {
        TenantSpec {
            name: self.name.clone(),
            schema_name: self.schema_name.clone(),
            quotas: TenantQuotas {
                max_connections: self.max_connections,
                max_qps: self.max_qps,
                max_storage_bytes: self.max_storage_bytes,
            },
            region_affinity: self.region_affinity.clone(),
        }
    }
}

pub async fn run(ctx: Arc<Context>) -> Result<(), ControllerError> {
    let api: Api<Tenant> = Api::default_namespaced(ctx.client.clone());
    info!("Tenant controller starting");
    Controller::new(api, watcher::Config::default())
        .run(reconcile, error_policy, ctx)
        .for_each(|res| async move {
            match res {
                Ok((object, _action)) => debug!(?object, "reconciled Tenant"),
                Err(error) => error!(?error, "Tenant reconcile error"),
            }
        })
        .await;
    Ok(())
}

async fn reconcile(tenant: Arc<Tenant>, ctx: Arc<Context>) -> Result<Action, ControllerError> {
    let resource_name = tenant
        .metadata
        .name
        .clone()
        .unwrap_or_else(|| tenant.spec.name.clone());
    let authoritative = tenant.spec.to_authoritative();
    let plan = TenantReconcilePlan::try_from(&authoritative)
        .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
    let boundary = ControllerBoundaryPlan::try_new(
        "Tenant",
        &resource_name,
        ctx.execution_mode,
        vec![
            BoundaryOperation::render_plan("render_tenant_plan"),
            BoundaryOperation::alpha("execute_tenant_sql", BoundaryOperationKind::DirectSql),
            BoundaryOperation::alpha("patch_tenant_status", BoundaryOperationKind::StatusMutation),
        ],
        ctx.default_requeue,
    )?;
    info!(
        tenant = %resource_name,
        max_qps = authoritative.quotas.max_qps,
        apply_steps = plan.steps.len(),
        sql_steps = plan.sql_step_count(),
        pool_configmap = %plan.pool_configmap_name(),
        boundary = %boundary.render_tsv(),
        "Tenant reconciled in bounded dry-run/apply contract"
    );
    Ok(Action::requeue(ctx.default_requeue))
}

fn error_policy(_tenant: Arc<Tenant>, error: &ControllerError, ctx: Arc<Context>) -> Action {
    let retry_class = retry_class_for_error(error);
    error!(
        ?error,
        retry_class = retry_class.as_str(),
        "Tenant controller classified reconcile error"
    );
    retry_class.action(ctx.default_requeue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cr_spec_round_trips_into_reconcile_plan() {
        let cr = TenantCrSpec {
            name: "tenant-a".to_string(),
            schema_name: "tenant_a".to_string(),
            max_connections: 32,
            max_qps: 5_000,
            max_storage_bytes: 1_099_511_627_776,
            region_affinity: Some("us-east-1".to_string()),
        };
        let authoritative = cr.to_authoritative();
        let plan = TenantReconcilePlan::try_from(&authoritative).expect("tenant plan");
        assert_eq!(plan.steps.len(), 5);
        assert_eq!(plan.sql_step_count(), 3);
    }
}
