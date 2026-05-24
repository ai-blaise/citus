# Runbook: Split Brain

`FEATURE: S5` `FEATURE: MR1` `FEATURE: MR2` `FEATURE: MR8`

## When to run this

The coordinator and a worker disagree on which placement is active for
a shard, or a region partitions and both halves accept writes. Symptoms
include divergent rows for the same primary key across regions, two
`sidecar/raft` instances reporting themselves as leader for the same
shard group, or the federation/replication-lag view showing unbounded
lag in one direction.

## Pre-conditions

- `SurvivalGoal` is configured for at least two regions
  (`operator/src/crds/survival_goal.rs`).
- `sidecar/hlc` is explicitly enabled on every region's worker fleet by a
  promoted command-center release overlay so closed timestamps are comparable
  across regions
  (`sidecar/hlc/src/lib.rs`, `FEATURE: S9`).
- `pgactive` conflict policy is declared via the `ConflictPolicy` CR
  (`operator/src/crds/conflict_policy.rs`, `FEATURE: C4`, `C5`).
- Operator and pool pods are running in every surviving region.
- An incident ticket is open before any forced state change.

## Detection

1. Confirm placement drift across regions. Run on the coordinator of
   each region:

   ```sql
   SELECT shardid, shardstate, nodename, nodeport, placementid
     FROM pg_dist_placement
    JOIN pg_dist_node USING (groupid)
    ORDER BY shardid, nodename;
   ```

   Diff the result sets between regions. Any row that exists in one
   region with `shardstate = 1` and in another region with
   `shardstate IN (3, 4)`, or any row whose `(shardid, nodename)`
   appears in only one region, is drift.

2. Confirm two raft leaders. Port-forward each region's raft sidecar
   and read its status:

   ```bash
   for region in $(kubectl get region.ai-blaise.io -o jsonpath='{.items[*].metadata.name}'); do
     kubectl -n ai-blaise-citus-${region} port-forward \
       deploy/ai-blaise-citus-sidecar-raft 8080:8080 &
     curl -sf localhost:8080/raft/status \
       | jq --arg r "${region}" '.shard_groups[] | {region: $r, group: .shard_group, decision: .decision, term: .term}'
     kill %% 2>/dev/null || true
   done
   ```

   A real split-brain shows two rows with `decision = "keep_leader"`
   for the same `group` but different `term`.

3. Confirm closed-timestamp divergence using the HLC sidecar canonical
   model and the per-region replication-lag view:

   ```bash
   cargo run -q -p ai_blaise_citus_sidecar_hlc -- run-canonical
   ```

   ```sql
   SELECT region, last_closed_ts, replication_lag_ms
     FROM companion.pg_dist_replication_lag;
   ```

   The authoritative region is the one whose `last_closed_ts` is
   newest *and* whose `replication_lag_ms` is below
   `SurvivalGoal.spec.maxReplicationLagMs`.

4. Confirm reference-table conflict status:

   ```sql
   SELECT table_name, conflict_count, last_conflict_at
     FROM companion.pgactive_conflict_summary
    ORDER BY conflict_count DESC
    LIMIT 20;
   ```

## Recovery procedure

1. Quarantine the minority region from client traffic. Patch the pool
   in the minority region to refuse new connections and drain the
   existing pool:

   ```bash
   kubectl -n ai-blaise-citus-${MINORITY_REGION} patch deploy/ai-blaise-citus-pool \
     --type=merge -p '{"spec":{"replicas":0}}'
   ```

2. Stop accepting writes from sidecars in the minority region. Suspend
   the vectorizer, edge-functions, schema-job, and webhook deployments:

   ```bash
   for s in vectorizer edge-functions schema-job webhooks; do
     kubectl -n ai-blaise-citus-${MINORITY_REGION} scale deploy/ai-blaise-citus-sidecar-${s} --replicas=0
   done
   ```

3. Confirm the authoritative region using HLC and closed-timestamp
   evidence. The authoritative region must have:

   - the highest `HlcTimestamp.physical_ms` advertised by
     `sidecar/hlc`,
   - a `companion.pg_dist_replication_lag` row whose
     `replication_lag_ms` is within the configured budget,
   - a `RaftShardGroupPlan` whose term is the highest seen across
     regions.

   Record the chosen region in the incident ticket. Do not proceed
   until two operators sign off.

