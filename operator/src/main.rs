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
// FEATURE: S2
// FEATURE: S4
// FEATURE: S10
// FEATURE: S11
// FEATURE: Search2
// FEATURE: Search7
// FEATURE: TO1
// FEATURE: TO2
// FEATURE: TO5
// FEATURE: TS7
// FEATURE: WH1

use ai_blaise_citus_operator::controllers;
use ai_blaise_citus_operator::{
    BackupEncryption, BackupProvider, BackupSpec, BackupTarget, BranchSpec, BranchStorageSpec,
    BranchType, ChunkingSpec, ChunkingStrategy, CitusClusterReconcilePlan, CitusClusterSpec,
    CitusTopology, CompressionPolicy, ConflictClass, ConflictPolicySpec, ConflictResolution,
    ContinuousAggregateSpec, EmbeddingProvider, FederationConnection, FederationReconcilePlan,
    FederationSpec, FederationType, FunctionEvent, FunctionReconcilePlan, FunctionRuntime,
    FunctionSource, FunctionSpec, FunctionStepKind, FunctionTrigger, HypertableReconcilePlan,
    HypertableSpec, MigrationConflictAction, MigrationSpec, MigrationType, PlacementPolicy,
    PoolSpec, RegionSpec, RepackStrategy, ResourceRequirements, RetentionPolicy,
    ScheduledRepackSpec, SearchColumnKind, SearchColumnSpec, SearchIndexReconcilePlan,
    SearchIndexSpec, SearchScorer, ShardGroupReconcilePlan, ShardGroupSpec, SidecarDeploymentSpec,
    SidecarDeploymentType, SidecarSpec, SidecarType, SurvivalGoalSpec, SurvivalGoalType,
    TenantQuotas, TenantSpec, UnsatisfiablePlacementAction, VectorDestinationSpec,
    VectorizerScheduleMode, VectorizerSchedulingSpec, VectorizerSpec, WebhookEvent,
    WebhookReconcilePlan, WebhookRetryPolicy, WebhookSpec,
};
use ai_blaise_citus_sidecar_shared::run_probe_server;
use std::env;
use std::error::Error;
use std::process;
use std::thread;

const CANONICAL_OPERATOR_CRDS: usize = 17;
const V2_OPERATOR_CATALOG_GATES: usize = 13;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }
    if args == ["serve"] {
        run_serve("operator", "0.0.0.0:8080");
        return;
    }

    match args.as_slice() {
        [] => run_canonical(),
        [command] if command == "run-canonical" => run_canonical(),
        [command] if command == "run-reconcile-plans" => run_reconcile_plans(),
        [command] if command == "run-reconcilers-batch-b" => run_reconcilers_batch_b(),
        _ => {
            eprintln!("operator: unknown command");
            print_usage();
            process::exit(2);
        }
    }
}

fn run_canonical() {
    let report = canonical_operator_execution_report().unwrap_or_else(|error| {
        eprintln!("operator: canonical execution failed: {error}");
        process::exit(1);
    });

    println!(
        "crds\tcluster_workers\tcluster_sidecars\tshards\thypertable_sql_plans\thypertable_apply_steps\tcatalog_gates\tbackup_retention_days\tvector_dimensions\tfunction_triggers\twebhook_events\tsearch_columns\tsidecar_replicas"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.crds,
        report.cluster_workers,
        report.cluster_sidecars,
        report.shards,
        report.hypertable_sql_plans,
        report.hypertable_apply_steps,
        report.catalog_gates,
        report.backup_retention_days,
        report.vector_dimensions,
        report.function_triggers,
        report.webhook_events,
        report.search_columns,
        report.sidecar_replicas
    );
}

fn print_usage() {
    println!("usage: operator [serve|run-canonical|run-reconcile-plans|run-reconcilers-batch-b]");
}

fn run_reconcile_plans() {
    let report = canonical_reconcile_plans_report().unwrap_or_else(|error| {
        eprintln!("operator: reconcile plans execution failed: {error}");
        process::exit(1);
    });

    println!(
        "cluster_name\tcnpg_instances\tcluster_deployments\ttimescale_enabled\tcoordinator_less\tshard_apply_steps\tshard_topology_constraints\tshard_replication_factor\tshard_hard_constraint"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.cluster_name,
        report.cnpg_instances,
        report.cluster_deployments,
        report.timescale_enabled,
        report.coordinator_less,
        report.shard_apply_steps,
        report.shard_topology_constraints,
        report.shard_replication_factor,
        report.shard_hard_constraint,
    );
}

