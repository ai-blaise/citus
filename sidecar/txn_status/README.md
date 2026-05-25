# sidecar/txn_status

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Raft-backed transaction-status boundary for parallel commits. The HTTP
`serve` runtime can append staging and terminal commit/abort decisions through
the live `sidecar/raft` network transport by setting
`AI_BLAISE_TXN_RAFT_LEADER_ADDR=host:port`; without that variable it uses the
deterministic in-process Raft model for canonical tests. Citus distributed
executor integration remains outside this sidecar boundary.

Current implemented surface:

- `TxnStatusServicePlan`
- `ParallelCommitRecord`
- `TxnIntent`
- `TxnFinalizeDecision`
- `TxnStatusRuntime` stage/finalize/status state machine
- loopback HTTP `serve` routes for `/txn/staging`, `/txn/ack`, `/txn/finalize`, and `/txn/<id>/status`
- networked Raft append mode via `AI_BLAISE_TXN_RAFT_LEADER_ADDR`
- `cargo run -p ai_blaise_citus_sidecar_txn_status -- run-canonical`
- `cargo run -p ai_blaise_citus_sidecar_txn_status -- run-runtime-canonical`
- `cargo run -p ai_blaise_citus_sidecar_txn_status -- run-parallel-commit-microbench 5`
- `ci/ai-blaise/txn-status-networked-raft-smoke.sh`
- `ci/ai-blaise/schema-txn-runtime-smoke.sh`

These contracts cover `FEATURE: T5`.
