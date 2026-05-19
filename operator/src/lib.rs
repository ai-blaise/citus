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
pub use crds::hypertable::{
    CompressionPolicy, ContinuousAggregateSpec, HypertableSpec, HypertableSpecError,
    RetentionPolicy,
};
pub use crds::region::{RegionSpec, RegionSpecError};
pub use crds::shard_group::{
    PlacementPolicy, ShardGroupSpec, ShardGroupSpecError, UnsatisfiablePlacementAction,
};
pub use crds::survival_goal::{SurvivalGoalSpec, SurvivalGoalSpecError, SurvivalGoalType};
pub use crds::tenant::{TenantQuotas, TenantSpec, TenantSpecError};
pub use reconcile::hypertable::{HypertableReconcileError, HypertableReconcilePlan};
