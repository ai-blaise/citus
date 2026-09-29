# License Audit

This file tracks dependency and fork targets. Going forward, the September 5
**CHIMERA — FINAL PRODUCTION PLAN** selects Apache-only TimescaleDB, no
toolkit/TSL artifacts, and the capability replacements in its section 14.
The attachment SHA-256 is
`c38eeae185222a0121e844eb43da362e7f33ff4a50bf5c9a7a6b881ff8e4c31e`;
the durable plan and adoption record are in the `ai-blaise/chimera` repository
under `docs/porting/`. This supersedes the earlier unresolved deployment choice.
Existing fork images and the dependency snapshot below have not thereby been
converted or qualified for that plan.

A row is a review boundary, not distribution approval. Keeping a binary
unmodified or placing it behind a service does not by itself satisfy its
license. The exact source revision, linkage, notices, source obligations, and
intended deployment must be reviewed for the release candidate.

Per-language records live at the repo root:

- [`ATTRIBUTIONS-Rust.md`](../../ATTRIBUTIONS-Rust.md) — a dated resolved-graph
  snapshot from `cargo metadata --locked --format-version 1`, with its lockfile
  checksum and package-declared license expressions.
- [`ATTRIBUTIONS-Go.md`](../../ATTRIBUTIONS-Go.md) — records that no Go module
  is imported under `tools/citus-admin/`; anticipated packages are not listed
  as dependencies.
- [`ATTRIBUTIONS-TypeScript.md`](../../ATTRIBUTIONS-TypeScript.md) — records
  that no npm project is imported under `tools/citus-schema-designer/` or
  `tools/citus-admin/`.

`ci/ai-blaise/license-check.sh` enforces these records' presence and links,
validates one complete locked Cargo metadata document, and applies the existing
Rust dependency policy. It does not regenerate the tables, scan Go or npm,
package license texts, or establish legal compatibility. A passing scan does
not complete B4.

## Required Checks

| Component | License posture | Integration rule |
|---|---|---|
| Citus | AGPL-3.0 | Fork source directly in `ai-blaise/citus`. |
| TimescaleDB Apache parts | Apache-2.0 | Selected Chimera posture: build the Apache-only artifact and implement the production plan's explicit capability replacements. Verify actual image contents and notices before qualification. |
| TimescaleDB TSL parts / toolkit | Excluded by the production plan | Do not include these artifacts in the qualifying Chimera operand. Historical Community fixtures do not establish Apache-only compliance. |
| pgcat | MIT | Vendor the selected pool data plane into Chimera with the production plan's integrations and reviewed provenance. |
| pgrx | MIT / Apache-2.0 | Use the pinned raw-only production substrate and parity-tested native inlines. Guarded bindings are measurement comparators, not a production tier. Preserve upstream SQL/ABI and add integration SQL through `chimera` on `citus.so`. |
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
| PostGIS | GPL-2.0-or-later | Preserve the exact imported source's notices and review the combined distribution. The [upstream source header](https://github.com/postgis/postgis/blob/d4a0809234543b13ec401d43ffd57290b6cc208a/postgis/lwgeom_functions_basic.c) expressly permits later GPL versions. |
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
| WhoDB reference | Apache-2.0 at reviewed revision | Not imported here. The [license at `95dcf00f2d237296b1758b73d88c0b68459f81de`](https://github.com/clidey/whodb/blob/95dcf00f2d237296b1758b73d88c0b68459f81de/LICENSE) was checked on 2026-09-05; review the exact candidate and retain notices before any import. |
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

- The Timescale deployment choice is Apache-only. The section 14 retirements
  and replacements are requirements, not optional commercial/self-hosted
  alternatives. Build-content, SQL-surface, upgrade, and distribution checks
  remain outstanding; the plan selection is not their receipt.
- Runtime-image LICENSE/NOTICE packaging and the applicable corresponding-source
  offer remain outstanding candidate-specific work. Attribution tables are not
  substitutes for those artifacts.
- Upstream Citus source changes stay in `patches/` until an upstreamable PR is
  prepared.
- Qualifying Chimera artifacts must exclude toolkit/TSL code and binaries;
  not patching TSL source is insufficient to establish this property.
- Optional sidecars may be disabled at deploy time.
- New bundled extension candidates must add a row above before code lands.
