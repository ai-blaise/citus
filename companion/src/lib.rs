//! ai-blaise Citus companion extension core.

pub mod auth;
pub mod citus_timescale;
pub mod geo_distributed;
pub mod graph_bridge;
pub mod jsonschema_bridge;
pub mod lsp_metadata;
pub mod observability;
pub mod router_assist;
pub mod schema_jobs;
pub mod search_bridge;
pub mod tenants;
pub mod toolkit_distributed;
pub mod vector;

pub use auth::{AuthError, JwtVerificationPlan, SessionClaims, TenantRlsPolicyPlan};
pub use citus_timescale::{
    distribute_hypertable_plan, AddContinuousAggregateDistributedPlan, AddPolicyDistributed,
    AddPolicyDistributedPlan, CompanionError, CompanionSqlPlan, DistributedHypertablePlan,
    TimeRangeShardPrunerPlan, FEATURE_DISTRIBUTE_HYPERTABLE, FEATURE_TIME_RANGE_SHARD_PRUNER,
};
pub use geo_distributed::{
    GeoDistributionPlan, GeoGrid, GeoPruningPlan, GeoSqlPlan, GeoValidationError,
};
pub use graph_bridge::{GraphBridgeError, GraphDistributionPlan, GraphSqlPlan};
pub use jsonschema_bridge::{
    JsonSchemaDistributedPlan, JsonSchemaError, JsonSchemaSqlPlan, ValidationTiming,
};
pub use lsp_metadata::{
    LspMetadataError, LspMetadataSqlPlan, LspMetadataView, LspMetadataViewPlan,
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
pub use search_bridge::{
    HybridRankPlan, RerankerPlan, SearchBridgeError, SearchColumnPlan, SearchColumnRole,
    SearchIndexDistributedPlan, SearchSqlPlan,
};
pub use tenants::{TenantArchivePlan, TenantMovePlan, TenantOperationError, TenantQuotaPlan};
pub use toolkit_distributed::{
    ToolkitAggregateKind, ToolkitDistributedError, ToolkitDistributedPlan, ToolkitSqlPlan,
};
pub use vector::{
    ChunkingPlan, EmbeddingPlan, VectorDestinationPlan, VectorProvider, VectorizerDefinition,
    VectorizerPlan, VectorizerSchedule, VectorizerSqlPlan, VectorizerValidationError,
};

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
            ("TS1", "distributed hypertable bridge", "sql-plan"),
            ("TS2", "distributed compression policy", "sql-plan"),
            ("TS3", "distributed continuous aggregates", "sql-plan"),
            ("TS4", "distributed retention policy", "sql-plan"),
            ("TS5", "time-range shard pruner", "sql-plan"),
            ("TS8", "LSP hypertable invariants", "sql-plan"),
            ("TS13", "distributed time_bucket_gapfill", "sql-plan"),
            ("TS14", "distributed metric toolkit aggregates", "sql-plan"),
            (
                "TS15",
                "distributed approximate toolkit aggregates",
                "sql-plan",
            ),
            (
                "TS16",
                "distributed downsampler toolkit aggregates",
                "sql-plan",
            ),
            ("TS17", "distributed state toolkit aggregates", "sql-plan"),
            ("T8", "toolkit two-step aggregate pushdown", "sql-plan"),
            ("L9", "worker partial aggregate pushdown", "sql-plan"),
            ("Search3", "hybrid BM25 and vector ranking", "sql-plan"),
            ("Search9", "reranker UDF plan", "sql-plan"),
            ("G2", "distributed graph bridge", "sql-plan"),
            ("G3", "graph colocation policy", "sql-plan"),
            ("JS2", "distributed JSON Schema validation", "sql-plan"),
            ("M13", "JSON Schema validation triggers", "sql-plan"),
            ("Geo2", "geo-aware distribution", "sql-plan"),
            ("Geo3", "geo shard pruning", "sql-plan"),
            ("A1", "pgai-compatible vectorizer DSL", "planned"),
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
            ("D4", "citus-lsp metadata views", "sql-plan"),
            ("M5", "LSP migration quick-fix metadata", "sql-plan"),
        ])
    }

    #[pg_extern]
    fn distribute_hypertable(
        table: &str,
        dist_col: &str,
        chunk_time_interval: &str,
        num_shards: i32,
    ) -> String {
        let Ok(num_shards) = u32::try_from(num_shards) else {
            pgrx::error!("num_shards must be greater than zero");
        };
        if num_shards == 0 {
            pgrx::error!("num_shards must be greater than zero");
        }

        crate::citus_timescale::DistributedHypertablePlan::new(
            table,
            dist_col,
            chunk_time_interval,
            num_shards,
        )
        .and_then(|plan| plan.to_sql_plan())
        .map(|plan| plan.script())
        .unwrap_or_else(|error| pgrx::error!("{error}"))
    }

    #[pg_extern]
    fn add_compression_policy_distributed(
        table: &str,
        older_than: &str,
        segment_by: &str,
        order_by: &str,
    ) -> String {
        crate::citus_timescale::AddPolicyDistributedPlan::new(
            table,
            crate::citus_timescale::AddPolicyDistributed::Compression {
                older_than: older_than.to_string(),
                segment_by: crate::citus_timescale::parse_identifier_list(segment_by),
                order_by: crate::citus_timescale::parse_identifier_list(order_by),
            },
        )
        .and_then(|plan| plan.to_sql_plan())
        .map(|plan| plan.script())
        .unwrap_or_else(|error| pgrx::error!("{error}"))
    }

    #[pg_extern]
    fn add_retention_policy_distributed(table: &str, drop_after: &str) -> String {
        crate::citus_timescale::AddPolicyDistributedPlan::new(
            table,
            crate::citus_timescale::AddPolicyDistributed::Retention {
                drop_after: drop_after.to_string(),
            },
        )
        .and_then(|plan| plan.to_sql_plan())
        .map(|plan| plan.script())
        .unwrap_or_else(|error| pgrx::error!("{error}"))
    }

    #[pg_extern]
    fn add_reorder_policy_distributed(table: &str, index_name: &str) -> String {
        crate::citus_timescale::AddPolicyDistributedPlan::new(
            table,
            crate::citus_timescale::AddPolicyDistributed::Reorder {
                index_name: index_name.to_string(),
            },
        )
        .and_then(|plan| plan.to_sql_plan())
        .map(|plan| plan.script())
        .unwrap_or_else(|error| pgrx::error!("{error}"))
    }

    #[pg_extern]
    fn add_continuous_aggregate_distributed(
        name: &str,
        query: &str,
        refresh_start: &str,
        refresh_end: &str,
        schedule: &str,
    ) -> String {
        crate::citus_timescale::AddContinuousAggregateDistributedPlan::new(name, query)
            .and_then(|plan| plan.with_refresh_policy(refresh_start, refresh_end, schedule))
            .and_then(|plan| plan.to_sql_plan())
            .map(|plan| plan.script())
            .unwrap_or_else(|error| pgrx::error!("{error}"))
    }

    #[pg_extern]
    fn time_range_shard_pruner(distributed_table: &str, time_column: &str) -> String {
        crate::citus_timescale::TimeRangeShardPrunerPlan::new(distributed_table, time_column)
            .and_then(|plan| plan.to_sql_plan())
            .map(|plan| plan.script())
            .unwrap_or_else(|error| pgrx::error!("{error}"))
    }
}
