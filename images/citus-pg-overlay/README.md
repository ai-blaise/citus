# citus-pg-overlay

CloudNativePG operand image containing Citus, companion, and bundled extension
dependencies.

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
  pgrx-compatible Timescale-on-Citus plan helpers before the compiled pgrx
  library is present.
- `ci/ai-blaise/image-check.sh` validates the image contract in CI.
