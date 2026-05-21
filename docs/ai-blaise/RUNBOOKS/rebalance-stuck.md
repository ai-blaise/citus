# Runbook: Rebalance Stuck

`FEATURE: S2` `FEATURE: S5`

## When to run this

A Citus shard rebalance is not progressing. `citus_rebalance_status()`
shows the same shard with the same percentage for more than the
configured progress window, no new placements are being created, and
the coordinator's `pg_dist_background_job` log shows the rebalance job
still in `running` state.

## Pre-conditions

- A rebalance job has been started by the operator's
  `ShardGroup` reconciler or manually with
  `SELECT citus_rebalance_start();`.
- `sidecar/raft` is enabled for every shard group involved
  (`operator/src/crds/shard_group.rs`, `sidecar/raft/src/lib.rs`).
- Replication slots used by the rebalance are healthy
  (`pg_stat_replication`).
- No active `pitr-restore.md` operation is running against the source
  or target node.

## Detection

1. Read the rebalance progress view:

   ```sql
   SELECT job_id, table_name, shardid,
          sourcename, sourceport, targetname, targetport,
          progress, error_message, status
     FROM citus_rebalance_status()
    ORDER BY job_id, shardid;
   ```

   A stuck rebalance has at least one row with `status = 'running'`
   whose `progress` has not advanced in the last 5 minutes.

2. Confirm the background job is still scheduled:

   ```sql
   SELECT job_id, status, started_at, finished_at, message
     FROM pg_dist_background_job
    WHERE job_type = 'rebalance_table_shards'
    ORDER BY started_at DESC
    LIMIT 5;
   ```

3. Identify the blocker. Three classes are common:

   - Lock conflict on the source or target. Run on both worker nodes
     listed in step 1:

     ```sql
     SELECT pid, locktype, relation::regclass, mode, granted,
            wait_event_type, wait_event, query_start, query
       FROM pg_locks l
       JOIN pg_stat_activity a USING (pid)
      WHERE NOT granted
         OR (relation::regclass::text LIKE '%_%shardid%')
      ORDER BY query_start NULLS LAST;
     ```

   - Replication slot lag. The rebalance uses a logical replication
     slot per move:

     ```sql
     SELECT slot_name, plugin, active, restart_lsn, confirmed_flush_lsn,
            pg_wal_lsn_diff(pg_current_wal_lsn(), restart_lsn)
              AS restart_lag_bytes,
            pg_wal_lsn_diff(pg_current_wal_lsn(), confirmed_flush_lsn)
              AS flush_lag_bytes
       FROM pg_replication_slots
      WHERE slot_name LIKE 'citus_shard_%'
      ORDER BY restart_lag_bytes DESC;
     ```

     A slot whose `restart_lag_bytes` exceeds
     `max_replication_slot_lag` is blocking the move.

   - Network blip between coordinator and worker, or between two
     workers. The pool exposes this in metrics:

     ```bash
     kubectl -n ai-blaise-citus port-forward svc/ai-blaise-citus-pool 9090:9090 &
     curl -sf localhost:9090/metrics \
       | grep -E 'ai_blaise_citus_pool_upstream_errors_total|ai_blaise_citus_pool_upstream_connect_seconds'
     ```

4. Cross-check raft. If a shard group's leader flipped during the
   move, the raft sidecar will report the new leader and the rebalance
   will be waiting on the old leader's lease:

   ```bash
   kubectl -n ai-blaise-citus port-forward \
     deploy/ai-blaise-citus-sidecar-raft 8080:8080 &
   curl -sf localhost:8080/raft/status \
     | jq '.shard_groups[] | {group: .shard_group, decision: .decision, term: .term}'
   ```

## Recovery procedure

1. Cancel the current move. This releases the logical replication slot
   and the lock set:

   ```sql
   SELECT citus_rebalance_stop();
   ```

   Confirm the job moved to `cancelled`:

   ```sql
   SELECT job_id, status, message
     FROM pg_dist_background_job
    WHERE job_type = 'rebalance_table_shards'
    ORDER BY started_at DESC
    LIMIT 1;
   ```

2. Drop any orphan replication slot the cancel did not clean:

   ```sql
   SELECT pg_drop_replication_slot(slot_name)
     FROM pg_replication_slots
    WHERE slot_name LIKE 'citus_shard_%'
      AND NOT active;
   ```

