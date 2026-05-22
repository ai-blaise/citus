//! Canonical specs shared between the operator binary's run-canonical
//! command, the conversion-webhook round-trip tests, and the conversion
//! smoke script. Having a single source of truth keeps the canonical
//! production-readiness numbers (workers=3, shards=32, retention=30, etc.)
//! aligned across all surfaces.

use crate::crds::backup::v1alpha1::{BackupEncryption, BackupProvider, BackupSpec, BackupTarget};
use crate::crds::branch::v1alpha1::{BranchSpec, BranchStorageSpec, BranchType};
use crate::crds::citus_cluster::v1alpha1::{
    CitusClusterSpec, CitusTopology, PoolSpec, SidecarSpec, SidecarType,
};
use crate::crds::conflict_policy::v1alpha1::{
    ConflictClass, ConflictPolicySpec, ConflictResolution,
};
use crate::crds::federation::v1alpha1::{FederationConnection, FederationSpec, FederationType};
use crate::crds::function::v1alpha1::{
    FunctionEvent, FunctionRuntime, FunctionSource, FunctionSpec, FunctionTrigger,
};
use crate::crds::hypertable::v1alpha1::{
    CompressionPolicy, ContinuousAggregateSpec, HypertableSpec, RetentionPolicy,
};
use crate::crds::migration::v1alpha1::{MigrationConflictAction, MigrationSpec, MigrationType};
use crate::crds::region::v1alpha1::RegionSpec;
use crate::crds::scheduled_repack::v1alpha1::{RepackStrategy, ScheduledRepackSpec};
use crate::crds::search_index::v1alpha1::{
    SearchColumnKind, SearchColumnSpec, SearchIndexSpec, SearchScorer,
};
use crate::crds::shard_group::v1alpha1::{
    PlacementPolicy, ShardGroupSpec, UnsatisfiablePlacementAction,
};
use crate::crds::sidecar::v1alpha1::{
    ResourceRequirements, SidecarDeploymentSpec, SidecarDeploymentType,
};
use crate::crds::survival_goal::v1alpha1::{SurvivalGoalSpec, SurvivalGoalType};
use crate::crds::tenant::v1alpha1::{TenantQuotas, TenantSpec};
use crate::crds::vectorizer::v1alpha1::{
    ChunkingSpec, ChunkingStrategy, EmbeddingProvider, VectorDestinationSpec,
    VectorizerScheduleMode, VectorizerSchedulingSpec, VectorizerSpec,
};
use crate::crds::webhook::v1alpha1::{WebhookEvent, WebhookRetryPolicy, WebhookSpec};

pub fn canonical_cluster_spec() -> CitusClusterSpec {
    CitusClusterSpec {
        topology: CitusTopology::CoordinatorWorker,
        image: "ghcr.io/ai-blaise/citus:pg18-v2".to_string(),
        workers: 3,
        coordinators: 1,
        storage_class: Some("fast-ssd".to_string()),
        timescale_enabled: true,
        extensions: vec!["citus".to_string(), "timescaledb".to_string()],
        pool: Some(PoolSpec {
            replicas: 2,
            geoip_db: Some("maxmind-city".to_string()),
        }),
        sidecars: vec![
            SidecarSpec {
                sidecar_type: SidecarType::Vectorizer,
                replicas: 1,
            },
            SidecarSpec {
                sidecar_type: SidecarType::Realtime,
                replicas: 2,
            },
            SidecarSpec {
                sidecar_type: SidecarType::Mcp,
                replicas: 1,
            },
        ],
    }
}

pub fn canonical_shard_group_spec() -> ShardGroupSpec {
    ShardGroupSpec {
        parent_table: "public.metrics".to_string(),
        distribution_column: "tenant_id".to_string(),
        num_shards: 32,
        colocation_group: Some("metrics".to_string()),
        replication_factor: 3,
        placement_policy: vec![PlacementPolicy {
            topology_key: "topology.kubernetes.io/zone".to_string(),
            max_skew: 1,
            when_unsatisfiable: UnsatisfiablePlacementAction::DoNotSchedule,
        }],
    }
}

pub fn canonical_hypertable_spec() -> HypertableSpec {
    HypertableSpec {
        table: "metrics".to_string(),
        time_column: "ts".to_string(),
        distribution_column: "tenant_id".to_string(),
        chunk_time_interval: "1 day".to_string(),
        num_shards: 32,
        compression: Some(CompressionPolicy {
            older_than: "7 days".to_string(),
            segment_by: vec!["tenant_id".to_string()],
            order_by: vec!["ts DESC".to_string()],
            bloom_filters: vec!["region".to_string()],
        }),
        retention: Some(RetentionPolicy {
            drop_after: "90 days".to_string(),
        }),
        continuous_aggregates: vec![ContinuousAggregateSpec {
            name: "metrics_hourly".to_string(),
            query: "SELECT 1".to_string(),
            refresh_start: Some("7 days".to_string()),
            refresh_end: Some("1 hour".to_string()),
            schedule: Some("15 minutes".to_string()),
            hierarchical_parent: None,
        }],
    }
}

pub fn canonical_branch_spec() -> BranchSpec {
    BranchSpec {
        source_cluster: "prod-us-east".to_string(),
        branch_type: BranchType::CopyOnWrite,
        storage: BranchStorageSpec {
            size: "256Gi".to_string(),
            storage_class: Some("fast-ssd".to_string()),
            snapshot_class: Some("csi-snapshot".to_string()),
        },
        suspend: true,
        retention_days: Some(7),
    }
}

