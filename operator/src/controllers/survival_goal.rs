//! `SurvivalGoal` controller.

use super::{Context, ControllerError};
use crate::crds::survival_goal::{SurvivalGoalSpec, SurvivalGoalType};
use crate::reconcile::survival_goal::SurvivalGoalReconcilePlan;
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

/// Kube-rs typed resource for the SurvivalGoal CRD.
#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "citus.ai-blaise.io",
    version = "v2",
    kind = "SurvivalGoal",
    namespaced
)]
#[serde(rename_all = "camelCase")]
pub struct SurvivalGoalCrSpec {
    #[serde(default = "default_goal")]
    pub goal: String,
    pub regions: Vec<String>,
    pub min_replicas: u32,
}

fn default_goal() -> String {
    "ZoneFailure".to_string()
}

impl SurvivalGoalCrSpec {
    pub fn to_authoritative(&self) -> Result<SurvivalGoalSpec, String> {
        Ok(SurvivalGoalSpec {
            goal: parse_goal(&self.goal)?,
            regions: self.regions.clone(),
            min_replicas: self.min_replicas,
        })
    }
}

fn parse_goal(goal: &str) -> Result<SurvivalGoalType, String> {
    match goal {
        "ZoneFailure" | "zoneFailure" | "zone-failure" => Ok(SurvivalGoalType::ZoneFailure),
        "RegionFailure" | "regionFailure" | "region-failure" => Ok(SurvivalGoalType::RegionFailure),
        other => Err(format!("unknown survival goal type: {other}")),
    }
}

pub async fn run(ctx: Arc<Context>) -> Result<(), ControllerError> {
    let api: Api<SurvivalGoal> = Api::default_namespaced(ctx.client.clone());
    info!("SurvivalGoal controller starting");
    Controller::new(api, watcher::Config::default())
        .run(reconcile, error_policy, ctx)
        .for_each(|res| async move {
            match res {
                Ok((object, _action)) => debug!(?object, "reconciled SurvivalGoal"),
                Err(error) => error!(?error, "SurvivalGoal reconcile error"),
            }
        })
        .await;
    Ok(())
}

async fn reconcile(
    survival_goal: Arc<SurvivalGoal>,
    ctx: Arc<Context>,
) -> Result<Action, ControllerError> {
    let authoritative = survival_goal
        .spec
        .to_authoritative()
        .map_err(ControllerError::InvalidSpec)?;
    let plan = SurvivalGoalReconcilePlan::try_from(&authoritative)
        .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
    info!(
        survival_goal = ?survival_goal.metadata.name,
        goal = ?authoritative.goal,
        regions = authoritative.regions.len(),
        min_replicas = authoritative.min_replicas,
        apply_steps = plan.steps.len(),
        topology_key = %plan.required_topology_key(),
        "SurvivalGoal reconcile plan built"
    );
    Ok(Action::requeue(ctx.default_requeue))
}

fn error_policy(
    _survival_goal: Arc<SurvivalGoal>,
    error: &ControllerError,
    ctx: Arc<Context>,
) -> Action {
    error!(?error, "SurvivalGoal controller backoff");
    Action::requeue(ctx.default_requeue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cr_spec_round_trips_into_reconcile_plan() {
        let cr = SurvivalGoalCrSpec {
            goal: "RegionFailure".to_string(),
            regions: vec!["us-east-1".to_string(), "us-west-2".to_string()],
            min_replicas: 2,
        };
        let authoritative = cr.to_authoritative().expect("goal spec");
        let plan = SurvivalGoalReconcilePlan::try_from(&authoritative).expect("survival plan");
        assert_eq!(plan.steps.len(), 4);
        assert_eq!(
            plan.required_topology_key(),
            "topology.kubernetes.io/region"
        );
    }

    #[test]
    fn cr_spec_rejects_unknown_goal() {
        let cr = SurvivalGoalCrSpec {
            goal: "EverythingFailure".to_string(),
            regions: vec!["us-east-1".to_string()],
            min_replicas: 1,
        };
        assert!(cr.to_authoritative().is_err());
    }
}
