# companion Modifications

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