pub fn canonical_tenant_spec() -> TenantSpec {
    TenantSpec {
        name: "tenant-a".to_string(),
        schema_name: "tenant_a".to_string(),
        quotas: TenantQuotas {
            max_connections: 64,
            max_qps: 10_000,
            max_storage_bytes: 1_099_511_627_776,
        },
        region_affinity: Some("us-east-1".to_string()),
    }
}

pub fn canonical_region_spec() -> RegionSpec {
    RegionSpec {
        name: "us-east-1".to_string(),
        kubernetes_zone: "us-east-1a".to_string(),
        tablespace_name: "ts_us_east_1".to_string(),
        leader_pinned: true,
    }
}

pub fn canonical_survival_goal_spec() -> SurvivalGoalSpec {
    SurvivalGoalSpec {
        goal: SurvivalGoalType::RegionFailure,
        regions: vec!["us-east-1".to_string(), "us-west-2".to_string()],
        min_replicas: 2,
    }
}

pub fn canonical_backup_spec() -> BackupSpec {
    BackupSpec {
        schedule: "0 */6 * * *".to_string(),
        retention_days: 30,
        target: BackupTarget {
            provider: BackupProvider::S3,
            bucket: "ai-blaise-citus-backups".to_string(),
            prefix: "prod/us-east-1".to_string(),
        },
        encryption: Some(BackupEncryption {
            kms_key_ref: "aws-kms-prod".to_string(),
        }),
    }
}

pub fn canonical_vectorizer_spec() -> VectorizerSpec {
    VectorizerSpec {
        source_table: "public.documents".to_string(),
        source_column: "body".to_string(),
        embedding_provider: EmbeddingProvider::OpenAi,
        embedding_model: "text-embedding-3-large".to_string(),
        destination: VectorDestinationSpec {
            table: "public.document_embeddings".to_string(),
            column: "embedding".to_string(),
            dimensions: 3_072,
        },
        chunking: ChunkingSpec {
            strategy: ChunkingStrategy::RecursiveText,
            max_tokens: 800,
            overlap_tokens: 80,
        },
        scheduling: VectorizerSchedulingSpec {
            mode: VectorizerScheduleMode::Interval,
            interval: Some("30 seconds".to_string()),
            max_concurrency: 8,
        },
        secret_ref: "openai-embeddings".to_string(),
    }
}

pub fn canonical_sidecar_deployment_spec() -> SidecarDeploymentSpec {
    SidecarDeploymentSpec {
        sidecar_type: SidecarDeploymentType::Realtime,
        replicas: 2,
        resources: ResourceRequirements {
            cpu_millis: 250,
            memory_mib: 512,
        },
        config_yaml: Some("subscriptions:\n  max_per_tenant: 1000".to_string()),
    }
}

pub fn canonical_migration_spec() -> MigrationSpec {
    MigrationSpec {
        migration_type: MigrationType::Pgroll,
        yaml: "operations:\n  - add_column:\n      table: users".to_string(),
        on_conflict: MigrationConflictAction::ManualReview,
    }
}

pub fn canonical_conflict_policy_spec() -> ConflictPolicySpec {
    ConflictPolicySpec {
        table: "public.reference_accounts".to_string(),
        class: ConflictClass::UpdateUpdate,
        resolution: ConflictResolution::LastWriteWins,
        custom_function: None,
    }
}

pub fn canonical_federation_spec() -> FederationSpec {
    FederationSpec {
        name: "warehouse".to_string(),
        federation_type: FederationType::Snowflake,
        connection: FederationConnection {
            secret_ref: "snowflake-warehouse".to_string(),
        },
        foreign_schema_prefix: "snowflake_".to_string(),
    }
}

pub fn canonical_search_index_spec() -> SearchIndexSpec {
    SearchIndexSpec {
        table: "public.documents".to_string(),
        columns: vec![
            SearchColumnSpec {
                name: "body".to_string(),
                kind: SearchColumnKind::Text,
            },
            SearchColumnSpec {
                name: "embedding".to_string(),
                kind: SearchColumnKind::Vector,
            },
        ],
        scorer: SearchScorer::Bm25Vector,
        analyzer: "english".to_string(),
        distributed: true,
    }
}

pub fn canonical_webhook_spec() -> WebhookSpec {
    WebhookSpec {
        table: "public.orders".to_string(),
        events: vec![WebhookEvent::Insert, WebhookEvent::Update],
        url: "https://example.com/orders".to_string(),
        headers_secret_ref: Some("orders-webhook".to_string()),
        retry_policy: WebhookRetryPolicy {
            max_attempts: 5,
            backoff: "exponential:1s:30s".to_string(),
            dead_letter_table: Some("webhook_dead_letters".to_string()),
        },
        payload_template: Some("{\"table\":\"orders\"}".to_string()),
    }
}

pub fn canonical_function_spec() -> FunctionSpec {
    FunctionSpec {
        name: "order-created".to_string(),
        runtime: FunctionRuntime::Deno,
        source: FunctionSource::GitRef {
            repository: "https://github.com/ai-blaise/functions".to_string(),
            reference: "main".to_string(),
            path: "orders/index.ts".to_string(),
        },
        triggers: vec![
            FunctionTrigger::Http {
                path: "/orders".to_string(),
            },
            FunctionTrigger::Event {
                table: "public.orders".to_string(),
                event: FunctionEvent::Insert,
            },
        ],
        env_secrets: vec!["orders-api-key".to_string()],
    }
}

pub fn canonical_scheduled_repack_spec() -> ScheduledRepackSpec {
    ScheduledRepackSpec {
        target: "public.orders".to_string(),
        schedule: "0 3 * * 0".to_string(),
        strategy: RepackStrategy::PgRepack,
    }
}
