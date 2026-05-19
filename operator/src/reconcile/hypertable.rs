// FEATURE: TS7

use ai_blaise_citus_companion::{
    distribute_hypertable_plan, AddContinuousAggregateDistributedPlan, AddPolicyDistributed,
    AddPolicyDistributedPlan, CompanionError, DistributedHypertablePlan,
};
use std::error::Error;
use std::fmt;

use crate::crds::hypertable::{
    CompressionPolicy, ContinuousAggregateSpec, HypertableSpec, HypertableSpecError,
    RetentionPolicy,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HypertableReconcilePlan {
    pub distributed_hypertable: DistributedHypertablePlan,
    pub policies: Vec<AddPolicyDistributedPlan>,
    pub continuous_aggregates: Vec<AddContinuousAggregateDistributedPlan>,
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

        Ok(Self {
            distributed_hypertable,
            policies,
            continuous_aggregates,
        })
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
        assert_eq!(
            plan.continuous_aggregates[0].schedule.as_deref(),
            Some("15 minutes")
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
