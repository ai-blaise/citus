//! Shard-aware pool primitives.

pub mod execution;
pub mod runtime;
pub mod shard_map;

pub use execution::{
    canonical_pool_execution_report, canonical_pool_runtime_contract, PoolExecutionError,
    PoolExecutionReport,
};
pub use runtime::{
    FastPathRouterPolicy, GeoRoutingPolicy, GeoRoutingRule, HtapRoutingPolicy, MirrorTrafficPolicy,
    PoolRuntimeContract, PoolRuntimeError, ProtocolPipelinePolicy, RouteDecision, RouteTarget,
    SessionSetting, SettingsBucketPolicy, TenantAdmissionPolicy, TlsSessionTicketPolicy,
    TokenIntrospectionCachePolicy,
};
pub use shard_map::{
    CachedPlan, CachedPlanGeneration, Placement, PlacementGeneration, PlanCache, ShardMap,
    ShardMapError, ShardRoute,
};
