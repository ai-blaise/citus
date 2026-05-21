# Runbook: Lost Shard

`FEATURE: S2` `FEATURE: S5`

## When to run this

A shard placement is unreachable, or the worker holding the only voting
replica is permanently gone. Pool error rate spikes for the affected key
range, distributed queries that touch that shard return
`ERROR: connection to shard X failed`, and the shard-group raft sidecar
reports `WaitForQuorum` or fails to elect a new leader on its own.

## Pre-conditions

- The `CitusCluster` CR exists and the coordinator pod is reachable.
- `sidecar/raft` is enabled in `values-prod.yaml` for the affected
  shard group (`sidecars[name=raft].enabled: true`).
- At least one non-failed placement of the shard exists in
  `pg_dist_placement`, or a recent encrypted base backup plus WAL archive
  is available from the `sidecar/backup` archive URI.
- Operator and pool pods are running; their `/readyz` probes return 200.
- The on-call has psql access to the coordinator and `kubectl` access to
  the cluster namespace.

## Detection

1. Inspect pool rejection counters from the affected key range:

   ```bash
   kubectl -n ai-blaise-citus port-forward svc/ai-blaise-citus-pool 9090:9090 &
   curl -sf localhost:9090/metrics \
     | grep -E 'ai_blaise_citus_pool_rejected_connections_total|ai_blaise_citus_pool_upstream_errors_total'
   ```

2. List shards and placements that the coordinator believes exist for
   the affected distributed table:

   ```sql
   SELECT shardid, shardminvalue, shardmaxvalue
     FROM pg_dist_shard
    WHERE logicalrelid = '<schema>.<table>'::regclass
    ORDER BY shardid;

   SELECT shardid, shardstate, nodename, nodeport, placementid
     FROM pg_dist_placement
    JOIN pg_dist_node USING (groupid)
    WHERE shardid IN (SELECT shardid FROM pg_dist_shard
                       WHERE logicalrelid = '<schema>.<table>'::regclass)
    ORDER BY shardid, placementid;
   ```

3. Cross-check what the raft sidecar believes about the shard-group
   leader and member set. The sidecar emits a canonical TSV report on
   demand; in a live cluster the same report is exposed over the
   sidecar's HTTP probe port:

   ```bash
   kubectl -n ai-blaise-citus port-forward \
     deploy/ai-blaise-citus-sidecar-raft 8080:8080 &
   curl -sf localhost:8080/raft/status \
     | jq '.shard_groups[] | select(.shard_group == "<group>")'
   ```

   The canonical model is reproducible offline with:

   ```bash
   cargo run -q -p ai_blaise_citus_sidecar_raft -- run-canonical
   ```

4. Confirm the placement is truly lost, not just slow. If the node is
   reachable but its postgres is down, this is a CNPG failover, not a
   lost shard — escalate to `disaster-recovery.md` instead.

   ```bash
   kubectl -n ai-blaise-citus get pod -l cnpg.io/cluster=<cnpgRef> \
     -o jsonpath='{range .items[*]}{.metadata.name}{"\t"}{.status.phase}{"\n"}{end}'
   ```

## Recovery procedure

1. Freeze writes to the affected shard's key range by patching the pool
   to mark that shard read-only. The pool's shard map honours the
   `pool.shard_map.readonly_shards` setting:

   ```bash
   kubectl -n ai-blaise-citus set env deploy/ai-blaise-citus-pool \
     AI_BLAISE_POOL_READONLY_SHARDS=<shardid>
   kubectl -n ai-blaise-citus rollout status deploy/ai-blaise-citus-pool
   ```

2. Identify the surviving placement that the raft sidecar wants to
   promote (`Promote { node_id, cnpg_pod }`). If the decision is
   `WaitForQuorum`, restore quorum first by adding a replica before any
   shard move — see step 6.

3. Force the coordinator placement state to match raft. For each lost
   placement, mark it inactive so Citus stops routing to it:

   ```sql
   UPDATE pg_dist_placement
      SET shardstate = 3
    WHERE placementid = <placementid>;
   ```

   `shardstate = 3` is `SHARD_STATE_INACTIVE`. Do not delete the row;
   the audit trail is required by `production-gap-audit.sh`.

