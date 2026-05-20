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
6. The pool `serve` mode exposed HTTP probes on the PostgreSQL service port,
   so live traffic tests proved readiness only, not PostgreSQL wire behavior.

## Corrections

- `images/rust-runtime/Dockerfile` and
  `scripts/citus-scale/build-app-images.sh` now build the real Rust app image
  matrix for the operator, pool, all sidecars, and `citusctl`.
- Every deployed Rust service now accepts `serve`; the operator and sidecars
  run the shared health/readiness/metrics endpoint, while the pool runs its
  PostgreSQL data proxy plus a separate admin endpoint.
- The Helm chart sets `serve`, `AI_BLAISE_LISTEN_ADDR`, readiness probes,
  liveness probes, non-root pod security, dropped capabilities, and read-only
  root filesystems for the Rust workloads.
- The SQL extension now defines the Timescale bridge state table, a public
  bridge-state view, executable internal routines, and `apply_*` SQL functions
  for the Timescale/Citus bridge.
- The pool now runs a byte-transparent PostgreSQL TCP proxy on the service
  port and a separate admin server for probes and metrics. Readiness checks the
  configured upstream before Kubernetes can route clients to the pod.
- The Kubernetes production smoke now port-forwards into the live operator and
  every sidecar deployment and verifies `/healthz`, `/readyz`, and `/metrics`
  from the real pods before it runs the pool SQL traffic job.
- After the SQL smoke, the same Kubernetes smoke port-forwards every pool pod
  and aggregates `ai_blaise_citus_pool_requests_total` across replicas, avoiding
  a false failure when the admin service selects a pool pod that did not handle
  the SQL connection.
- CI checks now assert the real image matrix, `serve` support, Helm probe
  contracts, live sidecar probe coverage, pool data/admin port separation,
  pool live-SQL smoke coverage, and SQL bridge-state smoke coverage.

## Verification Standard

Rule 10 completion for this branch requires local and VM verification of:

- Rust formatting and compile/test gates for all changed packages.
- SQL extension smoke against a real Postgres container.
- Helm render and Kubernetes rollout with the real app images.
- Live operator and sidecar `/healthz`, `/readyz`, and `/metrics` responses
  through Kubernetes port-forwarding.
- Live PostgreSQL traffic through the pool service data port, plus `/readyz`
  and `/metrics` verification on the pool admin port and per-pod pool metrics.

## Whole-Repo Production Readiness Audit

The deployment corrections above close the most dangerous false-positive path:
the chart now proves real Rust app images, real pods, sidecar probes, and live
SQL through the pool. The broader repository is still not production-ready as a
whole.

The current feature inventory contains 240 source `FEATURE:` markers and 161
feature headings in `docs/ai-blaise/NEW_FEATURES.md`. Every feature heading is
still `Status: alpha`. The remaining 79 source markers are represented as V2
completion references or addendum rows rather than standalone feature headings.
This is acceptable for catalog integrity, but it is not a production claim.
The audit guard also reports 77 feature headings without an explicit
Executable, CI, Acceptance, SQL runtime, or SQL extension reference line; those
entries may still have source markers, but they are not independently
evidenced enough for production signoff.

The audit found three classes of non-closure that must remain visible until
they are replaced by measured evidence:

1. Contract-only surfaces. Many entries validate deterministic Rust contracts,
   SQL plans, CRD schemas, image manifests, or runbook requirements. Those are
   useful acceptance artifacts, but they do not prove live end-to-end behavior
   for every advertised feature.
2. Modeled release gates. `e2e/src/release_gates.rs` records the V2 gate shape
   and expected thresholds, but several values are canonical model data rather
   than results from live performance, chaos, multi-region, vectorizer, search,
   and HTAP harnesses.
3. Alpha feature register. The register intentionally keeps all feature
   headings alpha until each feature has production evidence for its runtime
   behavior, rollback behavior, security posture, and operational ownership.

`ci/ai-blaise/production-readiness-check.sh` now enforces this boundary. In
normal audit mode it verifies source/doc synchronization, status semantics, and
that this audit explicitly blocks overclaiming. In `production-release` mode it
fails while any feature heading or source-only V2 addendum feature remains
non-production, so release promotion cannot treat alpha contracts as
production-ready functionality.
