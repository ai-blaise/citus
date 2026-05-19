# License Audit

This file tracks the dependency and fork targets called out by the V2 plan.
The release rule is simple: permissive or compatible copyleft components can
be integrated, source patches stay upstream-minimal, and restricted components
are consumed only through unmodified binaries or optional external services.

## Required Checks

| Component | License posture | Integration rule |
|---|---|---|
| Citus | AGPL-3.0 | Fork source directly in `ai-blaise/citus`. |
| TimescaleDB Apache parts | Apache-2.0 | Consume extension APIs and unmodified sources where license permits. |
| TimescaleDB TSL parts | Timescale License | Do not patch TSL source; consume unmodified binaries only where allowed. |
| pgcat | MIT | Fork or port pooler concepts into `pool/`. |
| pgrx | MIT / Apache-2.0 | Use for companion extension packaging. |
| kube-rs | MIT / Apache-2.0 | Use for operator implementation. |
| pg_repack | BSD-style | Bundle or call for online repack workflows. |
| pgvector | PostgreSQL License | Bundle for vector indexes. |
| pg_search | AGPL-3.0 | Bundle only under compatible AGPL distribution terms. |
| PostgREST | MIT | Run as sidecar, do not vendor Haskell runtime into core. |
| Deno | MIT | Use for edge function runtime sidecar. |
| Bun | MIT | Use as optional edge function runtime sidecar. |
| DataFusion / Arrow | Apache-2.0 | Use for analytical sidecar contracts. |
| Iceberg Rust | Apache-2.0 | Use for cold-tier and federation sidecars. |

## Guardrails

- Upstream Citus source changes stay in `patches/` until an upstreamable PR is
  prepared.
- TSL source is not patched in this repo.
- Optional sidecars may be disabled at deploy time.
- New bundled extension candidates must add a row above before code lands.
