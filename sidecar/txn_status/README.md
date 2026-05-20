# sidecar/txn_status

Raft-backed transaction status service for parallel commits.

Current implemented surface:

- `TxnStatusServicePlan`
- `ParallelCommitRecord`
- `TxnIntent`
- `TxnFinalizeDecision`
- `cargo run -p ai_blaise_citus_sidecar_txn_status -- run-canonical`

These contracts cover `FEATURE: T5`.
