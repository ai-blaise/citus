# companion Modifications

## 2026-05-23 -- companion runtime depth A

`src/migration.rs` now includes a guarded `MigrationRuntime` state machine in
addition to SQL rendering. It walks migrations through expand, backfill,
zero-diff validation, explicit cutover approval, publish, completion, failure,
and rollback decisions instead of presenting the gh-ost command list as one
undifferentiated script.

`src/queue.rs` adds a companion-owned durable queue runtime for the queue
substrate: idempotent enqueue, `FOR UPDATE SKIP LOCKED` lease SQL, visibility
expiry, retry backoff, ack token validation, and dead-letter handling. This is
runtime evidence for the queue contract, not a claim that packaged pgmq/pgque
binaries are fully installed by the operand image.

`src/replication_conflict.rs` adds a fail-closed seven-class replication
conflict resolver with monotonic-clock validation, home-region/origin-priority
policies, deterministic last-writer behavior, rejection for apply errors and
unsafe unique conflicts, and durable audit SQL emission.

`src/runtime_depth_a.rs`, `src/bin/companion_runtime_depth_a.rs`, and
`ci/ai-blaise/companion-runtime-depth-a-smoke.sh` provide the canonical TSV
report and focused VM smoke for this companion runtime batch.

## 2026-05-22 — gh-ost executor SQL surface

`src/migration.rs` — `MigrationPlan::to_sql_plan` no longer emits the
placeholder `companion_internal.migrate_start` / `migrate_complete` bookends
around bare operation calls. The plan now renders the full gh-ost cut-over
sequence as `citus_admin.*` invocations that the operator state machine
executes phase-by-phase:

1. `citus_admin.migrate_start(name, table, lock_timeout_ms, batch_size)`
2. `citus_admin.shadow_table_create(name, table)`  (gh-ost DELETE_ONLY)
3. operation-specific calls (`migration_add_column`,
   `migration_drop_column`, `migration_rename_column`,
   `migration_online_type_change`) targeting the shadow copy
4. `citus_admin.install_write_triggers(name, table)`  (gh-ost WRITE_ONLY)
5. `citus_admin.backfill_run(name, table, batch_size)`  (gh-ost BACKFILL)
6. `citus_admin.row_diff_verify(name, table)`
7. `citus_admin.shadow_table_publish(name, table)`  (gh-ost PUBLIC cut-over)
8. `citus_admin.migrate_complete(name)`

Regression coverage: the existing `migration_renders_expand_contract_sequence`
test still asserts the `migrate_start` and `migration_online_type_change`
substrings; the new `gh_ost_phases_are_present_in_sql_plan` test asserts every
phase invocation is present.
