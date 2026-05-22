# Runbook: Branch Suspend Stuck

`FEATURE: R2` `FEATURE: C6` `FEATURE: C7` `FEATURE: C8`

## When to run this

A `Branch` CR with `spec.suspend: true` is not scaling its compute down
to zero. The branch pod still appears in `kubectl get pods`, the
operator status reports `phase: SuspendInProgress` for more than the
configured grace window, and the underlying CNPG `Cluster` shows
non-zero `instances`.

## Pre-conditions

- The `Branch` CR exists and is owned by a healthy `CitusCluster`
  (`operator/src/crds/branch.rs`).
- The branch's storage class supports volume detach
  (`BranchStorageSpec.storage_class`).
- The operator's branch reconciler is running
  (`operator/src/reconcile/mod.rs`).
- `sidecar/repack` is not blocking the branch (no live `ScheduledRepack`
  active against the branch tables).
- The on-call has `kubectl` access to the namespace.

## Detection

1. Confirm the CR is asking for suspend:

   ```bash
   kubectl -n ai-blaise-citus get branch/<name> -o yaml \
     | yq '{name: .metadata.name, suspend: .spec.suspend,
            phase: .status.phase, last_transition: .status.lastTransitionTime,
            blockers: .status.suspendBlockers}'
   ```

   Expected for an unstuck branch: `phase: Suspended` and
   `suspendBlockers: []`. A stuck branch has `phase: SuspendInProgress`
   and a non-empty `suspendBlockers`.

2. Confirm the compute pod still exists:

   ```bash
   kubectl -n ai-blaise-citus get pod -l ai-blaise.io/branch=<name>
   ```

3. Read the live blocker list straight from postgres. Connect to the
   branch's compute pod and run:

   ```sql
   SELECT pid, usename, application_name, state, wait_event_type, wait_event,
          xact_start, query_start, backend_xmin, query
     FROM pg_stat_activity
    WHERE datname = '<branch_database>'
      AND state <> 'idle'
    ORDER BY xact_start ASC NULLS LAST;
   ```

   Any row with non-null `xact_start` is an open transaction blocking
   suspend.

4. Confirm replication-lag blocker:

   ```sql
   SELECT application_name, client_addr, state, sync_state,
          pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replay_bytes,
          pg_wal_lsn_diff(sent_lsn, flush_lsn) AS flush_bytes,
          write_lag, flush_lag, replay_lag
     FROM pg_stat_replication;
   ```

   The branch refuses to suspend while `replay_lag` exceeds
   `Branch.spec.maxReplayLagForSuspend` (default `5s`).

5. Confirm lock-held blocker:

   ```sql
   SELECT l.pid, l.locktype, l.relation::regclass AS relation,
          l.mode, l.granted, a.query, a.state
     FROM pg_locks l
     JOIN pg_stat_activity a ON a.pid = l.pid
    WHERE NOT l.granted
       OR l.locktype = 'advisory'
    ORDER BY l.pid;
   ```

## Recovery procedure

Run causes in the order their detection step matched. Stop at the first
cause that clears `suspendBlockers`.

1. Open transaction. Identify the blocking session and terminate only
   after confirming with the application owner; arbitrary termination
   of long transactions is destructive:

   ```sql
   SELECT pg_cancel_backend(<pid>);
   -- If the backend ignores cancel after 30s:
   SELECT pg_terminate_backend(<pid>);
   ```

   Document the terminated `pid`, `application_name`, and `query` in
   the incident ticket.

2. Lock held. If the holder is a replication slot or autovacuum, do
   not terminate the worker. Instead, raise the suspend grace window
   on the CR and wait:

   ```bash
   kubectl -n ai-blaise-citus patch branch/<name> --type=merge -p \
     '{"spec":{"suspendGraceSeconds": 600}}'
   ```

   If the holder is a stuck advisory lock from a crashed companion
   job:

   ```sql
   SELECT pg_advisory_unlock_all();
   ```

   Run the unlock only from the *same* session whose `pg_locks` row
   shows the holder, or escalate to dropping the offending session
   with `pg_terminate_backend`.

3. Replication lag too high. Confirm there is no upstream cause first
   (network blip, downstream pod evicted). If lag is intrinsic to the
   workload, raise the suspend threshold rather than forcing:

   ```bash
   kubectl -n ai-blaise-citus patch branch/<name> --type=merge -p \
     '{"spec":{"maxReplayLagForSuspend": "30s"}}'
   ```

   If lag is unbounded because the replica is broken, fix the replica
   before retrying suspend; see `lost-shard.md`.