4. Move the shard placement to the promoted replica:

   ```sql
   SELECT citus_move_shard_placement(
            <shardid>,
            '<old_nodename>', <old_nodeport>,
            '<new_nodename>', <new_nodeport>,
            transfer_mode := 'block_writes'
          );
   ```

   `block_writes` is required because the source placement is gone;
   `auto` and `force_logical` need the source to be live.

5. If no surviving placement exists, restore from the
   `sidecar/backup` archive into a new placement before invoking
   `citus_move_shard_placement`:

   ```bash
   kubectl -n ai-blaise-citus exec deploy/ai-blaise-citus-sidecar-backup -- \
     /usr/local/bin/citus-sidecar-backup restore-placement \
       --shard-id <shardid> \
       --target-node <new_nodename>:<new_nodeport> \
       --source-archive-uri "$BACKUP_ARCHIVE_URI" \
       --target-time "<RFC3339>"
   ```

6. Restore the raft quorum for the shard group. Add the new replica as
   a voting member through the raft sidecar admin endpoint:

   ```bash
   curl -sf -X POST localhost:8080/raft/<group>/members \
     -H 'content-type: application/json' \
     -d '{"node_id": "<new_node>", "voter": true, "cnpg_pod": "<pod>"}'
   ```

7. Re-enable writes to the shard:

   ```bash
   kubectl -n ai-blaise-citus set env deploy/ai-blaise-citus-pool \
     AI_BLAISE_POOL_READONLY_SHARDS-
   kubectl -n ai-blaise-citus rollout status deploy/ai-blaise-citus-pool
   ```

## Verification

1. The coordinator sees the new placement as active:

   ```sql
   SELECT shardid, shardstate, nodename, nodeport
     FROM pg_dist_placement
    JOIN pg_dist_node USING (groupid)
    WHERE shardid = <shardid>;
   ```

   Expected: at least one row with `shardstate = 1`.

2. The raft sidecar reports `KeepLeader` for the shard group:

   ```bash
   curl -sf localhost:8080/raft/status \
     | jq '.shard_groups[] | select(.shard_group == "<group>") | .decision'
   ```

   Expected: `"keep_leader"`.

3. A point query against the recovered key range returns rows from the
   new placement:

   ```sql
   EXPLAIN (VERBOSE, COSTS OFF)
   SELECT 1 FROM <schema>.<table>
    WHERE <distribution_column> = <key_in_recovered_range>;
   ```

   Expected: the plan references the new `nodename:nodeport`.

4. Pool error counters stop climbing:

   ```bash
   curl -sf localhost:9090/metrics \
     | grep ai_blaise_citus_pool_upstream_errors_total
   ```

   Expected: the counter is flat over a 5-minute window.

## Rollback

If `citus_move_shard_placement` fails partway and leaves the new
placement in `SHARD_STATE_TO_DELETE`:

1. Re-mark the shard read-only as in recovery step 1.
2. Roll back the new placement row:

   ```sql
   UPDATE pg_dist_placement
      SET shardstate = 4
    WHERE placementid = <new_placementid>;
   SELECT citus_cleanup_orphaned_resources();
   ```

3. Restore the original placement row to active only if its node has
   come back online; otherwise wait for a replacement worker and rerun
   the recovery procedure.
4. If the raft sidecar cannot reach quorum after rollback, follow
   `split-brain.md` to force-reset the placement on the minority side.

## References

- Related: `split-brain.md`, `rebalance-stuck.md`, `disaster-recovery.md`.
- CRD: `operator/src/crds/shard_group.rs` (`FEATURE: S2`).
- Companion module: `sidecar/raft/src/lib.rs` (`FEATURE: S5`),
  `sidecar/backup/src/lib.rs` (`FEATURE: B1`, `B3`, `B4`, `B6`).
- Pool surface: `pool/src/shard_map.rs` (`FEATURE: T2`, `T3`).
- ADR: `docs/ai-blaise/ADR/0007-raft-per-shardgroup.md`,
  `docs/ai-blaise/ADR/0006-cnpg-substrate-not-bypass.md`.
- agentmemory pattern: `CITUS-LOST-SHARD-<shardid>-<UTC>` recorded
  against `:3911` with the resolved placement and the raft decision
  TSV row attached as evidence.
