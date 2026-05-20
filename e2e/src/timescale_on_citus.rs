// FEATURE: TS1
// FEATURE: TS7

use ai_blaise_citus_operator::{
    CitusClusterSpec, CitusClusterSpecError, CitusTopology, CompressionPolicy,
    ContinuousAggregateSpec, HypertableReconcileError, HypertableReconcilePlan, HypertableSpec,
    PlacementPolicy, PoolSpec, RetentionPolicy, ShardGroupSpec, ShardGroupSpecError, SidecarSpec,
    SidecarType, UnsatisfiablePlacementAction,
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
    pub cluster: CitusClusterSpec,
    pub shard_group: ShardGroupSpec,
    pub preload: CohabitationPreloadConfig,
    pub hypertable: HypertableSpec,
}

impl TimescaleOnCitusAcceptance {
    pub fn canonical_metrics() -> Self {
        Self {
            cluster: CitusClusterSpec {
                topology: CitusTopology::CoordinatorWorker,
                image: "ghcr.io/ai-blaise/citus:pg18-v2".to_string(),
                workers: 3,
                coordinators: 1,
                storage_class: None,
                timescale_enabled: true,
                extensions: vec![
                    CITUS_EXTENSION.to_string(),
                    TIMESCALEDB_EXTENSION.to_string(),
                ],
                pool: Some(PoolSpec {
                    replicas: 2,
                    geoip_db: None,
                }),
                sidecars: vec![SidecarSpec {
                    sidecar_type: SidecarType::Vectorizer,
                    replicas: 1,
                }],
            },
            shard_group: ShardGroupSpec {
                parent_table: "public.metrics".to_string(),
                distribution_column: "tenant_id".to_string(),
                num_shards: 32,
                colocation_group: Some("metrics".to_string()),
                replication_factor: 3,
                placement_policy: vec![PlacementPolicy {
                    topology_key: "topology.kubernetes.io/zone".to_string(),
                    max_skew: 1,
                    when_unsatisfiable: UnsatisfiablePlacementAction::DoNotSchedule,
                }],
            },
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
        self.cluster.validate()?;
        self.shard_group.validate()?;
        self.preload.validate()?;
        let reconcile = HypertableReconcilePlan::try_from(&self.hypertable)?;

        Ok(TimescaleOnCitusPlan {
            cluster: self.cluster.clone(),
            shard_group: self.shard_group.clone(),
            preload: self.preload.clone(),
            reconcile,
            gates: vec![
                AcceptanceGate::CitusClusterTopology,
                AcceptanceGate::TopologyAwarePlacement,
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
    pub cluster: CitusClusterSpec,
    pub shard_group: ShardGroupSpec,
    pub preload: CohabitationPreloadConfig,
    pub reconcile: HypertableReconcilePlan,
    pub gates: Vec<AcceptanceGate>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AcceptanceGate {
    CitusClusterTopology,
    TopologyAwarePlacement,
    CohabitPreload,
    DistributedHypertable,
    CompressionPolicy,
    RetentionPolicy,
    ContinuousAggregate,
    TimeRangeShardPruning,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TimescaleOnCitusAcceptanceError {
    Cluster(CitusClusterSpecError),
    ShardGroup(ShardGroupSpecError),
    MissingSharedPreloadLibrary(&'static str),
    MissingCohabitExtension(&'static str),
    Reconcile(HypertableReconcileError),
}

impl fmt::Display for TimescaleOnCitusAcceptanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cluster(error) => write!(formatter, "{error}"),
            Self::ShardGroup(error) => write!(formatter, "{error}"),
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

impl From<CitusClusterSpecError> for TimescaleOnCitusAcceptanceError {
    fn from(error: CitusClusterSpecError) -> Self {
        Self::Cluster(error)
    }
}

impl From<ShardGroupSpecError> for TimescaleOnCitusAcceptanceError {
    fn from(error: ShardGroupSpecError) -> Self {
        Self::ShardGroup(error)
    }
}

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

        assert_eq!(plan.cluster.topology, CitusTopology::CoordinatorWorker);
        assert_eq!(plan.cluster.workers, 3);
        assert_eq!(plan.cluster.coordinators, 1);
        assert_eq!(plan.shard_group.parent_table, "public.metrics");
        assert_eq!(plan.shard_group.replication_factor, 3);
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
        assert_eq!(plan.reconcile.sql_plans.len(), 5);
        assert!(plan
            .reconcile
            .sql_script()
            .contains("create_distributed_table"));
        assert!(plan
            .reconcile
            .sql_script()
            .contains("enable_time_range_shard_pruner"));
        let apply_plan = plan.reconcile.apply_plan();
        assert_eq!(apply_plan.steps[0].name, "ensure_ai_blaise_citus_extension");
        assert_eq!(
            apply_plan.steps[2].name,
            "guard_citus_timescaledb_cohabitation"
        );
        assert!(apply_plan
            .sql_script()
            .contains("CREATE EXTENSION IF NOT EXISTS ai_blaise_citus"));
        assert!(apply_plan
            .sql_script()
            .contains("companion_feature_status()"));
        assert!(apply_plan.sql_script().contains("citus.cohabit_extensions"));
        assert_eq!(
            plan.gates,
            vec![
                AcceptanceGate::CitusClusterTopology,
                AcceptanceGate::TopologyAwarePlacement,
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

    #[test]
    fn plan_rejects_invalid_cluster_topology() {
        let mut acceptance = TimescaleOnCitusAcceptance::canonical_metrics();
        acceptance.cluster.topology = CitusTopology::CoordinatorLess;
        acceptance.cluster.coordinators = 1;

        assert_eq!(
            acceptance.plan(),
            Err(TimescaleOnCitusAcceptanceError::Cluster(
                CitusClusterSpecError::InvalidCoordinatorCount
            ))
        );
    }

    #[test]
    fn plan_rejects_invalid_shard_group_policy() {
        let mut acceptance = TimescaleOnCitusAcceptance::canonical_metrics();
        acceptance.shard_group.placement_policy[0].max_skew = 0;

        assert_eq!(
            acceptance.plan(),
            Err(TimescaleOnCitusAcceptanceError::ShardGroup(
                ShardGroupSpecError::InvalidPlacementSkew
            ))
        );
    }
}
