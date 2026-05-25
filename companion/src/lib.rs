//! ai-blaise Citus companion extension core.

pub mod advanced_planner;
pub mod auth;
pub mod bulk_distsql;
pub mod citus_timescale;
pub mod clone_node;
pub mod columnar_tiering;
pub mod cross_tier_query;
pub mod db_doctor;
pub mod domain_contracts;
pub mod extension_catalog;
pub mod fdw_rotation;
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
pub mod plan_runtime;
pub mod queue;
pub mod regional_placement;
pub mod regional_row_placement;
pub mod replication_conflict;
pub mod router_assist;
pub mod runtime_depth_a;
pub mod schema_drift;
pub mod schema_jobs;
pub mod search_bridge;
pub mod shard_split;
pub mod shard_temperature;
pub mod tenants;
pub mod timescale_advanced;
pub mod toolkit_distributed;
pub mod trace_context;
pub mod transaction_state;
pub mod txn_coord;
pub mod vector;
pub mod webhooks;

pub use advanced_planner::{
    canonical_advanced_planner_contract, canonical_advanced_planner_execution_report,
    canonical_advanced_planner_fail_closed_checks, canonical_advanced_planner_runtime_report,
    AdvancedPlannerContract, AdvancedPlannerError, AdvancedPlannerExecutionReport,
    AdvancedPlannerRuntimeReport, AdvancedPlannerRuntimeScenario, PlannerExecutionBoundary,
    PlannerSurface, PlannerSurfaceKind,
};
pub use auth::{AuthError, JwtVerificationPlan, SessionClaims, TenantRlsPolicyPlan};
pub use bulk_distsql::{
    canonical_bulk_distsql_report, canonical_bulk_distsql_sql_plan, BulkDistSqlError,
    BulkDistSqlPlan, BulkDistSqlReport, BulkDistSqlSqlPlan,
};
pub use citus_timescale::{
    distribute_hypertable_plan, enable_timescale_bridge_if_cohabiting,
    AddContinuousAggregateDistributedPlan, AddPolicyDistributed, AddPolicyDistributedPlan,
    CompanionError, CompanionSqlPlan, DistributedHypertablePlan, TimeRangeShardPrunerPlan,
    FEATURE_COHABIT_DETECTION, FEATURE_DISTRIBUTE_HYPERTABLE, FEATURE_TIME_RANGE_SHARD_PRUNER,
};
pub use clone_node::{
    canonical_clone_node_fail_closed_checks, canonical_clone_node_plan,
    canonical_clone_node_report, canonical_clone_node_sql_plan, CloneNodeError, CloneNodePlan,
    CloneNodeReport, CloneNodeSqlPlan,
};
pub use columnar_tiering::{
    canonical_columnar_tiering_fail_closed_checks, canonical_columnar_tiering_plan,
    canonical_columnar_tiering_report, canonical_columnar_tiering_sql_plan, ColumnarTieringError,
    ColumnarTieringPlan, ColumnarTieringReport, ColumnarTieringSqlPlan,
};
pub use cross_tier_query::{
    canonical_cross_tier_query_fail_closed_checks, canonical_cross_tier_query_plan,
    canonical_cross_tier_query_report, canonical_cross_tier_query_sql_plan, CrossTierQueryError,
    CrossTierQueryPlan, CrossTierQueryReport, CrossTierQuerySqlPlan,
};
pub use db_doctor::{
    CohabitPreflightPlan, DbDoctorError, DbDoctorPlan, DbDoctorReport, DbDoctorSqlPlan, DoctorRule,
    DoctorSeverity, DoctorViolation,
};
pub use domain_contracts::{
    canonical_domain_contracts_report, DomainContractError, DomainContractExecutionReport,
};
pub use extension_catalog::{
    canonical_cohabit_detection_report, canonical_extension_catalog_execution_report,
    cohabit_extension_specs, detect_cohabit_extensions, v2_extension_contracts,
    validate_extension_contracts, CohabitExtensionDetectionReport, CohabitExtensionObservation,
    CohabitExtensionRole, CohabitExtensionSpec, ExtensionCatalogError,
    ExtensionCatalogExecutionReport, ExtensionCatalogSummary, ExtensionContract, ExtensionTier,
};
pub use fdw_rotation::{
    canonical_fdw_credential_rotation_plan, canonical_fdw_credential_rotation_report,
    canonical_fdw_credential_rotation_sql_plan, FdwCredentialRotationPlan,
    FdwCredentialRotationReport, FdwCredentialRotationSqlPlan, FdwRotationError, FdwUserMapping,
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
    assert_migration_data_invariants_sql, canonical_migration_runtime_report,
    MigrationDataInvariant, MigrationError, MigrationOperation, MigrationPlan, MigrationRuntime,
    MigrationRuntimeAction, MigrationRuntimeDecision, MigrationRuntimeObservation,
    MigrationRuntimePhase, MigrationRuntimeReport, MigrationSqlPlan,
};
pub use observability::{
    IdleTransactionDetectorPlan, LatencyPercentile, LocalActivityStatPlan, ObservabilityError,
    OperationsGuardrailPlan, QueryPercentileViewPlan, ReplicationLagPlan,
};
pub use ops_contracts::{
    canonical_operations_readiness_contract, canonical_operations_readiness_report,
    canonical_release_hardening_report, OperationsCheck, OperationsContractError, OperationsGate,
    OperationsReadinessContract, OperationsReadinessReport, ReleaseHardeningReport,
    RELEASE_HARDENING_REQUIRED_GATES, RELEASE_RECORD_REQUIRED_FIELDS,
};
pub use plan_freeze::{
    PlanFreezeError, PlanFreezePlan, PlanFreezeSqlPlan, PlanPromotionPolicy, PlanRegressionPolicy,
    PlanRegressionSample,
};
pub use plan_runtime::{
    canonical_plan_runtime_report, canonical_plan_runtime_sql_plan, PlanRuntime,
    PlanRuntimeAuditEvent, PlanRuntimeCommand, PlanRuntimeConfig, PlanRuntimeError,
    PlanRuntimeOutcome, PlanRuntimeRecord, PlanRuntimeReport, PlanRuntimeRequest,
    PlanRuntimeSqlPlan,
};
pub use queue::{
    canonical_queue_runtime_report, DurableQueueRuntime, QueueAckOutcome, QueueEnqueueOutcome,
    QueueLeaseBatch, QueueMessage, QueueMessageState, QueueRetryOutcome, QueueRuntimeConfig,
    QueueRuntimeError, QueueRuntimeReport, QueueRuntimeSnapshot, QueueSqlPlan,
};
pub use regional_placement::{
    canonical_regional_placement_fail_closed_checks, canonical_regional_placement_plan,
    canonical_regional_placement_report, canonical_regional_placement_sql_plan,
    LocalityPrefixedPrimaryKey, RegionTablespaceMapping, RegionalPlacementError,
    RegionalPlacementPlan, RegionalPlacementReport, RegionalPlacementSqlPlan,
};
pub use regional_row_placement::{
    canonical_regional_row_placement_fail_closed_checks, canonical_regional_row_placement_plan,
    canonical_regional_row_placement_report, canonical_regional_row_placement_sql_plan,
    RegionalRowPlacementError, RegionalRowPlacementKey, RegionalRowPlacementPlan,
    RegionalRowPlacementReport, RegionalRowPlacementSqlPlan,
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
pub use schema_drift::{
    canonical_schema_drift_plan, canonical_schema_drift_report, canonical_schema_drift_sql_plan,
    ExpectedSchemaColumn, SchemaDriftError, SchemaDriftKind, SchemaDriftPlan, SchemaDriftReport,
    SchemaDriftSqlPlan,
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
pub use shard_split::{
    canonical_shard_split_fail_closed_checks, canonical_shard_split_plan,
    canonical_shard_split_report, canonical_shard_split_sql_plan, ShardSplitError, ShardSplitPlan,
    ShardSplitReport, ShardSplitSqlPlan,
};
pub use shard_temperature::{
    canonical_shard_temperature_fail_closed_checks, canonical_shard_temperature_ranking_plan,
    canonical_shard_temperature_ranking_report, canonical_shard_temperature_sql_plan,
    ShardTemperatureError, ShardTemperatureRankingPlan, ShardTemperatureRankingReport,
    ShardTemperatureSqlPlan,
};
pub use tenants::{
    TenantArchivePlan, TenantMovePlan, TenantOperationError, TenantQuotaPlan,
    TenantRegionAffinityPlan, TenantSqlPlan,
};
pub use timescale_advanced::{
    canonical_timescale_advanced_report, canonical_timescale_advanced_sql_plan,
    TimescaleAdvancedError, TimescaleAdvancedPlan, TimescaleAdvancedReport,
    TimescaleAdvancedSqlPlan,
};
pub use toolkit_distributed::{
    ToolkitAggregateKind, ToolkitDistributedError, ToolkitDistributedPlan, ToolkitSqlPlan,
};
pub use trace_context::{
    render_projection_sql, CompanionTraceContextError, CompanionTraceContextPlan,
};
pub use transaction_state::{
    canonical_transaction_state_fail_closed_checks, canonical_transaction_state_plan,
    canonical_transaction_state_report, canonical_transaction_state_sql_plan,
    DistributedTransactionStatePlan, DistributedTransactionStateReport,
    DistributedTransactionStateSqlPlan, TransactionStateError,
};
pub use txn_coord::{
    canonical_txn_coord_request, canonical_txn_coord_routing_plan, TxnCoordDecision, TxnCoordError,
    TxnCoordRoutingPlan, TxnCoordSqlPlan, TxnFinalizeRequest, TxnStageIntent, TxnStageRequest,
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
