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
// FEATURE: O14
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

use ai_blaise_citus_operator::fixtures::{
    canonical_backup_spec, canonical_branch_spec, canonical_cluster_spec,
    canonical_conflict_policy_spec, canonical_federation_spec, canonical_function_spec,
    canonical_hypertable_spec, canonical_migration_spec, canonical_region_spec,
    canonical_scheduled_repack_spec, canonical_search_index_spec, canonical_shard_group_spec,
    canonical_sidecar_deployment_spec, canonical_survival_goal_spec, canonical_tenant_spec,
    canonical_vectorizer_spec, canonical_webhook_spec,
};
use ai_blaise_citus_operator::{
    convert, registered_kind_count, CitusClusterReconcilePlan, ConversionPayload,
    ConversionRequest, HypertableReconcilePlan, ShardGroupReconcilePlan, CRD_CATALOG,
    STORAGE_VERSION, SUPPORTED_VERSIONS,
};
use ai_blaise_citus_sidecar_shared::run_probe_server;
use std::env;
use std::error::Error;
use std::process;

const CANONICAL_OPERATOR_CRDS: usize = 17;
const V2_OPERATOR_CATALOG_GATES: usize = 13;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }
    if args == ["serve"] {
        run_server("operator", "0.0.0.0:8080");
        return;
    }

    match args.as_slice() {
        [] => run_canonical(),
        [command] if command == "run-canonical" => run_canonical(),
        [command] if command == "run-reconcile-plans" => run_reconcile_plans(),
        [command] if command == "run-conversion-canonical" => run_conversion_canonical(),
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
    println!("usage: operator [serve|run-canonical|run-reconcile-plans|run-conversion-canonical]");
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

fn run_conversion_canonical() {
    let report = canonical_conversion_report().unwrap_or_else(|error| {
        eprintln!("operator: conversion canonical run failed: {error}");
        process::exit(1);
    });

    println!(
        "kinds\tserved_versions\tstorage_version\tround_trips_passed\twebhook_path\twebhook_port"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}",
        report.kinds,
        report.served_versions,
        report.storage_version,
        report.round_trips_passed,
        report.webhook_path,
        report.webhook_port
    );
}

fn run_server(component: &str, default_addr: &str) {
    if let Err(error) = run_probe_server(component, default_addr) {
        eprintln!("{component}: probe server failed: {error}");
        process::exit(1);
    }
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

#[derive(Debug, Clone, Eq, PartialEq)]
struct ConversionCanonicalReport {
    kinds: usize,
    served_versions: usize,
    storage_version: &'static str,
    round_trips_passed: usize,
    webhook_path: &'static str,
    webhook_port: u16,
}

fn canonical_conversion_report() -> Result<ConversionCanonicalReport, Box<dyn Error>> {
    let payloads = canonical_conversion_payloads();
    assert_eq!(
        payloads.len(),
        CRD_CATALOG.len(),
        "canonical payload set must cover every CRD"
    );

    let mut round_trips = 0;
    for payload in &payloads {
        for target in ["v1beta1", "v1alpha1"] {
            let source = if target == "v1beta1" {
                "v1alpha1"
            } else {
                "v1beta1"
            };
            let request = ConversionRequest {
                source_api_version: source.to_string(),
                target_api_version: target.to_string(),
                kind: payload.kind_name().to_string(),
                payload: payload.clone(),
            };
            let response = convert(&request)?;
            if response.payload != *payload {
                return Err(format!(
                    "round-trip for {kind} {source}->{target} altered the payload",
                    kind = payload.kind_name(),
                )
                .into());
            }
        }
        round_trips += 1;
    }

    Ok(ConversionCanonicalReport {
        kinds: registered_kind_count(),
        served_versions: SUPPORTED_VERSIONS.len(),
        storage_version: STORAGE_VERSION,
        round_trips_passed: round_trips,
        webhook_path: ai_blaise_citus_operator::CONVERSION_WEBHOOK_PATH,
        webhook_port: ai_blaise_citus_operator::CONVERSION_WEBHOOK_PORT,
    })
}

fn canonical_conversion_payloads() -> Vec<ConversionPayload> {
    vec![
        ConversionPayload::CitusCluster(canonical_cluster_spec()),
        ConversionPayload::ShardGroup(canonical_shard_group_spec()),
        ConversionPayload::Hypertable(canonical_hypertable_spec()),
        ConversionPayload::Branch(canonical_branch_spec()),
        ConversionPayload::Vectorizer(canonical_vectorizer_spec()),
        ConversionPayload::Sidecar(canonical_sidecar_deployment_spec()),
        ConversionPayload::Migration(canonical_migration_spec()),
        ConversionPayload::ConflictPolicy(canonical_conflict_policy_spec()),
        ConversionPayload::Tenant(canonical_tenant_spec()),
        ConversionPayload::Region(canonical_region_spec()),
        ConversionPayload::SurvivalGoal(canonical_survival_goal_spec()),
        ConversionPayload::Backup(canonical_backup_spec()),
        ConversionPayload::Federation(canonical_federation_spec()),
        ConversionPayload::SearchIndex(canonical_search_index_spec()),
        ConversionPayload::Webhook(canonical_webhook_spec()),
        ConversionPayload::Function(canonical_function_spec()),
        ConversionPayload::ScheduledRepack(canonical_scheduled_repack_spec()),
    ]
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

    #[test]
    fn canonical_conversion_report_round_trips_every_kind() {
        let report = canonical_conversion_report().expect("canonical conversion report");

        assert_eq!(
            report,
            ConversionCanonicalReport {
                kinds: 17,
                served_versions: 2,
                storage_version: "v1alpha1",
                round_trips_passed: 17,
                webhook_path: "/convert",
                webhook_port: 8443,
            }
        );
    }
}
