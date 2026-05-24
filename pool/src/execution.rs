use crate::{
    encode_cancel_request, parse_cancel_request, AdminCommand, AdminState, AuthVerificationCache,
    CdcEvent, CdcOperation, ClosestReplicaTable, ExtendedFrame, ExtendedPipelineBuffer,
    FastPathRouterPolicy, GeoRoutingPolicy, GeoRoutingRule, HtapDecision, HtapRoutingPolicy,
    MirrorDecision, MirrorTrafficPolicy, Placement, PlacementDelta, PlacementSubscriber, PlanCache,
    PoolRuntimeContract, PoolRuntimeError, PreparedStatement, PreparedStatementCache,
    ProtocolPipelinePolicy, QueryClass, QueryFeatures, RealBackend, RealtimeHookConfig,
    RealtimeHookQueue, RegionReplica, RouteDecision, RouteTarget, SessionSetting,
    SettingsBucketPolicy, SettingsBucketPoolMap, ShardMapError, TenantAdmissionPolicy,
    TenantMirrorPolicy, TenantMirrorRule, TenantQuotaTable, TicketKey, TicketKeyRing,
    TlsSessionTicketPolicy, TokenIntrospectionCachePolicy, VirtualPidTable, PGWIRE_CANCEL_MAGIC,
};
use std::error::Error;
use std::fmt;
use std::time::{Duration, SystemTime};

const SAMPLE_TICKET_HEX: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PoolExecutionReport {
    pub tracked_gucs: usize,
    pub settings_bucket_max_connections: u32,
    pub settings_bucket_count: usize,
    pub settings_bucket_assigned: u32,
    pub fast_path_routes: usize,
    pub mirror_sample_percent: u8,
    pub mirrored_canary_routes: usize,
    pub htap_max_staleness_ms: u64,
    pub htap_analytical_routes: usize,
    pub pipeline_max_in_flight: u32,
    pub transaction_pipelining: bool,
    pub pipeline_flushed_batches: u64,
    pub tls_rotation_seconds: u32,
    pub tls_rotation_due: bool,
    pub tls_previous_key_valid: bool,
    pub tenant_burst: u32,
    pub tenant_quota_admitted: usize,
    pub tenant_quota_rejected: usize,
    pub geo_rules: usize,
    pub geo_replica_regions: usize,
    pub token_cache_entries: u32,
    pub token_cache_hits: usize,
    pub revoked_token_rejections: usize,
    pub plan_cache_entries_before_invalidation: usize,
    pub invalidated_plans: usize,
    pub single_shard_generation: u64,
    pub placement_changed_shards: usize,
    pub prepared_cache_entries: usize,
    pub prepared_invalidated: usize,
    pub virtual_pid_entries: usize,
    pub virtual_cancel_rewrites: usize,
    pub realtime_events_enqueued: usize,
    pub realtime_events_drained: usize,
    pub admin_generation: u64,
    pub admin_kills: u64,
}

