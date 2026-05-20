// FEATURE: S9

use ai_blaise_citus_sidecar_hlc::canonical_hlc_report;
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("hlc: unknown command");
        print_usage();
        process::exit(2);
    }

    let report = canonical_hlc_report().unwrap_or_else(|error| {
        eprintln!("hlc: canonical report failed: {error}");
        process::exit(1);
    });
    let closed = &report.follower_read.closed_timestamp;

    println!(
        "node_id\ttick_physical_ms\ttick_logical\tobserved_physical_ms\tobserved_logical\tfollower_replica\tas_of_physical_ms\tclosed_shard_group\tclosed_physical_ms\tclosed_logical\tmax_staleness_ms\treplica_count"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.ticked_clock.node_id,
        report.ticked_clock.timestamp.physical_ms,
        report.ticked_clock.timestamp.logical,
        report.observed_clock.timestamp.physical_ms,
        report.observed_clock.timestamp.logical,
        report.follower_read.replica,
        report.follower_read.as_of.physical_ms,
        closed.shard_group,
        closed.closed_at.physical_ms,
        closed.closed_at.logical,
        closed.max_staleness_ms,
        closed.replica_count,
    );
}

fn print_usage() {
    println!("usage: hlc [run-canonical]");
    println!("runs the deterministic canonical HLC sidecar plan and emits TSV");
}
