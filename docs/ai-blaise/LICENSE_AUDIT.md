# License Audit

This file tracks the dependency and fork targets called out by the V2 plan.
The release rule is simple: permissive or compatible copyleft components can
be integrated, source patches stay upstream-minimal, and restricted components
are consumed only through unmodified binaries or optional external services.

Per-language transitive dependency tables live at the repo root and are
generated from each language's lockfile:

- [`ATTRIBUTIONS-Rust.md`](../../ATTRIBUTIONS-Rust.md) — every Rust crate in
  the Cargo workspace, grouped by license. Generated from
  `cargo metadata --format-version 1`.
- [`ATTRIBUTIONS-Go.md`](../../ATTRIBUTIONS-Go.md) — every Go module under
  `tools/citus-admin/` once the WhoDB fork lands. Generated from
  `go list -m -json all`.
- [`ATTRIBUTIONS-TypeScript.md`](../../ATTRIBUTIONS-TypeScript.md) — every
  npm package under `tools/citus-schema-designer/` and `tools/citus-admin/`
  once the DrawDB and WhoDB front-end forks land. Generated from
  `package.json` + the lockfile.

`ci/ai-blaise/license-check.sh` enforces the presence of those three files,
that they are linked from this audit, and that no Rust transitive dep
resolves to a GPL-2.0 or GPL-3.0 SPDX expression (AGPL / LGPL transitive
deps remain compatible).

## Required Checks

| Component | License posture | Integration rule |
|---|---|---|
| Citus | AGPL-3.0 | Fork source directly in `ai-blaise/citus`. |
| TimescaleDB Apache parts | Apache-2.0 | Consume extension APIs and unmodified sources where license permits. |
| TimescaleDB TSL parts | Timescale License | Do not patch TSL source; consume unmodified binaries only where allowed. |
| pgcat | MIT | Fork or port pooler concepts into `pool/`. |
| pgrx | MIT / Apache-2.0 | Use for companion extension packaging. |
| kube-rs | MIT / Apache-2.0 | Planned for alpha operator-controller implementation; the current production operator runtime has no kube-rs dependency. |
| pg_repack | BSD-style | Bundle or call for online repack workflows. |
| pgvector | PostgreSQL License | Bundle for vector indexes. |
| pg_cron | PostgreSQL License | Bundle for scheduled policy jobs. |
| pg_partman | PostgreSQL License | Bundle for partition management outside hypertable paths. |
| pgaudit | PostgreSQL License | Bundle for audit logging. |
| pgauditlogtofile | PostgreSQL License | Bundle as pgaudit file sink where available. |
| pgsodium | PostgreSQL License | Bundle for libsodium-backed crypto. |
| hll / topn / tdigest | Apache-2.0 | Bundle for merge-friendly approximations. |
| pgnodemx | Apache-2.0 | Bundle for OS and cgroup metrics. |
| PostGIS | GPL-2.0 | Bundle under compatible AGPL distribution terms. |
| pg_search | AGPL-3.0 | Bundle only under compatible AGPL distribution terms. |
| pg_graphql | Apache-2.0 | Bundle for GraphQL schema exposure. |
| pg_jsonschema | Apache-2.0 | Bundle for JSON Schema validation. |
| Apache AGE | Apache-2.0 | Bundle for graph query support. |
| plrust | PostgreSQL License | Bundle for Rust UDFs where supported. |
| plv8 | PostgreSQL License | Bundle for JavaScript UDFs where supported. |
| pg_uuidv7 | PostgreSQL License | Bundle for monotonic UUIDs. |
| pg_failover_slots | PostgreSQL License | Bundle for logical slot failover. |
| pg_warm | PostgreSQL License | Bundle for cache warming. |
| pgcrypto / pg_trgm / citext | PostgreSQL License | Use core contrib extensions. |
| rum | PostgreSQL License | Bundle for alternate full-text indexes. |
| PostgREST | MIT | Run as sidecar, do not vendor Haskell runtime into core. |
| Deno | MIT | Use for edge function runtime sidecar. |
| Bun | MIT | Use as optional edge function runtime sidecar. |
| DataFusion / Arrow | Apache-2.0 | Use for analytical sidecar contracts. |
| Iceberg Rust | Apache-2.0 | Use for cold-tier and federation sidecars. |
| hypopg / pg_qualstats | PostgreSQL License | Optional advisor extensions. |
| pg_stat_kcache / pg_wait_sampling / pgsentinel | PostgreSQL License | Optional observability extensions. |
| pgsql-http / pg_net | PostgreSQL License | Optional outbound HTTP extensions. |
| pgl_ddl_deploy | PostgreSQL License | Optional DDL replication extension. |
| pg_track_settings | PostgreSQL License | Optional configuration drift extension. |
| pg_lake | Apache-2.0 | Optional analytical substrate. |
| pg_duckdb | MIT | Optional analytical substrate. |
| pgactive | Apache-2.0 | Optional active-active reference-table replication. |
| oracle_fdw / mysql_fdw / mongo_fdw / tds_fdw | PostgreSQL-compatible licenses | Optional migration and federation FDWs. |
| pgmq / pgque | Apache-2.0 | Optional queue substrates. |
| pg_parquet | PostgreSQL License | Optional Parquet read/write extension. |
| pg_squeeze / pg_show_plans / pg_stat_monitor / pg_safeupdate | PostgreSQL License | Optional maintenance and observability extensions. |
| pg_walinspect | PostgreSQL License | Use core contrib WAL inspection extension. |
| anon | PostgreSQL License | Optional anonymization extension. |
| vchord | Apache-2.0 | Optional vector index extension. |
| pg_hint_plan / sr_plan | PostgreSQL License | Optional plan management extensions. |
| pgledger | MIT | Optional ledger substrate or vendored companion logic. |
| pglinter | Apache-2.0 | Optional schema linter substrate. |
| omnigres | Apache-2.0 | Reference integration target, not bundled by default. |

## Guardrails

- Upstream Citus source changes stay in `patches/` until an upstreamable PR is
  prepared.
- TSL source is not patched in this repo.
- Optional sidecars may be disabled at deploy time.
- New bundled extension candidates must add a row above before code lands.
