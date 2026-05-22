use ai_blaise_citus_mcp::{canonical_mcp_execution_report, handle_mcp_stdio_request};
use std::env;
use std::io::{self, BufRead, Write};
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if args == ["serve-stdio"] {
        run_stdio_server();
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
    println!("usage: citus-mcp [run-canonical|serve-stdio]");
    println!("runs the deterministic canonical report or a line-delimited MCP stdio server");
}

fn run_stdio_server() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                eprintln!("citus-mcp: failed reading stdin: {error}");
                process::exit(1);
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let response = handle_mcp_stdio_request(&line);
        if let Err(error) = writeln!(stdout, "{response}") {
            eprintln!("citus-mcp: failed writing stdout: {error}");
            process::exit(1);
        }
        if let Err(error) = stdout.flush() {
            eprintln!("citus-mcp: failed flushing stdout: {error}");
            process::exit(1);
        }
    }
}
