//! `Region` controller.

use super::{Context, ControllerError};
use crate::crds::region::RegionSpec;
use crate::reconcile::region::RegionReconcilePlan;
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

/// Kube-rs typed resource for the Region CRD.
#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "citus.ai-blaise.io",
    version = "v2",
    kind = "Region",
    namespaced
)]
#[serde(rename_all = "camelCase")]
pub struct RegionCrSpec {
    pub name: String,
    pub kubernetes_zone: String,
    pub tablespace_name: String,
    #[serde(default)]
    pub leader_pinned: bool,
}

impl RegionCrSpec {
    pub fn to_authoritative(&self) -> RegionSpec {
        RegionSpec {
            name: self.name.clone(),
            kubernetes_zone: self.kubernetes_zone.clone(),
            tablespace_name: self.tablespace_name.clone(),
            leader_pinned: self.leader_pinned,
        }
    }
}

pub async fn run(ctx: Arc<Context>) -> Result<(), ControllerError> {
    let api: Api<Region> = Api::default_namespaced(ctx.client.clone());
    info!("Region controller starting");
    Controller::new(api, watcher::Config::default())
        .run(reconcile, error_policy, ctx)
        .for_each(|res| async move {
            match res {
                Ok((object, _action)) => debug!(?object, "reconciled Region"),
                Err(error) => error!(?error, "Region reconcile error"),
            }
        })
        .await;
    Ok(())
}

async fn reconcile(region: Arc<Region>, ctx: Arc<Context>) -> Result<Action, ControllerError> {
    let authoritative = region.spec.to_authoritative();
    let plan = RegionReconcilePlan::try_from(&authoritative)
        .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
    info!(
        region = ?region.metadata.name,
        tablespace = %plan.spec.tablespace_name,
        apply_steps = plan.steps.len(),
        sql_steps = plan.sql_step_count(),
        node_affinity = %plan.node_affinity_label(),
        leader_affinity = ?plan.leader_affinity_label(),
        "Region reconcile plan built"
    );
    Ok(Action::requeue(ctx.default_requeue))
}

fn error_policy(_region: Arc<Region>, error: &ControllerError, ctx: Arc<Context>) -> Action {
    error!(?error, "Region controller backoff");
    Action::requeue(ctx.default_requeue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cr_spec_round_trips_into_reconcile_plan() {
        let cr = RegionCrSpec {
            name: "us-east-1".to_string(),
            kubernetes_zone: "us-east-1a".to_string(),
            tablespace_name: "ts_us_east_1".to_string(),
            leader_pinned: true,
        };
        let authoritative = cr.to_authoritative();
        let plan = RegionReconcilePlan::try_from(&authoritative).expect("region plan");
        assert_eq!(plan.steps.len(), 4);
        assert_eq!(
            plan.node_affinity_label(),
            "topology.kubernetes.io/zone=us-east-1a"
        );
    }
}
