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
pub use reconcile::citus_cluster::{
    CitusClusterReconcileError, CitusClusterReconcilePlan, ClusterTopologyPlan, CnpgClusterPlan,
    PoolDeploymentPlan, SidecarDeploymentPlan, CNPG_CLUSTER_NAME_SUFFIX,
    POOL_DEPLOYMENT_NAME_SUFFIX, POSTGRES_SHARED_PRELOAD_LIBRARIES,
};
pub use reconcile::conflict_policy::{
    ConflictPolicyApplyPlan, ConflictPolicyApplyStep, ConflictPolicyReconcileError,
    ConflictPolicyReconcilePlan, CONFLICT_POLICY_TABLE, CONFLICT_STATUS_TABLE,
};
pub use reconcile::hypertable::{
    HypertableApplyPlan, HypertableApplyStep, HypertableReconcileError, HypertableReconcilePlan,
};
pub use reconcile::migration::{
    MigrationApplyPlan, MigrationApplyStep, MigrationCommand, MigrationReconcileError,
    MigrationReconcilePlan, MigrationTeardownAction, SCHEMA_JOB_ADD_OPERATION_FUNCTION,
    SCHEMA_JOB_ADVANCE_FUNCTION, SCHEMA_JOB_START_FUNCTION, SCHEMA_JOB_STATUS_VIEW,
};
pub use reconcile::scheduled_repack::{
    ScheduledRepackApplyPlan, ScheduledRepackApplyStep, ScheduledRepackReconcileError,
    ScheduledRepackReconcilePlan, PG_CRON_SCHEDULE_FUNCTION, PG_CRON_UNSCHEDULE_FUNCTION,
    REPACK_POLICY_TABLE, REPACK_QUEUE_TABLE,
};
pub use reconcile::shard_group::{
    ShardGroupApplyPlan, ShardGroupApplyStep, ShardGroupReconcileError, ShardGroupReconcilePlan,
    TopologySpreadConstraintPlan, CITUS_CREATE_DISTRIBUTED_TABLE, CITUS_REPLICATION_FACTOR_GUC,
    CITUS_SHARD_COUNT_GUC, CITUS_UPDATE_COLOCATION,
};
pub use reconcile::sidecar::{
    SidecarDeletionPlan, SidecarDeletionStep, SidecarReconcileError, SidecarReconcilePlan,
    SidecarRuntimeProfile, SidecarStatusProbeUrls, SIDECAR_DEFAULT_PORT,
    SIDECAR_DELETION_GRACE_SECONDS, SIDECAR_DEPLOYMENT_NAME_PREFIX,
};
