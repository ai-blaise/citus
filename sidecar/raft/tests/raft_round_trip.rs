//! End-to-end Raft round-trip test exercising the in-process runtime.
//!
//! Spins up a 3-node cluster, drives an election from `worker-a`, proposes a
//! placement payload, and verifies majority commit + log convergence across
//! every voter.

use ai_blaise_citus_sidecar_raft::{
    canonical_raft_runtime_report, run_raft_round_trip, RaftMessage, RaftNode, RaftRole,
    VoteResponse,
};

fn three_voter_members() -> Vec<String> {
    vec![
        "worker-a".to_string(),
        "worker-b".to_string(),
        "worker-c".to_string(),
    ]
}

#[test]
fn round_trip_replicates_to_majority() {
    let report = run_raft_round_trip(three_voter_members(), "worker-a", b"placement-1".to_vec())
        .expect("round trip");

    assert_eq!(report.elected_leader, "worker-a");
    assert_eq!(report.term, 1);
    assert_eq!(report.committed_index, 1);
    assert_eq!(report.committed_payload, b"placement-1".to_vec());

    for (id, last) in &report.last_log_indices {
        assert_eq!(*last, 1, "node {id} did not append entry");
    }
    for (id, commit) in &report.commit_indices {
        assert_eq!(*commit, 1, "node {id} did not commit");
    }
}

#[test]
fn canonical_runtime_report_is_deterministic() {
    let report = canonical_raft_runtime_report().expect("canonical runtime report");

    assert_eq!(report.elected_leader, "worker-a");
    assert_eq!(
        report.committed_payload,
        b"shard-placement-canonical".to_vec()
    );
    assert_eq!(report.committed_index, 1);
}

#[test]
fn leader_loses_leadership_on_higher_term() {
    let members = three_voter_members();
    let mut a = RaftNode::new("worker-a", members.clone()).expect("worker-a");
    let _votes_a = a.become_candidate();
    // Pretend worker-b granted; majority of 2 reached.
    a.step(
        "worker-b",
        RaftMessage::VoteResponse(VoteResponse {
            term: 1,
            from: "worker-b".to_string(),
            granted: true,
        }),
    );
    assert_eq!(a.role(), RaftRole::Leader);

    // A higher-term vote request forces worker-a back to follower.
    let mut b = RaftNode::new("worker-b", members).expect("worker-b");
    b.become_candidate();
    b.become_candidate();
    let messages = b.become_candidate();
    let to_a = messages
        .into_iter()
        .find(|(peer, _)| peer == "worker-a")
        .expect("vote request to worker-a");
    a.step("worker-b", to_a.1);
    assert_eq!(a.role(), RaftRole::Follower);
    assert!(a.current_term() >= 3);
}
