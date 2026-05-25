// FEATURE: A8
// FEATURE: B2
// FEATURE: B6
// FEATURE: C4
// FEATURE: C5
// FEATURE: C6
// FEATURE: C7
// FEATURE: C8
// FEATURE: C9
// FEATURE: EF3
// FEATURE: F1
// FEATURE: M3
// FEATURE: MR1
// FEATURE: MR2
// FEATURE: MR4
// FEATURE: MR8
// FEATURE: O5
// FEATURE: R2
// FEATURE: R7
// FEATURE: S10
// FEATURE: S11
// FEATURE: Search2
// FEATURE: Search7
// FEATURE: TO1
// FEATURE: TO2
// FEATURE: TO5
// FEATURE: WH1

use ai_blaise_citus_operator::{
    BackupEncryption, BackupProvider, BackupSpec, BackupSpecError, BackupTarget, BranchSpec,
    BranchSpecError, BranchStorageSpec, BranchType, ChunkingSpec, ChunkingStrategy, ConflictClass,
    ConflictPolicySpec, ConflictPolicySpecError, ConflictResolution, EmbeddingProvider,
    FederationConnection, FederationSpec, FederationSpecError, FederationType, FunctionEvent,
    FunctionRuntime, FunctionSource, FunctionSpec, FunctionSpecError, FunctionTrigger,
    MigrationConflictAction, MigrationSpec, MigrationSpecError, MigrationType, RegionSpec,
    RegionSpecError, RepackStrategy, ResourceRequirements, ScheduledRepackSpec,
    ScheduledRepackSpecError, SearchColumnKind, SearchColumnSpec, SearchIndexSpec,
    SearchIndexSpecError, SearchScorer, SidecarDeploymentSpec, SidecarDeploymentSpecError,
    SidecarDeploymentType, SurvivalGoalSpec, SurvivalGoalSpecError, SurvivalGoalType, TenantQuotas,
    TenantSpec, TenantSpecError, VectorDestinationSpec, VectorizerScheduleMode,
    VectorizerSchedulingSpec, VectorizerSpec, VectorizerSpecError, WebhookEvent,
    WebhookRetryPolicy, WebhookSpec, WebhookSpecError,
};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct V2OperatorCatalogAcceptance {
    pub branch: BranchSpec,
    pub tenant: TenantSpec,
    pub region: RegionSpec,
    pub survival_goal: SurvivalGoalSpec,
    pub backup: BackupSpec,
    pub vectorizer: VectorizerSpec,
    pub sidecar: SidecarDeploymentSpec,
    pub migration: MigrationSpec,
    pub conflict_policy: ConflictPolicySpec,
    pub federation: FederationSpec,
    pub search_index: SearchIndexSpec,
    pub webhook: WebhookSpec,
    pub function: FunctionSpec,
    pub scheduled_repack: ScheduledRepackSpec,
}

impl V2OperatorCatalogAcceptance {
    pub fn canonical_platform() -> Self {
        Self {
            branch: BranchSpec {
                source_cluster: "prod-us-east".to_string(),
                target_cluster: "branch-review".to_string(),
                branch_type: BranchType::Snapshot,
                storage: BranchStorageSpec {
                    size: "256Gi".to_string(),
                    storage_class: Some("fast-ssd".to_string()),
                    snapshot_class: Some("csi-snapshot".to_string()),
                },
                suspend: true,
                retention_days: Some(7),
            },
            tenant: TenantSpec {
                name: "tenant-a".to_string(),
                schema_name: "tenant_a".to_string(),
                quotas: TenantQuotas {
                    max_connections: 64,
                    max_qps: 10_000,
                    max_storage_bytes: 1_099_511_627_776,
                },
                region_affinity: Some("us-east-1".to_string()),
            },
            region: RegionSpec {
                name: "us-east-1".to_string(),
                kubernetes_zone: "us-east-1a".to_string(),
                tablespace_name: "ts_us_east_1".to_string(),
                leader_pinned: true,
            },
            survival_goal: SurvivalGoalSpec {
                goal: SurvivalGoalType::RegionFailure,
                regions: vec!["us-east-1".to_string(), "us-west-2".to_string()],
                min_replicas: 2,
            },
            backup: BackupSpec {
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
            },
            vectorizer: VectorizerSpec {
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
            },
            sidecar: SidecarDeploymentSpec {
                sidecar_type: SidecarDeploymentType::Realtime,
                replicas: 2,
                resources: ResourceRequirements {
                    cpu_millis: 250,
                    memory_mib: 512,
                },
                image: Some(
                    "ghcr.io/ai-blaise/citus-sidecar-realtime:unreleased".to_string(),
                ),
                config_yaml: Some("subscriptions:\n  max_per_tenant: 1000".to_string()),
            },
            migration: MigrationSpec {
                migration_type: MigrationType::Pgroll,
                yaml: "twoVersionInvariantPrecheck: companion_internal.verify_two_version_invariant()\nrollback:\n  operation: companion_internal.schema_job_rollback_to\n  targetPhase: write_only\noperations:\n  - addColumn:\n      table: public.users\n      column: display_name\n      sqlType: text".to_string(),
                on_conflict: MigrationConflictAction::ManualReview,
            },
            conflict_policy: ConflictPolicySpec {
                table: "public.reference_accounts".to_string(),
                class: ConflictClass::UpdateUpdate,
                resolution: ConflictResolution::LastWriteWins,
                custom_function: None,
            },
            federation: FederationSpec {
                name: "warehouse".to_string(),
                federation_type: FederationType::Snowflake,
                connection: FederationConnection {
                    secret_ref: "snowflake-warehouse".to_string(),
                },
                foreign_schema_prefix: "snowflake_".to_string(),
            },
            search_index: SearchIndexSpec {
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
            },
            webhook: WebhookSpec {
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
            },
            function: FunctionSpec {
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
            },
            scheduled_repack: ScheduledRepackSpec {
                target: "public.orders".to_string(),
                schedule: "0 3 * * 0".to_string(),
                strategy: RepackStrategy::PgRepack,
            },
        }
    }

