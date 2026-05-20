// FEATURE: EF1
// FEATURE: EF2
// FEATURE: EF4
// FEATURE: EF5

use ai_blaise_citus_sidecar_cdc::CdcOperation;
use ai_blaise_citus_sidecar_edge_functions::{canonical_edge_function_report, FunctionTrigger};
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("edge-functions: unknown command");
        print_usage();
        process::exit(2);
    }

    let report = canonical_edge_function_report().unwrap_or_else(|error| {
        eprintln!("edge-functions: canonical report failed: {error}");
        process::exit(1);
    });

    println!(
        "function\texecutable\targs\tsecret_refs\tdb_callback_socket\ttrigger\ttenant_id\tpayload_bytes\ttimeout_ms"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.launch.function_name,
        report.launch.executable,
        report.launch.args.join(" "),
        report.launch.env_secret_refs.join(","),
        report
            .launch
            .db_callback_socket
            .as_deref()
            .unwrap_or("none"),
        trigger_name(&report.invocation.trigger),
        report.invocation.tenant_id,
        report.invocation.payload_bytes,
        report.invocation.timeout_ms,
    );
}

fn print_usage() {
    println!("usage: edge-functions [run-canonical]");
    println!("runs the deterministic canonical edge-function launch plan and emits TSV");
}

fn trigger_name(trigger: &FunctionTrigger) -> String {
    match trigger {
        FunctionTrigger::Http { path } => format!("http:{path}"),
        FunctionTrigger::Scheduled { schedule } => format!("scheduled:{schedule}"),
        FunctionTrigger::CdcEvent { table, operation } => {
            format!("cdc:{table}:{}", operation_name(operation))
        }
    }
}

fn operation_name(operation: &CdcOperation) -> &'static str {
    match operation {
        CdcOperation::Insert => "insert",
        CdcOperation::Update => "update",
        CdcOperation::Delete => "delete",
        CdcOperation::Truncate => "truncate",
    }
}
