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
7. The SQL extension smoke invoked `docker exec psql` without attaching stdin,
   so its heredoc could be skipped by Docker and the smoke could pass without
   installing or exercising `ai_blaise_citus`.
8. Operand-image docs described the required extension bundle as installed for
   every operand image even though `FEATURE: Bundle1` is still an alpha
   manifest/init contract without a real full-bundle image build smoke.
9. Production Helm values enabled alpha runtime/security intent fields for
   protocol pipelining, PG18 `io_uring`, External Secrets, TLS, release
   attestations, and pool CIDR allowlists even though the chart does not yet
   render or enforce the corresponding runtime/security objects.
10. The Kubernetes production smoke installed the exhaustive default profile
    with every app image enabled, but it did not install `values-prod.yaml`.
    Production-value claims were therefore guarded statically but not exercised
    through a live Helm rollout and pool traffic path.
11. The Argo application used `values-prod.yaml` but still tracked
    `ai-blaise/bootstrap-v2`; GitOps could therefore deploy an older branch
    after production fixes landed on the `main` release branch.
12. The operator ClusterRole granted wildcard ai-blaise resources and Secret
    access even though the current production operator path only serves
    probes/metrics and External Secrets integration remains alpha.
13. The Argo application enabled self-heal but disabled pruning, so stale alpha
    deployments from an earlier non-production profile could survive after
    switching GitOps to `values-prod.yaml`.
14. The Timescale bridge smoke treated a successful connection as readiness
    even though the TimescaleDB image can accept temporary init-time
    connections while its init scripts are still creating `timescaledb`,
    producing a duplicate extension-key race in CI.
15. Most custom CI workflows still ran on the obsolete
    `ai-blaise/bootstrap-v2` branch after `main` became the release branch,
    so stale branch pushes could receive production-readiness signals that no
    longer matched the GitOps release target.

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
- The SQL extension smoke now attaches stdin to `psql`, preloads and creates
  `pg_stat_statements`, verifies live percentile rows through
  `companion_pg_stat_statements_p95`, opens a real idle-in-transaction
  backend, and requires `companion_idle_transactions(...)` to detect it.
- The image workflow and `gate-close` now run every promoted SQL runtime
  smoke: the plain PostgreSQL extension smoke, the real TimescaleDB bridge
  smoke, and the primary/standby observability replication smoke. The
  production gap audit rejects regressions that leave a promoted runtime smoke
  out of those gates.
- The bundled-extension docs and operand-image README now explicitly state that
  `FEATURE: Bundle1` is a manifest/init contract, not production evidence that
  every binary package is installed in a runnable operand image. The production
  gap audit rejects the old operand-image overclaim until a real full-bundle
  image build smoke exists.
- Production values now keep alpha runtime/security intent controls disabled by
  default. The deploy check and production gap audit reject production values
  that enable protocol pipelining, PG18 `io_uring`, External Secrets, TLS,
  release attestations, or CIDR allowlists before those controls are rendered,
  enforced, and verified end to end.
- Operator RBAC now enumerates the ai-blaise CRD resources instead of using a
  wildcard grant, and it no longer grants Secret access while secret binding
  remains alpha. The deploy check and production gap audit reject wildcard CRD
  resources or Secret permissions in the operator role.
- The Kubernetes production smoke now runs two live Helm profiles in kind. The
  exhaustive image-matrix profile still proves every Rust app image can serve
  probes and pool SQL traffic, and a separate `values-prod.yaml` profile proves
  that production values install with operator/pool replicas, no alpha sidecar
  or tools deployments, monitoring CRDs present, and live SQL through the pool.
- The Argo application now uses `values-prod.yaml` so GitOps deployment matches
  the production profile, targets the `main` release branch, and the deploy
  workflow plus `gate-close` now invoke the live kind production smoke instead
  of leaving D13 as VM-only evidence.
- The Argo application now prunes stale rendered resources, self-heals drift,
  creates the target namespace, and prunes last so disabled alpha sidecars or
  tools cannot persist merely because they were created by a previous profile.
