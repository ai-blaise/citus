//! Shard-aware pool primitives.

pub mod shard_map;

pub use shard_map::{
    CachedPlanGeneration, Placement, PlacementGeneration, ShardMap, ShardMapError,
};
