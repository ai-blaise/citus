# sidecar/txn_status

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Raft-backed transaction status service for parallel commits.

Current implemented surface:

- `TxnStatusServicePlan`
- `ParallelCommitRecord`
- `TxnIntent`
- `TxnFinalizeDecision`
- `cargo run -p ai_blaise_citus_sidecar_txn_status -- run-canonical`

These contracts cover `FEATURE: T5`.
