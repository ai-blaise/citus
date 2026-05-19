//! ai-blaise Citus operator core.

pub mod crds;
pub mod reconcile;

pub use crds::hypertable::{
    CompressionPolicy, ContinuousAggregateSpec, HypertableSpec, HypertableSpecError,
    RetentionPolicy,
};
pub use reconcile::hypertable::{HypertableReconcileError, HypertableReconcilePlan};
