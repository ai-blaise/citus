# Migration CR Operator Guide

This document explains how operators run online schema changes on
ai-blaise/citus through the `Migration` Custom Resource and the F1-style
two-version invariant (2VI) runtime described in
[ADR 0008](ADR/0008-f1-style-schema-change.md).

## At a glance

| Layer | What it owns | File |
| --- | --- | --- |
| Migration CRD | Declarative description of the change | `operator/src/crds/migration.rs` |
| Migration Reconciler | Picks the next phase, picks the gate, hands off to sidecar | `operator/src/reconcile/migration.rs` |
| SchemaJobPlan + Controller | Phase machine + 2VI logic | `companion/src/schema_jobs/` |
| Worker Lease Registry | Per-worker schema-version acknowledgement | `companion/src/schema_jobs/worker_lease.rs` |
| Rollback Planner | Phase reversal + partial-backfill cleanup | `companion/src/schema_jobs/rollback.rs` |
| Phase Log | Audit trail of every transition | `companion.schema_job_phase_log` |
| Cluster Alarms | 2VI violations | `companion.cluster_alarms` |
| Sidecar Controller | tokio loop that polls the controller | `sidecar/schema_job/src/controller.rs` |
| SQL surface | Tables, views, functions installed by `ai_blaise_citus` | `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql` |

## Lifecycle

A `Migration` runs through six phases:

```
+-----------+   advance   +-----------+   advance   +----------+   advance   +--------+
| Absent    | ----------> | DELETE    | ----------> | WRITE    | ----------> | BACK   |
|           |             | _ONLY     |             | _ONLY    |             | FILL   |
+-----------+             +-----------+             +----------+             +--------+
                                                                                 |
                                                                                 | advance (one-way)
                                                                                 v
                                                                            +--------+
                                                                            | PUBLIC |
                                                                            +--------+
```

`PUBLIC` is the reorganization point: the change is committed and the prior
schema version is retired. Up to that point every transition is reversible.

At any moment the cluster holds at most **two** schema versions — that is
the *two-version invariant*. The continuous monitor
(`companion_internal.verify_two_version_invariant()`) is scheduled by
`pg_cron` every 60 seconds and writes a `companion.cluster_alarms` row if
the invariant is violated.

## Authoring a Migration CR

```yaml
apiVersion: aiblaise.dev/v1alpha1
kind: Migration
metadata:
  name: users-add-display-name
spec:
  migrationType: Pgroll
  onConflict: ManualReview
  yaml: |
    operations:
      - add_column:
          table: public.users
          column: display_name
          type: text
      - backfill:
          statement: "UPDATE public.users SET display_name = email"
```

* `migrationType` chooses the DDL renderer: `Pgroll` (expand/contract) or
  `GhOst` (shadow table). Both are validated by
  `companion/src/migration.rs`.
* `onConflict` chooses the controller gate. The reconciler maps:
  * `Fail` → `RollbackOnTimeout` — abort on worker timeout, walk back to
    the prior phase.
  * `Skip` → `SkipMissing` — proceed even if a worker has not
    acknowledged. Use only when a worker is permanently lost.
  * `Replace` / `ManualReview` → `WaitForever` — strict 2VI semantics.

## Operator runbook

### Starting a migration

1. `kubectl apply -f migration.yaml`
2. The MigrationReconciler validates the spec, derives the
   `MigrationReconcilePlan`, and posts an envelope to the
   `sidecar/schema_job` daemon for the cluster.
3. The sidecar opens `companion_internal.schema_job_start(...)` and walks
   the job forward one phase at a time.

### Watching progress

```sql
SELECT * FROM companion_schema_jobs
  WHERE job_name = 'users-add-display-name';

SELECT * FROM companion_schema_job_phase_log
  WHERE job_name = 'users-add-display-name'
  ORDER BY recorded_at DESC;
```

### Worker health

```sql
SELECT * FROM companion_worker_schema_lease
  WHERE job_name = 'users-add-display-name';

SELECT * FROM companion_two_version_invariant_state
  WHERE job_name = 'users-add-display-name';
```

A worker is *delinquent* when its `expires_at` is in the past or it never
recorded a lease. The controller treats delinquent workers per the
configured gate.

### 2VI alarms

```sql
SELECT alarm_id, alarm_kind, severity, detail, raised_at
FROM companion_cluster_alarms
WHERE cleared_at IS NULL
  AND alarm_kind = 'two_version_invariant_violation';
```

Anything in this view is *critical*: the cluster has more than two schema
versions in flight for one job. The operator should pause the responsible
Migration CR and inspect the worker leases. See
[runbook/lost-shard](RUNBOOKS/lost-shard.md) for the procedure.

### Rolling back

```sql
-- From SQL (operators rarely do this manually; the sidecar does it on
-- timeout when the gate is RollbackOnTimeout).
SELECT companion_internal.schema_job_rollback_to('write_only');
SELECT companion_internal.schema_job_phase_log_rollback(
  'users-add-display-name', 'backfill', 'write_only', now()
);
```

The Rust API:

```rust
let rollback = RollbackPlan::new(&plan, SchemaJobState::DeleteOnly, "2026-05-22T14:00:00Z")?;
for step in &rollback.steps {
    // sidecar issues step.to_sql()
}
```

After `PUBLIC`, rollback is no longer available: the schema-job is
committed. The forward fix is a *new* Migration CR.

### Pausing

```sql
SELECT companion_internal.schema_job_advance('users-add-display-name', 'paused');
```

The controller treats `paused` like a halted gate: no forward transitions,
no rollback driven from the sidecar. Worker leases keep refreshing.

## Failure modes

| Failure | Symptom | Response |
| --- | --- | --- |
| Worker times out before acknowledging | `WaitForAcknowledgement` decision, sidecar retries | Either restart the worker or set the CR `onConflict: Skip`. |
| Worker disagrees on the schema version | `StalePhase { observed }` status, controller stays put | Inspect `companion_worker_schema_lease`; revoke the stale lease. |
| 2VI violation | `companion.cluster_alarms` row with `severity=critical` | Pause migration, inspect leases, clear conflicting versions. |
| Sidecar pod restart | Controller cursor reloads from `companion.schema_jobs` | No action; transitions resume on the next poll. |
| Backfill row corruption | Forward progress impossible | Rollback to `DELETE_ONLY` and re-author the Migration. |

## What is and is not production

| Component | Status |
| --- | --- |
| F1 phase machine (Rust) | production-ready (`FEATURE: C10`, `FEATURE: M14`) |
| Worker lease registry + SQL surface | production-ready (`FEATURE: M14`) |
| 2VI verifier + continuous monitor wiring | production-ready (`FEATURE: M15`) |
| Rollback planner + SQL helpers | production-ready (`FEATURE: M14`) |
| Sidecar tokio loop (real polling) | alpha |
| Migration CRD reconciler (kube-rs client) | alpha |
| Trigger-enforced phase invariants (e.g. WRITE_ONLY returns NULL) | alpha — planner-hook track |
| Distributed backfill workers | alpha — see C11 |

The companion SQL extension and the in-process controller logic are
production-ready and exercised by `ci/ai-blaise/schema-job-f1-2vi-smoke.sh`.
The runtime that actually executes DDL against placements, drains traffic,
and reconciles K8s state remains alpha and is gated behind the existing
sidecar/operator alpha boundaries.