impl PoolExecutionReport {
    fn from_contract(contract: &PoolRuntimeContract) -> Result<Self, PoolExecutionError> {
        contract.validate().map_err(PoolExecutionError::Runtime)?;

        let settings = [SessionSetting {
            name: "citus.enable_repartition_joins".to_string(),
            value: "off".to_string(),
        }];
        let mut settings_map = SettingsBucketPoolMap::new(contract.settings_bucket.clone())
            .map_err(PoolExecutionError::Runtime)?;
        let settings_entry = settings_map
            .acquire(&settings)
            .map_err(PoolExecutionError::component)?;

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

        let mut placement_subscriber = PlacementSubscriber::new();
        placement_subscriber
            .apply_batch(
                1,
                vec![
                    PlacementDelta::Upsert(Placement::new(10, 1, "worker-a", 5432, 4)?),
                    PlacementDelta::Upsert(Placement::new(10, 2, "worker-b", 5433, 7)?),
                    PlacementDelta::Upsert(Placement::new(20, 3, "worker-c", 5432, 2)?),
                ],
            )
            .map_err(PoolExecutionError::component)?;
        let initial_shard_map = placement_subscriber.snapshot()?;
        let placement_changed_shards = placement_subscriber
            .apply_batch(
                2,
                vec![PlacementDelta::Upsert(Placement::new(
                    10, 2, "worker-b", 5433, 8,
                )?)],
            )
            .map_err(PoolExecutionError::component)?;
        let shard_map = placement_subscriber.snapshot()?;

        let mut plan_cache = PlanCache::default();
        plan_cache.upsert("select:orders-by-tenant", vec![10], &initial_shard_map)?;
        plan_cache.upsert("select:events-by-tenant", vec![20], &initial_shard_map)?;
        let entries_before = plan_cache.len();
        let invalidated_plans = plan_cache.invalidate_for_shards(&placement_changed_shards);
        let single_shard_generation = shard_map.single_shard_route(10)?.generation;

        let mut pipeline = ExtendedPipelineBuffer::new(&contract.pipeline)
            .map_err(PoolExecutionError::component)?;
        pipeline
            .append(ExtendedFrame::Parse {
                statement_name: "orders_by_tenant".to_string(),
                query: "SELECT count(*) FROM orders WHERE tenant_id = $1".to_string(),
            })
            .map_err(PoolExecutionError::component)?;
        pipeline
            .append(ExtendedFrame::Bind {
                portal_name: "orders_portal".to_string(),
                statement_name: "orders_by_tenant".to_string(),
            })
            .map_err(PoolExecutionError::component)?;
        pipeline
            .append(ExtendedFrame::Execute {
                portal_name: "orders_portal".to_string(),
                max_rows: 0,
            })
            .map_err(PoolExecutionError::component)?;
        let flushed = pipeline
            .append(ExtendedFrame::Sync)
            .map_err(PoolExecutionError::component)?
            .ok_or(PoolExecutionError::Invariant(
                "sync frame did not flush pipeline",
            ))?;
        if flushed.len() != 4 {
            return Err(PoolExecutionError::Invariant(
                "canonical pipeline did not flush parse/bind/execute/sync",
            ));
        }

        let mirror_policy = TenantMirrorPolicy {
            base: contract.mirror.clone(),
            rules: vec![TenantMirrorRule {
                tenant_id: "tenant-a".to_string(),
                query_class: QueryClass::Analytical,
                sample_percent: 100,
            }],
        };
        let mirrored_canary_routes = match mirror_policy
            .should_mirror("tenant-a", &QueryClass::Analytical, 42)
            .map_err(PoolExecutionError::component)?
        {
            MirrorDecision::Mirror(_) => 1,
            MirrorDecision::Skip => 0,
        };

        let htap_decision = crate::classify_htap_query(
            &contract.htap,
            &QueryFeatures {
                is_read_only: true,
                has_group_by: true,
                has_aggregate: true,
                references_analytical_table: true,
                limit: None,
            },
        )
        .map_err(PoolExecutionError::component)?;
        let htap_analytical_routes =
            usize::from(matches!(htap_decision, HtapDecision::Analytical { .. }));

        let mut prepared_cache = PreparedStatementCache::default();
        prepared_cache
            .insert(PreparedStatement {
                backend_id: "worker-b:5433".to_string(),
                statement_name: "orders_by_tenant".to_string(),
                query_text: "SELECT * FROM orders WHERE tenant_id = $1".to_string(),
                shard_ids: vec![10],
                generation: initial_shard_map.generation_for_shards(&[10])?,
            })
            .map_err(PoolExecutionError::component)?;
        let prepared_cache_entries = prepared_cache.len();
        let prepared_invalidated = prepared_cache.invalidate_for_shard_map(&shard_map)?.len();

        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        let mut auth_cache = AuthVerificationCache::new(&contract.token_cache)
            .map_err(PoolExecutionError::component)?;
        let claims = crate::VerifiedClaims {
            jti: "jti-1".to_string(),
            tenant_id: "tenant-a".to_string(),
            subject: "user-1".to_string(),
            roles: vec!["app_user".to_string()],
            expires_at: now + Duration::from_secs(600),
        };
        auth_cache
            .insert(claims, now)
            .map_err(PoolExecutionError::component)?;
        let token_cache_hits = usize::from(auth_cache.lookup("jti-1", now).is_ok());
        let revoked = auth_cache.revoke("jti-1");
        let revoked_token_rejections =
            usize::from(revoked && auth_cache.lookup("jti-1", now).is_err());

        let mut tenant_quota = TenantQuotaTable::new(contract.tenant_quota.clone())
            .map_err(PoolExecutionError::component)?;
        let tenant_quota_admitted = usize::from(
            tenant_quota
                .try_admit("tenant-a", 0, 500)
                .map_err(PoolExecutionError::component)?
                .admitted(),
        );
        let tenant_quota_rejected = usize::from(
            !tenant_quota
                .try_admit("tenant-a", 0, 600)
                .map_err(PoolExecutionError::component)?
                .admitted(),
        );

        let mut geo_table = ClosestReplicaTable::default();
        geo_table
            .insert(RegionReplica {
                region: "us-east-1".to_string(),
                latency_rank: 10,
                target: RouteTarget {
                    host: "worker-a".to_string(),
                    port: 5432,
                },
            })
            .map_err(PoolExecutionError::component)?;
        let _geo_route = crate::route_for_client(
            &contract.geo_router,
            &geo_table,
            "10.0.0.9".parse().expect("canonical ip"),
            None,
        )
        .map_err(PoolExecutionError::component)?;

        let current_key = TicketKey::from_hex(
            SAMPLE_TICKET_HEX,
            now - Duration::from_secs(contract.tls.rotation_seconds as u64 + 1),
        )
        .map_err(PoolExecutionError::component)?;
        let mut ticket_ring = TicketKeyRing::new(
            current_key,
            Duration::from_secs(contract.tls.rotation_seconds as u64),
        )
        .map_err(PoolExecutionError::component)?;
        let tls_rotation_due = ticket_ring.rotation_due(now);
        let next_key =
            TicketKey::from_hex(SAMPLE_TICKET_HEX, now).map_err(PoolExecutionError::component)?;
        ticket_ring.rotate(next_key);
        let tls_previous_key_valid = ticket_ring.validates(now);

        let virtual_pid_table = VirtualPidTable::new();
        let backend = RealBackend {
            backend_id: "worker-b:5433:pid-4242".to_string(),
            real_pid: 4242,
            cancel_key: 7_777_777,
            host: "worker-b".to_string(),
            port: 5433,
        };
        let virtual_pid = virtual_pid_table
            .allocate(backend.clone())
            .map_err(PoolExecutionError::component)?;
        let mut client_cancel = Vec::with_capacity(16);
        client_cancel.extend_from_slice(&16_u32.to_be_bytes());
        client_cancel.extend_from_slice(&PGWIRE_CANCEL_MAGIC.to_be_bytes());
        client_cancel.extend_from_slice(&virtual_pid.to_be_bytes());
        client_cancel.extend_from_slice(&(backend.cancel_key as u32).to_be_bytes());
        let (parsed_virtual_pid, parsed_secret) =
            parse_cancel_request(&client_cancel).map_err(PoolExecutionError::component)?;
        let resolved_backend = virtual_pid_table
            .resolve(parsed_virtual_pid)
            .map_err(PoolExecutionError::component)?;
        let upstream_cancel = if parsed_secret == resolved_backend.cancel_key {
            encode_cancel_request(resolved_backend.real_pid, resolved_backend.cancel_key)
        } else {
            return Err(PoolExecutionError::Invariant("cancel key mismatch"));
        };
        let virtual_cancel_rewrites = usize::from(upstream_cancel.len() == 16);

        let mut realtime_queue = RealtimeHookQueue::new(RealtimeHookConfig {
            uds_path: "/var/run/citus/realtime.sock".to_string(),
            max_queue_depth: 8,
        })
        .map_err(PoolExecutionError::component)?;
        let realtime_accepted = realtime_queue
            .enqueue(CdcEvent {
                tenant_id: "tenant-a".to_string(),
                schema: "public".to_string(),
                table: "orders".to_string(),
                operation: CdcOperation::Update,
                commit_lsn: 2,
                primary_key_json: "{\"id\":1}".to_string(),
                row_json: "{\"id\":1,\"status\":\"paid\"}".to_string(),
            })
            .map_err(PoolExecutionError::component)?;
        let realtime_events_enqueued = usize::from(realtime_accepted);
        let realtime_events_drained = realtime_queue.drain().len();

        let mut admin_state = AdminState::default();
        let reload = AdminCommand::parse("RELOAD").map_err(PoolExecutionError::component)?;
        admin_state
            .apply(&reload)
            .map_err(PoolExecutionError::component)?;
        let kill = AdminCommand::parse("KILL 1000").map_err(PoolExecutionError::component)?;
        admin_state
            .apply(&kill)
            .map_err(PoolExecutionError::component)?;

        Ok(Self {
            tracked_gucs: contract.settings_bucket.tracked_gucs.len(),
            settings_bucket_max_connections: contract.settings_bucket.max_connections,
            settings_bucket_count: settings_map.bucket_count(),
            settings_bucket_assigned: settings_entry.assigned,
            fast_path_routes,
            mirror_sample_percent: contract.mirror.sample_percent,
            mirrored_canary_routes,
            htap_max_staleness_ms: contract.htap.max_staleness_ms,
            htap_analytical_routes,
            pipeline_max_in_flight: contract.pipeline.max_in_flight,
            transaction_pipelining: contract.pipeline.transaction_pipelining,
            pipeline_flushed_batches: pipeline.flushed_batches(),
            tls_rotation_seconds: contract.tls.rotation_seconds,
            tls_rotation_due,
            tls_previous_key_valid,
            tenant_burst: contract.tenant_quota.burst,
            tenant_quota_admitted,
            tenant_quota_rejected,
            geo_rules: contract.geo_router.rules.len(),
            geo_replica_regions: geo_table.region_count(),
            token_cache_entries: contract.token_cache.max_entries,
            token_cache_hits,
            revoked_token_rejections,
            plan_cache_entries_before_invalidation: entries_before,
            invalidated_plans,
            single_shard_generation,
            placement_changed_shards: placement_changed_shards.len(),
            prepared_cache_entries,
            prepared_invalidated,
            virtual_pid_entries: virtual_pid_table.len(),
            virtual_cancel_rewrites,
            realtime_events_enqueued,
            realtime_events_drained,
            admin_generation: admin_state.generation,
            admin_kills: admin_state.kills,
        })
    }

