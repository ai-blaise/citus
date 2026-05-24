// FEATURE: C10
// FEATURE: M2

use ai_blaise_citus_companion::{SchemaJobOperation, SchemaJobState};
use ai_blaise_citus_sidecar_schema_job::{
    canonical_controller_tick_reports, canonical_schema_job_report, parse_worker_plan_manifest,
    ControllerTickDecision, SchemaJobAction, SchemaJobWorkerPlan,
};
use ai_blaise_citus_sidecar_shared::run_probe_server;
use std::env;
use std::fs;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if args == ["serve"] {
        run_server("schema-job", "0.0.0.0:8080");
        return;
    }

    if args == ["run-controller-canonical"] {
        emit_controller_canonical();
        return;
    }

    if args.first().map(String::as_str) == Some("validate-manifest") && args.len() == 2 {
        validate_manifest(&args[1]);
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("schema-job: unknown command");
        print_usage();
        process::exit(2);
    }

    let report = canonical_schema_job_report().unwrap_or_else(|error| {
        eprintln!("schema-job: canonical report failed: {error}");
        process::exit(1);
    });
    emit_worker_report(&report.worker, &report.action);
}

fn emit_worker_report(worker: &SchemaJobWorkerPlan, action: &SchemaJobAction) {
    let shadow = worker.shadow.as_ref();

    println!(
        "job\ttable\tstate\toperation\tworker_id\tlease_holder\tlease_epoch\taction\tbatch_size\tmax_parallel_shards\tthrottle_ms\tmax_lock_ms\tallow_blocking_cutover\tshadow_table"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        worker.job.name,
        worker.job.table,
        state_name(&worker.job.state),
        operation_name(worker),
        worker.worker_id,
        worker.lease.holder,
        worker.lease.epoch,
        action_name(action),
        worker.backfill.batch_size,
        worker.backfill.max_parallel_shards,
        worker.backfill.throttle_ms,
        worker.safety.max_lock_ms,
        worker.safety.allow_blocking_cutover,
        shadow
            .map(|plan| plan.shadow_table.as_str())
            .unwrap_or("none"),
    );
}

fn validate_manifest(path: &str) {
    let raw = fs::read_to_string(path).unwrap_or_else(|error| {
        eprintln!("schema-job: could not read manifest {path}: {error}");
        process::exit(1);
    });
    let worker = parse_worker_plan_manifest(&raw).unwrap_or_else(|error| {
        eprintln!("schema-job: manifest validation failed: {error}");
        process::exit(1);
    });
    let action = worker.next_action().unwrap_or_else(|error| {
        eprintln!("schema-job: manifest action failed: {error}");
        process::exit(1);
    });
    emit_worker_report(&worker, &action);
}

fn emit_controller_canonical() {
    let reports = canonical_controller_tick_reports().unwrap_or_else(|error| {
        eprintln!("schema-job: controller canonical report failed: {error}");
        process::exit(1);
    });
    println!(
        "scenario\tjob\tfrom_state\ttarget_state\tdecision\tsql_count\ttwo_version_check\tdelinquent_workers\trollback_steps"
    );
    for report in reports {
        let (scenario, delinquent_workers, rollback_steps) = match &report.decision {
            ControllerTickDecision::Advance(_) => ("advance", String::new(), 0),
            ControllerTickDecision::Wait { delinquent_workers } => {
                ("wait", delinquent_workers.join(","), 0)
            }
            ControllerTickDecision::Rollback { rollback, .. } => {
                ("rollback", String::new(), rollback.steps.len())
            }
        };
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            scenario,
            report.job_name,
            state_name(&report.from_state),
            state_name(&report.target_state),
            report.decision.as_canonical(),
            report.sql_statements.len(),
            report
                .two_version_invariant_check_sql
                .contains("verify_two_version_invariant"),
            delinquent_workers,
            rollback_steps,
        );
    }
}

fn print_usage() {
    println!(
        "usage: schema-job [serve|run-canonical|run-controller-canonical|validate-manifest PATH]"
    );
    println!("runs deterministic schema-job sidecar plans, controller ticks, or manifest validation and emits TSV");
}

fn run_server(component: &str, default_addr: &str) {
    if let Err(error) = run_probe_server(component, default_addr) {
        eprintln!("{component}: probe server failed: {error}");
        process::exit(1);
    }
}

fn state_name(state: &SchemaJobState) -> &'static str {
    match state {
        SchemaJobState::DeleteOnly => "delete_only",
        SchemaJobState::WriteOnly => "write_only",
        SchemaJobState::Backfill => "backfill",
        SchemaJobState::Public => "public",
        SchemaJobState::Paused => "paused",
        SchemaJobState::Canceled => "canceled",
    }
}

fn operation_name(worker: &SchemaJobWorkerPlan) -> String {
    worker
        .job
        .operations
        .iter()
        .map(|operation| match operation {
            SchemaJobOperation::AddColumn { column, sql_type } => {
                format!("add_column:{column}:{sql_type}")
            }
            SchemaJobOperation::Backfill { statement } => format!("backfill:{statement}"),
            SchemaJobOperation::SwapColumn {
                old_column,
                new_column,
            } => {
                format!("swap_column:{old_column}:{new_column}")
            }
            SchemaJobOperation::DropColumn { column } => format!("drop_column:{column}"),
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn action_name(action: &SchemaJobAction) -> &'static str {
    match action {
        SchemaJobAction::AcquireLease => "acquire_lease",
        SchemaJobAction::ApplyDeleteOnly => "apply_delete_only",
        SchemaJobAction::ApplyWriteOnly => "apply_write_only",
        SchemaJobAction::RunBackfill { .. } => "run_backfill",
        SchemaJobAction::Publish => "publish",
        SchemaJobAction::StopPaused => "stop_paused",
        SchemaJobAction::StopCanceled => "stop_canceled",
    }
}
