# Runbook: Tenant Migration

`FEATURE: S10` `FEATURE: S14` `FEATURE: TO1` `FEATURE: TO2`
`FEATURE: TO3` `FEATURE: TO4` `FEATURE: TO5`

## When to run this

Move an active tenant from one schema or region to another without a
maintenance window. Typical triggers: rebalancing tenants across
workers after a workload skew, evacuating a region for a planned
upgrade, or honouring a region-affinity change in
`TenantSpec.region_affinity`.

## Pre-conditions

- The `Tenant` CR exists for the tenant being moved
  (`operator/src/crds/tenant.rs`, `FEATURE: S10`).
- Source and target workers are healthy; their pool admin probes return
  `/readyz` 200.
- The target region (when changing region) exists and is registered as
  a `Region` CR (`operator/src/crds/region.rs`, `FEATURE: MR1`).
- `SurvivalGoal` for the tenant's shard groups still resolves after
  the move (`operator/src/crds/survival_goal.rs`).
- A recent encrypted backup exists in the `sidecar/backup` archive so
  rollback via PITR is possible.
- No active `rebalance-stuck.md` recovery is in progress.

## Pre-flight checks

1. Validate that the planned move is structurally valid using the
   companion canonical model. The model rejects same-worker moves and
   empty fields up front:

   ```bash
   cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- \
     run-operations-canonical \
     | grep '^tenant_move'
   ```

   Then construct the operational move plan and dry-run it against the
   coordinator:

   ```sql
   SELECT companion.tenant_move_plan(
            tenant_name := '<tenant>',
            source_worker := '<source_host>:<source_port>',
            target_worker := '<target_host>:<target_port>',
            region_affinity := '<region_or_null>'
          );
   ```

2. Capture a baseline. Record tenant counters before the move:

   ```sql
   SELECT
     COUNT(*) AS table_count,
     SUM(pg_total_relation_size(c.oid)) AS total_bytes,
     MAX(n.nspname) AS schema_name
   FROM pg_class c
   JOIN pg_namespace n ON n.oid = c.relnamespace
   WHERE n.nspname = '<tenant_schema>';
   ```

   Snapshot the ledger hash chain head (if `companion/src/ledger.rs`
   is enabled for the tenant):

   ```sql
   SELECT MAX(seq) AS head_seq,
          MAX(hash) AS head_hash
     FROM <tenant_schema>.ledger;
   ```

3. Confirm quota headroom on the target worker. The pool's tenant
   counters surface live connections and QPS:

   ```bash
   kubectl -n ai-blaise-citus port-forward svc/ai-blaise-citus-pool 9090:9090 &
   curl -sf localhost:9090/metrics \
     | grep -E 'ai_blaise_citus_pool_tenant_(connections|qps)\{tenant="<tenant>"'
   ```

   The target worker's available connections must exceed
   `TenantQuotas.max_connections`.

4. Confirm that no in-flight `tenant_archive` job is running for the
   same tenant (`companion/src/tenants.rs`).

## Migration procedure

1. Freeze writes from new clients. The pool exposes a per-tenant
   read-only flag; setting it stops accepting new write transactions
   while letting reads continue:

   ```bash
   kubectl -n ai-blaise-citus set env deploy/ai-blaise-citus-pool \
     AI_BLAISE_POOL_TENANT_READONLY=<tenant>
   kubectl -n ai-blaise-citus rollout status deploy/ai-blaise-citus-pool
   ```

2. Drain in-flight writes. Wait until the tenant has zero active
   non-read transactions:

   ```sql
   SELECT pid, state, query_start, query
     FROM pg_stat_activity
    WHERE backend_xmin IS NOT NULL
      AND application_name LIKE '<tenant>%';
   ```

   Expected: zero rows for at most 30 seconds. If a transaction does
   not drain, cancel it explicitly per `branch-suspend-stuck.md`.

3. Invoke `tenant_move()`. The companion function implements an online
   move using logical replication for catch-up and a final cutover
   that swaps placement rows under the tenant's catalog lock:

   ```sql
   SELECT companion.tenant_move(
            tenant_name := '<tenant>',
            source_worker := '<source_host>:<source_port>',
            target_worker := '<target_host>:<target_port>',
            region_affinity := '<region_or_null>',
            transfer_mode := 'logical',
            cutover_timeout_seconds := 60
          );
   ```

