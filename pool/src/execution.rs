use crate::{
    FastPathRouterPolicy, GeoRoutingPolicy, GeoRoutingRule, HtapRoutingPolicy, MirrorTrafficPolicy,
    Placement, PlanCache, PoolRuntimeContract, PoolRuntimeError, ProtocolPipelinePolicy,
    RouteDecision, RouteTarget, SessionSetting, SettingsBucketPolicy, ShardMap, ShardMapError,
    TenantAdmissionPolicy, TlsSessionTicketPolicy, TokenIntrospectionCachePolicy,
};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PoolExecutionReport {
    pub tracked_gucs: usize,
    pub settings_bucket_max_connections: u32,
    pub fast_path_routes: usize,
    pub mirror_sample_percent: u8,
    pub htap_max_staleness_ms: u64,
    pub pipeline_max_in_flight: u32,
    pub transaction_pipelining: bool,
    pub tls_rotation_seconds: u32,
    pub tenant_burst: u32,
    pub geo_rules: usize,
    pub token_cache_entries: u32,
    pub plan_cache_entries_before_invalidation: usize,
    pub invalidated_plans: usize,
    pub single_shard_generation: u64,
}

impl PoolExecutionReport {
    fn from_contract(contract: &PoolRuntimeContract) -> Result<Self, PoolExecutionError> {
        contract.validate().map_err(PoolExecutionError::Runtime)?;

        let settings = [SessionSetting {
            name: "citus.enable_repartition_joins".to_string(),
            value: "off".to_string(),
        }];
        contract
            .settings_bucket
            .fingerprint(&settings)
            .map_err(PoolExecutionError::Runtime)?;

        let fast_path_target = RouteTarget {
            host: "worker-b".to_string(),
            port: 5433,
        };
        let fast_path_routes = match contract
            .fast_path_router
            .decide(Some(fast_path_target))
            .map_err(PoolExecutionError::Runtime)?
        {
            RouteDecision::FastPath(_) => 1,
            RouteDecision::Fallback(_) => 0,
        };

        let shard_map = ShardMap::from_placements(vec![
            Placement::new(10, 1, "worker-a", 5432, 4)?,
            Placement::new(10, 2, "worker-b", 5433, 7)?,
            Placement::new(20, 3, "worker-c", 5432, 2)?,
        ])?;
        let mut plan_cache = PlanCache::default();
        plan_cache.upsert("select:orders-by-tenant", vec![10], &shard_map)?;
        plan_cache.upsert("select:events-by-tenant", vec![20], &shard_map)?;
        let entries_before = plan_cache.len();
        let invalidated_plans = plan_cache.invalidate_for_shards(&[10]);
        let single_shard_generation = shard_map.single_shard_route(10)?.generation;

        Ok(Self {
            tracked_gucs: contract.settings_bucket.tracked_gucs.len(),
            settings_bucket_max_connections: contract.settings_bucket.max_connections,
            fast_path_routes,
            mirror_sample_percent: contract.mirror.sample_percent,
            htap_max_staleness_ms: contract.htap.max_staleness_ms,
            pipeline_max_in_flight: contract.pipeline.max_in_flight,
            transaction_pipelining: contract.pipeline.transaction_pipelining,
            tls_rotation_seconds: contract.tls.rotation_seconds,
            tenant_burst: contract.tenant_quota.burst,
            geo_rules: contract.geo_router.rules.len(),
            token_cache_entries: contract.token_cache.max_entries,
            plan_cache_entries_before_invalidation: entries_before,
            invalidated_plans,
            single_shard_generation,
        })
    }
}

pub fn canonical_pool_runtime_contract() -> PoolRuntimeContract {
    PoolRuntimeContract {
        settings_bucket: SettingsBucketPolicy {
            bucket_name: "default".to_string(),
            tracked_gucs: vec!["citus.enable_repartition_joins".to_string()],
            max_connections: 1_000,
        },
        fast_path_router: FastPathRouterPolicy {
            enabled: true,
            single_shard_only: true,
            fallback_target: RouteTarget {
                host: "coordinator".to_string(),
                port: 5432,
            },
        },
        mirror: MirrorTrafficPolicy {
            enabled: true,
            target: Some(RouteTarget {
                host: "canary".to_string(),
                port: 5432,
            }),
            sample_percent: 5,
        },
        htap: HtapRoutingPolicy {
            analytical_target: RouteTarget {
                host: "analytical-sidecar".to_string(),
                port: 7432,
            },
            max_staleness_ms: 2_000,
            predicate_hints: vec!["/*+ analytical */".to_string()],
        },
        pipeline: ProtocolPipelinePolicy {
            max_in_flight: 32,
            transaction_pipelining: true,
        },
        tls: TlsSessionTicketPolicy {
            enabled: true,
            rotation_seconds: 3_600,
        },
        tenant_quota: TenantAdmissionPolicy {
            tenant_id: "tenant-a".to_string(),
            burst: 1_000,
            refill_per_second: 100,
        },
        geo_router: GeoRoutingPolicy {
            default_region: "us-east-1".to_string(),
            rules: vec![GeoRoutingRule {
                cidr: "10.0.0.0/8".to_string(),
                region: "us-east-1".to_string(),
            }],
        },
        token_cache: TokenIntrospectionCachePolicy {
            max_entries: 10_000,
            ttl_seconds: 60,
        },
    }
}

pub fn canonical_pool_execution_report() -> Result<PoolExecutionReport, PoolExecutionError> {
    PoolExecutionReport::from_contract(&canonical_pool_runtime_contract())
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PoolExecutionError {
    Runtime(PoolRuntimeError),
    ShardMap(ShardMapError),
}

impl fmt::Display for PoolExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runtime(error) => write!(formatter, "{error}"),
            Self::ShardMap(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for PoolExecutionError {}

impl From<ShardMapError> for PoolExecutionError {
    fn from(error: ShardMapError) -> Self {
        Self::ShardMap(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_pool_execution_report_is_deterministic() {
        let report = canonical_pool_execution_report().expect("canonical pool execution report");

        assert_eq!(
            report,
            PoolExecutionReport {
                tracked_gucs: 1,
                settings_bucket_max_connections: 1_000,
                fast_path_routes: 1,
                mirror_sample_percent: 5,
                htap_max_staleness_ms: 2_000,
                pipeline_max_in_flight: 32,
                transaction_pipelining: true,
                tls_rotation_seconds: 3_600,
                tenant_burst: 1_000,
                geo_rules: 1,
                token_cache_entries: 10_000,
                plan_cache_entries_before_invalidation: 2,
                invalidated_plans: 1,
                single_shard_generation: 7,
            }
        );
    }
}
