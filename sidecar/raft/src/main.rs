// FEATURE: S5
// FEATURE: S5

use ai_blaise_citus_sidecar_raft::{
    canonical_raft_report, canonical_raft_runtime_report, run_durable_log_snapshot_round_trip,
    FailoverDecision, RaftDurableLogReport, RaftRoundTripReport,
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
        run_server("raft", "0.0.0.0:8080");
        return;
    }

    if args == ["run-runtime-canonical"] {
        run_runtime_canonical();
        return;
    }

    if args == ["run-durable-canonical"] {
        run_durable_canonical();
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("raft: unknown command");
        print_usage();
        process::exit(2);
    }

    let report = canonical_raft_report().unwrap_or_else(|error| {
        eprintln!("raft: canonical report failed: {error}");
        process::exit(1);
    });
    let intent = &report.plan.placement_intents[0];
    let (decision, decision_node, decision_pod) = decision_fields(&report.decision);

    println!(
        "shard_group\tterm\tleader\tquorum_size\tlive_nodes\tlease_holder\tlease_expires_physical_ms\tobserved_physical_ms\tintent_shard_id\tintent_target_node\tintent_generation\tdecision\tdecision_node\tdecision_pod"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.plan.shard_group,
        report.plan.term,
        report.plan.leader.as_deref().unwrap_or("none"),
        report.plan.quorum_size(),
        report.live_nodes.join(","),
        report.plan.lease.holder,
        report.plan.lease.expires_at.physical_ms,
        report.observed_at.physical_ms,
        intent.shard_id,
        intent.target_node,
        intent.placement_generation,
        decision,
        decision_node,
        decision_pod,
    );
}

fn run_durable_canonical() {
    let dir = env::var("AI_BLAISE_RAFT_DURABLE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::temp_dir().join(format!("ai-blaise-raft-durable-{}", std::process::id()))
        });
    let _ = std::fs::remove_dir_all(&dir);
    let report = run_durable_log_snapshot_round_trip(&dir).unwrap_or_else(|error| {
        eprintln!("raft: durable canonical report failed: {error}");
        process::exit(1);
    });
    emit_durable_report(&report);
}

fn emit_durable_report(report: &RaftDurableLogReport) {
    println!(
        "appended_entries\treplayed_entries\tsnapshot_index\tsnapshot_term\tlog_path\tsnapshot_path"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}",
        report.appended_entries,
        report.replayed_entries,
        report.snapshot_index,
        report.snapshot_term,
        report.log_path,
        report.snapshot_path,
    );
}

fn run_runtime_canonical() {
    let report = canonical_raft_runtime_report().unwrap_or_else(|error| {
        eprintln!("raft: canonical runtime report failed: {error}");
        process::exit(1);
    });
    emit_runtime_report(&report);
}

fn emit_runtime_report(report: &RaftRoundTripReport) {
    println!(
        "elected_leader\tterm\tcommitted_index\tcommitted_payload\tcommit_indices\tlast_log_indices"
    );
    let payload = std::str::from_utf8(&report.committed_payload).unwrap_or("<binary>");
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}",
        report.elected_leader,
        report.term,
        report.committed_index,
        payload,
        format_node_indices(&report.commit_indices),
        format_node_indices(&report.last_log_indices),
    );
}

fn format_node_indices(map: &std::collections::BTreeMap<String, u64>) -> String {
    map.iter()
        .map(|(id, value)| format!("{id}={value}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn print_usage() {
    println!("usage: raft [serve|run-canonical|run-runtime-canonical|run-durable-canonical]");
    println!(
        "runs the deterministic canonical Raft sidecar plan or the 3-node round-trip runtime and emits TSV"
    );
}

fn run_server(component: &str, default_addr: &str) {
    if let Err(error) = run_probe_server(component, default_addr) {
        eprintln!("{component}: probe server failed: {error}");
        process::exit(1);
    }
}

fn decision_fields(decision: &FailoverDecision) -> (&'static str, &str, &str) {
    match decision {
        FailoverDecision::KeepLeader { node_id } => ("keep_leader", node_id.as_str(), "none"),
        FailoverDecision::Promote { node_id, cnpg_pod } => {
            ("promote", node_id.as_str(), cnpg_pod.as_str())
        }
        FailoverDecision::WaitForQuorum => ("wait_for_quorum", "none", "none"),
    }
}
