use ai_blaise_citus_pool::{
    canonical_pool_execution_report, run_pool_service_from_env, PoolExecutionReport,
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
        run_server();
        return;
    }

    match args.as_slice() {
        [] => run_canonical(),
        [command] if command == "run-canonical" => run_canonical(),
        _ => {
            eprintln!("pool: unknown command");
            print_usage();
            process::exit(2);
        }
    }
}

fn run_canonical() {
    let report = canonical_pool_execution_report().unwrap_or_else(|error| {
        eprintln!("pool: canonical execution failed: {error}");
        process::exit(1);
    });

    println!("{}", PoolExecutionReport::tsv_header());
    println!("{}", report.tsv_row());
}

fn print_usage() {
    println!("usage: ai_blaise_citus_pool [serve|run-canonical]");
    println!("serve proxies PostgreSQL TCP traffic and exposes admin probes on a separate port");
    println!("run-canonical emits the deterministic canonical pool execution TSV");
}

fn run_server() {
    if let Err(error) = run_pool_service_from_env() {
        eprintln!("pool: service failed: {error}");
        process::exit(1);
    }
}
