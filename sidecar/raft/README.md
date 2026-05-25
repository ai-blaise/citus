# sidecar/raft

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Raft coordination per shard group for leases, placement intent, and failover
decisions.

Current implemented surface:

- `RaftShardGroupPlan`
- `RaftMember`
- `PlacementLeasePlan`
- `ShardPlacementIntent`
- `FailoverDecision`
- `cargo run -p ai_blaise_citus_sidecar_raft -- run-canonical`
- `cargo run -p ai_blaise_citus_sidecar_raft -- run-runtime-canonical`
- `cargo run -p ai_blaise_citus_sidecar_raft -- run-durable-canonical`
- `cargo run -p ai_blaise_citus_sidecar_raft -- serve`
- `ci/ai-blaise/sidecar-raft-smoke.sh`

The `serve` mode exposes the bounded HTTP transport used by the live smoke:
`/raft/campaign`, `/raft/propose`, `/raft/message`, and `/raft/status`, plus
the shared `/healthz`, `/readyz`, `/drain`, and `/metrics` probe surface.
`sidecar-raft-smoke.sh` starts three separate OS processes on loopback ports,
elects `worker-a`, proposes a placement-intent payload, verifies all voters
commit the same entry, and checks malformed transport/propose requests fail
closed.

These contracts cover `FEATURE: S5`.
