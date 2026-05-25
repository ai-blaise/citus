# Schema Drift Detection Runbook

This runbook covers the production-ready `FEATURE: M4` boundary: detecting
schema drift by comparing an expected column manifest with live
`information_schema.columns` output. It does not apply DDL or generate
migrations.

## Preconditions

- The target database is reachable with read access to `information_schema`.
- The expected manifest lists every column that must exist for the checked
  table.
- The operator understands that this check is read-only apart from a temporary
  table scoped to the current session.

## Procedure

1. Render the schema drift SQL:

   ```bash
   cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-schema-drift-sql-canonical
   ```

2. Execute the rendered SQL in the target database:

   ```bash
   psql "$DATABASE_URL" -qAt -F $'\t' -f schema-drift.sql
   ```

3. Treat any returned row as drift. The final column is one of
   `missing_column`, `type_mismatch`, `nullability_mismatch`, or
   `unexpected_column`.

4. Route remediation through the normal migration workflow. Do not apply
   generated DDL from this detector because it intentionally does not plan
   remediation.

5. Rerun the detector after remediation. A clean schema returns zero rows.

## Evidence

`REQUIRE_DOCKER=1 ci/ai-blaise/schema-drift-live-smoke.sh` is the release
evidence for this boundary. It proves all four drift kinds against a live
PostgreSQL catalog and then proves that the same detector returns zero rows
after the schema is corrected.

## Non-Claims

- No DDL remediation plan is generated.
- No operator apply loop is executed.
- No cross-database inventory fanout is performed.
- No migration is automatically created.
