# Production Readiness Corrective Audit

This audit records the gaps found after the initial V2 merge was treated as
more production-ready than the artifacts justified.

## Findings

1. The Helm chart referenced operator, pool, sidecar, and tool images, but the
   repository only contained the Postgres operand Dockerfile. Production
   deployment tests could therefore substitute fake responder images without
   exercising the real Rust binaries.
2. Deployed Rust binaries defaulted to one-shot canonical TSV reports. Real
   Kubernetes pods would exit after start rather than serving readiness,
   liveness, or metrics traffic.
3. The chart had no runtime command, probe, or container hardening contract for
   the Rust workloads.
4. The TimescaleDB bridge exposed useful SQL plan text, but the SQL it emitted
   referenced `companion_internal` functions that were not defined by the
   extension.
5. The SQL smoke test verified plan rendering, but not durable bridge state or
   executable internal routines.

## Corrections

- `images/rust-runtime/Dockerfile` and
  `scripts/citus-scale/build-app-images.sh` now build the real Rust app image
  matrix for the operator, pool, all sidecars, and `citusctl`.
- Every deployed Rust service now accepts `serve` and runs the shared TCP
  health/readiness/metrics endpoint.
- The Helm chart sets `serve`, `AI_BLAISE_LISTEN_ADDR`, readiness probes,
  liveness probes, non-root pod security, dropped capabilities, and read-only
  root filesystems for the Rust workloads.
- The SQL extension now defines the Timescale bridge state table, a public
  bridge-state view, executable internal routines, and `apply_*` SQL functions
  for the Timescale/Citus bridge.
- CI checks now assert the real image matrix, `serve` support, Helm probe
  contracts, and SQL bridge-state smoke coverage.

## Verification Standard

Rule 10 completion for this branch requires local and VM verification of:

- Rust formatting and compile/test gates for all changed packages.
- SQL extension smoke against a real Postgres container.
- Helm render and Kubernetes rollout with the real app images.
- Live traffic against `/healthz`, `/readyz`, and `/metrics` on deployed
  services.
