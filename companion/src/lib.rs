//! ai-blaise Citus companion extension core.

pub mod advanced_planner;
pub mod auth;
pub mod citus_timescale;
pub mod db_doctor;
pub mod domain_contracts;
pub mod extension_catalog;
pub mod feature_status;
pub mod geo_distributed;
pub mod graph_bridge;
pub mod index_advisor;
pub mod jsonschema_bridge;
pub mod ledger;
pub mod log_view;
pub mod lsp_metadata;
pub mod migration;
pub mod observability;
pub mod ops_contracts;
pub mod plan_freeze;
pub mod queue;
pub mod replication_conflict;
pub mod router_assist;
pub mod runtime_depth_a;
pub mod schema_jobs;
pub mod search_bridge;
pub mod tenants;
pub mod toolkit_distributed;
pub mod trace_context;
pub mod vector;
pub mod webhooks;

pub use advanced_planner::{
    canonical_advanced_planner_contract, canonical_advanced_planner_execution_report,
    AdvancedPlannerContract, AdvancedPlannerError, AdvancedPlannerExecutionReport, PlannerSurface,
    PlannerSurfaceKind,
};
pub use auth::{AuthError, JwtVerificationPlan, SessionClaims, TenantRlsPolicyPlan};
pub use citus_timescale::{
    distribute_hypertable_plan, AddContinuousAggregateDistributedPlan, AddPolicyDistributed,
    AddPolicyDistributedPlan, CompanionError, CompanionSqlPlan, DistributedHypertablePlan,
    TimeRangeShardPrunerPlan, FEATURE_DISTRIBUTE_HYPERTABLE, FEATURE_TIME_RANGE_SHARD_PRUNER,
};
pub use db_doctor::{
    CohabitPreflightPlan, DbDoctorError, DbDoctorPlan, DbDoctorReport, DbDoctorSqlPlan, DoctorRule,
    DoctorSeverity, DoctorViolation,
};
pub use domain_contracts::{
    canonical_domain_contracts_report, DomainContractError, DomainContractExecutionReport,
};
pub use extension_catalog::{
    canonical_extension_catalog_execution_report, v2_extension_contracts,
    validate_extension_contracts, ExtensionCatalogError, ExtensionCatalogExecutionReport,
    ExtensionCatalogSummary, ExtensionContract, ExtensionTier,
};
pub use feature_status::{
    companion_feature_statuses, validate_companion_feature_statuses, FeatureStatus,
    FeatureStatusError, COMPANION_FEATURE_STATUSES,
};
pub use geo_distributed::{
    GeoDistributionPlan, GeoGrid, GeoPruningPlan, GeoSqlPlan, GeoValidationError,
};
pub use graph_bridge::{GraphBridgeError, GraphDistributionPlan, GraphSqlPlan};
pub use index_advisor::{
    IndexAdvisorError, IndexAdvisorPlan, IndexAdvisorSqlPlan, IndexCandidate, IndexMethod,
};
pub use jsonschema_bridge::{
    JsonSchemaDistributedPlan, JsonSchemaError, JsonSchemaSqlPlan, ValidationTiming,
};
pub use ledger::{
    HmacAlgorithm, LedgerChain, LedgerChainEntry, LedgerError, LedgerSealPlan, LedgerSqlPlan,
    LedgerTransferPlan,
};
pub use log_view::{
    canonical_log_view_plans, render_all_views, JsonPath, LogFieldProjection, LogViewError,
    LogViewPlan, DEFAULT_RAW_TABLE, DEFAULT_VIEW_SCHEMA,
};
pub use lsp_metadata::{
    LspMetadataError, LspMetadataSqlPlan, LspMetadataView, LspMetadataViewPlan,
};
pub use migration::{
    canonical_migration_runtime_report, MigrationError, MigrationOperation, MigrationPlan,
    MigrationRuntime, MigrationRuntimeAction, MigrationRuntimeDecision,
    MigrationRuntimeObservation, MigrationRuntimePhase, MigrationRuntimeReport, MigrationSqlPlan,
};
pub use observability::{
    IdleTransactionDetectorPlan, LatencyPercentile, LocalActivityStatPlan, ObservabilityError,
    OperationsGuardrailPlan, QueryPercentileViewPlan, ReplicationLagPlan,
};
pub use ops_contracts::{
    canonical_operations_readiness_contract, canonical_operations_readiness_report,
    OperationsCheck, OperationsContractError, OperationsGate, OperationsReadinessContract,
    OperationsReadinessReport,
};
pub use plan_freeze::{
    PlanFreezeError, PlanFreezePlan, PlanFreezeSqlPlan, PlanPromotionPolicy, PlanRegressionPolicy,
    PlanRegressionSample,
};
pub use queue::{
    canonical_queue_runtime_report, DurableQueueRuntime, QueueAckOutcome, QueueEnqueueOutcome,
    QueueLeaseBatch, QueueMessage, QueueMessageState, QueueRetryOutcome, QueueRuntimeConfig,
    QueueRuntimeError, QueueRuntimeReport, QueueRuntimeSnapshot, QueueSqlPlan,
};
pub use replication_conflict::{
    canonical_conflict_cases, canonical_conflict_policy, canonical_replication_conflict_report,
    ConflictPolicy, ConflictResolution, ConflictResolutionStrategy, ConflictWinner,
    ReplicationConflict, ReplicationConflictClass, ReplicationConflictError,
    ReplicationConflictReport, ReplicationConflictResolver, RowVersion,
};
pub use router_assist::{
    InvalidationHint, LocalPlacementCheck, PlacementGenerationQuery, PlacementGenerationSample,
    PlacementGenerationSubscriber, RouterAssistError, RouterAssistSqlPlan, ShardForValuePlan,
    ShardRoutingStrategy,
};
pub use runtime_depth_a::{
    canonical_companion_runtime_depth_a_report, CompanionRuntimeDepthAError,
    CompanionRuntimeDepthAReport,
};
pub use schema_jobs::{
    controller::{
        PhaseAcknowledgement, PhaseCheckpoint, PhaseTransitionDecision, PhaseTransitionPlan,
        SchemaJobController, SchemaJobControllerError, TransitionGate,
    },
    rollback::{RollbackError, RollbackPlan, RollbackStep},
    verify_two_version_invariant_sql,
    worker_lease::{
        WorkerLease, WorkerLeaseError, WorkerLeaseRegistry, WorkerLeaseSqlPlan, WorkerLeaseStatus,
    },
    SchemaJobError, SchemaJobOperation, SchemaJobPlan, SchemaJobState, COMPANION_INTERNAL_SCHEMA,
    TWO_VERSION_INVARIANT_MAX_VERSIONS,
};
pub use search_bridge::{
    HybridRankPlan, RerankerPlan, SearchBridgeError, SearchColumnPlan, SearchColumnRole,
    SearchIndexDistributedPlan, SearchSqlPlan,
};
pub use tenants::{TenantArchivePlan, TenantMovePlan, TenantOperationError, TenantQuotaPlan};
pub use toolkit_distributed::{
    ToolkitAggregateKind, ToolkitDistributedError, ToolkitDistributedPlan, ToolkitSqlPlan,
};
pub use trace_context::{
    render_projection_sql, CompanionTraceContextError, CompanionTraceContextPlan,
};
pub use vector::{
    ChunkingPlan, EmbeddingPlan, VectorDestinationPlan, VectorProvider, VectorizerDefinition,
    VectorizerPlan, VectorizerSchedule, VectorizerSqlPlan, VectorizerValidationError,
};
pub use webhooks::{
    WebhookError, WebhookEvent, WebhookHeader, WebhookRegistrationPlan, WebhookSqlPlan,
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
        TableIterator::new(
            crate::feature_status::companion_feature_statuses()
                .iter()
                .map(|feature| (feature.feature_id, feature.feature_name, feature.status))
                .collect(),
        )
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
