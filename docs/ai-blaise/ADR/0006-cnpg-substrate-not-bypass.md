# ADR 0006: CloudNativePG as the Postgres-Lifecycle Substrate

## Status

Accepted (2026-05-21)

## Context

A Citus cluster on Kubernetes needs primary/replica election, streaming
replication, failover, scheduled backup, point-in-time restore, minor
and major version upgrades, certificate rotation, and pooled
PersistentVolume reconciliation. Building these ourselves duplicates
years of work that CloudNativePG (CNPG) already ships and runs at
scale. The choice is whether our operator owns the full Postgres
lifecycle directly or layers on top of CNPG's `Cluster` CR.

## Decision

CNPG is the Postgres-lifecycle substrate. CNPG manages the Postgres
pods (primary, standbys, instance managers, the WAL archive). Our
operator manages everything Citus-specific that CNPG does not
understand: shard topology (`pg_dist_shard`, `pg_dist_placement`),
shard groups, hypertables, sidecar fleets, the pool, vectorizer
schedules, branch CRs, and survival-goal placement. The
`CitusCluster.spec.cnpgRef` field points at the underlying CNPG
`Cluster`; our reconcilers translate Citus-level intent into changes
on that CR and the operand image. The `sidecar/raft` group decides
which pod is the leaseholder for a shard group and signals CNPG via
the K8s API; CNPG performs the actual streaming-replica promotion.

## Alternatives considered

- Own the full Postgres lifecycle in our operator. Rejected —
  re-implementing CNPG's `instance manager`, `WAL archive`,
  `barman-cloud` integration, and pg_basebackup orchestration would
  triple the operator's surface area and divert effort from the
  features that differentiate ai-blaise/citus (shard awareness, the
  sidecar fleet, the analytical substrate).
- Use Zalando postgres-operator. Rejected — less production share than
  CNPG, fewer hooks for an external controller to layer on, and the
  Spilo image model is less amenable to our operand-image build
  pattern.
- Use Crunchy PGO. Rejected — Apache 2.0 but less idiomatic on modern
  Kubernetes, and the integration surface for an external Citus-aware
  controller is smaller.
- StatefulSet by hand. Rejected on the same grounds as building
  failover ourselves.

## Consequences

- Positive: failover, backup, PITR, minor upgrades, certificate
  rotation, and PVC reconciliation are someone else's problem and
  already battle-tested.
- Positive: operators of an ai-blaise/citus cluster who already run
  CNPG get a familiar mental model. Migrations from CNPG-managed
  vanilla Postgres to ai-blaise/citus are a CR mutation, not a data
  copy.
- Positive: CNPG's primary/replica machinery composes with our Raft
  shard-group leaseholder decision (ADR 0007) — Raft picks the
  leaseholder, CNPG executes the failover.
- Negative: two operators reconcile related state. Drift between
  CNPG's view of "which pod is primary" and our view of "which pod
  holds the shard-group lease" can occur during failover. Mitigation:
  the operator reconciler treats CNPG status as authoritative for pod
  identity and only signals the desired leaseholder; the
  `sidecar/raft` group converges via lease renewal.
- Negative: CNPG releases on its own cadence. Major-version bumps may
  require coordinated changes in our operand image. Mitigation: the
  CNPG API version is pinned in `ai-blaise/command-center (helm/charts/citus-cluster + deploy/citus-cluster)`, and the upstream-sync
  workflow extends to CNPG.
- Risks: a future CNPG release could remove a hook we depend on.
  Mitigation: depend only on the documented `Cluster` CR surface; no
  reaching into CNPG internals.

## References

- Plan §6.5 (`operator/`)
- Plan §2.7 (K8s operator landscape)
- Plan §4.4 (HA without external election service)
- CloudNativePG `Cluster` CR documentation
- ADR 0005 — operator language choice
- ADR 0007 — Raft per shard group
