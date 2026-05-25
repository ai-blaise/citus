# ADR 0009: Keep the libsql Read Tier Behind a Research Guard

## Status

Accepted (2026-05-25)

## Context

The V2 plan tracks a possible libsql-shaped read tier for edge and offline read
workloads. That idea is not yet a production integration: the repository has no
libsql replication adapter, no workload-isolation contract, no consistency SLO,
and no production query-routing implementation. Treating a planner placeholder
as a deployable read path would create split-brain and stale-read risk.

## Decision

`FEATURE: Edge2` is a production-ready fail-closed research guard, not a
libsql implementation. The companion advanced-planner contract must report the
surface as `ResearchGuard`, point at this ADR, and emit a canonical guard report
with `live_execution_claims=0`, `replication_adapter_claimed=false`,
`workload_isolation_claimed=false`, and
`production_query_routing_claimed=false`.

No production profile may expose a libsql read-tier endpoint, replication
setting, workload-routing path, or operator reconciliation branch until all of
these promotion requirements are satisfied by a replacement ADR and measured
runtime evidence:

- libsql replication semantics ADR accepted
- tenant isolation and workload routing tests
- lag and consistency SLO runbook
- failure-mode drill with stale-read rejection
- production rollout owner signoff

## Consequences

- Positive: production releases cannot accidentally imply libsql read-tier
  behavior just because the V2 feature exists in the planner inventory.
- Positive: the required promotion evidence is machine-checkable through
  `ci/ai-blaise/edge2-libsql-research-guard-smoke.sh` and
  `ci/ai-blaise/production-gap-audit.sh`.
- Negative: users who want a libsql read tier must wait for a separate
  implementation PR with real replication, isolation, and routing evidence.

## References

- `companion/src/advanced_planner.rs`
- `ci/ai-blaise/edge2-libsql-research-guard-smoke.sh`
- `docs/ai-blaise/NEW_FEATURES.md` (`FEATURE: Edge2`)
