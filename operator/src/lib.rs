//! ai-blaise Citus operator core.

pub mod crds;
pub mod reconcile;

pub use crds::citus_cluster::{
    CitusClusterSpec, CitusClusterSpecError, CitusTopology, PoolSpec, SidecarSpec, SidecarType,
};
pub use crds::hypertable::{
    CompressionPolicy, ContinuousAggregateSpec, HypertableSpec, HypertableSpecError,
    RetentionPolicy,
};
pub use crds::shard_group::{
    PlacementPolicy, ShardGroupSpec, ShardGroupSpecError, UnsatisfiablePlacementAction,
};
pub use reconcile::hypertable::{HypertableReconcileError, HypertableReconcilePlan};
