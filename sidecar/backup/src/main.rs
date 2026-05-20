// FEATURE: B1
// FEATURE: B3
// FEATURE: B4
// FEATURE: B6

use ai_blaise_citus_sidecar_backup::{
    canonical_backup_report, canonical_backup_runtime_report, WalCompression,
};
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
        run_server("backup", "0.0.0.0:8080");
        return;
    }

    if args == ["run-runtime-canonical"] {
        run_runtime_canonical();
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

fn run_runtime_canonical() {
    let report = canonical_backup_runtime_report().unwrap_or_else(|error| {
        eprintln!("backup: canonical runtime failed: {error}");
        process::exit(1);
    });

    println!(
        "cluster\tbase_destination\twal_archive\twal_segments\tbase_size_bytes\tencrypted\tpitr_target\ttarget_cluster\tqueryable_branch\tread_only\tcompleted_base_backups\tarchived_wal_segments\tpitr_restores\tqueryable_branches\tencrypted_artifacts"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.backup.cluster,
        report.backup.base_destination_uri,
        report.backup.wal_archive_uri,
        report.backup.wal_segments,
        report.backup.base_size_bytes,
        report.backup.encrypted,
        report.restore.target_time,
        report.restore.target_cluster,
        report.queryable_branch.branch_name,
        report.queryable_branch.read_only,
        report.state.completed_base_backups,
        report.state.archived_wal_segments,
        report.state.pitr_restores,
        report.state.queryable_branches,
        report.state.encrypted_artifacts,
    );
}

fn print_usage() {
    println!("usage: backup [serve|run-canonical|run-runtime-canonical]");
    println!("runs deterministic canonical backup sidecar plan/runtime reports and emits TSV");
}

fn run_server(component: &str, default_addr: &str) {
    if let Err(error) = run_probe_server(component, default_addr) {
        eprintln!("{component}: probe server failed: {error}");
        process::exit(1);
    }
}

fn compression_name(compression: &WalCompression) -> &'static str {
    match compression {
        WalCompression::None => "none",
        WalCompression::Gzip => "gzip",
        WalCompression::Zstd => "zstd",
    }
}
