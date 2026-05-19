//! Shard-aware pool primitives.

pub mod runtime;
pub mod shard_map;

pub use runtime::{
    FastPathRouterPolicy, GeoRoutingPolicy, GeoRoutingRule, HtapRoutingPolicy, MirrorTrafficPolicy,
    PoolRuntimeContract, PoolRuntimeError, ProtocolPipelinePolicy, RouteTarget,
    SettingsBucketPolicy, TenantAdmissionPolicy, TlsSessionTicketPolicy,
    TokenIntrospectionCachePolicy,
};
pub use shard_map::{
    CachedPlanGeneration, Placement, PlacementGeneration, ShardMap, ShardMapError,
};
