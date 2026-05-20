use ai_blaise_citus_pool::canonical_pool_execution_report;
use ai_blaise_citus_sidecar_shared::run_probe_server;
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }
    if args == ["serve"] {
        run_server("pool", "0.0.0.0:5432");
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

    println!(
        "tracked_gucs\tsettings_bucket_max_connections\tfast_path_routes\tmirror_sample_percent\thtap_max_staleness_ms\tpipeline_max_in_flight\ttransaction_pipelining\ttls_rotation_seconds\ttenant_burst\tgeo_rules\ttoken_cache_entries\tplan_cache_entries_before_invalidation\tinvalidated_plans\tsingle_shard_generation"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.tracked_gucs,
        report.settings_bucket_max_connections,
        report.fast_path_routes,
        report.mirror_sample_percent,
        report.htap_max_staleness_ms,
        report.pipeline_max_in_flight,
        report.transaction_pipelining,
        report.tls_rotation_seconds,
        report.tenant_burst,
        report.geo_rules,
        report.token_cache_entries,
        report.plan_cache_entries_before_invalidation,
        report.invalidated_plans,
        report.single_shard_generation,
    );
}

fn print_usage() {
    println!("usage: ai_blaise_citus_pool [serve|run-canonical]");
    println!("runs the deterministic canonical pool execution report and emits TSV");
}

fn run_server(component: &str, default_addr: &str) {
    if let Err(error) = run_probe_server(component, default_addr) {
        eprintln!("{component}: probe server failed: {error}");
        process::exit(1);
    }
}
