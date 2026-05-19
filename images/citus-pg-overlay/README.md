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
- `ci/ai-blaise/image-check.sh` validates the image contract in CI.
