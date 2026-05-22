# ADR 0008: F1-style schema change with the two-version invariant

Status: Accepted
Date: 2026-05-22
Deciders: ai-blaise/citus maintainers
Related: ADR 0001 (fork-not-rewrite), ADR 0002 (overlay-not-patch), ADR 0006
(CNPG substrate), ADR 0007 (Raft per shard group)

## Context

ai-blaise/citus needs an online schema-change story that survives across the
Citus topology (one coordinator, many workers, every shard placed twice or
more). Vanilla Citus distributes DDL, but it does not give us:

* a state machine that guarantees `INSERT`/`SELECT` semantics against an
  in-progress column change,
* a coordinated rollout that lets workers temporarily disagree without
  corrupting reads, or
* a reversible cutover for type changes that cannot tolerate a long
  `AccessExclusiveLock` on a wide distributed table.

We need to pick a coordination model.

## Options considered

### Option A: Google F1's two-version invariant (2VI)

F1's "Online, Asynchronous Schema Change" (VLDB 2013) drives every change
through a small number of intermediate phases — `DELETE_ONLY`, `WRITE_ONLY`,
`BACKFILL`, `PUBLIC` — and guarantees that **at most two schema versions are
in use across the cluster at any moment**. A change is reversible until the
`BACKFILL -> PUBLIC` reorganization point. F1 uses a controller that waits
for every replica to acknowledge the current phase before driving the next
one; non-acknowledging replicas force the controller to wait, skip (with an
operator flag), or roll back.

Strengths:

* Reversibility: every phase before PUBLIC can be unwound.
* Composes cleanly with distributed DDL: the coordinator can apply DDL to
  the shard placements at any rate as long as the controller drives phase
  transitions only when *all* workers have caught up.
* Simple invariant ("never more than two schema versions in flight") that
  is easy to monitor.
* Well-understood failure modes — an F1 schema migration that fails halfway
  through is a normal operational event, not a recovery scenario.

Weaknesses:

* Requires a controller cursor that survives operator pod restart.
* Phase invariants must be enforced by triggers or planner hooks — pure SQL
  is not enough.

### Option B: CockroachDB's online schema change

Cockroach's design is itself derived from F1. It adds a leaseholder protocol
and uses `MERGING`/`DELETE_AND_WRITE_ONLY` instead of F1's terminology, but
the underlying invariant is the same. Cockroach uses table descriptors plus
gossip; Citus has no equivalent gossip layer.

Strengths:

* Mature, battle-tested.

Weaknesses:

* The leaseholder pattern assumes a Raft-replicated descriptor — a fit for
  ai-blaise/citus only inside one shard group, not across the cluster.
* Borrowing the protocol without the descriptor system means re-implementing
  most of F1 anyway.

### Option C: MySQL gh-ost / pt-online-schema-change shadow table

gh-ost copies the target table into a shadow table, mirrors writes through a
binlog watcher, then atomically renames the shadow over the original. We
have a shadow-table contract in `SchemaJobWorkerPlan::shadow`, so this is
already half-built. It is the right answer for *whole-table rewrites*, e.g.
the type-change path in `MigrationOperation::AlterColumnType`.

Strengths:

* Tolerant of foreign keys and large rows.
* Zero metadata changes on the original table until cutover.

Weaknesses:

* Doubles the disk footprint of the target table during the rollout.
* Needs a row-level mirror (CDC sidecar) that we currently scope as alpha.
* Cutover is a single short blocking lock; long-tail readers can be killed.

### Option D: pgroll expand/contract

pgroll splits the migration into an expand step (add the new column or
table, dual-write, backfill) and a contract step (drop the old column or
table) separated by an operator-driven cutover. The state model is simpler
than F1 (no `DELETE_ONLY`) and pgroll lives at the SQL layer — both fit our
companion overlay.

Strengths:

* Simple to reason about.
* Already implemented in `companion/src/migration.rs` as the M1 DSL.

Weaknesses:

* No formal invariant on the number of in-flight schema versions, so we get
  no continuous monitor.
* No first-class rollback during backfill — the operator must drop the new
  column manually.

## Decision

Adopt **Option A (F1 two-version invariant) as the orchestration backbone**
and treat the other options as composable building blocks:

* The F1 phase model (`DELETE_ONLY -> WRITE_ONLY -> BACKFILL -> PUBLIC`)
  drives `SchemaJobPlan` in `companion/src/schema_jobs/mod.rs`.
* The pgroll expand/contract DSL (Option D) is the *operation language*
  that `MigrationOperation` expresses — F1 is the *coordination protocol*
  that drives those operations to completion.
* The gh-ost shadow-table contract (Option C) is preserved in
  `GhOstShadowPlan` for `ALTER COLUMN TYPE` operations that cannot avoid
  rewriting the heap.
* Cockroach-style leaseholder semantics (Option B) are out of scope; we
  rely on the Citus coordinator as the single authority and on the
  per-shard-group Raft cluster from ADR 0007 for placement membership.

## Consequences

### Positive

* The controller surface is small: `SchemaJobController::transition`,
  `SchemaJobController::rollback`, and `verify_two_version_invariant()`
  are the only entry points.
* The continuous monitor (`pg_cron` every 60 s) gives us an early-warning
  signal — any deviation from "two schema versions" raises a critical
  `companion.cluster_alarms` row.
* Worker churn is tolerated. Workers register a TTL lease in
  `companion.worker_schema_lease`; expired leases force the controller to
  wait, skip, or roll back per the configured `TransitionGate`.
* Migration CRs have a deterministic interpretation: the
  `MigrationReconciler` derives `(target_state, gate)` from
  `(current_state, on_conflict)` and hands the rest to the sidecar.

### Negative

* Phase invariants must be enforced *somewhere*. We chose to enforce them
  in companion-installed triggers and planner hooks (the alpha boundary
  for `FEATURE: M14`). Until those triggers ship, the invariant is only
  enforced by the SQL functions and the sidecar — a misbehaving client
  that calls raw DDL can still violate it.
* The controller cursor lives in the schema-job sidecar. Operator pod
  restart is safe (state is in `companion.schema_jobs`), but the sidecar
  itself must restart cleanly. We rely on the existing health probe
  surface in `sidecar/shared`.

### Operational

* The `MigrationCRD.on_conflict` field now drives the controller gate:
  `Fail -> RollbackOnTimeout`, `Skip -> SkipMissing`, others ->
  `WaitForever`.
* Operators monitoring 2VI compliance should watch
  `companion_two_version_invariant_state` and `companion_cluster_alarms`.

## References

* J. Rae et al., "Online, Asynchronous Schema Change in F1," VLDB 2013.
* CockroachDB Labs, "Online Schema Changes in CockroachDB."
* Xata, `pgroll` v0.13.1 release notes.
* gh-ost 1.2 design doc.
* `companion/src/schema_jobs/` (this overlay's implementation).
