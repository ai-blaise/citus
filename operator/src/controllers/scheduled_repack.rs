//! `ScheduledRepack` controller.

use super::{Context, ControllerError};
use crate::crds::scheduled_repack::{RepackStrategy, ScheduledRepackSpec};
use crate::reconcile::scheduled_repack::ScheduledRepackReconcilePlan;
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
    kind = "ScheduledRepack",
    namespaced,
    status = "ScheduledRepackStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledRepackCrSpec {
    pub target: String,
    pub schedule: String,
    #[serde(default = "default_strategy")]
    pub strategy: String,
}

fn default_strategy() -> String {
    "PgRepack".to_string()
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledRepackStatus {
    pub job_name: String,
    pub apply_steps: usize,
    pub bloat_estimate_sql: String,
}

impl ScheduledRepackCrSpec {
    pub fn to_authoritative(&self) -> Result<ScheduledRepackSpec, String> {
        Ok(ScheduledRepackSpec {
            target: self.target.clone(),
            schedule: self.schedule.clone(),
            strategy: parse_strategy(&self.strategy)?,
        })
    }
}

fn parse_strategy(value: &str) -> Result<RepackStrategy, String> {
    match normalize_token(value).as_str() {
        "pgrepack" => Ok(RepackStrategy::PgRepack),
        "repackconcurrentlypg19" | "pg19" => Ok(RepackStrategy::RepackConcurrentlyPg19),
        other => Err(format!("unsupported repack strategy: {other}")),
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
    let api: Api<ScheduledRepack> = Api::default_namespaced(ctx.client.clone());
    info!("ScheduledRepack controller starting");
    Controller::new(api, watcher::Config::default())
        .run(reconcile, error_policy, ctx)
        .for_each(|res| async move {
            match res {
                Ok((object, _action)) => debug!(?object, "reconciled ScheduledRepack"),
                Err(error) => error!(?error, "ScheduledRepack reconcile error"),
            }
        })
        .await;
    Ok(())
}

async fn reconcile(
    scheduled_repack: Arc<ScheduledRepack>,
    ctx: Arc<Context>,
) -> Result<Action, ControllerError> {
    let resource_name = scheduled_repack
        .metadata
        .name
        .as_deref()
        .unwrap_or("scheduled-repack");
    let authoritative = scheduled_repack
        .spec
        .to_authoritative()
        .map_err(ControllerError::InvalidSpec)?;
    let plan = ScheduledRepackReconcilePlan::from_spec(resource_name, &authoritative)
        .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
    let apply = plan.apply_plan();
    info!(
        scheduled_repack = ?scheduled_repack.metadata.name,
        job = %plan.job_name,
        target = %plan.target,
        strategy = %plan.strategy_str(),
        apply_steps = apply.steps.len(),
        "ScheduledRepack reconcile plan built"
    );
    Ok(Action::requeue(ctx.default_requeue))
}

fn error_policy(
    _scheduled_repack: Arc<ScheduledRepack>,
    error: &ControllerError,
    ctx: Arc<Context>,
) -> Action {
    error!(?error, "ScheduledRepack controller backoff");
    Action::requeue(ctx.default_requeue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cr_spec_round_trips_into_reconcile_plan() {
        let cr = ScheduledRepackCrSpec {
            target: "public.orders".to_string(),
            schedule: "0 3 * * 0".to_string(),
            strategy: "PgRepack".to_string(),
        };
        let authoritative = cr.to_authoritative().expect("valid spec");
        let plan = ScheduledRepackReconcilePlan::from_spec("weekly-orders", &authoritative)
            .expect("valid plan");
        assert_eq!(plan.job_name, "ai-blaise-citus-repack-weekly-orders");
        assert_eq!(plan.apply_plan().steps.len(), 5);
    }

    #[test]
    fn unsupported_strategy_is_rejected() {
        assert_eq!(
            parse_strategy("vacuum-full"),
            Err("unsupported repack strategy: vacuumfull".to_string())
        );
    }
}
