//! ai-blaise Citus operator core.

pub mod controllers;
pub mod crds;
pub mod reconcile;

pub use crds::backup::{
    BackupEncryption, BackupProvider, BackupSpec, BackupSpecError, BackupTarget,
};
pub use crds::branch::{BranchSpec, BranchSpecError, BranchStorageSpec, BranchType};
pub use crds::citus_cluster::{
    CitusClusterSpec, CitusClusterSpecError, CitusTopology, PoolSpec, SidecarSpec, SidecarType,
};
pub use crds::conflict_policy::{
    ConflictClass, ConflictPolicySpec, ConflictPolicySpecError, ConflictResolution,
};
pub use crds::federation::{
    FederationConnection, FederationSpec, FederationSpecError, FederationType,
};
pub use crds::function::{
    FunctionEvent, FunctionRuntime, FunctionSource, FunctionSpec, FunctionSpecError,
    FunctionTrigger,
};
pub use crds::hypertable::{
    CompressionPolicy, ContinuousAggregateSpec, HypertableSpec, HypertableSpecError,
    RetentionPolicy,
};
pub use crds::migration::{
    MigrationConflictAction, MigrationPhase, MigrationSpec, MigrationSpecError, MigrationType,
    PhaseEvidence, StateMachineError,
};
pub use crds::region::{RegionSpec, RegionSpecError};
pub use crds::scheduled_repack::{RepackStrategy, ScheduledRepackSpec, ScheduledRepackSpecError};
pub use crds::search_index::{
    SearchColumnKind, SearchColumnSpec, SearchIndexSpec, SearchIndexSpecError, SearchScorer,
};
pub use crds::shard_group::{
    PlacementPolicy, ShardGroupSpec, ShardGroupSpecError, UnsatisfiablePlacementAction,
};
pub use crds::sidecar::{
    ResourceRequirements, SidecarDeploymentSpec, SidecarDeploymentSpecError, SidecarDeploymentType,
};
pub use crds::survival_goal::{SurvivalGoalSpec, SurvivalGoalSpecError, SurvivalGoalType};
pub use crds::tenant::{TenantQuotas, TenantSpec, TenantSpecError};
pub use crds::vectorizer::{
    ChunkingSpec, ChunkingStrategy, EmbeddingProvider, VectorDestinationSpec,
    VectorizerScheduleMode, VectorizerSchedulingSpec, VectorizerSpec, VectorizerSpecError,
};
pub use crds::webhook::{WebhookEvent, WebhookRetryPolicy, WebhookSpec, WebhookSpecError};
pub use reconcile::backup::{
    BackupApplyStep, BackupApplyStepKind, BackupDeletePlan, BackupReconcileError,
    BackupReconcilePlan,
};
pub use reconcile::citus_cluster::{
    CitusClusterReconcileError, CitusClusterReconcilePlan, ClusterTopologyPlan, CnpgClusterPlan,
    PoolDeploymentPlan, SidecarDeploymentPlan, CNPG_CLUSTER_NAME_SUFFIX,
    POOL_DEPLOYMENT_NAME_SUFFIX, POSTGRES_SHARED_PRELOAD_LIBRARIES,
};
pub use reconcile::federation::{
    FederationApplyPlan, FederationApplyStep, FederationBackend, FederationReconcileError,
    FederationReconcilePlan, FEDERATION_METADATA_TABLES_SQL, FEDERATION_PROVENANCE_TABLE,
    MONGO_FDW_EXTENSION, MYSQL_FDW_EXTENSION, ORACLE_FDW_EXTENSION, TDS_FDW_EXTENSION,
};
pub use reconcile::function::{
    FunctionApplyPlan, FunctionApplyStep, FunctionEventTriggerPlan, FunctionHttpTriggerPlan,
    FunctionReconcileError, FunctionReconcilePlan, FunctionScheduledTriggerPlan,
    FunctionSidecarRegistration, FunctionStepKind, FUNCTION_METADATA_TABLES_SQL,
    FUNCTION_NOTIFY_CHANNEL_PREFIX, PG_CRON_EXTENSION, SIDECAR_REGISTER_ENDPOINT,
};
pub use reconcile::hypertable::{
    HypertableApplyPlan, HypertableApplyStep, HypertableReconcileError, HypertableReconcilePlan,
};
pub use reconcile::migration::{MigrationCommand, MigrationReconcileError, MigrationReconcilePlan};
pub use reconcile::region::{
    RegionApplyStep, RegionApplyStepKind, RegionReconcileError, RegionReconcilePlan,
};
pub use reconcile::search_index::{
    SearchIndexApplyPlan, SearchIndexApplyStep, SearchIndexReconcileError,
    SearchIndexReconcilePlan, PG_SEARCH_ACCESS_METHOD, SEARCH_METADATA_TABLES_SQL,
};
pub use reconcile::shard_group::{
    ShardGroupApplyPlan, ShardGroupApplyStep, ShardGroupReconcileError, ShardGroupReconcilePlan,
    TopologySpreadConstraintPlan, CITUS_CREATE_DISTRIBUTED_TABLE, CITUS_REPLICATION_FACTOR_GUC,
    CITUS_SHARD_COUNT_GUC, CITUS_UPDATE_COLOCATION,
};
pub use reconcile::survival_goal::{
    SurvivalGoalApplyStep, SurvivalGoalApplyStepKind, SurvivalGoalReconcileError,
    SurvivalGoalReconcilePlan,
};
pub use reconcile::tenant::{
    TenantApplyStep, TenantApplyStepKind, TenantDeletePlan, TenantReconcileError,
    TenantReconcilePlan, TENANT_ARCHIVE_DEFAULT_RETENTION_DAYS,
};
pub use reconcile::webhook::{
    WebhookApplyPlan, WebhookApplyStep, WebhookReconcileError, WebhookReconcilePlan,
    WEBHOOK_METADATA_TABLES_SQL, WEBHOOK_QUEUE,
};
