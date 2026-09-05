# Bundled Extensions

The canonical operand-image extension contract lives in
`images/citus-pg-overlay/extension-manifest.tsv`.
`FEATURE: Bundle1 remains alpha` for its declared required manifest. The new
`bundle1-pgdg-runtime` Dockerfile stage installs every PGDG and Timescale
binary-package required extension on top of `postgres:17-bookworm`;
`bundle1-final-light` and `bundle1-final-full` layer in the source-built
extensions. The light target is partial B1/PR evidence and excludes the
lockfile's `full` rows; only the full target may claim
`full-bundle-required-minus-plrust` and release eligibility. There is no
current release-qualified full-target default-boot receipt from a reviewed
clean commit, so release/publishing stays blocked.
The fast default `bundle1-contract` image stays a manifest/init contract for
cheap PR coverage and does not carry production claims by itself.
The plrust PG17 upstream gap is unchanged (upstream main still pg13-pg16 with
pgrx 0.11.0); plrust has been moved from `required` to `optional` in the
manifest and is tracked separately under `FEATURE: EF6`. The pg_cron
cohabitation smoke remains production evidence for TS19's bounded clock-
reservation path beside this Citus fork.


## PG17 Source-Build Path

The Dockerfile has two explicit PG17 source-build targets in addition to the
fast default PG17/PG18 contract target:

| Target | Intended use | Source-built/smoked extensions |
| --- | --- | --- |
| `bundle1-final-light` | PR-time or VM targeted proof for the feasible light subset | `citus`, `pgsodium`, `topn`, `pg_jsonschema`, `pg_graphql`, local `pg_warm`, plus `ai_blaise_citus` |
| `bundle1-final-full` | release-boundary proof for heavy feasible extensions | everything in light plus `pg_search` and `plv8` |

The upstream pins are recorded in
`images/citus-pg-overlay/bundle1-source-build.lock.tsv` and mirrored as labels
and build args in `images/citus-pg-overlay/Dockerfile`; each external clone
checks the tag's resolved commit before building. The structured Bundle1
contract check (`ci/ai-blaise/bundle1-contract-check.py`) fails closed if the
manifest, lockfile, Dockerfile, smoke, evidence, or docs drift apart.

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
| `plrust` | pgcentralfoundation/plrust | `v1.2.8` | `bd76906a43c05a2afdb7839263431a066f5b42fb` | alpha-deferred upstream pg17 blocker; plrust is now optional in the manifest and tracked separately under FEATURE: EF6 |

The Citus `v13.3.0` value is historical tracking metadata, not the runtime
extension version and not Chimera's separately selected latest-upstream pin.
The image compiles the in-tree fork, binds it to the source Git SHA/tree-state
labels, and the default-boot smoke requires its observed `pg_extension`
version to equal `src/backend/distributed/citus.control` (currently `15.0-1`).

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
`PGSODIUM_KEY_FILE`. The smoke test first proves `pgsodium_getkey` fails closed
without a key, then provides an explicit deterministic test key only for the
disposable test container. Source-build images carry the
`ai-blaise.citus.source-git-sha` and `ai-blaise.citus.source-tree-state`
labels plus target-specific scope and release-target labels. Light carries
`light-required-subset-minus-heavy-and-plrust` and `release-target=false`;
only full carries `full-bundle-required-minus-plrust` and
`release-target=true`.

Both source-build targets copy the canonical preload file to
`/etc/postgresql/ai-blaise/shared-preload-libraries.conf` and append a
fail-closed `include` to the `postgresql.conf.sample` used by initdb. The
`ci-image.yml` PG17 leg explicitly builds `bundle1-final-light` and invokes
`ci/ai-blaise/bundle1-default-boot-smoke.sh` without a postgres command or
`-c` override. Light expectations are derived from required manifest rows
minus source-build lock rows marked `full`. A separate push-time full build
runs the same smoke against every required manifest entry. Both modes check
the applied `pg_file_settings.sourcefile`, preload GUCs, init completion,
readiness, installed Citus control version and `ai_blaise_citus` version
`0.1.2`, target/scope/release labels, and expected source Git SHA/tree-state.
The workflow does not publish an image.

## pg_cron Cohabitation Subset

`ci/ai-blaise/pg-cron-cohabitation-smoke.sh` builds
`images/citus-pg-cron-cohabitation/Dockerfile`, installs the PGDG
`postgresql-17-cron` package plus this Citus fork and `ai_blaise_citus`, and
boots PostgreSQL with `shared_preload_libraries=pg_cron,citus` and
`citus.cohabit_extensions=pg_cron`. It creates real `citus`, `pg_cron`, and
`ai_blaise_citus` extensions, verifies `citus_cohabit_clock_tick_reserved()`,
verifies SQL-visible Citus classifier/configuration UDFs for `pg_cron`,
`timescaledb`, `pg_partman`, and unknown names, waits for a scheduled pg_cron
worker to write clock-reserved evidence rows, records
`artifacts/pg-cron-cohabitation-evidence.tsv`, and proves both the Citus UDF and
SQL cohabit detectors fail closed when the `pg_cron` allowlist entry is omitted.

This pg_cron cohabitation smoke is bounded to the TS19/TS20 clock-reservation
path: it does not promote Bundle1 as a whole, does not cover plrust, does not
make `pg_cron` a trusted hook-chain coextension, and does not prove live
TimescaleDB or pg_partman extension execution beyond extension creation.

## Required Bundle

The required bundle records the intended extension set for ai-blaise/citus
Postgres operand images. It covers the V2 plan's mandatory Citus, TimescaleDB,
vector, search, graph, JSON Schema, observability, security, geo, and
online-maintenance substrates.

Required entries are statically validated by `ci/ai-blaise/image-check.sh`.
Cluster initialization uses
`images/citus-pg-overlay/initdb.d/00-ai-blaise-extensions.sql` as the
deterministic extension creation order. The two lockfile `full` entries are
conditional so light can boot; the full default-boot smoke fails closed unless
all required manifest entries are present.

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
| 16       | SQL smoke covered     | The companion SQL extension smoke runs against stock PG16 on every PR; Bundle1 binary/source manifest rows remain PG17-centered unless their package/build evidence names PG16. |
| 17       | default               | All required and optional manifest entries target PG17; `pg_versions` column lists `17` for every active row.                     |
| 18       | alpha (T6 forward)    | Core extensions and PGDG-stable required extensions (citus, timescaledb, postgis, pgvector, pg_cron, pg_partman, pgaudit, pgauditlogtofile, pg_uuidv7, pg_repack, pg_hint_plan, hypopg, pg_qualstats, pg_stat_kcache, pg_wait_sampling, pg_walinspect) are flagged `17,18`. Source-built and lagging binary-packaged extensions stay `17` until their PG18 build path is verified. |

`ci/ai-blaise/sql-extension-smoke.sh` runs the companion SQL extension against
the PG16, PG17, and PG18 base images on every PR. PG18 additionally exercises
the new `io_method` GUC (default `worker`, override with
`SQL_EXTENSION_SMOKE_IO_METHOD=io_uring` on kernels with io_uring enabled) to
confirm Citus and the bundled SQL surface come up cleanly under PG18.

Extensions whose PG18 build is still pending verification carry a
`PG18 build pending` note in the manifest's `policy` column. Flipping a row
to `pg_versions=17,18` is gated by a per-extension PG18 build proof (PGDG
package or in-tree source-build evidence) recorded in a follow-up PR.
