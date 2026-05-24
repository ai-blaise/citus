// FEATURE: R7

use ai_blaise_citus_sidecar_repack::{
    canonical_repack_job, canonical_repack_report, execute_live_pg_repack,
    RepackLiveExecutionRequest, RepackRuntimeEnvironment,
};
use ai_blaise_citus_sidecar_shared::{run_probe_server, RepackExecutionStrategy};
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if args == ["serve"] {
        run_server("repack", "0.0.0.0:8080");
        return;
    }

    if args == ["run-live-pg-repack"] {
        run_live_pg_repack();
        return;
    }

    if args.is_empty() || args == ["run-canonical"] {
        run_canonical();
        return;
    }

    eprintln!("repack: unknown command");
    print_usage();
    process::exit(2);
}

fn run_canonical() {
    let report = canonical_repack_report().unwrap_or_else(|error| {
        eprintln!("repack: canonical report failed: {error}");
        process::exit(1);
    });
    let first_shard = &report.job.shard_targets[0];

    println!(
        "target\tstrategy\tschedule\tmax_concurrency\tlock_timeout_ms\tshard_count\tfirst_shard_id\tfirst_worker\tfirst_table\tpg_major\tpg_repack_available\tpg19_repack_concurrently_available\tdry_run\texecuted\tevidence_boundary\texecutable\targs"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.job.contract.target,
        strategy_name(&report.execution.selected_strategy),
        report.job.schedule,
        report.job.contract.max_concurrency,
        report.command.lock_timeout_ms,
        report.command.shard_count,
        first_shard.shard_id,
        first_shard.worker,
        first_shard.table,
        report.environment.pg_major,
        report.environment.pg_repack_available,
        report.environment.repack_concurrently_available,
        report.execution.dry_run,
        report.execution.executed,
        report.execution.evidence_boundary,
        report.command.executable,
        report.command.args.join(" "),
    );
}

fn run_live_pg_repack() {
    let request = live_request_from_env().unwrap_or_else(|error| {
        eprintln!("repack: invalid live pg_repack request: {error}");
        process::exit(2);
    });
    let report = execute_live_pg_repack(&request).unwrap_or_else(|error| {
        eprintln!("repack: live pg_repack execution failed: {error}");
        process::exit(1);
    });

    println!(
        "target\tstrategy\tdry_run\texecuted\texit_code\tevidence_boundary\texecutable\targs\tstdout_bytes\tstderr_bytes"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.target,
        strategy_name(&report.strategy),
        report.dry_run,
        report.executed,
        report.exit_code,
        report.evidence_boundary,
        report.executable,
        report.redacted_args.join(" "),
        report.stdout.len(),
        report.stderr.len(),
    );
}

fn live_request_from_env() -> Result<RepackLiveExecutionRequest, String> {
    let database_url = env::var("AI_BLAISE_REPACK_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .map_err(|_| {
            "AI_BLAISE_REPACK_DATABASE_URL or DATABASE_URL must be set for live execution"
                .to_string()
        })?;
    let executable =
        env::var("AI_BLAISE_REPACK_BINARY").unwrap_or_else(|_| "pg_repack".to_string());
    let mut job = canonical_repack_job();
    job.contract.target =
        env::var("AI_BLAISE_REPACK_TARGET").unwrap_or_else(|_| job.contract.target.clone());
    job.contract.max_concurrency =
        parse_env_u32("AI_BLAISE_REPACK_JOBS", job.contract.max_concurrency)?;
    job.lock_timeout_ms = parse_env_u32("AI_BLAISE_REPACK_LOCK_TIMEOUT_MS", job.lock_timeout_ms)?;
    let wait_default = job.lock_timeout_ms.saturating_add(999) / 1000;
    let wait_timeout_secs =
        parse_env_u32("AI_BLAISE_REPACK_WAIT_TIMEOUT_SECS", wait_default.max(1))?;
    let pg_major = parse_env_u16("AI_BLAISE_REPACK_PG_MAJOR", 17)?;

    Ok(RepackLiveExecutionRequest {
        job,
        environment: RepackRuntimeEnvironment {
            pg_major,
            pg_repack_available: true,
            repack_concurrently_available: false,
        },
        database_url,
        executable,
        wait_timeout_secs,
    })
}

fn parse_env_u32(name: &str, default: u32) -> Result<u32, String> {
    match env::var(name) {
        Ok(value) => value
            .parse::<u32>()
            .map_err(|_| format!("{name} must be an unsigned integer")),
        Err(_) => Ok(default),
    }
}

fn parse_env_u16(name: &str, default: u16) -> Result<u16, String> {
    match env::var(name) {
        Ok(value) => value
            .parse::<u16>()
            .map_err(|_| format!("{name} must be an unsigned integer")),
        Err(_) => Ok(default),
    }
}

fn print_usage() {
    println!("usage: repack [serve|run-canonical|run-live-pg-repack]");
    println!("run-canonical emits the deterministic repack sidecar plan as TSV");
    println!("run-live-pg-repack executes pg_repack using AI_BLAISE_REPACK_DATABASE_URL");
}

fn run_server(component: &str, default_addr: &str) {
    if let Err(error) = run_probe_server(component, default_addr) {
        eprintln!("{component}: probe server failed: {error}");
        process::exit(1);
    }
}

fn strategy_name(strategy: &RepackExecutionStrategy) -> &'static str {
    match strategy {
        RepackExecutionStrategy::PgRepack => "pg_repack",
        RepackExecutionStrategy::RepackConcurrentlyPg19 => "repack_concurrently_pg19",
    }
}
