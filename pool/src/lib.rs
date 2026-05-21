//! Shard-aware pool primitives.

pub mod execution;
pub mod proxy;
pub mod runtime;
pub mod shard_map;

pub use execution::{
    canonical_pool_execution_report, canonical_pool_runtime_contract, PoolExecutionError,
    PoolExecutionReport,
};
pub use proxy::{
    handle_admin_request, handle_proxy_connection, run_pool_service, run_pool_service_from_env,
    AdminRequest, ClientCidrAllowlist, PoolProxyConfig, PoolProxyError, PoolProxyState,
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
