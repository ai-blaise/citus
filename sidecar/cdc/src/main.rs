// FEATURE: C1
// FEATURE: C2
// FEATURE: C3
// FEATURE: C14
// FEATURE: C15

use ai_blaise_citus_sidecar_cdc::{canonical_cdc_event, canonical_delivery_plan, CdcOperation};
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
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

fn print_usage() {
    println!("usage: cdc [run-canonical]");
    println!("runs the deterministic canonical CDC delivery plan and emits TSV");
}

fn operation_name(operation: &CdcOperation) -> &'static str {
    match operation {
        CdcOperation::Insert => "insert",
        CdcOperation::Update => "update",
        CdcOperation::Delete => "delete",
        CdcOperation::Truncate => "truncate",
    }
}
