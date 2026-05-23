// FEATURE: TS1
// FEATURE: TS2
// FEATURE: TS3
// FEATURE: TS4
// FEATURE: TS5
// FEATURE: TS12
// FEATURE: TS20

use crate::extension_catalog::CohabitExtensionDetectionReport;
use std::error::Error;
use std::fmt;

pub const FEATURE_DISTRIBUTE_HYPERTABLE: &str = "TS1";
pub const FEATURE_TIME_RANGE_SHARD_PRUNER: &str = "TS5";
pub const FEATURE_COHABIT_DETECTION: &str = "TS20";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CompanionSqlPlan {
    pub feature_id: &'static str,
    pub commands: Vec<String>,
}

impl CompanionSqlPlan {
    pub fn new(feature_id: &'static str, commands: Vec<String>) -> Result<Self, CompanionError> {
        if commands.is_empty() || commands.iter().any(|command| command.trim().is_empty()) {
            return Err(CompanionError::MissingRequiredField("commands"));
        }

        Ok(Self {
            feature_id,
            commands,
        })
    }

    pub fn script(&self) -> String {
        self.commands.join("\n")
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DistributedHypertablePlan {
    pub table: String,
    pub distribution_column: String,
    pub chunk_time_interval: String,
    pub num_shards: u32,
}

impl DistributedHypertablePlan {
    pub fn new(
        table: impl Into<String>,
        distribution_column: impl Into<String>,
        chunk_time_interval: impl Into<String>,
        num_shards: u32,
    ) -> Result<Self, CompanionError> {
        let plan = Self {
            table: table.into(),
            distribution_column: distribution_column.into(),
            chunk_time_interval: chunk_time_interval.into(),
            num_shards,
        };
        plan.validate()?;
        Ok(plan)
    }

    pub fn validate(&self) -> Result<(), CompanionError> {
        validate_required("table", &self.table)?;
        validate_required("distribution_column", &self.distribution_column)?;
        validate_required("chunk_time_interval", &self.chunk_time_interval)?;
        if self.num_shards == 0 {
            return Err(CompanionError::InvalidShardCount);
        }
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<CompanionSqlPlan, CompanionError> {
        self.validate()?;
        CompanionSqlPlan::new(
            "TS1",
            vec![
                format!(
                    "SELECT create_distributed_table({}::regclass, {});",
                    sql_literal(&self.table),
                    sql_literal(&self.distribution_column)
                ),
                format!(
                    "SELECT companion_internal.create_worker_hypertables({}, {}, {}, {});",
                    sql_literal(&self.table),
                    sql_literal(&self.distribution_column),
                    sql_literal(&self.chunk_time_interval),
                    self.num_shards
                ),
            ],
        )
    }
}

pub fn distribute_hypertable_plan(
    table: impl Into<String>,
    distribution_column: impl Into<String>,
    chunk_time_interval: impl Into<String>,
    num_shards: u32,
) -> Result<DistributedHypertablePlan, CompanionError> {
    DistributedHypertablePlan::new(table, distribution_column, chunk_time_interval, num_shards)
}

pub fn enable_timescale_bridge_if_cohabiting(
    detection: &CohabitExtensionDetectionReport,
) -> Result<CompanionSqlPlan, CompanionError> {
    if !detection.is_ready("timescaledb") {
        return Err(CompanionError::MissingTrustedCohabitExtension(
            "timescaledb",
        ));
    }

    CompanionSqlPlan::new(
        FEATURE_COHABIT_DETECTION,
        vec![
            "SELECT companion_internal.assert_citus_cohabit_extension_order(ARRAY['timescaledb', 'citus'], ARRAY['timescaledb']);"
                .to_string(),
        ],
    )
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AddPolicyDistributedPlan {
    pub table: String,
    pub policy: AddPolicyDistributed,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AddPolicyDistributed {
    Compression {
        older_than: String,
        segment_by: Vec<String>,
        order_by: Vec<String>,
    },
    Retention {
        drop_after: String,
    },
    Reorder {
        index_name: String,
    },
}

impl AddPolicyDistributedPlan {
    pub fn new(
        table: impl Into<String>,
        policy: AddPolicyDistributed,
    ) -> Result<Self, CompanionError> {
        let plan = Self {
            table: table.into(),
            policy,
        };
        validate_required("table", &plan.table)?;
        plan.policy.validate()?;
        Ok(plan)
    }

    pub fn to_sql_plan(&self) -> Result<CompanionSqlPlan, CompanionError> {
        validate_required("table", &self.table)?;
        self.policy.validate()?;
        self.policy.to_sql_plan(&self.table)
    }
}

impl AddPolicyDistributed {
    fn validate(&self) -> Result<(), CompanionError> {
        match self {
            Self::Compression {
                older_than,
                segment_by,
                order_by,
            } => {
                validate_required("older_than", older_than)?;
                validate_required_list("segment_by", segment_by)?;
                validate_required_list("order_by", order_by)
            }
            Self::Retention { drop_after } => validate_required("drop_after", drop_after),
            Self::Reorder { index_name } => validate_required("index_name", index_name),
        }
    }

    fn feature_id(&self) -> &'static str {
        match self {
            Self::Compression { .. } => "TS2",
            Self::Retention { .. } => "TS4",
            Self::Reorder { .. } => "TS12",
        }
    }

    fn to_sql_plan(&self, table: &str) -> Result<CompanionSqlPlan, CompanionError> {
        let command = match self {
            Self::Compression {
                older_than,
                segment_by,
                order_by,
            } => format!(
                "SELECT companion_internal.add_compression_policy_distributed({}, {}, {}, {});",
                sql_literal(table),
                sql_literal(older_than),
                sql_array_literal(segment_by),
                sql_array_literal(order_by)
            ),
            Self::Retention { drop_after } => format!(
                "SELECT companion_internal.add_retention_policy_distributed({}, {});",
                sql_literal(table),
                sql_literal(drop_after)
            ),
            Self::Reorder { index_name } => format!(
                "SELECT companion_internal.add_reorder_policy_distributed({}, {});",
                sql_literal(table),
                sql_literal(index_name)
            ),
        };

        CompanionSqlPlan::new(self.feature_id(), vec![command])
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AddContinuousAggregateDistributedPlan {
    pub name: String,
    pub query: String,
    pub refresh_start: Option<String>,
    pub refresh_end: Option<String>,
    pub schedule: Option<String>,
}

impl AddContinuousAggregateDistributedPlan {
    pub fn new(name: impl Into<String>, query: impl Into<String>) -> Result<Self, CompanionError> {
        let plan = Self {
            name: name.into(),
            query: query.into(),
            refresh_start: None,
            refresh_end: None,
            schedule: None,
        };
        validate_required("name", &plan.name)?;
        validate_required("query", &plan.query)?;
        Ok(plan)
    }

    pub fn with_refresh_policy(
        mut self,
        refresh_start: impl Into<String>,
        refresh_end: impl Into<String>,
        schedule: impl Into<String>,
    ) -> Result<Self, CompanionError> {
        self.refresh_start = Some(refresh_start.into());
        self.refresh_end = Some(refresh_end.into());
        self.schedule = Some(schedule.into());

        validate_required("refresh_start", self.refresh_start.as_deref().unwrap_or(""))?;
        validate_required("refresh_end", self.refresh_end.as_deref().unwrap_or(""))?;
        validate_required("schedule", self.schedule.as_deref().unwrap_or(""))?;

        Ok(self)
    }

    pub fn to_sql_plan(&self) -> Result<CompanionSqlPlan, CompanionError> {
        validate_required("name", &self.name)?;
        validate_required("query", &self.query)?;

        let mut command = format!(
            "SELECT companion_internal.add_continuous_aggregate_distributed({}, {}",
            sql_literal(&self.name),
            sql_literal(&self.query)
        );

        if let (Some(refresh_start), Some(refresh_end), Some(schedule)) =
            (&self.refresh_start, &self.refresh_end, &self.schedule)
        {
            validate_required("refresh_start", refresh_start)?;
            validate_required("refresh_end", refresh_end)?;
            validate_required("schedule", schedule)?;
            command.push_str(&format!(
                ", refresh_start => {}, refresh_end => {}, schedule => {}",
                sql_literal(refresh_start),
                sql_literal(refresh_end),
                sql_literal(schedule)
            ));
        }

        command.push_str(");");
        CompanionSqlPlan::new("TS3", vec![command])
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TimeRangeShardPrunerPlan {
    pub distributed_table: String,
    pub time_column: String,
}

impl TimeRangeShardPrunerPlan {
    pub fn new(
        distributed_table: impl Into<String>,
        time_column: impl Into<String>,
    ) -> Result<Self, CompanionError> {
        let plan = Self {
            distributed_table: distributed_table.into(),
            time_column: time_column.into(),
        };
        validate_required("distributed_table", &plan.distributed_table)?;
        validate_required("time_column", &plan.time_column)?;
        Ok(plan)
    }

    pub fn to_sql_plan(&self) -> Result<CompanionSqlPlan, CompanionError> {
        validate_required("distributed_table", &self.distributed_table)?;
        validate_required("time_column", &self.time_column)?;
        CompanionSqlPlan::new(
            "TS5",
            vec![format!(
                "SELECT companion_internal.enable_time_range_shard_pruner({}, {});",
                sql_literal(&self.distributed_table),
                sql_literal(&self.time_column)
            )],
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CompanionError {
    InvalidShardCount,
    MissingRequiredField(&'static str),
    MissingTrustedCohabitExtension(&'static str),
}

impl fmt::Display for CompanionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidShardCount => write!(formatter, "num_shards must be greater than zero"),
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
            Self::MissingTrustedCohabitExtension(extension) => write!(
                formatter,
                "trusted cohabit extension {extension} is not ready"
            ),
        }
    }
}

impl Error for CompanionError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), CompanionError> {
    if value.trim().is_empty() {
        return Err(CompanionError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_required_list(field: &'static str, values: &[String]) -> Result<(), CompanionError> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(CompanionError::MissingRequiredField(field));
    }
    Ok(())
}

pub fn parse_identifier_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn sql_array_literal(values: &[String]) -> String {
    let items = values
        .iter()
        .map(|value| sql_literal(value))
        .collect::<Vec<_>>()
        .join(", ");
    format!("ARRAY[{items}]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distribute_hypertable_plan_requires_positive_shards() {
        let error = distribute_hypertable_plan("metrics", "tenant_id", "1 day", 0)
            .expect_err("zero shards must fail");

        assert_eq!(error, CompanionError::InvalidShardCount);
    }

    #[test]
    fn distribute_hypertable_plan_accepts_required_fields() {
        let plan =
            distribute_hypertable_plan("metrics", "tenant_id", "1 day", 32).expect("valid plan");

        assert_eq!(
            plan,
            DistributedHypertablePlan {
                table: "metrics".to_string(),
                distribution_column: "tenant_id".to_string(),
                chunk_time_interval: "1 day".to_string(),
                num_shards: 32,
            }
        );
    }

    #[test]
    fn distribute_hypertable_renders_two_step_bridge_plan() {
        let script = distribute_hypertable_plan("public.metrics", "tenant_id", "1 day", 32)
            .expect("valid plan")
            .to_sql_plan()
            .expect("sql plan")
            .script();

        assert!(script.contains("create_distributed_table"));
        assert!(script.contains("create_worker_hypertables"));
        assert!(script.contains("'public.metrics'::regclass"));
    }

    #[test]
    fn compression_policy_requires_segment_and_order_columns() {
        let error = AddPolicyDistributedPlan::new(
            "metrics",
            AddPolicyDistributed::Compression {
                older_than: "7 days".to_string(),
                segment_by: Vec::new(),
                order_by: vec!["time DESC".to_string()],
            },
        )
        .expect_err("empty segment_by must fail");

        assert_eq!(error, CompanionError::MissingRequiredField("segment_by"));
    }

    #[test]
    fn compression_policy_renders_worker_fanout_command() {
        let plan = AddPolicyDistributedPlan::new(
            "public.metrics",
            AddPolicyDistributed::Compression {
                older_than: "7 days".to_string(),
                segment_by: vec!["tenant_id".to_string()],
                order_by: vec!["ts DESC".to_string()],
            },
        )
        .expect("valid compression policy")
        .to_sql_plan()
        .expect("sql plan");

        assert_eq!(plan.feature_id, "TS2");
        assert!(plan.script().contains("add_compression_policy_distributed"));
        assert!(plan.script().contains("ARRAY['tenant_id']"));
    }

    #[test]
    fn continuous_aggregate_refresh_policy_requires_schedule() {
        let error = AddContinuousAggregateDistributedPlan::new(
            "metrics_hourly",
            "SELECT tenant_id, time_bucket('1 hour', ts), count(*) FROM metrics GROUP BY 1, 2",
        )
        .expect("valid cagg")
        .with_refresh_policy("7 days", "1 hour", "")
        .expect_err("empty schedule must fail");

        assert_eq!(error, CompanionError::MissingRequiredField("schedule"));
    }

    #[test]
    fn continuous_aggregate_renders_refresh_policy() {
        let plan = AddContinuousAggregateDistributedPlan::new(
            "metrics_hourly",
            "SELECT tenant_id, time_bucket('1 hour', ts), count(*) FROM metrics GROUP BY 1, 2",
        )
        .expect("valid cagg")
        .with_refresh_policy("7 days", "1 hour", "15 minutes")
        .expect("refresh policy")
        .to_sql_plan()
        .expect("sql plan");

        assert_eq!(plan.feature_id, "TS3");
        assert!(plan
            .script()
            .contains("add_continuous_aggregate_distributed"));
        assert!(plan.script().contains("refresh_start => '7 days'"));
    }

    #[test]
    fn time_range_shard_pruner_requires_table_and_time_column() {
        let error =
            TimeRangeShardPrunerPlan::new("metrics", "").expect_err("empty time column must fail");

        assert_eq!(error, CompanionError::MissingRequiredField("time_column"));
    }

    #[test]
    fn identifier_list_parser_trims_empty_values() {
        assert_eq!(
            parse_identifier_list("tenant_id, region, ,"),
            vec!["tenant_id".to_string(), "region".to_string()]
        );
    }

    #[test]
    fn timescale_bridge_enablement_requires_ready_timescaledb_detection() {
        let detection = crate::extension_catalog::detect_cohabit_extensions(
            &["timescaledb"],
            &["timescaledb"],
            &["timescaledb"],
        );
        let plan = enable_timescale_bridge_if_cohabiting(&detection).expect("ready detection");

        assert_eq!(plan.feature_id, FEATURE_COHABIT_DETECTION);
        assert!(plan
            .script()
            .contains("assert_citus_cohabit_extension_order"));
    }

    #[test]
    fn timescale_bridge_enablement_fails_closed_without_preload() {
        let detection = crate::extension_catalog::detect_cohabit_extensions(
            &[],
            &["timescaledb"],
            &["timescaledb"],
        );

        assert_eq!(
            enable_timescale_bridge_if_cohabiting(&detection),
            Err(CompanionError::MissingTrustedCohabitExtension(
                "timescaledb"
            ))
        );
    }
}
