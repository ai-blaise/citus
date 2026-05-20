// FEATURE: C1
// FEATURE: C2
// FEATURE: C3
// FEATURE: C14
// FEATURE: C15

use ai_blaise_citus_sidecar_cdc::{
    canonical_cdc_event, canonical_cdc_runtime_report, canonical_delivery_plan, CdcOperation,
};
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if args == ["run-runtime-canonical"] {
        run_runtime_canonical();
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("cdc: unknown command");
        print_usage();
        process::exit(2);
    }

    let event = canonical_cdc_event();
    let delivery = canonical_delivery_plan().unwrap_or_else(|error| {
        eprintln!("cdc: canonical delivery failed: {error}");
        process::exit(1);
    });
    let anonymized_columns = delivery.anonymized_columns.join(",");

    println!("lsn\ttable\ttenant_id\toperation\tsink\ttarget\tmax_attempts\tdead_letter_queue\tanonymized_columns");
    for sink in &delivery.routed_sinks {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            delivery.event_lsn,
            delivery.table,
            event.tenant_id,
            operation_name(&delivery.operation),
            sink.sink,
            sink.target,
            sink.retry_policy.max_attempts,
            sink.retry_policy.dead_letter_queue,
            anonymized_columns,
        );
    }
}

fn run_runtime_canonical() {
    let report = canonical_cdc_runtime_report().unwrap_or_else(|error| {
        eprintln!("cdc: canonical runtime failed: {error}");
        process::exit(1);
    });
    let runtime_delivery = &report.batch.deliveries[0];
    let anonymized_columns = runtime_delivery.delivery.anonymized_columns.join(",");

    println!(
        "slot\tstart_lsn\tend_lsn\tevent_count\tdelivered_sinks\tack_flush_lsn\tlast_delivered_lsn\ttable\ttenant_id\toperation\tanonymized_columns"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.state.slot_name,
        report.batch.start_lsn,
        report.batch.end_lsn,
        report.batch.deliveries.len(),
        report.state.delivered_sink_writes,
        report.batch.ack.flush_lsn,
        report.state.last_delivered_lsn,
        runtime_delivery.delivery.table,
        runtime_delivery.event.tenant_id,
        operation_name(&runtime_delivery.delivery.operation),
        anonymized_columns,
    );
}

fn print_usage() {
    println!("usage: cdc [run-canonical|run-runtime-canonical]");
    println!("runs deterministic canonical CDC delivery/runtime plans and emits TSV");
}

fn operation_name(operation: &CdcOperation) -> &'static str {
    match operation {
        CdcOperation::Insert => "insert",
        CdcOperation::Update => "update",
        CdcOperation::Delete => "delete",
        CdcOperation::Truncate => "truncate",
    }
}