- The Timescale bridge smoke now waits for the TimescaleDB image init process
  to complete before it runs bridge SQL, preventing CI from racing the image's
  own `timescaledb` extension creation.
- Custom CI push triggers now target only `main` and `ai-blaise/dev`; the
  deploy check and production gap audit reject any `ci-*` workflow that still
  targets the stale `ai-blaise/bootstrap-v2` branch.
- The observability dashboard and alert templates now query
  `ai_blaise_sidecar_ready`, the metric emitted by the sidecar runtime.
- O2 and R4 production-ready wording now matches the implemented SQL runtime:
  O2 is local-node activity stats with a compatibility alias, and R4 is
  idle-transaction detection only, not cancellation or termination.

## Verification Standard

Rule 10 completion for this branch requires local and VM verification of:

- Rust formatting and compile/test gates for all changed packages.
- SQL extension smoke against a real Postgres container.
- SQL smoke commands must be fed into the Postgres container with stdin
  attached; static image checks reject the old false-positive pattern.
- Helm render and Kubernetes rollout with the real app images.
- Live operator and sidecar `/healthz`, `/readyz`, and `/metrics` responses
  through Kubernetes port-forwarding.
- Live PostgreSQL traffic through the pool service data port, plus `/readyz`
  and `/metrics` verification on the pool admin port and per-pod pool metrics.
- Live `values-prod.yaml` Helm rollout that keeps alpha workloads disabled
  while the production operator and pool deployments become available and serve
  SQL/admin traffic.
- Every production-promoted SQL runtime smoke must be part of the GitHub image
  workflow, `gate-close`, and static production gap audit guards.

## Whole-Repo Production Readiness Audit

The deployment corrections above close the most dangerous false-positive path:
the chart now proves real Rust app images, real pods, sidecar probes, and live
SQL through the pool. The broader repository is still not production-ready as a
whole.

The current feature inventory contains 240 source `FEATURE:` markers and 161
feature headings in `docs/ai-blaise/NEW_FEATURES.md`. Seven narrow headings are
`Status: production-ready` because they have live VM/GitHub evidence: `D13`
for the production runtime image matrix, `O4` for the shared sidecar
health/readiness/metrics runtime, `O1` for the installable
`pg_stat_statements` percentile view, `O2` for the installable local activity
stats view, `O3` for the installable replication-lag view against a real
streaming standby, `R4` for the installable idle transaction detection SQL
surface, and `TS18` for the installable Timescale bridge-state SQL surface
verified in both plain PostgreSQL and a real TimescaleDB container. The other
154 feature headings remain
`Status: alpha`. The remaining 79 source markers are represented as V2
completion references or addendum rows rather than standalone feature headings;
those rows also remain alpha. This is acceptable for catalog integrity, but it
is not a production claim for the full feature plan.
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
3. Alpha feature register. The register intentionally keeps feature headings
   alpha until each feature has production evidence for its runtime behavior,
   rollback behavior, security posture, and operational ownership.

`ci/ai-blaise/production-readiness-check.sh` now enforces this boundary. In
normal audit mode it verifies source/doc synchronization, status semantics, and
that this audit explicitly blocks overclaiming. In `production-release` mode it
fails while any feature heading or source-only V2 addendum feature remains
non-production, so release promotion cannot treat alpha contracts as
production-ready functionality.
`ci/ai-blaise/production-gap-audit.sh` enforces the same line from the other
direction: the V2 acceptance model must not be cited as production evidence,
`v2-acceptance-check.sh` must stay out of `production-release` mode, modeled
release-gate constants must remain documented as non-production evidence, and
the SQL/Kubernetes smoke guards must keep proving real stdin, live Postgres,
live TimescaleDB behavior, live primary/standby replication behavior, live pool
SQL traffic, and live pod probe traffic.

Production Helm values must also keep alpha sidecars disabled by default.
`values-prod.yaml` can carry replica/resource intent for those components, but
`ci/ai-blaise/deploy-check.sh` rejects production values that enable any alpha
sidecar before the corresponding feature is promoted with measured production
evidence.