fn run_reconcilers_batch_b() {
    let report = canonical_reconcilers_batch_b_report().unwrap_or_else(|error| {
        eprintln!("operator: reconcilers batch-b execution failed: {error}");
        process::exit(1);
    });

    println!(
        "federation_apply_steps\tfederation_iceberg\tsearch_apply_steps\tsearch_hybrid\twebhook_apply_steps\twebhook_events\tfunction_apply_steps\tfunction_sidecar_steps\tfunction_kubernetes_steps"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.federation_apply_steps,
        report.federation_iceberg,
        report.search_apply_steps,
        report.search_hybrid,
        report.webhook_apply_steps,
        report.webhook_events,
        report.function_apply_steps,
        report.function_sidecar_steps,
        report.function_kubernetes_steps,
    );
}

/// Spawn the probe server on a dedicated thread (blocking std net) while a
/// tokio runtime drives every kube-rs controller concurrently. The probe
/// server is the readiness signal the operator deployment uses; the
/// controllers reconcile CRDs against the live API server.
fn run_serve(component: &'static str, default_addr: &'static str) {
    let component_owned = component.to_string();
    let default_owned = default_addr.to_string();
    let probe = thread::spawn(move || {
        if let Err(error) = run_probe_server(&component_owned, &default_owned) {
            eprintln!("{component_owned}: probe server failed: {error}");
            process::exit(1);
        }
    });

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|error| {
            eprintln!("{component}: tokio runtime failed: {error}");
            process::exit(1);
        });

    runtime.block_on(async move {
        match kube::Client::try_default().await {
            Ok(client) => {
                if let Err(error) = controllers::serve_all(client).await {
                    eprintln!("{component}: controllers exited: {error}");
                    process::exit(1);
                }
            }
            Err(error) => {
                // No in-cluster kube config: keep probes alive so the
                // deployment surfaces NotReady rather than crash-looping.
                tracing::warn!(?error, "kube client unavailable; running probe-only");
                let _ = probe.join();
            }
        }
    });
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct OperatorExecutionReport {
    crds: usize,
    cluster_workers: u32,
    cluster_sidecars: usize,
    shards: u32,
    hypertable_sql_plans: usize,
    hypertable_apply_steps: usize,
    catalog_gates: usize,
    backup_retention_days: u32,
    vector_dimensions: u32,
    function_triggers: usize,
    webhook_events: usize,
    search_columns: usize,
    sidecar_replicas: u32,
}

