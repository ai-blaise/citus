# citus-pg-overlay

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

CloudNativePG operand image contract for Citus, the companion SQL fallback,
and the bundled extension policy. `FEATURE: Bundle1 remains alpha` because its
declared required manifest has no current release-qualified full-target proof
from a reviewed clean commit. Historical light and dirty-context full receipts
remain bounded observations only. `bundle1-final-light` is the
bounded B1/PR target and deliberately excludes the lockfile's `full` entries;
it carries `light-required-subset-minus-heavy-and-plrust` and is never a
release target. `bundle1-final-full` adds pg_search and plv8, is the only image
allowed to carry `full-bundle-required-minus-plrust`, and must pass the stock-
entrypoint default-boot smoke before release or publishing. There is no current
full-target default-boot receipt, and publishing is not implemented by this
workflow. The fast default `bundle1-contract` image stays a manifest/init
contract for cheap PR coverage and is not production evidence by itself.
plrust remains alpha-deferred upstream (pg13-pg16 pgrx 0.11.0), optional in the
manifest, and tracked separately under FEATURE: EF6.

## Contract

- `extension-manifest.tsv` is the source of truth for required, optional, and
  hard-blocked extensions from the V2 plan.
- `bundle1-source-build.lock.tsv` pins the PG17 source-build subset and
  records which entries are light, heavy, local-shim, or deferred.
  Its Citus `v13.3.0` value is historical tracking metadata only: the image
  compiles the in-tree fork, and the smoke compares the runtime Citus
  extversion to `src/backend/distributed/citus.control` while binding the exact
  source Git SHA/tree-state. It is not Chimera's latest-upstream selection.
- `extensions/ai_blaise_citus-upgrade-manifest.tsv` is the bounded
  upgrade/rollback contract for the local companion SQL extension. CI fails
  closed when a new install or transition SQL file appears without an explicit
  rollback and version-skew statement.
- `shared-preload-libraries.conf` records the load-order-sensitive preload
  contract. The Dockerfile adds a fail-closed include to PostgreSQL's sample
  configuration before initdb so the stock entrypoint's temporary server and
  final server both read the same file.
- `initdb.d/00-ai-blaise-extensions.sql` creates the light extension set in a
  deterministic order and conditionally creates the two lockfile `full`
  entries when their control files are present. The full-target smoke derives
  its expected set from the required manifest rows and fails if any is absent.
- `extensions/ai_blaise_citus.control`, the `0.1.0` base SQL, and the
  `0.1.0--0.1.1` plus `0.1.1--0.1.2` forward transitions install the companion
  SQL fallback at the shipped `0.1.2` default. The `0.1.2` security floor is
  forward-only: the manifested `0.1.1--0.1.0` reverse transition applies only
  to that historical edge. The transactional
  `upgrades/ai_blaise_citus--0.1.2.sql` admin wrapper is packaged separately;
  recovery from 0.1.2 uses the documented backup/PITR/redeploy path, not a
  privilege-reopening reverse migration. Bare
  `CREATE EXTENSION ai_blaise_citus` therefore exposes the current
  feature-status and pgrx-compatible Timescale-on-Citus plan helpers,
  including the TS5 time-range shard-pruner helper, before the compiled pgrx
  library is present.
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
- `ci/ai-blaise/bundle1-default-boot-smoke.sh` boots the explicit
  target with no PostgreSQL command override and proves the canonical preload
  file is the applied setting source. Light expectations are derived from the
  required manifest rows minus lockfile `full` rows; full expectations are all
  required manifest rows. It also binds target/scope/release labels, source Git
  SHA/tree-state labels, and installed companion version `0.1.2`.
- Source-build images carry `ai-blaise.citus.source-git-sha`,
  `ai-blaise.citus.source-tree-state`, and a target-specific evidence scope.
  Only `bundle1-final-full` carries
  `ai-blaise.citus.bundle1.evidence-scope=full-bundle-required-minus-plrust`
  and `ai-blaise.citus.bundle1.release-target=true`.
