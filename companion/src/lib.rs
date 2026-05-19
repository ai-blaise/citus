//! ai-blaise Citus companion extension core.

pub mod auth;
pub mod citus_timescale;
pub mod observability;
pub mod router_assist;
pub mod schema_jobs;
pub mod tenants;

pub use auth::{AuthError, JwtVerificationPlan, SessionClaims, TenantRlsPolicyPlan};
pub use citus_timescale::{
    distribute_hypertable_plan, AddContinuousAggregateDistributedPlan, AddPolicyDistributed,
    AddPolicyDistributedPlan, CompanionError, DistributedHypertablePlan, TimeRangeShardPrunerPlan,
    FEATURE_DISTRIBUTE_HYPERTABLE, FEATURE_TIME_RANGE_SHARD_PRUNER,
};
pub use observability::{
    DistributedStatPlan, IdleTransactionAction, IdleTransactionReaperPlan, LatencyPercentile,
    ObservabilityError, OperationsGuardrailPlan, QueryPercentileViewPlan, ReplicationLagPlan,
};
pub use router_assist::{
    LocalPlacementCheck, PlacementGenerationQuery, RouterAssistError, ShardForValuePlan,
    ShardRoutingStrategy,
};
pub use schema_jobs::{SchemaJobError, SchemaJobOperation, SchemaJobPlan, SchemaJobState};
pub use tenants::{TenantArchivePlan, TenantMovePlan, TenantOperationError, TenantQuotaPlan};

#[cfg(feature = "pg18")]
mod pg18 {
    use pgrx::prelude::*;

    pgrx::pg_module_magic!();

    #[pg_extern]
    fn companion_feature_status() -> TableIterator<
        'static,
        (
            name!(feature_id, &'static str),
            name!(feature_name, &'static str),
            name!(status, &'static str),
        ),
    > {
        TableIterator::new(vec![
            ("TS1", "distributed hypertable bridge", "planned"),
            ("TS5", "time-range shard pruner", "planned"),
            ("O1", "query percentile views", "planned"),
            ("O2", "distributed stats view", "planned"),
            ("O3", "replication lag view", "planned"),
            ("R4", "idle transaction reaper", "planned"),
            ("Auth2", "tenant-aware claims", "planned"),
            ("Sec1", "RLS helpers", "planned"),
            ("Sec2", "JWT verification UDF", "planned"),
            ("S6", "placement generation helpers", "planned"),
            ("S13", "range routing helpers", "planned"),
            ("C10", "online schema job state machine", "planned"),
            ("M2", "gh-ost-style online DDL", "planned"),
            ("S14", "tenant migration online", "planned"),
            ("TO3", "tenant migration online", "planned"),
            ("TO4", "tenant archive", "planned"),
            ("TO5", "tenant region affinity", "planned"),
        ])
    }
}
