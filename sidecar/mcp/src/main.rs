// FEATURE: MCP1
// FEATURE: MCP2
// FEATURE: MCP3

use ai_blaise_citus_mcp::{McpTool, SafeMode};
use ai_blaise_citus_sidecar_mcp::canonical_mcp_execution_plan;
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
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
    println!("usage: mcp-sidecar [run-canonical]");
    println!("runs the deterministic canonical MCP sidecar plan and emits TSV");
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
