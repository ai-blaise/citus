# sidecar/txn_status

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Deterministic in-process Raft-backed transaction status boundary for parallel commits. The broader multi-process/networked Raft and Citus executor integration remain alpha until separately live-gated.

Current implemented surface:

- `TxnStatusServicePlan`
- `ParallelCommitRecord`
- `TxnIntent`
- `TxnFinalizeDecision`
- `TxnStatusRuntime` stage/finalize/status state machine
- loopback HTTP `serve` routes for `/txn/staging`, `/txn/ack`, `/txn/finalize`, and `/txn/<id>/status`
- `cargo run -p ai_blaise_citus_sidecar_txn_status -- run-canonical`
- `cargo run -p ai_blaise_citus_sidecar_txn_status -- run-runtime-canonical`
- `cargo run -p ai_blaise_citus_sidecar_txn_status -- run-parallel-commit-microbench 5`
- `ci/ai-blaise/schema-txn-runtime-smoke.sh`

These contracts cover `FEATURE: T5`.
