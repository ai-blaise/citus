// FEATURE: EF1
// FEATURE: EF2
// FEATURE: EF3
// FEATURE: EF4
// FEATURE: EF5
// FEATURE: EF6

use ai_blaise_citus_sidecar_cdc::CdcOperation;
use ai_blaise_citus_sidecar_edge_functions::{
    canonical_bun_edge_function_runtime_report, canonical_edge_function_registry_report,
    canonical_edge_function_report, canonical_edge_function_runtime_report,
    serve_edge_functions_sidecar_http_forever, EdgeFunctionRuntime, EdgeFunctionRuntimeReport,
    EdgeFunctionTriggerKind, FunctionTrigger, InvocationStatus,
};
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if args == ["serve"] {
        run_http_server("0.0.0.0:8080");
        return;
    }

    if args == ["run-runtime-canonical"] {
        run_runtime_canonical();
        return;
    }

    if args == ["run-registry-canonical"] {
        run_registry_canonical();
        return;
    }

    if args == ["run-bun-runtime-canonical"] {
        run_bun_runtime_canonical();
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

fn run_runtime_canonical() {
    let report = canonical_edge_function_runtime_report().unwrap_or_else(|error| {
        eprintln!("edge-functions: canonical runtime report failed: {error}");
        process::exit(1);
    });

    println!(
        "function\truntime\tcommand\ttrigger\ttenant_id\tpayload_bytes\tresponse_bytes\tdb_callback_used\tlaunched_functions\tinvocations\tdb_callbacks\tstatus"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.execution.function_name,
        runtime_name(&report.execution.runtime),
        report.execution.command.join(" "),
        trigger_name(&report.execution.trigger),
        report.execution.tenant_id,
        report.execution.payload_bytes,
        report.execution.response_bytes,
        report.execution.db_callback_used,
        report.state.launched_functions,
        report.state.invocations,
        report.state.db_callbacks,
        status_name(&report.execution.status),
    );
}


fn run_bun_runtime_canonical() {
    let report = canonical_bun_edge_function_runtime_report().unwrap_or_else(|error| {
        eprintln!("edge-functions: canonical Bun runtime report failed: {error}");
        process::exit(1);
    });
    print_runtime_report(&report);
}

fn print_runtime_report(report: &EdgeFunctionRuntimeReport) {
    println!(
        "function	runtime	command	trigger	tenant_id	payload_bytes	response_bytes	db_callback_used	launched_functions	invocations	db_callbacks	status"
    );
    println!(
        "{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}",
        report.execution.function_name,
        runtime_name(&report.execution.runtime),
        report.execution.command.join(" "),
        trigger_name(&report.execution.trigger),
        report.execution.tenant_id,
        report.execution.payload_bytes,
        report.execution.response_bytes,
        report.execution.db_callback_used,
        report.state.launched_functions,
        report.state.invocations,
        report.state.db_callbacks,
        status_name(&report.execution.status),
    );
}

fn run_registry_canonical() {
    let report = canonical_edge_function_registry_report().unwrap_or_else(|error| {
        eprintln!("edge-functions: canonical registry report failed: {error}");
        process::exit(1);
    });
    let summary = &report.snapshot.functions[0];
    println!(
        "function\truntime\ttriggers\tdb_callback_socket\tinvocations\ttriggered_invocations\tdb_callbacks\tdue_schedules\tcdc_events\tuds_socket\tuds_statements"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        summary.name,
        runtime_name(&summary.runtime),
        summary
            .triggers
            .iter()
            .map(|kind| trigger_kind_name(kind).to_string())
            .collect::<Vec<_>>()
            .join(","),
        summary.db_callback_socket.as_deref().unwrap_or("none"),
        report.snapshot.invocations,
        report.snapshot.triggered_invocations,
        report.snapshot.db_callbacks,
        report.due_schedules.len(),
        report.events.len(),
        report.uds_callback.socket_path,
        report.uds_callback.statements().len(),
    );
}

fn print_usage() {
    println!(
        "usage: edge-functions [serve|run-canonical|run-runtime-canonical|run-registry-canonical|run-bun-runtime-canonical]"
    );
    println!("serves the edge-functions sidecar or emits canonical TSV reports");
}

fn run_http_server(default_addr: &str) {
    if let Err(error) = serve_edge_functions_sidecar_http_forever(default_addr) {
        eprintln!("edge-functions: HTTP server failed: {error}");
        process::exit(1);
    }
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

fn trigger_kind_name(kind: &EdgeFunctionTriggerKind) -> &'static str {
    kind.as_str()
}

fn operation_name(operation: &CdcOperation) -> &'static str {
    match operation {
        CdcOperation::Insert => "insert",
        CdcOperation::Update => "update",
        CdcOperation::Delete => "delete",
        CdcOperation::Truncate => "truncate",
    }
}

fn runtime_name(runtime: &EdgeFunctionRuntime) -> &'static str {
    match runtime {
        EdgeFunctionRuntime::Deno => "deno",
        EdgeFunctionRuntime::Bun => "bun",
    }
}

fn status_name(status: &InvocationStatus) -> &'static str {
    match status {
        InvocationStatus::Succeeded => "succeeded",
    }
}
