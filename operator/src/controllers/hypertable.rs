//! `Hypertable` controller.

use super::{
    boundary::{
        retry_class_for_error, BoundaryOperation, BoundaryOperationKind, ControllerBoundaryPlan,
        ExecutionMode,
    },
    Context, ControllerError,
};
use crate::crds::hypertable::{
    CompressionPolicy, ContinuousAggregateSpec, HypertableSpec, RetentionPolicy,
};
use crate::reconcile::hypertable::{
    HypertableApplyPlan, HypertableApplyStep, HypertableReconcilePlan,
};
use futures::StreamExt;
use kube::{
    api::{Api, Patch, PatchParams},
    runtime::{controller::Action, watcher, Controller},
    CustomResource, ResourceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::{NoTls, Transaction};
use tracing::{debug, error, info, warn};

pub const HYPERTABLE_DATABASE_URL_ENV: &str = "AI_BLAISE_HYPERTABLE_DATABASE_URL";

/// Kube-rs typed resource for the Hypertable CRD.
#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "citus.ai-blaise.io",
    version = "v2",
    kind = "Hypertable",
    namespaced,
    status = "HypertableStatus"
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

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HypertableStatus {
    pub phase: String,
    pub table: String,
    pub sql_plan_count: u32,
    pub apply_step_count: u32,
    pub applied_step_count: u32,
    pub skipped_step_count: u32,
    pub observed_generation: Option<i64>,
    pub last_applied_sql_hash: String,
    pub last_applied_unix_seconds: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(default)]
    pub conditions: Vec<HypertableCondition>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HypertableCondition {
    #[serde(rename = "type")]
    pub condition_type: String,
    pub status: String,
    pub reason: String,
    pub message: String,
    pub last_transition_unix_seconds: u64,
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
    let namespace = hypertable
        .namespace()
        .unwrap_or_else(|| "default".to_string());
    let authoritative = hypertable.spec.to_authoritative();
    let observed_generation = hypertable.metadata.generation;
    let plan = HypertableReconcilePlan::try_from(&authoritative)
        .map_err(|error| ControllerError::Companion(error.to_string()))?;
    let apply_plan = plan.apply_plan();
    let boundary = ControllerBoundaryPlan::try_new(
        "Hypertable",
        &resource_name,
        ctx.execution_mode,
        vec![
            BoundaryOperation::render_plan("render_hypertable_apply_plan"),
            BoundaryOperation::implemented(
                "execute_hypertable_sql",
                BoundaryOperationKind::DirectSql,
            ),
            BoundaryOperation::implemented(
                "patch_hypertable_status",
                BoundaryOperationKind::StatusMutation,
            ),
        ],
        ctx.default_requeue,
    )?;
    info!(
        hypertable = %resource_name,
        sql_plans = plan.sql_plans.len(),
        apply_steps = apply_plan.steps.len(),
        mode = ctx.execution_mode.as_str(),
        boundary = %boundary.render_tsv(),
        "Hypertable reconcile plan built"
    );

    if matches!(ctx.execution_mode, ExecutionMode::Apply) {
        match apply_hypertable_sql(&plan, &apply_plan).await {
            Ok(report) => {
                patch_hypertable_status(
                    &hypertable,
                    &ctx,
                    &namespace,
                    hypertable_status_applied(
                        &resource_name,
                        &plan,
                        &apply_plan,
                        &report,
                        observed_generation,
                    ),
                )
                .await?;
            }
            Err(error) => {
                let message = error.to_string();
                if let Err(status_error) = patch_hypertable_status(
                    &hypertable,
                    &ctx,
                    &namespace,
                    hypertable_status_failed(
                        &resource_name,
                        &plan,
                        &apply_plan,
                        observed_generation,
                        &message,
                    ),
                )
                .await
                {
                    warn!(?status_error, "failed to patch Hypertable failure status");
                }
                return Err(error);
            }
        }
    }

    Ok(Action::requeue(ctx.default_requeue))
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct HypertableSqlApplyReport {
    sql_hash: String,
    applied_steps: u32,
    skipped_steps: u32,
}

async fn apply_hypertable_sql(
    plan: &HypertableReconcilePlan,
    apply_plan: &HypertableApplyPlan,
) -> Result<HypertableSqlApplyReport, ControllerError> {
    let database_url = env::var(HYPERTABLE_DATABASE_URL_ENV).map_err(|_| {
        ControllerError::InvalidSpec(format!(
            "{HYPERTABLE_DATABASE_URL_ENV} must be set in apply mode"
        ))
    })?;
    let sql_hash = stable_sql_hash(&apply_plan.sql_script());
    let (mut client, connection) = tokio_postgres::connect(&database_url, NoTls)
        .await
        .map_err(|error| {
            ControllerError::Companion(format!("hypertable SQL connect failed: {error}"))
        })?;
    tokio::spawn(async move {
        if let Err(error) = connection.await {
            error!(?error, "hypertable SQL connection task failed");
        }
    });

    let transaction = client.transaction().await.map_err(|error| {
        ControllerError::Companion(format!("hypertable SQL transaction failed: {error}"))
    })?;
    transaction
        .batch_execute("SET LOCAL lock_timeout = '5s'; SET LOCAL statement_timeout = '5min';")
        .await
        .map_err(|error| {
            ControllerError::Companion(format!("hypertable SQL timeout setup failed: {error}"))
        })?;
    transaction
        .execute(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0));",
            &[&plan.distributed_hypertable.table],
        )
        .await
        .map_err(|error| {
            ControllerError::Companion(format!("hypertable SQL lock failed: {error}"))
        })?;

    let mut applied_steps = 0;
    let mut skipped_steps = 0;
    for step in &apply_plan.steps {
        if bridge_state_exists(&transaction, step).await? {
            skipped_steps += 1;
            continue;
        }
        transaction
            .batch_execute(&step.sql)
            .await
            .map_err(|error| {
                ControllerError::Companion(format!(
                    "hypertable SQL step {} failed: {error:?}",
                    step.name,
                ))
            })?;
        applied_steps += 1;
    }
    transaction.commit().await.map_err(|error| {
        ControllerError::Companion(format!("hypertable SQL commit failed: {error}"))
    })?;

    Ok(HypertableSqlApplyReport {
        sql_hash,
        applied_steps,
        skipped_steps,
    })
}

