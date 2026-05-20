# sidecar/raft

Raft coordination per shard group for leases, placement intent, and failover
decisions.

Current implemented surface:

- `RaftShardGroupPlan`
- `RaftMember`
- `PlacementLeasePlan`
- `ShardPlacementIntent`
- `FailoverDecision`

These contracts cover `FEATURE: S5`.
