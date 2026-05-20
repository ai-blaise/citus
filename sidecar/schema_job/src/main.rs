// FEATURE: C10
// FEATURE: M2

use ai_blaise_citus_companion::{SchemaJobOperation, SchemaJobState};
use ai_blaise_citus_sidecar_schema_job::{
    canonical_schema_job_report, SchemaJobAction, SchemaJobWorkerPlan,
};
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
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
    let shadow = report.worker.shadow.as_ref();

    println!(
        "job\ttable\tstate\toperation\tworker_id\tlease_holder\tlease_epoch\taction\tbatch_size\tmax_parallel_shards\tthrottle_ms\tmax_lock_ms\tallow_blocking_cutover\tshadow_table"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.worker.job.name,
        report.worker.job.table,
        state_name(&report.worker.job.state),
        operation_name(&report.worker),
        report.worker.worker_id,
        report.worker.lease.holder,
        report.worker.lease.epoch,
        action_name(&report.action),
        report.worker.backfill.batch_size,
        report.worker.backfill.max_parallel_shards,
        report.worker.backfill.throttle_ms,
        report.worker.safety.max_lock_ms,
        report.worker.safety.allow_blocking_cutover,
        shadow
            .map(|plan| plan.shadow_table.as_str())
            .unwrap_or("none"),
    );
}

fn print_usage() {
    println!("usage: schema-job [run-canonical]");
    println!("runs the deterministic canonical schema-job sidecar plan and emits TSV");
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
