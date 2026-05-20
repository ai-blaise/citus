# sidecar/txn_status

Raft-backed transaction status service for parallel commits.

Current implemented surface:

- `TxnStatusServicePlan`
- `ParallelCommitRecord`
- `TxnIntent`
- `TxnFinalizeDecision`

These contracts cover `FEATURE: T5`.