    pub fn plan(&self) -> Result<V2OperatorCatalogPlan, V2OperatorCatalogAcceptanceError> {
        self.branch
            .validate()
            .map_err(V2OperatorCatalogAcceptanceError::Branch)?;
        self.tenant
            .validate()
            .map_err(V2OperatorCatalogAcceptanceError::Tenant)?;
        self.region
            .validate()
            .map_err(V2OperatorCatalogAcceptanceError::Region)?;
        self.survival_goal
            .validate()
            .map_err(V2OperatorCatalogAcceptanceError::SurvivalGoal)?;
        self.backup
            .validate()
            .map_err(V2OperatorCatalogAcceptanceError::Backup)?;
        self.vectorizer
            .validate()
            .map_err(V2OperatorCatalogAcceptanceError::Vectorizer)?;
        self.sidecar
            .validate()
            .map_err(V2OperatorCatalogAcceptanceError::Sidecar)?;
        self.migration
            .validate()
            .map_err(V2OperatorCatalogAcceptanceError::Migration)?;
        self.conflict_policy
            .validate()
            .map_err(V2OperatorCatalogAcceptanceError::ConflictPolicy)?;
        self.federation
            .validate()
            .map_err(V2OperatorCatalogAcceptanceError::Federation)?;
        self.search_index
            .validate()
            .map_err(V2OperatorCatalogAcceptanceError::SearchIndex)?;
        self.webhook
            .validate()
            .map_err(V2OperatorCatalogAcceptanceError::Webhook)?;
        self.function
            .validate()
            .map_err(V2OperatorCatalogAcceptanceError::Function)?;
        self.scheduled_repack
            .validate()
            .map_err(V2OperatorCatalogAcceptanceError::ScheduledRepack)?;

        Ok(V2OperatorCatalogPlan {
            branch: self.branch.clone(),
            tenant: self.tenant.clone(),
            region: self.region.clone(),
            survival_goal: self.survival_goal.clone(),
            backup: self.backup.clone(),
            vectorizer: self.vectorizer.clone(),
            sidecar: self.sidecar.clone(),
            migration: self.migration.clone(),
            conflict_policy: self.conflict_policy.clone(),
            federation: self.federation.clone(),
            search_index: self.search_index.clone(),
            webhook: self.webhook.clone(),
            function: self.function.clone(),
            scheduled_repack: self.scheduled_repack.clone(),
            gates: vec![
                OperatorCatalogGate::BranchLifecycle,
                OperatorCatalogGate::TenantOperations,
                OperatorCatalogGate::MultiRegionSurvival,
                OperatorCatalogGate::BackupPolicy,
                OperatorCatalogGate::VectorizerContract,
                OperatorCatalogGate::SidecarDeployment,
                OperatorCatalogGate::MigrationContract,
                OperatorCatalogGate::ConflictPolicy,
                OperatorCatalogGate::FederationContract,
                OperatorCatalogGate::SearchIndex,
                OperatorCatalogGate::WebhookDelivery,
                OperatorCatalogGate::FunctionDeployment,
                OperatorCatalogGate::ScheduledRepack,
            ],
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct V2OperatorCatalogPlan {
    pub branch: BranchSpec,
    pub tenant: TenantSpec,
    pub region: RegionSpec,
    pub survival_goal: SurvivalGoalSpec,
    pub backup: BackupSpec,
    pub vectorizer: VectorizerSpec,
    pub sidecar: SidecarDeploymentSpec,
    pub migration: MigrationSpec,
    pub conflict_policy: ConflictPolicySpec,
    pub federation: FederationSpec,
    pub search_index: SearchIndexSpec,
    pub webhook: WebhookSpec,
    pub function: FunctionSpec,
    pub scheduled_repack: ScheduledRepackSpec,
    pub gates: Vec<OperatorCatalogGate>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum OperatorCatalogGate {
    BranchLifecycle,
    TenantOperations,
    MultiRegionSurvival,
    BackupPolicy,
    VectorizerContract,
    SidecarDeployment,
    MigrationContract,
    ConflictPolicy,
    FederationContract,
    SearchIndex,
    WebhookDelivery,
    FunctionDeployment,
    ScheduledRepack,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum V2OperatorCatalogAcceptanceError {
    Backup(BackupSpecError),
    Branch(BranchSpecError),
    ConflictPolicy(ConflictPolicySpecError),
    Federation(FederationSpecError),
    Function(FunctionSpecError),
    Migration(MigrationSpecError),
    Region(RegionSpecError),
    ScheduledRepack(ScheduledRepackSpecError),
    SearchIndex(SearchIndexSpecError),
    Sidecar(SidecarDeploymentSpecError),
    SurvivalGoal(SurvivalGoalSpecError),
    Tenant(TenantSpecError),
    Vectorizer(VectorizerSpecError),
    Webhook(WebhookSpecError),
}

impl fmt::Display for V2OperatorCatalogAcceptanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backup(error) => write!(formatter, "{error}"),
            Self::Branch(error) => write!(formatter, "{error}"),
            Self::ConflictPolicy(error) => write!(formatter, "{error}"),
            Self::Federation(error) => write!(formatter, "{error}"),
            Self::Function(error) => write!(formatter, "{error}"),
            Self::Migration(error) => write!(formatter, "{error}"),
            Self::Region(error) => write!(formatter, "{error}"),
            Self::ScheduledRepack(error) => write!(formatter, "{error}"),
            Self::SearchIndex(error) => write!(formatter, "{error}"),
            Self::Sidecar(error) => write!(formatter, "{error}"),
            Self::SurvivalGoal(error) => write!(formatter, "{error}"),
            Self::Tenant(error) => write!(formatter, "{error}"),
            Self::Vectorizer(error) => write!(formatter, "{error}"),
            Self::Webhook(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for V2OperatorCatalogAcceptanceError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_platform_plan_covers_v2_operator_catalog() {
        let plan = V2OperatorCatalogAcceptance::canonical_platform()
            .plan()
            .expect("canonical operator catalog plan");

        assert_eq!(plan.branch.source_cluster, "prod-us-east");
        assert_eq!(plan.branch.target_cluster, "branch-review");
        assert_eq!(plan.branch.branch_type, BranchType::Snapshot);
        assert!(plan.branch.is_scale_to_zero_enabled());
        assert_eq!(plan.tenant.schema_name, "tenant_a");
        assert_eq!(plan.region.tablespace_name, "ts_us_east_1");
        assert_eq!(plan.survival_goal.min_replicas, 2);
        assert_eq!(plan.backup.retention_days, 30);
        assert_eq!(plan.vectorizer.destination.dimensions, 3_072);
        assert_eq!(plan.sidecar.replicas, 2);
        assert_eq!(
            plan.sidecar.image.as_deref(),
            Some("ghcr.io/ai-blaise/citus-sidecar-realtime:unreleased")
        );
        assert_eq!(plan.migration.migration_type, MigrationType::Pgroll);
        assert_eq!(plan.conflict_policy.class, ConflictClass::UpdateUpdate);
        assert_eq!(plan.federation.name, "warehouse");
        assert_eq!(plan.search_index.scorer, SearchScorer::Bm25Vector);
        assert_eq!(plan.webhook.events.len(), 2);
        assert_eq!(plan.function.triggers.len(), 2);
        assert_eq!(plan.scheduled_repack.strategy, RepackStrategy::PgRepack);
        assert_eq!(
            plan.gates,
            vec![
                OperatorCatalogGate::BranchLifecycle,
                OperatorCatalogGate::TenantOperations,
                OperatorCatalogGate::MultiRegionSurvival,
                OperatorCatalogGate::BackupPolicy,
                OperatorCatalogGate::VectorizerContract,
                OperatorCatalogGate::SidecarDeployment,
                OperatorCatalogGate::MigrationContract,
                OperatorCatalogGate::ConflictPolicy,
                OperatorCatalogGate::FederationContract,
                OperatorCatalogGate::SearchIndex,
                OperatorCatalogGate::WebhookDelivery,
                OperatorCatalogGate::FunctionDeployment,
                OperatorCatalogGate::ScheduledRepack,
            ]
        );
    }

    #[test]
    fn catalog_rejects_invalid_vectorizer_chunking() {
        let mut acceptance = V2OperatorCatalogAcceptance::canonical_platform();
        acceptance.vectorizer.chunking.overlap_tokens = acceptance.vectorizer.chunking.max_tokens;

        assert_eq!(
            acceptance.plan(),
            Err(V2OperatorCatalogAcceptanceError::Vectorizer(
                VectorizerSpecError::InvalidChunkOverlap
            ))
        );
    }

    #[test]
    fn catalog_rejects_invalid_hybrid_search_index() {
        let mut acceptance = V2OperatorCatalogAcceptance::canonical_platform();
        acceptance
            .search_index
            .columns
            .retain(|column| column.kind == SearchColumnKind::Text);

        assert_eq!(
            acceptance.plan(),
            Err(V2OperatorCatalogAcceptanceError::SearchIndex(
                SearchIndexSpecError::MissingHybridColumns
            ))
        );
    }
}
