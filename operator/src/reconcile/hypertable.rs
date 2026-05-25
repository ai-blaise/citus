// FEATURE: TS7

use ai_blaise_citus_companion::{
    distribute_hypertable_plan, AddContinuousAggregateDistributedPlan, AddPolicyDistributed,
    AddPolicyDistributedPlan, CompanionError, CompanionSqlPlan, DistributedHypertablePlan,
    TimeRangeShardPrunerPlan,
};
use std::error::Error;
use std::fmt;

use crate::crds::hypertable::{
    CompressionPolicy, ContinuousAggregateSpec, HypertableSpec, HypertableSpecError,
    RetentionPolicy,
};

pub const AI_BLAISE_CITUS_EXTENSION: &str = "ai_blaise_citus";
pub const TIMESCALEDB_EXTENSION: &str = "timescaledb";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HypertableReconcilePlan {
    pub distributed_hypertable: DistributedHypertablePlan,
    pub policies: Vec<AddPolicyDistributedPlan>,
    pub continuous_aggregates: Vec<AddContinuousAggregateDistributedPlan>,
    pub time_range_pruner: TimeRangeShardPrunerPlan,
    pub sql_plans: Vec<CompanionSqlPlan>,
}

impl TryFrom<&HypertableSpec> for HypertableReconcilePlan {
    type Error = HypertableReconcileError;

    fn try_from(spec: &HypertableSpec) -> Result<Self, Self::Error> {
        spec.validate()?;

        let distributed_hypertable = distribute_hypertable_plan(
            spec.table.clone(),
            spec.distribution_column.clone(),
            spec.chunk_time_interval.clone(),
            spec.num_shards,
        )?;

        let mut policies = Vec::new();
        if let Some(compression) = &spec.compression {
            policies.push(compression_policy(&spec.table, compression)?);
        }
        if let Some(retention) = &spec.retention {
            policies.push(retention_policy(&spec.table, retention)?);
        }

        let continuous_aggregates = spec
            .continuous_aggregates
            .iter()
            .map(continuous_aggregate_plan)
            .collect::<Result<Vec<_>, _>>()?;

        let time_range_pruner =
            TimeRangeShardPrunerPlan::new(spec.table.clone(), spec.time_column.clone())?;

        let mut sql_plans = Vec::new();
        sql_plans.push(distributed_hypertable.to_sql_plan()?);
        for policy in &policies {
            sql_plans.push(policy.to_sql_plan()?);
        }
        for continuous_aggregate in &continuous_aggregates {
            sql_plans.push(continuous_aggregate.to_sql_plan()?);
        }
        sql_plans.push(time_range_pruner.to_sql_plan()?);

        Ok(Self {
            distributed_hypertable,
            policies,
            continuous_aggregates,
            time_range_pruner,
            sql_plans,
        })
    }
}