    pub fn tsv_header() -> &'static str {
        "tracked_gucs\tsettings_bucket_max_connections\tsettings_bucket_count\tsettings_bucket_assigned\tfast_path_routes\tmirror_sample_percent\tmirrored_canary_routes\thtap_max_staleness_ms\thtap_analytical_routes\tpipeline_max_in_flight\ttransaction_pipelining\tpipeline_flushed_batches\ttls_rotation_seconds\ttls_rotation_due\ttls_previous_key_valid\ttenant_burst\ttenant_quota_admitted\ttenant_quota_rejected\tgeo_rules\tgeo_replica_regions\ttoken_cache_entries\ttoken_cache_hits\trevoked_token_rejections\tplan_cache_entries_before_invalidation\tinvalidated_plans\tsingle_shard_generation\tplacement_changed_shards\tprepared_cache_entries\tprepared_invalidated\tvirtual_pid_entries\tvirtual_cancel_rewrites\trealtime_events_enqueued\trealtime_events_drained\tadmin_generation\tadmin_kills"
    }

    pub fn tsv_row(&self) -> String {
        [
            self.tracked_gucs.to_string(),
            self.settings_bucket_max_connections.to_string(),
            self.settings_bucket_count.to_string(),
            self.settings_bucket_assigned.to_string(),
            self.fast_path_routes.to_string(),
            self.mirror_sample_percent.to_string(),
            self.mirrored_canary_routes.to_string(),
            self.htap_max_staleness_ms.to_string(),
            self.htap_analytical_routes.to_string(),
            self.pipeline_max_in_flight.to_string(),
            self.transaction_pipelining.to_string(),
            self.pipeline_flushed_batches.to_string(),
            self.tls_rotation_seconds.to_string(),
            self.tls_rotation_due.to_string(),
            self.tls_previous_key_valid.to_string(),
            self.tenant_burst.to_string(),
            self.tenant_quota_admitted.to_string(),
            self.tenant_quota_rejected.to_string(),
            self.geo_rules.to_string(),
            self.geo_replica_regions.to_string(),
            self.token_cache_entries.to_string(),
            self.token_cache_hits.to_string(),
            self.revoked_token_rejections.to_string(),
            self.plan_cache_entries_before_invalidation.to_string(),
            self.invalidated_plans.to_string(),
            self.single_shard_generation.to_string(),
            self.placement_changed_shards.to_string(),
            self.prepared_cache_entries.to_string(),
            self.prepared_invalidated.to_string(),
            self.virtual_pid_entries.to_string(),
            self.virtual_cancel_rewrites.to_string(),
            self.realtime_events_enqueued.to_string(),
            self.realtime_events_drained.to_string(),
            self.admin_generation.to_string(),
            self.admin_kills.to_string(),
        ]
        .join("	")
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
    Component(String),
    Invariant(&'static str),
    Runtime(PoolRuntimeError),
    ShardMap(ShardMapError),
}

impl PoolExecutionError {
    fn component(error: impl fmt::Display) -> Self {
        Self::Component(error.to_string())
    }
}

impl fmt::Display for PoolExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Component(error) => write!(formatter, "{error}"),
            Self::Invariant(message) => write!(formatter, "{message}"),
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
                settings_bucket_count: 1,
                settings_bucket_assigned: 1,
                fast_path_routes: 1,
                mirror_sample_percent: 5,
                mirrored_canary_routes: 1,
                htap_max_staleness_ms: 2_000,
                htap_analytical_routes: 1,
                pipeline_max_in_flight: 32,
                transaction_pipelining: true,
                pipeline_flushed_batches: 1,
                tls_rotation_seconds: 3_600,
                tls_rotation_due: true,
                tls_previous_key_valid: true,
                tenant_burst: 1_000,
                tenant_quota_admitted: 1,
                tenant_quota_rejected: 1,
                geo_rules: 1,
                geo_replica_regions: 1,
                token_cache_entries: 10_000,
                token_cache_hits: 1,
                revoked_token_rejections: 1,
                plan_cache_entries_before_invalidation: 2,
                invalidated_plans: 1,
                single_shard_generation: 8,
                placement_changed_shards: 1,
                prepared_cache_entries: 1,
                prepared_invalidated: 1,
                virtual_pid_entries: 1,
                virtual_cancel_rewrites: 1,
                realtime_events_enqueued: 1,
                realtime_events_drained: 1,
                admin_generation: 2,
                admin_kills: 1,
            }
        );
        assert_eq!(PoolExecutionReport::tsv_header().split('\t').count(), 35);
        assert_eq!(report.tsv_row().split('\t').count(), 35);
    }
}