3. Resolve the matched blocker.

   - Lock conflict. Cancel the holding session only after confirming
     it is safe to cancel:

     ```sql
     SELECT pg_cancel_backend(<pid>);
     ```

   - Replication slot lag. Wait for the slot to catch up before
     restarting. If the lag is unbounded because the source is down,
     follow `lost-shard.md` first.

   - Network blip. Roll the pool to reset its upstream pool:

     ```bash
     kubectl -n ai-blaise-citus rollout restart deploy/ai-blaise-citus-pool
     kubectl -n ai-blaise-citus rollout status deploy/ai-blaise-citus-pool
     ```

4. Restart the rebalance with a smaller batch. The default batch is
   one shard at a time; force serial moves and a single placement at a
   time so a re-stall is bounded:

   ```sql
   SELECT citus_rebalance_start(
            rebalance_strategy := 'by_disk_size',
            shard_transfer_mode := 'auto',
            max_shard_move_size := '4GB',
            max_parallel_moves := 1
          );
   ```

5. Watch progress. The rebalance status view advances on every
   committed placement:

   ```sql
   \watch 10
   SELECT job_id, shardid, progress, status, error_message
     FROM citus_rebalance_status()
    WHERE status = 'running';
   ```

## Verification

1. The rebalance job finishes:

   ```sql
   SELECT job_id, status, finished_at
     FROM pg_dist_background_job
    WHERE job_type = 'rebalance_table_shards'
    ORDER BY started_at DESC
    LIMIT 1;
   ```

   Expected: `status = 'succeeded'` and `finished_at` is not null.

2. No leftover replication slots:

   ```sql
   SELECT slot_name FROM pg_replication_slots
    WHERE slot_name LIKE 'citus_shard_%';
   ```

   Expected: zero rows.

3. Placement balance matches the `PlacementPolicy` constraints from
   the `ShardGroup` spec:

   ```sql
   SELECT nodename, COUNT(*) AS placements,
          SUM(pg_total_relation_size(format('%I.%I_%s',
              n.nspname, c.relname, p.shardid)::regclass))
            AS bytes
     FROM pg_dist_placement p
     JOIN pg_dist_node USING (groupid)
     JOIN pg_dist_shard USING (shardid)
     JOIN pg_class c ON c.oid = logicalrelid
     JOIN pg_namespace n ON n.oid = c.relnamespace
    GROUP BY nodename
    ORDER BY bytes DESC;
   ```

   Expected: `(max_bytes - min_bytes) / avg_bytes <= max_skew` where
   `max_skew` is the `PlacementPolicy.max_skew` from the CR.

4. Raft decisions are stable across the moved groups:

   ```bash
   curl -sf localhost:8080/raft/status \
     | jq '.shard_groups[] | select(.decision != "keep_leader")'
   ```

   Expected: `[]`.

## Rollback

If the rebalance corrupted placement metadata (rare, only after a
forced cancel mid-WAL-apply):

1. Stop the rebalance job:

   ```sql
   SELECT citus_rebalance_stop();
   ```

2. Inspect placement states for `SHARD_STATE_TO_DELETE` (4) or
   `SHARD_STATE_INACTIVE` (3) rows that should be active:

   ```sql
   SELECT shardid, placementid, shardstate, nodename
     FROM pg_dist_placement
    JOIN pg_dist_node USING (groupid)
    WHERE shardstate IN (3, 4)
    ORDER BY shardid;
   ```

3. For each row that the raft sidecar still considers the active
   placement, restore `shardstate = 1`:

   ```sql
   UPDATE pg_dist_placement
      SET shardstate = 1
    WHERE placementid = <placementid>;
   SELECT citus_cleanup_orphaned_resources();
   ```

4. If the raft sidecar and the coordinator can no longer agree, follow
   `split-brain.md`.

## References

- Related: `lost-shard.md`, `split-brain.md`, `tenant-migration.md`.
- CRD: `operator/src/crds/shard_group.rs` (`FEATURE: S2`),
  `operator/src/crds/citus_cluster.rs` (`FEATURE: S4`).
- Companion module: `sidecar/raft/src/lib.rs` (`FEATURE: S5`),
  `companion/src/observability.rs` (`FEATURE: O3`),
  `pool/src/runtime.rs` (`FEATURE: T1`, `T3`, `T9`, `T12`).
- ADR: `docs/ai-blaise/ADR/0007-raft-per-shardgroup.md`.
- agentmemory pattern: `CITUS-REBALANCE-STUCK-<jobid>-<UTC>` recorded
  against `:3911` with the cancelled job ID, the matched blocker, and
  the restart parameters used.
