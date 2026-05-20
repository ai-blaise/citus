//! ai-blaise Citus operator core.

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
    MigrationConflictAction, MigrationSpec, MigrationSpecError, MigrationType,
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
pub use reconcile::hypertable::{
    HypertableApplyPlan, HypertableApplyStep, HypertableReconcileError, HypertableReconcilePlan,
};
