# CDC output-plugin admission during PostgreSQL upgrades

This is an operator-controlled deployment prerequisite, not permission for an
application, extension installer, or reconciliation loop to weaken a production
server's decoder policy automatically.

PostgreSQL 17.11, 18.6 and 16.15 introduced `output_plugin_libraries` as part of
CVE-2026-6471 remediation. Its default admits `pgoutput` and `test_decoding`,
not arbitrary shared libraries. Citus operations that create internal logical
replication slots need the `citus` decoder. A deployment that deliberately uses
wal2json also needs `wal2json`; do not authorize it merely because this test
suite uses it. Keep the server security restriction enabled.

Primary sources:

- [PostgreSQL 17.11 release notes](https://www.postgresql.org/docs/17/release-17-11.html)
- [PostgreSQL 18.6 release notes](https://www.postgresql.org/docs/release/18.6/)
- [PostgreSQL security notice](https://www.postgresql.org/support/security/)

## Deployment and upgrade procedure

1. On each coordinator and worker that may decode changes, inventory the exact
   server build, installed trusted decoder libraries, current policy, and slots:

   ```sql
   SELECT version();
   SELECT current_setting('output_plugin_libraries', true);
   SELECT slot_name, plugin, slot_type, active FROM pg_replication_slots;
   ```

   A null setting indicates an older minor without this security control; it is
   not proof that a newer server will accept that older server's decoders.

2. The operator reviews and explicitly approves each required decoder and its
   installed artifact provenance. Preserve existing approved entries. For a
   server starting with the documented default and needing only Citus, the
   resulting configuration is:

   ```conf
   output_plugin_libraries = 'pgoutput, test_decoding, citus'
   ```

   Add `wal2json` only where its usage has independently been approved. Do not
   use a wildcard, arbitrary library path, automatic enumeration of every
   installed library, or a blanket restoration of pre-update behavior.

3. Apply through the deployment's normal reviewed configuration mechanism,
   reload, and read back the effective setting on every affected node. Check
   `pg_file_settings` for errors. Test actual slot creation and the intended
   Citus concurrent-distribution or shard-movement operation in an isolated
   preproduction fixture before enabling production traffic.
4. Before `pg_upgrade --check`, configure the new cluster to admit the approved
   plugins required by old slots. PostgreSQL checks this when migrating slots
   from PG17 and later. Re-run upgrade validation and retain the effective
   policy alongside both old/new server and extension artifact identities.
5. Keep the old data/artifact rollback evidence. Do not blindly copy a new GUC
   into a server version that lacks it, remove a plugin while active slots still
   require it, or interpret a successful reload as an end-to-end CDC proof.

## Test boundary and retained failure

The CDC TAP helper detects this GUC in its own disposable nodes, preserves the
current policy, explicitly appends `citus` and `wal2json`, reloads, and waits for
the exact resulting setting before any decoder operation. Older supported
minors keep their existing behavior. This helper is not production automation.

On source `35c39b7d2be0438119d020dc6000c6da8bc9de6a`, PG16.14 passed all
16 CDC files / 43 assertions. PG17.11 and PG18.6 each failed eight files because
the required decoder was not admitted, before reaching the repaired ownership
path. The original failed runs must remain attached to their original source;
the compatibility change requires a fresh complete TAP matrix.
