# Bundled Extensions

The canonical operand-image extension contract lives in
`images/citus-pg-overlay/extension-manifest.tsv`. This is still not
production-ready as a whole: the fast default image remains a manifest/init
contract, while the explicit PG17 source-build targets now provide live build
and `CREATE EXTENSION` evidence for the feasible PGDG-missing extensions.
`FEATURE: Bundle1` remains alpha until the full required bundle, including the
plrust PG17 upstream gap and complete initdb path, is verified end to end.


## PG17 Source-Build Path

The Dockerfile has two explicit PG17 source-build targets in addition to the
fast default PG17/PG18 contract target:

| Target | Intended use | Source-built/smoked extensions |
| --- | --- | --- |
| `bundle1-final-light` | PR-time or VM targeted proof for the feasible light subset | `citus`, `pgsodium`, `topn`, `pg_jsonschema`, `pg_graphql`, local `pg_warm`, plus `ai_blaise_citus` |
| `bundle1-final-full` | release-boundary proof for heavy feasible extensions | everything in light plus `pg_search` and `plv8` |

The upstream pins are labels and build args in
`images/citus-pg-overlay/Dockerfile`; each clone checks the tag's resolved
commit before building.

| Extension | Upstream | Tag | Commit | Status |
| --- | --- | --- | --- | --- |
| `citus` | this fork, tracking citusdata/citus | `v13.3.0` | in-tree source | feasible, light target |
| `pgsodium` | michelp/pgsodium | `v3.1.9` | `7222ebc5ed87084a68d526aef977be0f4eb319a2` | feasible, light target |
| `topn` | citusdata/postgresql-topn | `v2.7.0` | `f636ff1b3586025c81fb84c20483412f3991ed84` | feasible, light target |
| `pg_jsonschema` | supabase/pg_jsonschema | `v0.3.4` | `cbe74b570d38aa0c4d42914e7a118bcb3adaee7a` | feasible, light target |
| `pg_graphql` | supabase/pg_graphql | `v1.6.1` | `66d4c551db213000506fd858676269ba8f801a44` | feasible, light target |
| `pg_search` | paradedb/paradedb | `v0.20.11` | `cd1ba46a116c5a98bd6fe9ae370a2f260aee1394` | feasible, heavy target |
| `plv8` | plv8/plv8 | `v3.2.4` | `cafc37f7aee850de5478773a4e56f7fadfad8e00` | feasible, heavy target |
| `pg_warm` | local shim over core `pg_prewarm` | `0.1.0` | in-tree SQL | feasible, light target |
| `plrust` | pgcentralfoundation/plrust | `v1.2.8` | `bd76906a43c05a2afdb7839263431a066f5b42fb` | alpha boundary: upstream exposes only pg13-pg16 pgrx features and pins pgrx 0.11.0 |

Run the targeted smoke with:

```bash
BUNDLE1_BUILD_IMAGE=1 BUNDLE1_EVIDENCE_FILE=images/citus-pg-overlay/bundle1-source-build-evidence.tsv SQL_EXTENSION_SMOKE_PG_MAJORS=17 REQUIRE_DOCKER=1 bash ci/ai-blaise/sql-extension-smoke.sh
```

Set `BUNDLE1_BUILD_HEAVY=1` only at a large release/integration boundary; it
adds the `pg_search` and `plv8` builds and is intentionally not part of every
iteration cycle.

`pgsodium` is preloaded with a fail-closed getkey contract. The Bundle1 image
installs `pgsodium_getkey` at pgsodium's default extension path; production
deployments must set `PGSODIUM_KEY` or mount a 64-hex-character secret and set
`PGSODIUM_KEY_FILE`. The smoke test provides an explicit deterministic test key
only for the disposable test container.

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
