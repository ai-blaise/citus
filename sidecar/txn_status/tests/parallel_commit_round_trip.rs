//! End-to-end parallel-commit round-trip test.
//!
//! Drives the in-process txn-status runtime through stage -> ack ->
//! finalize for a three-shard transaction and checks the local state-machine boundary.

use ai_blaise_citus_sidecar_hlc::HlcTimestamp;
use ai_blaise_citus_sidecar_txn_status::{
    canonical_txn_runtime_report, run_parallel_commit_microbench, ParallelCommitRecord,
    TxnFinalizeDecision, TxnIntent, TxnStatus, TxnStatusRuntime,
};

#[test]
fn three_shard_parallel_commit_round_trip() {
    let voters = vec![
        "worker-a".to_string(),
        "worker-b".to_string(),
        "worker-c".to_string(),
    ];
    let mut runtime = TxnStatusRuntime::new("orders-txn", voters, 5_000).expect("runtime");

    let record = ParallelCommitRecord {
        txn_id: "txn-integration-1".to_string(),
        coordinator: "worker-a".to_string(),
        status: TxnStatus::Staging,
        staging_at: HlcTimestamp {
            physical_ms: 1_700_000_000,
            logical: 0,
        },
        intents: vec![
            TxnIntent {
                shard_id: 10,
                key_range: "[a,f)".to_string(),
                replica_ack_count: 0,
                required_acks: 2,
            },
            TxnIntent {
                shard_id: 11,
                key_range: "[f,m)".to_string(),
                replica_ack_count: 0,
                required_acks: 2,
            },
            TxnIntent {
                shard_id: 12,
                key_range: "[m,z)".to_string(),
                replica_ack_count: 0,
                required_acks: 2,
            },
        ],
    };
    let staged = runtime.stage(record).expect("stage");
    assert_eq!(staged.status, TxnStatus::Staging);
    assert_eq!(staged.intents.len(), 3);

    // Acks arrive in parallel.
    for shard_id in [10_u64, 11, 12] {
        runtime
            .record_replica_ack("txn-integration-1", shard_id, 2)
            .expect("ack");
    }

    let (record, decision) = runtime
        .finalize(
            "txn-integration-1",
            HlcTimestamp {
                physical_ms: 1_700_000_050,
                logical: 0,
            },
        )
        .expect("finalize");

    assert_eq!(decision, TxnFinalizeDecision::Commit);
    assert_eq!(record.status, TxnStatus::Committed);
    // The per-call Raft round-trip resets indices, so the staged and
    // committed records both report raft_index == 1; persistence beyond a
    // single staging/finalize pair is tracked by `next_raft_index`.
    assert!(record.raft_index >= 1);
}

#[test]
fn canonical_runtime_report_is_stable() {
    let report = canonical_txn_runtime_report().expect("report");
    assert_eq!(report.staged_record.status, TxnStatus::Staging);
    assert_eq!(report.finalize_decision, TxnFinalizeDecision::Commit);
}

#[test]
fn microbench_demonstrates_parallel_commit_speedup() {
    let micro = run_parallel_commit_microbench(5);
    // 2PC: 2 * 5 = 10 round trips.
    assert_eq!(micro.two_phase_commit_steps, 10);
    // Parallel commit: 2 round trips regardless of shard count.
    assert_eq!(micro.parallel_commit_steps, 2);
    // Speedup must be at least 2x for the 40% latency gate.
    assert!(micro.speedup() >= 2.0);
    // ratio = 5 -> 1/5 = 0.2x latency -> well below 0.6x of 2PC baseline.
    assert!(1.0 / micro.speedup() <= 0.6);
}