async fn bridge_state_exists(
    transaction: &Transaction<'_>,
    step: &HypertableApplyStep,
) -> Result<bool, ControllerError> {
    let Some(key) = &step.bridge_state_key else {
        return Ok(false);
    };
    let row = transaction
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM companion_timescale_bridge_state WHERE feature_id = $1 AND object_name = $2);",
            &[&key.feature_id, &key.object_name],
        )
        .await
        .map_err(|error| {
            ControllerError::Companion(format!(
                "hypertable bridge-state check failed for {}:{}: {error}",
                key.feature_id, key.object_name
            ))
        })?;
    Ok(row.get::<_, bool>(0))
}

async fn patch_hypertable_status(
    hypertable: &Hypertable,
    ctx: &Context,
    namespace: &str,
    status: HypertableStatus,
) -> Result<(), ControllerError> {
    let apply = PatchParams::apply("ai-blaise-citus-operator").force();
    let status_patch = json!({
        "apiVersion": "citus.ai-blaise.io/v2",
        "kind": "Hypertable",
        "metadata": {
            "name": hypertable.name_any(),
            "namespace": namespace,
        },
        "status": status,
    });
    let hypertables: Api<Hypertable> = Api::namespaced(ctx.client.clone(), namespace);
    hypertables
        .patch_status(&hypertable.name_any(), &apply, &Patch::Apply(&status_patch))
        .await?;
    Ok(())
}

fn hypertable_status_applied(
    resource_name: &str,
    plan: &HypertableReconcilePlan,
    apply_plan: &HypertableApplyPlan,
    report: &HypertableSqlApplyReport,
    observed_generation: Option<i64>,
) -> HypertableStatus {
    let now = now_unix_seconds();
    HypertableStatus {
        phase: "Applied".to_string(),
        table: resource_name.to_string(),
        sql_plan_count: plan.sql_plans.len() as u32,
        apply_step_count: apply_plan.steps.len() as u32,
        applied_step_count: report.applied_steps,
        skipped_step_count: report.skipped_steps,
        observed_generation,
        last_applied_sql_hash: report.sql_hash.clone(),
        last_applied_unix_seconds: now,
        last_error: None,
        conditions: vec![HypertableCondition {
            condition_type: "Ready".to_string(),
            status: "True".to_string(),
            reason: "SqlApplied".to_string(),
            message: "Hypertable SQL apply plan committed and status patched".to_string(),
            last_transition_unix_seconds: now,
        }],
    }
}

fn hypertable_status_failed(
    resource_name: &str,
    plan: &HypertableReconcilePlan,
    apply_plan: &HypertableApplyPlan,
    observed_generation: Option<i64>,
    message: &str,
) -> HypertableStatus {
    let now = now_unix_seconds();
    HypertableStatus {
        phase: "Failed".to_string(),
        table: resource_name.to_string(),
        sql_plan_count: plan.sql_plans.len() as u32,
        apply_step_count: apply_plan.steps.len() as u32,
        applied_step_count: 0,
        skipped_step_count: 0,
        observed_generation,
        last_applied_sql_hash: stable_sql_hash(&apply_plan.sql_script()),
        last_applied_unix_seconds: now,
        last_error: Some(message.to_string()),
        conditions: vec![HypertableCondition {
            condition_type: "Ready".to_string(),
            status: "False".to_string(),
            reason: "SqlApplyFailed".to_string(),
            message: message.to_string(),
            last_transition_unix_seconds: now,
        }],
    }
}

fn stable_sql_hash(sql: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in sql.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("fnv1a64:{hash:016x}")
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_sql_hash_changes_with_sql() {
        assert_eq!(stable_sql_hash("SELECT 1;"), stable_sql_hash("SELECT 1;"));
        assert_ne!(stable_sql_hash("SELECT 1;"), stable_sql_hash("SELECT 2;"));
    }
}
