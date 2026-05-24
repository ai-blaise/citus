# Runbook: Point-in-Time Restore

`FEATURE: B1` `FEATURE: B2` `FEATURE: B3` `FEATURE: B4` `FEATURE: B6`

## When to run this

Restore a cluster or single tenant to a point in time. Triggers
include: a destructive deploy, a corrupt batch job that has been
replicated to all replicas, a logical bug that has poisoned data, or
a rollback step from another runbook (`split-brain.md`,
`tenant-migration.md`, `branch-suspend-stuck.md`).

## Pre-conditions

- The `Backup` CR exists and at least one successful base backup is
  present in the archive (`operator/src/crds/backup.rs`,
  `FEATURE: B2`, `B6`).
- The WAL archive at `BackupTarget.bucket / BackupTarget.prefix` is
  contiguous between the base backup's start time and the target
  timestamp.
- `sidecar/backup` is explicitly enabled by a promoted command-center release
  overlay for the restore scope. The default production profile keeps alpha
  sidecars disabled until their feature status and measured evidence are
  promoted in `docs/ai-blaise/NEW_FEATURES.md`.
- The KMS key referenced by `BackupEncryption.kms_key_ref` is reachable
  from the restore environment.
- An incident ticket exists with the requested RPO and RTO.

## Identifying RPO/RTO targets

1. The RPO is the latest acceptable timestamp before the incident.
   Confirm the available WAL archive coverage:

   ```bash
   kubectl -n ai-blaise-citus exec deploy/ai-blaise-citus-sidecar-backup -- \
     /usr/local/bin/citus-sidecar-backup archive-summary
   ```

   Read the `earliest_wal`, `latest_wal`, and `latest_base_backup`
   fields. The smallest restorable timestamp is `latest_base_backup`;
   the largest is `latest_wal - sidecar/backup.wal_safety_margin`
   (default 30 seconds).

2. The RTO is bounded by the size of the base backup plus the WAL
   replay span. Estimate using the canonical runtime report:

   ```bash
   cargo run -q -p ai_blaise_citus_sidecar_backup -- run-runtime-canonical
   ```

   The TSV columns include `base_backup_size_bytes` and
   `wal_replay_seconds_per_gb`; multiply to get the restore window.

3. Record both targets in the incident ticket and have two operators
   sign off before proceeding. PITR is destructive when applied to the
   live cluster.

## Picking the restore target

A PITR has three valid targets. Pick one before running any command.

- Read-only branch (`QueryableBackupBranchPlan`). Spins up a new
  branch CR backed by the archive, attached at the requested
  timestamp. Use this to validate before promoting.
- New cluster (`PitrRestorePlan.target_cluster`). Restores into a
  brand-new `CitusCluster`; the original cluster is untouched.
- In-place restore. Restores into the existing cluster, replacing all
  data after the target timestamp. Only run after a read-only branch
  has confirmed the data.

## Restore procedure

### Path A — read-only branch (recommended first step)

1. Create a `Branch` CR with `branch_type: Snapshot` and pointing at
   the archive:

   ```yaml
   apiVersion: ai-blaise.io/v1alpha1
   kind: Branch
   metadata:
     name: pitr-<incident>-<UTC>
     namespace: ai-blaise-citus
   spec:
     source_cluster: <source>
     branch_type: Snapshot
     suspend: false
     storage:
       size: <same_as_source>
       storage_class: <class>
     restore:
       source_archive_uri: <BACKUP_ARCHIVE_URI>
       target_time: "<RFC3339>"
       read_only: true
   ```

   ```bash
   kubectl apply -f /tmp/branch-pitr.yaml
   ```

2. Wait for the branch to report `phase: Running`:

   ```bash
   kubectl -n ai-blaise-citus wait --for=jsonpath='{.status.phase}=Running' \
     branch/pitr-<incident>-<UTC> --timeout=$((<rto_minutes> * 60))s
   ```

3. Validate the data on the branch using the queries in the
   `Validation` section below. The branch is read-only and the source
   is not affected.

### Path B — new cluster

1. Render a new `CitusCluster` manifest with `spec.restore`:

   ```bash
   helm template ai-blaise-citus-pitr ai-blaise/command-center: helm/charts/citus-cluster \
     -f ai-blaise/command-center: helm/charts/citus-cluster/values-prod.yaml \
     --set "restore.source_archive_uri=$BACKUP_ARCHIVE_URI" \
     --set "restore.target_time=<RFC3339>" \
     --set "global.requireImageDigest=true" \
     --set "operator.image.digest=$OPERATOR_IMAGE_DIGEST" \
     --set "pool.image.digest=$POOL_IMAGE_DIGEST" \
     > /tmp/pitr.yaml
   ```

2. Apply through the production-safe deploy wrapper:

   ```bash
   DEPLOY_PROFILE=prod MODE=install \
   OPERATOR_IMAGE_DIGEST=$OPERATOR_IMAGE_DIGEST \
   POOL_IMAGE_DIGEST=$POOL_IMAGE_DIGEST \
   NAMESPACE=ai-blaise-citus-pitr \
   scripts/citus-scale/deploy.sh
   ```

3. Wait for the new cluster's coordinator to be ready and for
   `sidecar/backup` to report `PitrRestorePlan.status = Completed`.

### Path C — in-place restore

Only run after a Path A branch has confirmed the data.

1. Freeze pool traffic everywhere:

   ```bash
   kubectl -n ai-blaise-citus scale deploy/ai-blaise-citus-pool --replicas=0
   ```

2. Snapshot the pre-restore state by writing a final base backup so
   the in-place restore itself is reversible:

   ```bash
   kubectl -n ai-blaise-citus exec deploy/ai-blaise-citus-sidecar-backup -- \
     /usr/local/bin/citus-sidecar-backup backup-now \
       --label "pre-pitr-<incident>"
   ```