4. Force-reset placement state on the minority region. For each
   shard whose minority `pg_dist_placement` row disagrees with the
   authoritative region, mark the minority placement inactive:

   ```sql
   UPDATE pg_dist_placement
      SET shardstate = 3
    WHERE placementid = <minority_placementid>;
   ```

5. Force the minority raft sidecar to step down. The raft sidecar
   admin endpoint accepts a forced-step-down request that requires the
   term observed in the authoritative region:

   ```bash
   curl -sf -X POST localhost:8080/raft/<group>/step-down \
     -H 'content-type: application/json' \
     -d '{"new_term": <authoritative_term>, "reason": "split-brain"}'
   ```

   The sidecar refuses the request if the supplied term is not strictly
   greater than its own; if that happens the authoritative region is
   misidentified, return to step 3.

6. Replay diverging reference-table writes through the configured
   `ConflictPolicy`. The companion module exposes a canonical operation
   that emits the SQL plan; run the resulting commands on the
   authoritative coordinator:

   ```bash
   cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- \
     run-operations-canonical \
     | grep '^conflict_policy'
   ```

7. Restore traffic to the minority region only after its raft sidecar
   reports `WaitForQuorum`, its replication lag falls below the
   `SurvivalGoal` budget, and its `pg_dist_placement` matches the
   authoritative region. Scale the pool back up:

   ```bash
   kubectl -n ai-blaise-citus-${MINORITY_REGION} scale deploy/ai-blaise-citus-pool --replicas=3
   kubectl -n ai-blaise-citus-${MINORITY_REGION} rollout status deploy/ai-blaise-citus-pool
   ```

## Verification

1. Placement state is identical across regions:

   ```bash
   for region in $REGIONS; do
     kubectl -n ai-blaise-citus-${region} exec deploy/ai-blaise-citus-coordinator -- \
       psql -tAc "SELECT shardid, shardstate, nodename FROM pg_dist_placement ORDER BY shardid, nodename" \
       > "/tmp/placement-${region}.tsv"
   done
   diff -u /tmp/placement-*.tsv
   ```

   Expected: empty diff.

2. Only one raft leader per shard group:

   ```bash
   curl -sf localhost:8080/raft/status \
     | jq '[.shard_groups[] | select(.decision == "keep_leader") | .shard_group] | group_by(.) | map(select(length > 1))'
   ```

   Expected: `[]`.

3. The conflict-policy report is empty or all entries are classified:

   ```sql
   SELECT COUNT(*) FROM companion.pgactive_conflict_summary
    WHERE last_conflict_at > now() - interval '15 minutes';
   ```

   Expected: `0`, or every recent conflict has a non-null
   `resolution_policy`.

4. Federation/replication-lag is bounded:

   ```sql
   SELECT region, replication_lag_ms
     FROM companion.pg_dist_replication_lag
    WHERE replication_lag_ms > <SurvivalGoal.maxReplicationLagMs>;
   ```

   Expected: zero rows.

## Rollback

If the forced step-down corrupts state in the authoritative region:

1. Stop the pool everywhere:

   ```bash
   kubectl get ns -l app.kubernetes.io/part-of=ai-blaise-citus -o name \
     | xargs -I {} kubectl -n {} scale deploy/ai-blaise-citus-pool --replicas=0
   ```

2. Snapshot the authoritative region's WAL position:

   ```sql
   SELECT pg_current_wal_lsn();
   ```

3. Follow `pitr-restore.md` to restore the cluster to the timestamp
   immediately before step 4 of the recovery procedure ran. The
   `incident.timestamp_before_force_reset` field on the incident ticket
   is the target time.

4. Re-execute recovery, this time pausing for human review between
   steps 3 and 4.

## References

- Related: `lost-shard.md`, `disaster-recovery.md`, `pitr-restore.md`.
- CRD: `operator/src/crds/region.rs` (`FEATURE: MR1`),
  `operator/src/crds/survival_goal.rs` (`FEATURE: S11`, `MR2`),
  `operator/src/crds/conflict_policy.rs` (`FEATURE: C4`, `C5`).
- Companion module: `sidecar/raft/src/lib.rs` (`FEATURE: S5`),
  `sidecar/hlc/src/lib.rs` (`FEATURE: S9`),
  `companion/src/observability.rs` (`FEATURE: O3`, `R4`).
- ADR: `docs/ai-blaise/ADR/0007-raft-per-shardgroup.md`.
- agentmemory pattern: `CITUS-SPLIT-BRAIN-<region-pair>-<UTC>` recorded
  against `:3911` with the authoritative-region decision and signed-off
  operator IDs.
