// FEATURE: Auth1
// FEATURE: Auth3
// FEATURE: B1
// FEATURE: B3
// FEATURE: B4
// FEATURE: C1
// FEATURE: L8
// FEATURE: MR5
// FEATURE: R7
// FEATURE: R10
// FEATURE: RT1
// FEATURE: RT2
// FEATURE: RT3
// FEATURE: RT4
// FEATURE: Search8
// FEATURE: Sec12
// FEATURE: Sto1
// FEATURE: Sto3
// FEATURE: Sto4
// FEATURE: T1
// FEATURE: T3
// FEATURE: T9
// FEATURE: T12
// FEATURE: T15
// FEATURE: WH3

use ai_blaise_citus_pool::{
    FastPathRouterPolicy, GeoRoutingPolicy, GeoRoutingRule, HtapRoutingPolicy, MirrorTrafficPolicy,
    PoolRuntimeContract, PoolRuntimeError, ProtocolPipelinePolicy, RouteTarget,
    SettingsBucketPolicy, TenantAdmissionPolicy, TlsSessionTicketPolicy,
    TokenIntrospectionCachePolicy,
};
use ai_blaise_citus_sidecar_shared::{
    AnalyticalMirrorContract, AuthIssuerContract, BackupRestoreContract, CdcSink,
    CdcStreamContract, DeliveryRetryPolicy, RealtimeContract, RepackContract,
    RepackExecutionStrategy, SidecarContractError, SidecarRuntimeContracts, StorageContract,
};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct V2RuntimeContractAcceptance {
    pub sidecar: SidecarRuntimeContracts,
    pub pool: PoolRuntimeContract,
}

impl V2RuntimeContractAcceptance {
    pub fn canonical_runtime() -> Self {
        Self {
            sidecar: SidecarRuntimeContracts {
                cdc: CdcStreamContract {
                    slot_name: "ai_blaise_cdc".to_string(),
                    publication_name: "ai_blaise_publication".to_string(),
                    sinks: vec![
                        CdcSink::Realtime {
                            topic_prefix: "tenant".to_string(),
                        },
                        CdcSink::Webhook {
                            url: "https://example.com/webhooks".to_string(),
                        },
                        CdcSink::AnalyticalMirror {
                            stream_name: "metrics_mirror".to_string(),
                        },
                    ],
                    retry_policy: DeliveryRetryPolicy {
                        max_attempts: 5,
                        dead_letter_queue: "cdc_dead_letters".to_string(),
                    },
                },
                realtime: RealtimeContract {
                    topic: "tenant-a:public.orders".to_string(),
                    tenant_id: "tenant-a".to_string(),
                    filters: vec!["status = 'open'".to_string()],
                    presence_enabled: true,
                },
                auth: AuthIssuerContract {
                    issuer: "https://auth.example.com".to_string(),
                    signing_key_ref: "jwt-signing-key".to_string(),
                    token_ttl_seconds: 3_600,
                    tenant_claim: "tenant_id".to_string(),
                },
                storage: StorageContract {
                    bucket: "tenant-files".to_string(),
                    metadata_table: "storage.objects".to_string(),
                    presigned_url_ttl_seconds: 900,
                    acl_tenant_column: "tenant_id".to_string(),
                },
                backup_restore: BackupRestoreContract {
                    schedule: "0 */6 * * *".to_string(),
                    archive_uri: "s3://ai-blaise-citus-backups/prod".to_string(),
                    pitr_target: Some("2026-05-19T12:00:00Z".to_string()),
                    queryable_branch_name: Some("prod-at-noon".to_string()),
                },
                repack: RepackContract {
                    target: "public.orders".to_string(),
                    strategy: RepackExecutionStrategy::PgRepack,
                    max_concurrency: 2,
                },
                analytical_mirror: AnalyticalMirrorContract {
                    source_slot: "ai_blaise_cdc".to_string(),
                    mirror_name: "metrics_mirror".to_string(),
                    storage_uri: "s3://ai-blaise-cold/metrics".to_string(),
                    search_index_enabled: true,
                },
            },
            pool: PoolRuntimeContract {
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
            },
        }
    }

