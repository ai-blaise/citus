//! `Hypertable` controller.

use super::{
    boundary::{
        retry_class_for_error, BoundaryOperation, BoundaryOperationKind, ControllerBoundaryPlan,
    },
    Context, ControllerError,
};
use crate::crds::hypertable::{
    CompressionPolicy, ContinuousAggregateSpec, HypertableSpec, RetentionPolicy,
};
use crate::reconcile::hypertable::HypertableReconcilePlan;
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

/// Kube-rs typed resource for the Hypertable CRD.
#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "citus.ai-blaise.io",
    version = "v2",
    kind = "Hypertable",
    namespaced
)]
#[serde(rename_all = "camelCase")]
pub struct HypertableCrSpec {
    pub table: String,
    pub time_column: String,
    pub distribution_column: String,
    pub chunk_time_interval: String,
    pub num_shards: u32,
    #[serde(default)]
    pub compression: Option<CompressionPolicyCr>,
    #[serde(default)]
    pub retention: Option<RetentionPolicyCr>,
    #[serde(default)]
    pub continuous_aggregates: Vec<ContinuousAggregateCr>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompressionPolicyCr {
    pub older_than: String,
    #[serde(default)]
    pub segment_by: Vec<String>,
    #[serde(default)]
    pub order_by: Vec<String>,
    #[serde(default)]
    pub bloom_filters: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RetentionPolicyCr {
    pub drop_after: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContinuousAggregateCr {
    pub name: String,
    pub query: String,
    #[serde(default)]
    pub refresh_start: Option<String>,
    #[serde(default)]
    pub refresh_end: Option<String>,
    #[serde(default)]
    pub schedule: Option<String>,
    #[serde(default)]
    pub hierarchical_parent: Option<String>,
}

impl HypertableCrSpec {
    pub fn to_authoritative(&self) -> HypertableSpec {
        HypertableSpec {
            table: self.table.clone(),
            time_column: self.time_column.clone(),
            distribution_column: self.distribution_column.clone(),
            chunk_time_interval: self.chunk_time_interval.clone(),
            num_shards: self.num_shards,
            compression: self.compression.as_ref().map(|policy| CompressionPolicy {
                older_than: policy.older_than.clone(),
                segment_by: policy.segment_by.clone(),
                order_by: policy.order_by.clone(),
                bloom_filters: policy.bloom_filters.clone(),
            }),
            retention: self.retention.as_ref().map(|policy| RetentionPolicy {
                drop_after: policy.drop_after.clone(),
            }),
            continuous_aggregates: self
                .continuous_aggregates
                .iter()
                .map(|cagg| ContinuousAggregateSpec {
                    name: cagg.name.clone(),
                    query: cagg.query.clone(),
                    refresh_start: cagg.refresh_start.clone(),
                    refresh_end: cagg.refresh_end.clone(),
                    schedule: cagg.schedule.clone(),
                    hierarchical_parent: cagg.hierarchical_parent.clone(),
                })
                .collect(),
        }
    }
}

pub async fn run(ctx: Arc<Context>) -> Result<(), ControllerError> {
    let api: Api<Hypertable> = Api::default_namespaced(ctx.client.clone());
    info!("Hypertable controller starting");
    Controller::new(api, watcher::Config::default())
        .run(reconcile, error_policy, ctx)
        .for_each(|res| async move {
            match res {
                Ok((object, _action)) => debug!(?object, "reconciled Hypertable"),
                Err(error) => error!(?error, "Hypertable reconcile error"),
            }
        })
        .await;
    Ok(())
}

async fn reconcile(
    hypertable: Arc<Hypertable>,
    ctx: Arc<Context>,
) -> Result<Action, ControllerError> {
    let resource_name = hypertable
        .metadata
        .name
        .clone()
        .unwrap_or_else(|| authoritative_resource_name(&hypertable.spec.table));
    let authoritative = hypertable.spec.to_authoritative();
    let plan = HypertableReconcilePlan::try_from(&authoritative)
        .map_err(|error| ControllerError::Companion(error.to_string()))?;
    let boundary = ControllerBoundaryPlan::try_new(
        "Hypertable",
        &resource_name,
        ctx.execution_mode,
        vec![
            BoundaryOperation::render_plan("render_hypertable_apply_plan"),
            BoundaryOperation::alpha("execute_hypertable_sql", BoundaryOperationKind::DirectSql),
            BoundaryOperation::alpha(
                "patch_hypertable_status",
                BoundaryOperationKind::StatusMutation,
            ),
        ],
        ctx.default_requeue,
    )?;
    info!(
        hypertable = %resource_name,
        sql_plans = plan.sql_plans.len(),
        apply_steps = plan.apply_plan().steps.len(),
        boundary = %boundary.render_tsv(),
        "Hypertable reconciled in bounded dry-run/apply contract"
    );
    Ok(Action::requeue(ctx.default_requeue))
}

fn authoritative_resource_name(table: &str) -> String {
    table.trim().replace('.', "-")
}

fn error_policy(
    _hypertable: Arc<Hypertable>,
    error: &ControllerError,
    ctx: Arc<Context>,
) -> Action {
    let retry_class = retry_class_for_error(error);
    error!(
        ?error,
        retry_class = retry_class.as_str(),
        "Hypertable controller classified reconcile error"
    );
    retry_class.action(ctx.default_requeue)
}