fn canonical_operator_execution_report() -> Result<OperatorExecutionReport, Box<dyn Error>> {
    let cluster = canonical_cluster_spec();
    cluster.validate()?;

    let shard_group = canonical_shard_group_spec();
    shard_group.validate()?;

    let hypertable = canonical_hypertable_spec();
    hypertable.validate()?;
    let hypertable_plan = HypertableReconcilePlan::try_from(&hypertable)?;
    let hypertable_apply_plan = hypertable_plan.apply_plan();

    let branch = canonical_branch_spec();
    branch.validate()?;

    let tenant = canonical_tenant_spec();
    tenant.validate()?;

    let region = canonical_region_spec();
    region.validate()?;

    let survival_goal = canonical_survival_goal_spec();
    survival_goal.validate()?;

    let backup = canonical_backup_spec();
    backup.validate()?;

    let vectorizer = canonical_vectorizer_spec();
    vectorizer.validate()?;

    let sidecar = canonical_sidecar_deployment_spec();
    sidecar.validate()?;

    let migration = canonical_migration_spec();
    migration.validate()?;

    let conflict_policy = canonical_conflict_policy_spec();
    conflict_policy.validate()?;

    let federation = canonical_federation_spec();
    federation.validate()?;

    let search_index = canonical_search_index_spec();
    search_index.validate()?;

    let webhook = canonical_webhook_spec();
    webhook.validate()?;

    let function = canonical_function_spec();
    function.validate()?;

    let scheduled_repack = canonical_scheduled_repack_spec();
    scheduled_repack.validate()?;

    Ok(OperatorExecutionReport {
        crds: CANONICAL_OPERATOR_CRDS,
        cluster_workers: cluster.workers,
        cluster_sidecars: cluster.sidecars.len(),
        shards: shard_group.num_shards,
        hypertable_sql_plans: hypertable_plan.sql_plans.len(),
        hypertable_apply_steps: hypertable_apply_plan.steps.len(),
        catalog_gates: V2_OPERATOR_CATALOG_GATES,
        backup_retention_days: backup.retention_days,
        vector_dimensions: vectorizer.destination.dimensions,
        function_triggers: function.triggers.len(),
        webhook_events: webhook.events.len(),
        search_columns: search_index.columns.len(),
        sidecar_replicas: sidecar.replicas,
    })
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ReconcilersBatchBReport {
    federation_apply_steps: usize,
    federation_iceberg: bool,
    search_apply_steps: usize,
    search_hybrid: bool,
    webhook_apply_steps: usize,
    webhook_events: usize,
    function_apply_steps: usize,
    function_sidecar_steps: usize,
    function_kubernetes_steps: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ReconcilePlansReport {
    cluster_name: String,
    cnpg_instances: u32,
    cluster_deployments: usize,
    timescale_enabled: bool,
    coordinator_less: bool,
    shard_apply_steps: usize,
    shard_topology_constraints: usize,
    shard_replication_factor: u32,
    shard_hard_constraint: bool,
}

fn canonical_reconcilers_batch_b_report() -> Result<ReconcilersBatchBReport, Box<dyn Error>> {
    let federation_spec = canonical_federation_spec();
    let federation_plan = FederationReconcilePlan::try_from(&federation_spec)?;
    let federation_apply_plan = federation_plan.apply_plan();

    let search_index_spec = canonical_search_index_spec();
    let search_plan = SearchIndexReconcilePlan::from_spec("documents-search", &search_index_spec)?;
    let search_apply_plan = search_plan.apply_plan();

    let webhook_spec = canonical_webhook_spec();
    let webhook_plan = WebhookReconcilePlan::from_spec("orders-hook", &webhook_spec)?;
    let webhook_apply_plan = webhook_plan.apply_plan();

    let function_spec = canonical_function_spec();
    let function_plan = FunctionReconcilePlan::try_from(&function_spec)?;
    let function_apply_plan = function_plan.apply_plan();

    Ok(ReconcilersBatchBReport {
        federation_apply_steps: federation_apply_plan.steps.len(),
        federation_iceberg: federation_plan.backend.is_iceberg(),
        search_apply_steps: search_apply_plan.steps.len(),
        search_hybrid: search_plan.is_hybrid(),
        webhook_apply_steps: webhook_apply_plan.steps.len(),
        webhook_events: webhook_plan.events.len(),
        function_apply_steps: function_apply_plan.steps.len(),
        function_sidecar_steps: function_apply_plan.step_count_of(FunctionStepKind::Sidecar),
        function_kubernetes_steps: function_apply_plan.step_count_of(FunctionStepKind::Kubernetes),
    })
}

fn canonical_reconcile_plans_report() -> Result<ReconcilePlansReport, Box<dyn Error>> {
    let cluster_spec = canonical_cluster_spec();
    let cluster_plan = CitusClusterReconcilePlan::from_spec("ai-blaise-citus", &cluster_spec)?;

    let shard_group_spec = canonical_shard_group_spec();
    let shard_plan = ShardGroupReconcilePlan::try_from(&shard_group_spec)?;
    let shard_apply_plan = shard_plan.apply_plan();

    Ok(ReconcilePlansReport {
        cluster_name: cluster_plan.cluster_name.clone(),
        cnpg_instances: cluster_plan.total_postgres_instances(),
        cluster_deployments: cluster_plan.total_deployments(),
        timescale_enabled: cluster_plan.timescale_enabled,
        coordinator_less: cluster_plan.coordinator_less,
        shard_apply_steps: shard_apply_plan.steps.len(),
        shard_topology_constraints: shard_plan.topology_constraint_count(),
        shard_replication_factor: shard_plan.replication_factor,
        shard_hard_constraint: shard_plan.has_hard_constraint(),
    })
}

fn canonical_cluster_spec() -> CitusClusterSpec {
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

fn canonical_shard_group_spec() -> ShardGroupSpec {
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

fn canonical_hypertable_spec() -> HypertableSpec {
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

fn canonical_branch_spec() -> BranchSpec {
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

fn canonical_tenant_spec() -> TenantSpec {
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

fn canonical_region_spec() -> RegionSpec {
    RegionSpec {
        name: "us-east-1".to_string(),
        kubernetes_zone: "us-east-1a".to_string(),
        tablespace_name: "ts_us_east_1".to_string(),
        leader_pinned: true,
    }
}

fn canonical_survival_goal_spec() -> SurvivalGoalSpec {
    SurvivalGoalSpec {
        goal: SurvivalGoalType::RegionFailure,
        regions: vec!["us-east-1".to_string(), "us-west-2".to_string()],
        min_replicas: 2,
    }
}

fn canonical_backup_spec() -> BackupSpec {
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

fn canonical_vectorizer_spec() -> VectorizerSpec {
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

fn canonical_sidecar_deployment_spec() -> SidecarDeploymentSpec {
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

fn canonical_migration_spec() -> MigrationSpec {
    MigrationSpec {
        migration_type: MigrationType::Pgroll,
        yaml: "operations:\n  - add_column:\n      table: users".to_string(),
        on_conflict: MigrationConflictAction::ManualReview,
    }
}

fn canonical_conflict_policy_spec() -> ConflictPolicySpec {
    ConflictPolicySpec {
        table: "public.reference_accounts".to_string(),
        class: ConflictClass::UpdateUpdate,
        resolution: ConflictResolution::LastWriteWins,
        custom_function: None,
    }
}

fn canonical_federation_spec() -> FederationSpec {
    FederationSpec {
        name: "warehouse".to_string(),
        federation_type: FederationType::Snowflake,
        connection: FederationConnection {
            secret_ref: "snowflake-warehouse".to_string(),
        },
        foreign_schema_prefix: "snowflake_".to_string(),
    }
}

fn canonical_search_index_spec() -> SearchIndexSpec {
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

fn canonical_webhook_spec() -> WebhookSpec {
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

fn canonical_function_spec() -> FunctionSpec {
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

fn canonical_scheduled_repack_spec() -> ScheduledRepackSpec {
    ScheduledRepackSpec {
        target: "public.orders".to_string(),
        schedule: "0 3 * * 0".to_string(),
        strategy: RepackStrategy::PgRepack,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_report_covers_operator_crds_and_reconcile_plan() {
        let report =
            canonical_operator_execution_report().expect("canonical operator execution report");

        assert_eq!(
            report,
            OperatorExecutionReport {
                crds: 17,
                cluster_workers: 3,
                cluster_sidecars: 3,
                shards: 32,
                hypertable_sql_plans: 5,
                hypertable_apply_steps: 8,
                catalog_gates: 13,
                backup_retention_days: 30,
                vector_dimensions: 3_072,
                function_triggers: 2,
                webhook_events: 2,
                search_columns: 2,
                sidecar_replicas: 2,
            }
        );
    }

    #[test]
    fn canonical_reconcilers_batch_b_report_covers_requested_batch() {
        let report = canonical_reconcilers_batch_b_report().expect("batch-b reconcile report");

        assert_eq!(
            report,
            ReconcilersBatchBReport {
                federation_apply_steps: 4,
                federation_iceberg: true,
                search_apply_steps: 5,
                search_hybrid: true,
                webhook_apply_steps: 6,
                webhook_events: 2,
                function_apply_steps: 6,
                function_sidecar_steps: 1,
                function_kubernetes_steps: 2,
            }
        );
    }

    #[test]
    fn canonical_reconcile_plans_report_covers_cluster_and_shard_group() {
        let report = canonical_reconcile_plans_report().expect("canonical reconcile plans report");

        assert_eq!(
            report,
            ReconcilePlansReport {
                cluster_name: "ai-blaise-citus".to_string(),
                cnpg_instances: 4,
                cluster_deployments: 4,
                timescale_enabled: true,
                coordinator_less: false,
                shard_apply_steps: 5,
                shard_topology_constraints: 1,
                shard_replication_factor: 3,
                shard_hard_constraint: true,
            }
        );
    }
}
