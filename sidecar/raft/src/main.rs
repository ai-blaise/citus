// FEATURE: S5

use ai_blaise_citus_sidecar_raft::{canonical_raft_report, FailoverDecision};
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
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

fn print_usage() {
    println!("usage: raft [run-canonical]");
    println!("runs the deterministic canonical Raft sidecar plan and emits TSV");
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
