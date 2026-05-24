//! Shard-aware pool primitives.

pub mod admin;
pub mod admission;
pub mod auth_cache;
pub mod auth_introspection;
pub mod execution;
pub mod geoip;
pub mod htap;
pub mod mirror;
pub mod pipeline;
pub mod placement_subscriber;
pub mod prepared;
pub mod proxy;
pub mod realtime_hook;
pub mod runtime;
pub mod settings_bucket;
pub mod shard_map;
pub mod tenant_quota;
pub mod tls;
pub mod trace_tap;
pub mod virtual_pid;

pub use admin::{AdminAck, AdminCommand, AdminError, AdminState};
pub use auth_cache::{AuthCacheError, AuthVerificationCache, VerifiedClaims};
pub use auth_introspection::{
    token_from_startup, PoolAuthConfig, PoolAuthDecision, PoolAuthError, PoolAuthGate,
    VerifiedPoolClaims,
};
pub use geoip::{
    policy_with_default, route_for_client, ClosestReplicaTable, GeoIpError, RegionReplica,
};
pub use htap::{classify as classify_htap_query, HtapClassifierError, HtapDecision, QueryFeatures};
pub use mirror::{
    MirrorDecision, MirrorPolicyError, QueryClass, TenantMirrorPolicy, TenantMirrorRule,
};
pub use pipeline::{DescribeTarget, ExtendedFrame, ExtendedPipelineBuffer, PipelineError};
pub use placement_subscriber::{PlacementDelta, PlacementSubscriber, PlacementSubscriberError};
pub use prepared::{PreparedCacheError, PreparedStatement, PreparedStatementCache};
pub use realtime_hook::{
    CdcEvent, CdcOperation, RealtimeHookConfig, RealtimeHookError, RealtimeHookQueue,
};
pub use settings_bucket::{SettingsBucketEntry, SettingsBucketError, SettingsBucketPoolMap};
pub use tenant_quota::{TenantAdmission, TenantQuotaError, TenantQuotaState, TenantQuotaTable};
pub use tls::{ring_from_policy, TicketKey, TicketKeyRing, TlsTicketError, TICKET_KEY_LEN};
pub use virtual_pid::{
    encode_cancel_request, parse_cancel_request, RealBackend, VirtualPidError, VirtualPidTable,
    PGWIRE_CANCEL_MAGIC,
};

pub use admission::{
    tenant_id_from_startup, PoolAdmissionConfig, PoolAdmissionController, PoolAdmissionError,
    PoolConnectionPermit, TenantQuotaAdmission, TenantQuotaBucket, TenantQuotaConfig,
    TenantQuotaSnapshot, DEFAULT_STARTUP_TIMEOUT,
};
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
pub use trace_tap::{
    render_tap_log, tap_startup_message, StartupTraceTap, STARTUP_TAP_MIN_TIMEOUT,
};
