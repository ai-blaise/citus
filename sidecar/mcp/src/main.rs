// FEATURE: MCP1
// FEATURE: MCP2
// FEATURE: MCP3
// FEATURE: D11

use ai_blaise_citus_mcp::{McpTool, SafeMode};
use ai_blaise_citus_sidecar_mcp::{canonical_mcp_execution_plan, handle_mcp_sidecar_stdio_request};
use ai_blaise_citus_sidecar_shared::run_probe_server;
use std::env;
use std::io::{self, BufRead, Write};
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if args == ["serve"] {
        run_server("mcp-sidecar", "0.0.0.0:8080");
        return;
    }

    if args == ["serve-stdio"] {
        run_stdio_server();
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("mcp-sidecar: unknown command");
        print_usage();
        process::exit(2);
    }

    let plan = canonical_mcp_execution_plan().unwrap_or_else(|error| {
        eprintln!("mcp-sidecar: canonical plan failed: {error}");
        process::exit(1);
    });
    let request = &plan.allowed_requests[0];
    let scope = request.tenant_scope.as_ref().expect("canonical scope");

    println!(
        "listen_addr\tissuer\taudience\tsafe_mode\tmax_sessions\tidle_timeout_seconds\ttool\ttenant_id\tallowed_schemas"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        plan.listen_addr,
        plan.auth.issuer,
        plan.auth.audience,
        safe_mode_name(&plan.session_policy.safe_mode),
        plan.session_policy.max_concurrent_sessions,
        plan.session_policy.idle_timeout_seconds,
        tool_name(&request.tool),
        scope.tenant_id,
        scope.allowed_schemas.join(","),
    );
}

fn print_usage() {
    println!("usage: mcp-sidecar [serve|serve-stdio|run-canonical]");
    println!("runs probes, a line-delimited MCP stdio policy server, or the canonical TSV plan");
}

fn run_server(component: &str, default_addr: &str) {
    if let Err(error) = run_probe_server(component, default_addr) {
        eprintln!("{component}: probe server failed: {error}");
        process::exit(1);
    }
}

fn run_stdio_server() {
    if let Err(error) = canonical_mcp_execution_plan() {
        eprintln!("mcp-sidecar: canonical plan failed: {error}");
        process::exit(1);
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                eprintln!("mcp-sidecar: failed reading stdin: {error}");
                process::exit(1);
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let response = handle_mcp_sidecar_stdio_request(&line).unwrap_or_else(|error| {
            eprintln!("mcp-sidecar: MCP stdio policy failed: {error}");
            process::exit(1);
        });
        if let Err(error) = writeln!(stdout, "{response}") {
            eprintln!("mcp-sidecar: failed writing stdout: {error}");
            process::exit(1);
        }
        if let Err(error) = stdout.flush() {
            eprintln!("mcp-sidecar: failed flushing stdout: {error}");
            process::exit(1);
        }
    }
}

fn safe_mode_name(safe_mode: &SafeMode) -> &'static str {
    match safe_mode {
        SafeMode::Required => "required",
        SafeMode::Disabled => "disabled",
    }
}

fn tool_name(tool: &McpTool) -> &'static str {
    match tool {
        McpTool::ListShards => "list_shards",
        McpTool::ListHypertables => "list_hypertables",
        McpTool::RunExplain { .. } => "run_explain",
        McpTool::RebalanceDryRun { .. } => "rebalance_dry_run",
        McpTool::SuggestIndex { .. } => "suggest_index",
        McpTool::QueryWithTimeout { .. } => "query_with_timeout",
        McpTool::CurrentLag => "current_lag",
        McpTool::CurrentReplicationStatus => "current_replication_status",
        McpTool::TenantArchive { .. } => "tenant_archive",
    }
}
