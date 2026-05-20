# sidecar/raft

Raft coordination per shard group for leases, placement intent, and failover
decisions.

Current implemented surface:

- `RaftShardGroupPlan`
- `RaftMember`
- `PlacementLeasePlan`
- `ShardPlacementIntent`
- `FailoverDecision`
- `cargo run -p ai_blaise_citus_sidecar_raft -- run-canonical`

These contracts cover `FEATURE: S5`.