4. Force-terminate the compute pod only after the operator-tracked
   blocker list has been cleared on the CR. The operator's branch
   reconciler refuses to delete a pod with non-empty
   `status.suspendBlockers`, so a force-delete that bypasses the
   reconciler will leave orphan resources. The supported escape hatch
   is to clear the blockers explicitly:

   ```bash
   kubectl -n ai-blaise-citus patch branch/<name>/status --type=merge \
     --subresource=status -p '{"status":{"suspendBlockers":[]}}'
   ```

   Then let the reconciler re-evaluate; if it re-populates the list,
   the underlying cause has not been resolved — go back to detection.

5. Drive a fresh reconcile loop:

   ```bash
   kubectl -n ai-blaise-citus annotate branch/<name> \
     ai-blaise.io/reconcile-trigger="$(date -u +%s)" --overwrite
   ```

## Verification

1. The CR reports `phase: Suspended`:

   ```bash
   kubectl -n ai-blaise-citus get branch/<name> \
     -o jsonpath='{.status.phase}{"\n"}'
   ```

   Expected: `Suspended`.

2. The compute pod is gone:

   ```bash
   kubectl -n ai-blaise-citus get pod -l ai-blaise.io/branch=<name> \
     --no-headers
   ```

   Expected: no output.

3. The CNPG cluster has scaled to zero instances for the branch:

   ```bash
   kubectl -n ai-blaise-citus get cluster.postgresql.cnpg.io \
     -l ai-blaise.io/branch=<name> \
     -o jsonpath='{range .items[*]}{.metadata.name}{"\t"}{.spec.instances}{"\n"}{end}'
   ```

   Expected: `0` for every matching row.

4. The branch's PVC is retained per `BranchStorageSpec.storage_class`
   retention; the data is not deleted on suspend:

   ```bash
   kubectl -n ai-blaise-citus get pvc -l ai-blaise.io/branch=<name>
   ```

   Expected: PVCs remain in `Bound` phase.

## Rollback

If clearing blockers caused a deletion of in-flight work:

1. Resume the branch immediately:

   ```bash
   kubectl -n ai-blaise-citus patch branch/<name> --type=merge -p \
     '{"spec":{"suspend": false}}'
   ```

2. Wait for `phase: Running` and the compute pod to be ready:

   ```bash
   kubectl -n ai-blaise-citus wait --for=condition=Ready \
     pod -l ai-blaise.io/branch=<name> --timeout=300s
   ```

3. If the underlying database state needs to be reverted because a
   committed transaction was rolled back, follow `pitr-restore.md`
   with the timestamp recorded immediately before the suspend was
   forced.

## References

- Related: `pitr-restore.md`, `lost-shard.md`, `rebalance-stuck.md`.
- CRD: `operator/src/crds/branch.rs` (`FEATURE: R2`, `C6`, `C7`, `C8`).
- Companion module: `operator/src/reconcile/mod.rs`,
  `operator/src/crds/scheduled_repack.rs` (`FEATURE: R7`).
- Probe surface: `sidecar/shared/README.md`
  (the branch compute pod inherits the probe contract).
- agentmemory pattern: `CITUS-BRANCH-SUSPEND-STUCK-<branch>-<UTC>`
  recorded against `:3911` with the matched blocker cause and the
  cleared `suspendBlockers` snapshot.

## Automated drill

`FEATURE: DR5`

Creates a Branch CR, drives the suspend -> resume -> promote-to-primary cycle, and asserts the branch reaches each target phase inside the RTO budget. Reports the suspend/resume p50 as `rto_s`.

```bash
# Quick mode (1-minute cap; mock-when-missing fallback if no kind cluster):
make -f Makefile.ai-blaise dr-drill-branch-promote

# Full mode against a live kind cluster:
DR_DRILL_QUICK=0 DR_DRILL_NAMESPACE=ai-blaise-citus DR_DRILL_CLUSTER=primary \
  bash benchmarks/dr-drills/branch-promote-drill.sh
```

The drill writes a structured JSON report to
`benchmarks/dr-drills/reports/<drill>-<timestamp>.json` with `rto_s`,
`rpo_s`, `errors_during`, and `success`. The CI smoke runner
`ci/ai-blaise/dr-drill-quick-mode-smoke.sh` runs every drill once and
emits an `aggregate-<timestamp>.json` containing the per-drill rows.