    pub fn plan(&self) -> Result<V2RuntimeContractPlan, V2RuntimeContractAcceptanceError> {
        self.sidecar
            .validate()
            .map_err(V2RuntimeContractAcceptanceError::Sidecar)?;
        self.pool
            .validate()
            .map_err(V2RuntimeContractAcceptanceError::Pool)?;

        Ok(V2RuntimeContractPlan {
            sidecar: self.sidecar.clone(),
            pool: self.pool.clone(),
            gates: vec![
                RuntimeContractGate::CdcDelivery,
                RuntimeContractGate::RealtimeBroadcast,
                RuntimeContractGate::AuthIssuance,
                RuntimeContractGate::StorageAccess,
                RuntimeContractGate::BackupRestore,
                RuntimeContractGate::Repack,
                RuntimeContractGate::AnalyticalMirror,
                RuntimeContractGate::SettingsBucketPool,
                RuntimeContractGate::FastPathRouting,
                RuntimeContractGate::MirrorTraffic,
                RuntimeContractGate::HtapRouting,
                RuntimeContractGate::ProtocolPipelining,
                RuntimeContractGate::TlsReuse,
                RuntimeContractGate::TenantAdmission,
                RuntimeContractGate::GeoRouting,
                RuntimeContractGate::TokenCache,
            ],
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct V2RuntimeContractPlan {
    pub sidecar: SidecarRuntimeContracts,
    pub pool: PoolRuntimeContract,
    pub gates: Vec<RuntimeContractGate>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RuntimeContractGate {
    CdcDelivery,
    RealtimeBroadcast,
    AuthIssuance,
    StorageAccess,
    BackupRestore,
    Repack,
    AnalyticalMirror,
    SettingsBucketPool,
    FastPathRouting,
    MirrorTraffic,
    HtapRouting,
    ProtocolPipelining,
    TlsReuse,
    TenantAdmission,
    GeoRouting,
    TokenCache,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum V2RuntimeContractAcceptanceError {
    Pool(PoolRuntimeError),
    Sidecar(SidecarContractError),
}

impl fmt::Display for V2RuntimeContractAcceptanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(error) => write!(formatter, "{error}"),
            Self::Sidecar(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for V2RuntimeContractAcceptanceError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_runtime_plan_covers_pool_and_sidecar_contracts() {
        let plan = V2RuntimeContractAcceptance::canonical_runtime()
            .plan()
            .expect("canonical runtime plan");

        assert_eq!(plan.sidecar.cdc.slot_name, "ai_blaise_cdc");
        assert_eq!(plan.sidecar.realtime.tenant_id, "tenant-a");
        assert_eq!(plan.sidecar.storage.bucket, "tenant-files");
        assert_eq!(plan.pool.settings_bucket.max_connections, 1_000);
        assert_eq!(plan.pool.mirror.sample_percent, 5);
        assert_eq!(plan.pool.htap.max_staleness_ms, 2_000);
        assert_eq!(
            plan.gates,
            vec![
                RuntimeContractGate::CdcDelivery,
                RuntimeContractGate::RealtimeBroadcast,
                RuntimeContractGate::AuthIssuance,
                RuntimeContractGate::StorageAccess,
                RuntimeContractGate::BackupRestore,
                RuntimeContractGate::Repack,
                RuntimeContractGate::AnalyticalMirror,
                RuntimeContractGate::SettingsBucketPool,
                RuntimeContractGate::FastPathRouting,
                RuntimeContractGate::MirrorTraffic,
                RuntimeContractGate::HtapRouting,
                RuntimeContractGate::ProtocolPipelining,
                RuntimeContractGate::TlsReuse,
                RuntimeContractGate::TenantAdmission,
                RuntimeContractGate::GeoRouting,
                RuntimeContractGate::TokenCache,
            ]
        );
    }

    #[test]
    fn runtime_rejects_invalid_cdc_webhook_url() {
        let mut acceptance = V2RuntimeContractAcceptance::canonical_runtime();
        acceptance.sidecar.cdc.sinks = vec![CdcSink::Webhook {
            url: "ftp://example.com".to_string(),
        }];

        assert_eq!(
            acceptance.plan(),
            Err(V2RuntimeContractAcceptanceError::Sidecar(
                SidecarContractError::InvalidUrl
            ))
        );
    }

    #[test]
    fn runtime_rejects_invalid_mirror_sample_percent() {
        let mut acceptance = V2RuntimeContractAcceptance::canonical_runtime();
        acceptance.pool.mirror.sample_percent = 101;

        assert_eq!(
            acceptance.plan(),
            Err(V2RuntimeContractAcceptanceError::Pool(
                PoolRuntimeError::InvalidPercent
            ))
        );
    }
}
