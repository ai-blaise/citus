// FEATURE: R7

use ai_blaise_citus_sidecar_repack::canonical_repack_report;
use ai_blaise_citus_sidecar_shared::RepackExecutionStrategy;
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("repack: unknown command");
        print_usage();
        process::exit(2);
    }

    let report = canonical_repack_report().unwrap_or_else(|error| {
        eprintln!("repack: canonical report failed: {error}");
        process::exit(1);
    });
    let first_shard = &report.job.shard_targets[0];

    println!(
        "target\tstrategy\tschedule\tmax_concurrency\tlock_timeout_ms\tshard_count\tfirst_shard_id\tfirst_worker\tfirst_table\texecutable\targs"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.job.contract.target,
        strategy_name(&report.job.contract.strategy),
        report.job.schedule,
        report.job.contract.max_concurrency,
        report.command.lock_timeout_ms,
        report.command.shard_count,
        first_shard.shard_id,
        first_shard.worker,
        first_shard.table,
        report.command.executable,
        report.command.args.join(" "),
    );
}

fn print_usage() {
    println!("usage: repack [run-canonical]");
    println!("runs the deterministic canonical repack sidecar plan and emits TSV");
}

fn strategy_name(strategy: &RepackExecutionStrategy) -> &'static str {
    match strategy {
        RepackExecutionStrategy::PgRepack => "pg_repack",
        RepackExecutionStrategy::RepackConcurrentlyPg19 => "repack_concurrently_pg19",
    }
}
