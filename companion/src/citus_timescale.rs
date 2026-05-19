// FEATURE: TS1
// FEATURE: TS5

use std::error::Error;
use std::fmt;

pub const FEATURE_DISTRIBUTE_HYPERTABLE: &str = "TS1";
pub const FEATURE_TIME_RANGE_SHARD_PRUNER: &str = "TS5";

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
}

pub fn distribute_hypertable_plan(
    table: impl Into<String>,
    distribution_column: impl Into<String>,
    chunk_time_interval: impl Into<String>,
    num_shards: u32,
) -> Result<DistributedHypertablePlan, CompanionError> {
    DistributedHypertablePlan::new(table, distribution_column, chunk_time_interval, num_shards)
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
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CompanionError {
    InvalidShardCount,
    MissingRequiredField(&'static str),
}

impl fmt::Display for CompanionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidShardCount => write!(formatter, "num_shards must be greater than zero"),
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
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
    fn time_range_shard_pruner_requires_table_and_time_column() {
        let error =
            TimeRangeShardPrunerPlan::new("metrics", "").expect_err("empty time column must fail");

        assert_eq!(error, CompanionError::MissingRequiredField("time_column"));
    }
}
