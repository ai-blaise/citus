# citus-pg-overlay

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

CloudNativePG operand image contract for Citus, the companion SQL fallback, and
the bundled extension policy. This directory is not production evidence that
every binary package in the manifest is installed in a runnable operand image;
`FEATURE: Bundle1` remains alpha until a real image build smoke verifies the
required extension control files and initdb extension creation end to end.

## Contract

- `extension-manifest.tsv` is the source of truth for required, optional, and
  hard-blocked extensions from the V2 plan.
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
- `ci/ai-blaise/image-check.sh` validates the image contract in CI.
