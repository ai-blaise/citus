# Internal Modifications

This bootstrap commit records the fork policy and rebase anchor. Subsequent
bootstrap-v2 changes are summarized here; each modified subsystem also carries
its own adjacent `MODIFICATION.md`.

## 2026-08-26 — Upstream sync: citusdata/citus main 008b391a7 (23 commits)

- `src/` — True-ancestry merge of `efa65fc4d..008b391a7`. One conflict
  (`citus--14.0-1--15.0-1.sql`) resolved as ordered union: upstream's
  `citus_internal_distribute_object` + `citus_finish_citus_upgrade` includes
  first, the fork's five FEATURE UDF includes (T2, T3, TS19, TS20) as the
  trailing block. The pre-merge patch series 0001-0008 forward-applies
  cleanly to 008b391a7, so no patch context refresh was needed;
  `patches-check.sh` passes against the merged tree unchanged.
- Router seam verified, not assumed: #8692 (constant-false modify) and
  #8625 (`allow_unsafe_insert_select_pushdown`) rework `RouterJob` /
  `CheckAndBuildDelayedFastPathPlan` / `ConvertToQueryOnShard` at lines
  178-2355; patch 0004's placement-intersection hash table (~3800 region)
  and patch 0006's coordinator-skip probe (file head + tail) are disjoint,
  and neither upstream commit alters the `ShardExists` /
  `ActiveShardPlacementList` semantics the T3 probe reads.
- New upstream USERSET GUCs (`citus.executor_batch_size`,
  `citus.executor_chunk_size`, `citus.enable_or_clause_arm_pruning`,
  `citus.allow_unsafe_insert_select_pushdown`) are covered by patch 0003's
  blanket `PGC_USERSET` → `GUC_REPORT` loop; `pool/README.md` now lists them
  as tracked-GUC candidates for `AI_BLAISE_POOL_SETTINGS_BUCKET_GUCS`.
- `UPSTREAM_REBASE_BASE` → `base=008b391a7`, `capturedAt=2026-08-26`.

## 2026-05-22 — CDC sidecar + operator + gh-ost state machine runtime wiring

- `sidecar/cdc/` — New tokio-postgres logical-replication consumer with an
  async-nats sink and an axum probe server. Subjects published under
  `citus.cdc.<schema>.<table>` with NATS headers carrying `tx_xid`, `lsn`,
  `op_type`, and source label. `cdc serve` boots the probe server, optionally
  connects NATS + replication targets from env, and shuts down on SIGTERM.
- `operator/` — New `kube-rs` `Controller` reconcilers for `CitusCluster`,
  `Migration`, `Tenant`, and `Hypertable` running concurrently under a single
  tokio runtime, with a dedicated probe thread for readiness. CR specs mirror
  the validated `*Spec` types and are translated into the authoritative
  validator before action.
- `operator/src/crds/migration/` — Promoted to a directory module exposing the
  `MigrationPhase` lifecycle enum and the gh-ost-style `state_machine` with
  per-phase evidence guards.
- `companion/src/migration.rs` — Replaced the stub `companion_internal.*`
  emitters with real `citus_admin.*` invocations covering every gh-ost phase
  (`shadow_table_create`, `install_write_triggers`, `backfill_run`,
  `row_diff_verify`, `shadow_table_publish`, plus the
  `migrate_start`/`migrate_complete` envelopes).

Unblocks PLAN §2.4 (sidecar runtime) and §11.7 (gh-ost cut-over executor).

Future bootstrap-v2 changes must document the modified subsystem here or in a
nearer `MODIFICATION.md`, include regression coverage, and preserve the
platform rule that `ai-blaise/platform` never imports Citus source code.