3. Invoke PITR through the sidecar:

   ```bash
   kubectl -n ai-blaise-citus exec deploy/ai-blaise-citus-sidecar-backup -- \
     /usr/local/bin/citus-sidecar-backup restore \
       --cluster <source_cluster> \
       --source-archive-uri "$BACKUP_ARCHIVE_URI" \
       --target-time "<RFC3339>" \
       --target-cluster <source_cluster> \
       --in-place
   ```

   The companion canonical model rejects empty fields and timestamps
   that are not RFC3339; rerun
   `cargo run -q -p ai_blaise_citus_sidecar_backup -- run-canonical`
   if the sidecar refuses the request.

4. Wait for `PitrRestorePlan.status = Completed` on the sidecar's
   probe port:

   ```bash
   kubectl -n ai-blaise-citus port-forward \
     deploy/ai-blaise-citus-sidecar-backup 8080:8080 &
   curl -sf localhost:8080/backup/status \
     | jq '.pitr'
   ```

## Validation

1. The cluster reaches a consistent recovery point:

   ```sql
   SELECT pg_is_in_recovery(), pg_last_wal_replay_lsn(),
          pg_last_xact_replay_timestamp();
   ```

   Expected: `pg_is_in_recovery() = false` after recovery completes,
   and `pg_last_xact_replay_timestamp()` is no later than the target.

2. Tenant counts match expectations:

   ```sql
   SELECT COUNT(*) AS tenant_count FROM pg_dist_tenant;
   SELECT n.nspname, COUNT(*) AS tables
     FROM pg_class c
     JOIN pg_namespace n ON n.oid = c.relnamespace
    WHERE n.nspname IN (SELECT name FROM pg_dist_tenant)
    GROUP BY n.nspname
    ORDER BY n.nspname;
   ```

3. Shard placements are intact and consistent:

   ```sql
   SELECT shardid, shardstate, COUNT(*) AS replicas
     FROM pg_dist_placement
    GROUP BY shardid, shardstate
   HAVING COUNT(*) FILTER (WHERE shardstate = 1) = 0
   ORDER BY shardid;
   ```

   Expected: zero rows (every shard has at least one active
   placement).

4. Ledger hash chains advance from the pre-restore head only after the
   target time:

   ```sql
   SELECT MIN(recorded_at), MAX(recorded_at), COUNT(*)
     FROM ledger.entries
    WHERE recorded_at > '<RFC3339>';
   ```

   Expected: `COUNT(*) = 0` immediately after restore; new entries
   only after traffic is restored in step 5 below.

5. Search-index freshness has reset to the restore point. The
   `SearchIndex` reconciler will re-emit refresh tasks; confirm:

   ```sql
   SELECT name, last_refresh_at, lag_seconds
     FROM companion.search_index_status
    ORDER BY lag_seconds DESC
    LIMIT 10;
   ```

## Switching traffic

1. Only after validation has passed, scale the pool back up:

   ```bash
   kubectl -n ai-blaise-citus scale deploy/ai-blaise-citus-pool --replicas=3
   kubectl -n ai-blaise-citus rollout status deploy/ai-blaise-citus-pool
   ```

2. If the restore target was a new cluster (Path B), update DNS or the
   `Federation` CR pointing at the cluster so clients route to the
   restored coordinator. Cut over only after the new coordinator's
   `/readyz` probe returns 200 on every pool replica.

3. Resume sidecar workloads that were drained for the restore:

   ```bash
   for s in vectorizer edge-functions schema-job webhooks; do
     kubectl -n ai-blaise-citus scale deploy/ai-blaise-citus-sidecar-${s} --replicas=1
   done
   ```

## Rollback

If validation fails on a restored cluster:

1. Stop traffic to the restored cluster.
2. Path A (branch). Delete the branch CR; the source cluster is
   untouched:

   ```bash
   kubectl -n ai-blaise-citus delete branch/pitr-<incident>-<UTC>
   ```

3. Path B (new cluster). Uninstall the new release; the original
   cluster is untouched:

   ```bash
   helm -n ai-blaise-citus-pitr uninstall ai-blaise-citus-pitr
   ```

4. Path C (in-place). Restore the `pre-pitr-<incident>` base backup
   captured in step 2 of Path C, choosing a target time after that
   backup label but before the in-place restore ran:

   ```bash
   kubectl -n ai-blaise-citus exec deploy/ai-blaise-citus-sidecar-backup -- \
     /usr/local/bin/citus-sidecar-backup restore \
       --cluster <source_cluster> \
       --source-archive-uri "$BACKUP_ARCHIVE_URI" \
       --target-time "<RFC3339_after_pre_backup>" \
       --target-cluster <source_cluster> \
       --in-place
   ```

## References

- Related: `disaster-recovery.md`, `split-brain.md`,
  `tenant-migration.md`, `branch-suspend-stuck.md`.
- CRD: `operator/src/crds/backup.rs` (`FEATURE: B2`, `B6`),
  `operator/src/crds/branch.rs` (`FEATURE: R2`, `C6`, `C7`, `C8`).
- Companion module: `sidecar/backup/src/lib.rs`
  (`FEATURE: B1`, `B3`, `B4`, `B6`).
- Production gate: `ci/ai-blaise/production-gap-audit.sh` verifies
  this runbook is registered alongside `disaster-recovery.md` and
  `production.md`.
- agentmemory pattern: `CITUS-PITR-RESTORE-<cluster>-<UTC>` recorded
  against `:3911` with the requested target timestamp, the chosen
  path (A/B/C), and the validation query results.
