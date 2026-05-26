# citus-pg-overlay

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

CloudNativePG operand image contract for Citus, the companion SQL fallback,
and the bundled extension policy. `FEATURE: Bundle1 is production-ready` for
the `full-bundle-required-minus-plrust` boundary: `bundle1-final-light`
and `bundle1-final-full` build images install every required Bundle1
extension (PGDG + Timescale binary packages plus source-built citus, pgsodium,
topn, pg_jsonschema, pg_graphql, and heavy pg_search + plv8) and the complete
initdb path runs `CREATE EXTENSION` for every required extension at first
container boot. The fast default `bundle1-contract` image stays a
manifest/init contract for cheap PR coverage and is not a production claim by
itself. The separate pg_cron cohabitation smoke remains TS19/TS20 production
evidence for the bounded clock-reservation path. plrust remains alpha-deferred
upstream (pg13-pg16 pgrx 0.11.0); plrust is now optional in the manifest and
the plrust PG17 upstream gap is tracked separately under FEATURE: EF6.

## Contract

- `extension-manifest.tsv` is the source of truth for required, optional, and
  hard-blocked extensions from the V2 plan.
- `bundle1-source-build.lock.tsv` pins the PG17 source-build subset and
  records which entries are light, heavy, local-shim, or deferred.
- `extensions/ai_blaise_citus-upgrade-manifest.tsv` is the bounded
  upgrade/rollback contract for the local companion SQL extension. CI fails
  closed when a new install or transition SQL file appears without an explicit
  rollback and version-skew statement.
- `shared-preload-libraries.conf` records the load-order-sensitive preload
  contract.
- `initdb.d/00-ai-blaise-extensions.sql` creates the required extension set in a
  deterministic order and intentionally fails when a required extension control
  file is absent.
- `extensions/ai_blaise_citus.control` and
  `extensions/ai_blaise_citus--0.1.0.sql` install the companion SQL fallback
  surface so `CREATE EXTENSION ai_blaise_citus` exposes feature-status and
  pgrx-compatible Timescale-on-Citus plan helpers, including the TS5
  time-range shard-pruner helper, before the compiled pgrx library is present.
- `Dockerfile` exposes a fast default `bundle1-contract` target plus explicit `bundle1-final-light` and `bundle1-final-full` PG17 source-build targets.
- `Dockerfile.pgcore-patches` builds PostgreSQL `REL_17_10` from source, applies
  `patches/postgres/series`, builds Citus against the patched `pg_config`, and
  installs the smoke-only `ai_blaise_pgc_probe` extension used by
  `ci/ai-blaise/postgres-core-patches-live-smoke.sh` for PGC1/PGC2 runtime
  evidence.
- `bin/pgsodium_getkey` is installed at pgsodium's default `pgsodium_getkey`
  path and fails closed unless `PGSODIUM_KEY` is set or
  `PGSODIUM_KEY_FILE` points at a readable 64-hex-character secret.
- `ci/ai-blaise/bundle1-contract-check.py` is the structured Bundle1
  contract check: it cross-validates the manifest, lockfile, Dockerfile labels,
  smoke coverage, tracked evidence, and docs so Bundle1 cannot be claimed by
  prose alone.
- `ci/ai-blaise/image-check.sh` validates the image contract in CI and invokes
  the structured Bundle1 contract check.
- Source-build images carry `ai-blaise.citus.source-git-sha`,
  `ai-blaise.citus.source-tree-state`, plus
  `ai-blaise.citus.bundle1.evidence-scope=full-bundle-required-minus-plrust`;
  and `ai-blaise.citus.bundle1.full-initdb-path=true` recording that the complete initdb path runs at first container boot.
