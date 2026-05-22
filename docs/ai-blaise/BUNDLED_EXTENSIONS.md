# Bundled Extensions

The canonical operand-image extension contract lives in
`images/citus-pg-overlay/extension-manifest.tsv`. This is a manifest/init
contract, not production evidence that every binary package is installed in a
runnable operand image. `FEATURE: Bundle1` remains alpha until a real operand
image build smoke verifies the required extension control files and initdb
extension creation end to end.

## Required Bundle

The required bundle records the intended extension set for ai-blaise/citus
Postgres operand images. It covers the V2 plan's mandatory Citus, TimescaleDB,
vector, search, graph, JSON Schema, observability, security, geo, and
online-maintenance substrates.

Required entries are statically validated by `ci/ai-blaise/image-check.sh`.
Cluster initialization uses
`images/citus-pg-overlay/initdb.d/00-ai-blaise-extensions.sql` as the
deterministic extension creation order and intentionally fails when a required
extension control file is absent.

The overlay also installs `ai_blaise_citus`, a local SQL fallback companion
extension. It exposes `companion_feature_status()` plus pgrx-compatible
Timescale-on-Citus plan helpers, including distributed hypertable and
time-range shard-pruner plans, in the operand image even before the compiled
pgrx companion library is loaded, so smoke tests and operators have a stable
extension name to target.

## Optional Bundle

Optional entries are chart- or image-build flags. They are kept in the same
manifest so licensing, packaging, and hard-block rules remain reviewable in one
place.

## Hard Blocks

Hard-block entries are not bundled because they replace heap access methods,
install conflicting planner or transaction hooks, or compete with Citus shard
management. Adding a blocked extension to the required or optional bundle must
first change the manifest and explain the conflict resolution in an ADR.

## PostgreSQL Version Matrix

`images/citus-pg-overlay/extension-manifest.tsv` carries a `pg_versions` column
recording which PostgreSQL major releases each entry is currently expected to
build against. The overlay image is build-arg parameterized
(`--build-arg PG_MAJOR=17` or `18`) so the same manifest emits a PG17 and a
PG18 operand image; the Helm value `postgres.postgresMajor` selects which one
the operator deploys.

| PG major | Status                | Coverage                                                                                                                          |
| -------- | --------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| 17       | default               | All required and optional manifest entries target PG17; `pg_versions` column lists `17` for every active row.                     |
| 18       | alpha (T6 forward)    | Core extensions and PGDG-stable required extensions (citus, timescaledb, postgis, pgvector, pg_cron, pg_partman, pgaudit, pgauditlogtofile, pg_uuidv7, pg_repack, pg_hint_plan, hypopg, pg_qualstats, pg_stat_kcache, pg_wait_sampling, pg_walinspect) are flagged `17,18`. Source-built and lagging binary-packaged extensions stay `17` until their PG18 build path is verified. |
| 16       | suppressed (flake)    | Re-enable once the `background_rebalance_parallel_reference_tables` upstream flake fix lands.                                     |

`ci/ai-blaise/sql-extension-smoke.sh` runs the companion SQL extension against
the PG17 and PG18 base images on every PR. PG18 additionally exercises the
new `io_method` GUC (default `worker`, override with
`SQL_EXTENSION_SMOKE_IO_METHOD=io_uring` on kernels with io_uring enabled) to
confirm Citus and the bundled SQL surface come up cleanly under PG18.

Extensions whose PG18 build is still pending verification carry a
`PG18 build pending` note in the manifest's `policy` column. Flipping a row
to `pg_versions=17,18` is gated by a per-extension PG18 build proof (PGDG
package or in-tree source-build evidence) recorded in a follow-up PR.
