// FEATURE: TS1
// FEATURE: TS7

use ai_blaise_citus_operator::{
    CompressionPolicy, ContinuousAggregateSpec, HypertableReconcileError, HypertableReconcilePlan,
    HypertableSpec, RetentionPolicy,
};
use std::error::Error;
use std::fmt;

pub const CITUS_EXTENSION: &str = "citus";
pub const TIMESCALEDB_EXTENSION: &str = "timescaledb";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CohabitationPreloadConfig {
    pub shared_preload_libraries: Vec<String>,
    pub cohabit_extensions: Vec<String>,
}

impl CohabitationPreloadConfig {
    pub fn validate(&self) -> Result<(), TimescaleOnCitusAcceptanceError> {
        require_list_member(
            &self.shared_preload_libraries,
            CITUS_EXTENSION,
            TimescaleOnCitusAcceptanceError::MissingSharedPreloadLibrary(CITUS_EXTENSION),
        )?;
        require_list_member(
            &self.shared_preload_libraries,
            TIMESCALEDB_EXTENSION,
            TimescaleOnCitusAcceptanceError::MissingSharedPreloadLibrary(TIMESCALEDB_EXTENSION),
        )?;
        require_list_member(
            &self.cohabit_extensions,
            TIMESCALEDB_EXTENSION,
            TimescaleOnCitusAcceptanceError::MissingCohabitExtension(TIMESCALEDB_EXTENSION),
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TimescaleOnCitusAcceptance {
    pub preload: CohabitationPreloadConfig,
    pub hypertable: HypertableSpec,
}

impl TimescaleOnCitusAcceptance {
    pub fn canonical_metrics() -> Self {
        Self {
            preload: CohabitationPreloadConfig {
                shared_preload_libraries: vec![
                    CITUS_EXTENSION.to_string(),
                    TIMESCALEDB_EXTENSION.to_string(),
                ],
                cohabit_extensions: vec![TIMESCALEDB_EXTENSION.to_string()],
            },
            hypertable: HypertableSpec {
                table: "public.metrics".to_string(),
                time_column: "ts".to_string(),
                distribution_column: "tenant_id".to_string(),
                chunk_time_interval: "1 day".to_string(),
                num_shards: 32,
                compression: Some(CompressionPolicy {
                    older_than: "7 days".to_string(),
                    segment_by: vec!["tenant_id".to_string()],
                    order_by: vec!["ts DESC".to_string()],
                    bloom_filters: vec!["region".to_string()],
                }),
                retention: Some(RetentionPolicy {
                    drop_after: "90 days".to_string(),
                }),
                continuous_aggregates: vec![ContinuousAggregateSpec {
                    name: "metrics_hourly".to_string(),
                    query: concat!(
                        "SELECT tenant_id, time_bucket('1 hour', ts), count(*) ",
                        "FROM public.metrics GROUP BY 1, 2"
                    )
                    .to_string(),
                    refresh_start: Some("7 days".to_string()),
                    refresh_end: Some("1 hour".to_string()),
                    schedule: Some("15 minutes".to_string()),
                    hierarchical_parent: None,
                }],
            },
        }
    }

    pub fn plan(&self) -> Result<TimescaleOnCitusPlan, TimescaleOnCitusAcceptanceError> {
        self.preload.validate()?;
        let reconcile = HypertableReconcilePlan::try_from(&self.hypertable)?;

        Ok(TimescaleOnCitusPlan {
            preload: self.preload.clone(),
            reconcile,
            gates: vec![
                AcceptanceGate::CohabitPreload,
                AcceptanceGate::DistributedHypertable,
                AcceptanceGate::CompressionPolicy,
                AcceptanceGate::RetentionPolicy,
                AcceptanceGate::ContinuousAggregate,
                AcceptanceGate::TimeRangeShardPruning,
            ],
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TimescaleOnCitusPlan {
    pub preload: CohabitationPreloadConfig,
    pub reconcile: HypertableReconcilePlan,
    pub gates: Vec<AcceptanceGate>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AcceptanceGate {
    CohabitPreload,
    DistributedHypertable,
    CompressionPolicy,
    RetentionPolicy,
    ContinuousAggregate,
    TimeRangeShardPruning,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TimescaleOnCitusAcceptanceError {
    MissingSharedPreloadLibrary(&'static str),
    MissingCohabitExtension(&'static str),
    Reconcile(HypertableReconcileError),
}

impl fmt::Display for TimescaleOnCitusAcceptanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSharedPreloadLibrary(library) => {
                write!(formatter, "shared_preload_libraries must include {library}")
            }
            Self::MissingCohabitExtension(extension) => {
                write!(
                    formatter,
                    "citus.cohabit_extensions must include {extension}"
                )
            }
            Self::Reconcile(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for TimescaleOnCitusAcceptanceError {}

impl From<HypertableReconcileError> for TimescaleOnCitusAcceptanceError {
    fn from(error: HypertableReconcileError) -> Self {
        Self::Reconcile(error)
    }
}

fn require_list_member(
    values: &[String],
    expected: &'static str,
    error: TimescaleOnCitusAcceptanceError,
) -> Result<(), TimescaleOnCitusAcceptanceError> {
    if values
        .iter()
        .any(|value| value.trim().eq_ignore_ascii_case(expected))
    {
        return Ok(());
    }
    Err(error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_metrics_plan_covers_timescale_on_citus_path() {
        let plan = TimescaleOnCitusAcceptance::canonical_metrics()
            .plan()
            .expect("canonical acceptance plan");

        assert_eq!(
            plan.preload.shared_preload_libraries,
            vec![
                CITUS_EXTENSION.to_string(),
                TIMESCALEDB_EXTENSION.to_string()
            ]
        );
        assert_eq!(
            plan.preload.cohabit_extensions,
            vec![TIMESCALEDB_EXTENSION.to_string()]
        );
        assert_eq!(
            plan.reconcile.distributed_hypertable.table,
            "public.metrics"
        );
        assert_eq!(
            plan.reconcile.distributed_hypertable.distribution_column,
            "tenant_id"
        );
        assert_eq!(plan.reconcile.policies.len(), 2);
        assert_eq!(plan.reconcile.continuous_aggregates.len(), 1);
        assert_eq!(
            plan.gates,
            vec![
                AcceptanceGate::CohabitPreload,
                AcceptanceGate::DistributedHypertable,
                AcceptanceGate::CompressionPolicy,
                AcceptanceGate::RetentionPolicy,
                AcceptanceGate::ContinuousAggregate,
                AcceptanceGate::TimeRangeShardPruning,
            ]
        );
    }

    #[test]
    fn plan_rejects_missing_timescaledb_cohabit_entry() {
        let mut acceptance = TimescaleOnCitusAcceptance::canonical_metrics();
        acceptance.preload.cohabit_extensions.clear();

        assert_eq!(
            acceptance.plan(),
            Err(TimescaleOnCitusAcceptanceError::MissingCohabitExtension(
                TIMESCALEDB_EXTENSION
            ))
        );
    }
}