impl HypertableReconcilePlan {
    pub fn sql_script(&self) -> String {
        self.sql_plans
            .iter()
            .map(CompanionSqlPlan::script)
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn apply_plan(&self) -> HypertableApplyPlan {
        let mut steps = vec![
            HypertableApplyStep::new(
                "ensure_ai_blaise_citus_extension",
                "TS7",
                format!("CREATE EXTENSION IF NOT EXISTS {AI_BLAISE_CITUS_EXTENSION};"),
                true,
            ),
            HypertableApplyStep::new(
                "guard_companion_feature_status",
                "TS7",
                companion_status_guard_sql(&self.sql_plans),
                true,
            ),
            HypertableApplyStep::new(
                "guard_citus_timescaledb_cohabitation",
                "TS6",
                cohabitation_guard_sql(),
                true,
            ),
            HypertableApplyStep::new(
                "apply_distributed_hypertable",
                "TS1",
                format!(
                    "SELECT apply_distribute_hypertable({}, {}, {}, {}, {});",
                    sql_literal(&self.distributed_hypertable.table),
                    sql_literal(&self.distributed_hypertable.distribution_column),
                    sql_literal(&self.time_range_pruner.time_column),
                    sql_literal(&self.distributed_hypertable.chunk_time_interval),
                    self.distributed_hypertable.num_shards,
                ),
                false,
            )
            .with_bridge_state_key("TS1", self.distributed_hypertable.table.clone()),
        ];

        for policy in &self.policies {
            match &policy.policy {
                AddPolicyDistributed::Compression {
                    older_than,
                    segment_by,
                    order_by,
                } => steps.push(
                    HypertableApplyStep::new(
                        "apply_compression_policy_distributed",
                        "TS2",
                        format!(
                            "SELECT apply_compression_policy_distributed({}, {}, {}, {});",
                            sql_literal(&policy.table),
                            sql_literal(older_than),
                            sql_literal(&segment_by.join(",")),
                            sql_literal(&order_by.join(",")),
                        ),
                        false,
                    )
                    .with_bridge_state_key("TS2", policy.table.clone()),
                ),
                AddPolicyDistributed::Retention { drop_after } => steps.push(
                    HypertableApplyStep::new(
                        "apply_retention_policy_distributed",
                        "TS4",
                        format!(
                            "SELECT apply_retention_policy_distributed({}, {});",
                            sql_literal(&policy.table),
                            sql_literal(drop_after),
                        ),
                        false,
                    )
                    .with_bridge_state_key("TS4", policy.table.clone()),
                ),
                AddPolicyDistributed::Reorder { index_name } => steps.push(
                    HypertableApplyStep::new(
                        "apply_reorder_policy_distributed",
                        "TS12",
                        format!(
                            "SELECT apply_reorder_policy_distributed({}, {});",
                            sql_literal(&policy.table),
                            sql_literal(index_name),
                        ),
                        false,
                    )
                    .with_bridge_state_key("TS12", policy.table.clone()),
                ),
            }
        }

        for continuous_aggregate in &self.continuous_aggregates {
            let refresh_start = continuous_aggregate
                .refresh_start
                .as_deref()
                .unwrap_or("7 days");
            let refresh_end = continuous_aggregate
                .refresh_end
                .as_deref()
                .unwrap_or("1 hour");
            let schedule = continuous_aggregate.schedule.as_deref().unwrap_or("1 hour");
            steps.push(
                HypertableApplyStep::new(
                    "apply_continuous_aggregate_distributed",
                    "TS3",
                    format!(
                        "SELECT apply_continuous_aggregate_distributed({}, {}, {}, {}, {});",
                        sql_literal(&continuous_aggregate.name),
                        sql_literal(&continuous_aggregate.query),
                        sql_literal(refresh_start),
                        sql_literal(refresh_end),
                        sql_literal(schedule),
                    ),
                    false,
                )
                .with_bridge_state_key("TS3", continuous_aggregate.name.clone()),
            );
        }

        steps.push(
            HypertableApplyStep::new(
                "apply_time_range_shard_pruner",
                "TS5",
                format!(
                    "SELECT apply_time_range_shard_pruner({}, {});",
                    sql_literal(&self.time_range_pruner.distributed_table),
                    sql_literal(&self.time_range_pruner.time_column),
                ),
                false,
            )
            .with_bridge_state_key("TS5", self.time_range_pruner.distributed_table.clone()),
        );

        HypertableApplyPlan { steps }
    }

    pub fn apply_sql_script(&self) -> String {
        self.apply_plan().sql_script()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HypertableApplyPlan {
    pub steps: Vec<HypertableApplyStep>,
}

impl HypertableApplyPlan {
    pub fn sql_script(&self) -> String {
        self.steps
            .iter()
            .map(|step| step.sql.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HypertableApplyBridgeStateKey {
    pub feature_id: String,
    pub object_name: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HypertableApplyStep {
    pub name: String,
    pub feature_id: String,
    pub sql: String,
    pub idempotent: bool,
    pub bridge_state_key: Option<HypertableApplyBridgeStateKey>,
}

impl HypertableApplyStep {
    fn new(
        name: impl Into<String>,
        feature_id: impl Into<String>,
        sql: impl Into<String>,
        idempotent: bool,
    ) -> Self {
        Self {
            name: name.into(),
            feature_id: feature_id.into(),
            sql: sql.into(),
            idempotent,
            bridge_state_key: None,
        }
    }

    fn with_bridge_state_key(
        mut self,
        feature_id: impl Into<String>,
        object_name: impl Into<String>,
    ) -> Self {
        self.bridge_state_key = Some(HypertableApplyBridgeStateKey {
            feature_id: feature_id.into(),
            object_name: object_name.into(),
        });
        self
    }
}

fn compression_policy(
    table: &str,
    compression: &CompressionPolicy,
) -> Result<AddPolicyDistributedPlan, HypertableReconcileError> {
    AddPolicyDistributedPlan::new(
        table,
        AddPolicyDistributed::Compression {
            older_than: compression.older_than.clone(),
            segment_by: compression.segment_by.clone(),
            order_by: compression.order_by.clone(),
        },
    )
    .map_err(HypertableReconcileError::from)
}

fn retention_policy(
    table: &str,
    retention: &RetentionPolicy,
) -> Result<AddPolicyDistributedPlan, HypertableReconcileError> {
    AddPolicyDistributedPlan::new(
        table,
        AddPolicyDistributed::Retention {
            drop_after: retention.drop_after.clone(),
        },
    )
    .map_err(HypertableReconcileError::from)
}

fn continuous_aggregate_plan(
    continuous_aggregate: &ContinuousAggregateSpec,
) -> Result<AddContinuousAggregateDistributedPlan, HypertableReconcileError> {
    let plan = AddContinuousAggregateDistributedPlan::new(
        continuous_aggregate.name.clone(),
        continuous_aggregate.query.clone(),
    )?;

    match (
        &continuous_aggregate.refresh_start,
        &continuous_aggregate.refresh_end,
        &continuous_aggregate.schedule,
    ) {
        (Some(refresh_start), Some(refresh_end), Some(schedule)) => Ok(plan.with_refresh_policy(
            refresh_start.clone(),
            refresh_end.clone(),
            schedule.clone(),
        )?),
        _ => Ok(plan),
    }
}

fn companion_status_guard_sql(sql_plans: &[CompanionSqlPlan]) -> String {
    let feature_values = sql_plans
        .iter()
        .map(|plan| format!("({})", sql_literal(plan.feature_id)))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        r#"DO $ai_blaise_citus$
DECLARE
    missing_feature text;
BEGIN
    IF EXISTS (SELECT 1 FROM companion_feature_status() WHERE status = 'planned') THEN
        RAISE EXCEPTION 'companion_feature_status must not report planned features';
    END IF;

    SELECT required.feature_id INTO missing_feature
    FROM (VALUES {feature_values}) AS required(feature_id)
    WHERE NOT EXISTS (
        SELECT 1
        FROM companion_feature_status() status
        WHERE status.feature_id = required.feature_id
    )
    LIMIT 1;

    IF missing_feature IS NOT NULL THEN
        RAISE EXCEPTION 'companion_feature_status must report %', missing_feature;
    END IF;
END
$ai_blaise_citus$;"#
    )
}

fn cohabitation_guard_sql() -> String {
    format!(
        r#"DO $ai_blaise_citus$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM regexp_split_to_table(
            COALESCE(current_setting('citus.cohabit_extensions', true), ''),
            '\s*,\s*'
        ) AS cohabit_extension(extension_name)
        WHERE lower(cohabit_extension.extension_name) = {timescaledb}
    ) THEN
        RAISE EXCEPTION 'citus.cohabit_extensions must include {TIMESCALEDB_EXTENSION}';
    END IF;
END
$ai_blaise_citus$;"#,
        timescaledb = sql_literal(TIMESCALEDB_EXTENSION)
    )
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum HypertableReconcileError {
    InvalidSpec(HypertableSpecError),
    Companion(CompanionError),
}

impl fmt::Display for HypertableReconcileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSpec(error) => write!(formatter, "{error}"),
            Self::Companion(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for HypertableReconcileError {}

impl From<HypertableSpecError> for HypertableReconcileError {
    fn from(error: HypertableSpecError) -> Self {
        Self::InvalidSpec(error)
    }
}

impl From<CompanionError> for HypertableReconcileError {
    fn from(error: CompanionError) -> Self {
        Self::Companion(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconcile_plan_maps_spec_to_companion_plan() {
        let spec = HypertableSpec {
            table: "metrics".to_string(),
            time_column: "ts".to_string(),
            distribution_column: "tenant_id".to_string(),
            chunk_time_interval: "1 day".to_string(),
            num_shards: 32,
            compression: Some(CompressionPolicy {
                older_than: "7 days".to_string(),
                segment_by: vec!["tenant_id".to_string()],
                order_by: vec!["ts DESC".to_string()],
                bloom_filters: Vec::new(),
            }),
            retention: Some(RetentionPolicy {
                drop_after: "90 days".to_string(),
            }),
            continuous_aggregates: vec![ContinuousAggregateSpec {
                name: "metrics_hourly".to_string(),
                query: "SELECT 1".to_string(),
                refresh_start: Some("7 days".to_string()),
                refresh_end: Some("1 hour".to_string()),
                schedule: Some("15 minutes".to_string()),
                hierarchical_parent: None,
            }],
        };

        let plan = HypertableReconcilePlan::try_from(&spec).expect("valid reconcile plan");

        assert_eq!(plan.distributed_hypertable.table, "metrics");
        assert_eq!(plan.distributed_hypertable.distribution_column, "tenant_id");
        assert_eq!(plan.distributed_hypertable.num_shards, 32);
        assert_eq!(plan.policies.len(), 2);
        assert_eq!(plan.continuous_aggregates.len(), 1);
        assert_eq!(plan.time_range_pruner.time_column, "ts");
        assert_eq!(plan.sql_plans.len(), 5);
        assert!(plan.sql_script().contains("create_distributed_table"));
        assert!(plan.sql_script().contains("enable_time_range_shard_pruner"));
        assert_eq!(
            plan.continuous_aggregates[0].schedule.as_deref(),
            Some("15 minutes")
        );

        let apply_plan = plan.apply_plan();
        assert_eq!(apply_plan.steps.len(), 8);
        assert_eq!(apply_plan.steps[0].name, "ensure_ai_blaise_citus_extension");
        assert_eq!(apply_plan.steps[0].feature_id, "TS7");
        assert!(apply_plan.steps[0].idempotent);
        assert_eq!(apply_plan.steps[1].name, "guard_companion_feature_status");
        assert!(apply_plan.steps[1].idempotent);
        assert_eq!(
            apply_plan.steps[2].name,
            "guard_citus_timescaledb_cohabitation"
        );
        assert_eq!(apply_plan.steps[3].feature_id, "TS1");
        assert_eq!(apply_plan.steps[4].feature_id, "TS2");
        assert_eq!(apply_plan.steps[5].feature_id, "TS4");
        assert_eq!(apply_plan.steps[6].feature_id, "TS3");
        assert_eq!(apply_plan.steps[7].feature_id, "TS5");
        assert!(!apply_plan.steps[3].idempotent);

        let apply_sql = plan.apply_sql_script();
        assert!(apply_sql.starts_with("CREATE EXTENSION IF NOT EXISTS ai_blaise_citus;"));
        assert!(apply_sql.contains("companion_feature_status()"));
        assert!(apply_sql.contains("VALUES ('TS1'), ('TS2'), ('TS4'), ('TS3'), ('TS5')"));
        assert!(apply_sql.contains("citus.cohabit_extensions"));
        assert!(apply_sql.contains("timescaledb"));
        assert!(apply_sql.contains("apply_distribute_hypertable"));
        assert!(apply_sql.contains("apply_compression_policy_distributed"));
        assert!(apply_sql.contains("apply_retention_policy_distributed"));
        assert!(apply_sql.contains("apply_continuous_aggregate_distributed"));
        assert!(apply_sql.contains("apply_time_range_shard_pruner"));
        assert_eq!(
            apply_plan.steps[3]
                .bridge_state_key
                .as_ref()
                .map(|key| (key.feature_id.as_str(), key.object_name.as_str())),
            Some(("TS1", "metrics"))
        );
    }

    #[test]
    fn reconcile_plan_rejects_invalid_spec() {
        let spec = HypertableSpec {
            table: "metrics".to_string(),
            time_column: "ts".to_string(),
            distribution_column: "tenant_id".to_string(),
            chunk_time_interval: "1 day".to_string(),
            num_shards: 0,
            compression: None,
            retention: None,
            continuous_aggregates: Vec::new(),
        };

        assert_eq!(
            HypertableReconcilePlan::try_from(&spec),
            Err(HypertableReconcileError::InvalidSpec(
                HypertableSpecError::InvalidShardCount
            ))
        );
    }
}
