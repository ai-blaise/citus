// FEATURE: B1
// FEATURE: B3
// FEATURE: B4
// FEATURE: B6

use ai_blaise_citus_sidecar_backup::{canonical_backup_report, WalCompression};
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("backup: unknown command");
        print_usage();
        process::exit(2);
    }

    let report = canonical_backup_report().unwrap_or_else(|error| {
        eprintln!("backup: canonical report failed: {error}");
        process::exit(1);
    });

    println!(
        "cluster\tarchive_uri\tbase_destination\twal_archive\twal_compression\tretention_days\tconcurrency\tpitr_target\tqueryable_branch\tkms_key_ref"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.job.cluster,
        report.job.contract.archive_uri,
        report.job.base_backup.destination_uri,
        report.job.wal_archive.archive_uri,
        compression_name(&report.job.wal_archive.compression),
        report.job.base_backup.retention_days,
        report.job.base_backup.concurrency,
        report.restore.target_time,
        report.queryable_branch.branch_name,
        report
            .job
            .encryption
            .as_ref()
            .map(|encryption| encryption.kms_key_ref.as_str())
            .unwrap_or("none"),
    );
}

fn print_usage() {
    println!("usage: backup [run-canonical]");
    println!("runs the deterministic canonical backup sidecar plan and emits TSV");
}

fn compression_name(compression: &WalCompression) -> &'static str {
    match compression {
        WalCompression::None => "none",
        WalCompression::Gzip => "gzip",
        WalCompression::Zstd => "zstd",
    }
}
