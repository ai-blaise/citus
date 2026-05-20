// FEATURE: T5

use ai_blaise_citus_sidecar_shared::run_probe_server;
use ai_blaise_citus_sidecar_txn_status::{
    canonical_txn_status_report, TxnFinalizeDecision, TxnStatus,
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
        run_server("txn-status", "0.0.0.0:8080");
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("txn-status: unknown command");
        print_usage();
        process::exit(2);
    }

    let report = canonical_txn_status_report().unwrap_or_else(|error| {
        eprintln!("txn-status: canonical report failed: {error}");
        process::exit(1);
    });
    let first_intent = &report.record.intents[0];

    println!(
        "raft_group\tvoters\tclock_physical_ms\ttxn_id\tcoordinator\tstatus\tstaging_physical_ms\tobserved_physical_ms\tmax_staging_ms\tintent_count\tfirst_shard_id\tfirst_key_range\tfirst_replica_acks\tfirst_required_acks\tdecision"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.service.raft_group,
        report.service.voters.join(","),
        report.service.clock.physical_ms,
        report.record.txn_id,
        report.record.coordinator,
        status_name(&report.record.status),
        report.record.staging_at.physical_ms,
        report.observed_at.physical_ms,
        report.service.max_staging_ms,
        report.record.intents.len(),
        first_intent.shard_id,
        first_intent.key_range,
        first_intent.replica_ack_count,
        first_intent.required_acks,
        decision_name(&report.decision),
    );
}

fn print_usage() {
    println!("usage: txn-status [serve|run-canonical]");
    println!("runs the deterministic canonical transaction-status sidecar plan and emits TSV");
}

fn run_server(component: &str, default_addr: &str) {
    if let Err(error) = run_probe_server(component, default_addr) {
        eprintln!("{component}: probe server failed: {error}");
        process::exit(1);
    }
}

fn status_name(status: &TxnStatus) -> &'static str {
    match status {
        TxnStatus::Pending => "pending",
        TxnStatus::Staging => "staging",
        TxnStatus::Committed => "committed",
        TxnStatus::Aborted => "aborted",
    }
}

fn decision_name(decision: &TxnFinalizeDecision) -> &'static str {
    match decision {
        TxnFinalizeDecision::Commit => "commit",
        TxnFinalizeDecision::WaitForReplicationEvidence => "wait_for_replication_evidence",
        TxnFinalizeDecision::AbortStaleStagingRecord => "abort_stale_staging_record",
        TxnFinalizeDecision::FallbackToTwoPhaseCommit => "fallback_to_two_phase_commit",
        TxnFinalizeDecision::AlreadyCommitted => "already_committed",
        TxnFinalizeDecision::AlreadyAborted => "already_aborted",
    }
}
