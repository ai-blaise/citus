use ai_blaise_citus_mcp::canonical_mcp_execution_report;
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("citus-mcp: unknown command");
        print_usage();
        process::exit(2);
    }

    let report = canonical_mcp_execution_report().unwrap_or_else(|error| {
        eprintln!("citus-mcp: canonical execution failed: {error}");
        process::exit(1);
    });

    println!("requests\ttenant_scoped_requests\tsafe_mode_required\tdestructive_denials");
    println!(
        "{}\t{}\t{}\t{}",
        report.requests,
        report.tenant_scoped_requests,
        report.safe_mode_required,
        report.destructive_denials,
    );
}

fn print_usage() {
    println!("usage: citus-mcp [run-canonical]");
    println!("runs the deterministic canonical MCP tool policy report and emits TSV");
}
