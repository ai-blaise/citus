# citus-pg-overlay

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

CloudNativePG operand image contract for Citus, the companion SQL fallback, and
the bundled extension policy. The fast default image is still not production
evidence that every binary package in the manifest is installed in a runnable
operand image; `FEATURE: Bundle1` remains alpha until the full required bundle,
including plrust and the complete initdb path, is verified end to end. Explicit
PG17 source-build targets now provide targeted live evidence for the feasible
PGDG-missing extensions. The separate pg_cron cohabitation smoke is TS19
production evidence for the bounded clock-reservation path beside this Citus
fork, including SQL-visible reservation and scheduled worker execution; it is
not full Bundle1 production evidence.

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
  `ai-blaise.citus.bundle1.evidence-scope=source-build-subset-no-complete-initdb`;
  the label is intentionally not a full initdb production claim.