4. Poll progress. The function returns immediately; the long-running
   move is tracked in `pg_dist_background_job`:

   ```sql
   SELECT job_id, status, started_at,
          (SELECT message FROM pg_dist_background_task
            WHERE job_id = j.job_id
            ORDER BY task_id DESC LIMIT 1) AS task_message
     FROM pg_dist_background_job j
    WHERE job_type = 'tenant_move'
      AND command LIKE '%<tenant>%'
    ORDER BY started_at DESC
    LIMIT 1;
   ```

5. Cutover. Once the job reports `status = 'cutover_ready'`, flip the
   tenant's authoritative placement:

   ```sql
   SELECT companion.tenant_move_cutover('<tenant>');
   ```

   The cutover step holds an exclusive lock on the tenant's catalog
   row for the duration of `cutover_timeout_seconds`; if the timeout
   fires, the function rolls back and the tenant remains on the source
   worker.

6. Patch the `Tenant` CR so the operator reconciler reflects the new
   region affinity (if any):

   ```bash
   kubectl -n ai-blaise-citus patch tenant/<tenant> --type=merge -p \
     '{"spec":{"region_affinity":"<region>"}}'
   ```

7. Re-enable writes:

   ```bash
   kubectl -n ai-blaise-citus set env deploy/ai-blaise-citus-pool \
     AI_BLAISE_POOL_TENANT_READONLY-
   kubectl -n ai-blaise-citus rollout status deploy/ai-blaise-citus-pool
   ```

## Validation

1. Tenant counters match the baseline:

   ```sql
   SELECT
     COUNT(*) AS table_count,
     SUM(pg_total_relation_size(c.oid)) AS total_bytes
   FROM pg_class c
   JOIN pg_namespace n ON n.oid = c.relnamespace
   WHERE n.nspname = '<tenant_schema>';
   ```

   Expected: `table_count` matches the baseline exactly;
   `total_bytes` is within 2% (compression and bloat vary across
   workers).

2. Ledger head matches:

   ```sql
   SELECT MAX(seq) AS head_seq, MAX(hash) AS head_hash
     FROM <tenant_schema>.ledger;
   ```

   Expected: `head_seq` matches the pre-move baseline; the move is
   write-suspended so no new ledger rows should exist between snapshot
   and cutover.

3. The pool routes the tenant to the new worker:

   ```bash
   curl -sf localhost:9090/metrics \
     | grep "ai_blaise_citus_pool_upstream_route_total{tenant=\"<tenant>\"" \
     | sort
   ```

   Expected: traffic flows to `target_host:target_port`; the source
   row stops incrementing.

4. The `Tenant` CR reports the new region affinity if applicable:

   ```bash
   kubectl -n ai-blaise-citus get tenant/<tenant> \
     -o jsonpath='{.spec.region_affinity}{"\n"}'
   ```

5. A representative tenant read returns the expected row count:

   ```sql
   SELECT COUNT(*) FROM <tenant_schema>.<largest_table>;
   ```

## Rollback

The move is reversible until step 5 (`tenant_move_cutover`) succeeds.

1. Before cutover. Cancel the background job; logical replication
   tears down and the tenant continues on the source:

   ```sql
   SELECT pg_dist_background_job_cancel(<job_id>);
   ```

   Re-enable writes via step 7 of the migration procedure.

2. After cutover but within the configured rollback window
   (default 1 hour). Run the cutover in reverse:

   ```sql
   SELECT companion.tenant_move_cutover('<tenant>',
            reverse := true,
            source_worker := '<source_host>:<source_port>',
            target_worker := '<target_host>:<target_port>');
   ```

3. Outside the rollback window. The reverse cutover is unsafe because
   downstream writes have already landed on the new worker. Follow
   `pitr-restore.md` to restore the tenant from the encrypted backup
   to the timestamp immediately before step 5 ran, then redo the move
   after the underlying cause is fixed.

## References

- Related: `lost-shard.md`, `split-brain.md`, `rebalance-stuck.md`,
  `pitr-restore.md`.
- CRD: `operator/src/crds/tenant.rs`
  (`FEATURE: S10`, `TO1`, `TO2`, `TO5`),
  `operator/src/crds/region.rs` (`FEATURE: MR1`).
- Companion module: `companion/src/tenants.rs`
  (`FEATURE: S14`, `TO3`, `TO4`, `TO5`),
  `companion/src/ledger.rs`.
- Pool surface: `pool/src/runtime.rs` (`FEATURE: T1`, `T3`),
  `pool/src/shard_map.rs` (`FEATURE: T2`).
- agentmemory pattern: `CITUS-TENANT-MIGRATION-<tenant>-<UTC>` recorded
  against `:3911` with the baseline counters, the cutover timestamp,
  and the post-move counters.
