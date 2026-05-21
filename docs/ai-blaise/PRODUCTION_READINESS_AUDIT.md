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
16. Broader image, architecture, and upgrade docs still implied the Postgres
    operand image was built or packaged as a release artifact even though the
    detailed Bundle1 docs correctly kept it alpha pending a real operand image
    build/initdb smoke.
17. The GitOps application and Kubernetes smoke used `values-prod.yaml`, but
    the human deploy wrapper still defaulted to `values.yaml`, the exhaustive
    alpha profile. A direct `MODE=install scripts/citus-scale/deploy.sh`
    invocation could therefore install alpha sidecars and alpha runtime/security
    intent without making that choice explicit.
18. The disaster-recovery runbook described a region-loss drill before every
    production release without stating that `FEATURE: MR9` is still alpha and
    that the checklist is not production evidence until real failover, PITR,
    backup-restore, sidecar, and conflict-policy drill logs exist.
19. Component READMEs and custom catalog/benchmark/image overview docs
    described deterministic contracts, benchmark targets, and runtime models
    without a shared production boundary. A reader could therefore mistake
    local canonical reports or empty benchmark scaffolding for production
    evidence on alpha features.
20. The cohabitation docs and TS6 patch metadata referred to a cohabitation
    suite without stating that default contract checks and the opt-in kind
    smoke shape are not production evidence. A reader could therefore treat
    the `citus.cohabit_extensions` trust contract as proof that a real
    Citus+TimescaleDB hook chain was production-verified.
21. `values-prod.yaml` still inherited mutable `:0.1.0` operator and pool
    image tags from the base chart. GitOps could therefore render a production
    profile without immutable release digests even though the runbooks require
    exact image digests for promotion.
22. `scripts/citus-scale/build-app-images.sh` could push image tags but did
    not write a durable digest manifest or fail when a pushed image lacked a
    repo digest. Production values could require digests while the release path
    still left operators to discover them manually.
23. `make -f Makefile.ai-blaise gate-close` invoked Docker-backed live smokes
    through targets that did not set `REQUIRE_DOCKER=1`. On a machine without
    Docker, the documented release gate could silently skip live Docker smokes
    while still reporting success for those targets.
24. The D13 runtime image matrix included `citusctl`, but the shared Dockerfile
    defaulted every image to `serve` and no live smoke executed the built
    `citusctl` image. The image could therefore be built and loaded while its
    default container behavior failed before running any tool command.
25. The documented `gate-close` release path did not directly run
    `image-check` or `deploy-check`, and `deploy-check.sh` skipped rendered Helm
    checks when Helm was unavailable. A release gate could silently skip
    rendered Helm chart checks and image contract validation unless the separate
    GitHub workflows were inspected manually.
26. `TS18` was marked production-ready even though its real TimescaleDB smoke
    stubbed the Citus distribution entrypoint. That made the bridge-state SQL
    surface look fully cohabitation-proven before a real Citus+TimescaleDB run
    existed.
27. The D8 deploy wrapper supported `MODE=install`, but the live kind smoke used
    `helm upgrade --install` directly for the production-values phase. The
    production-safe human deploy wrapper install path was therefore documented
    and rendered, but not live-gated.

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
- The image overview, architecture, and upgrade runbook now carry the same
  Bundle1 alpha boundary as the detailed bundle docs: static manifests and SQL
  file smokes are not production evidence for the full operand image until a
  real operand image build/initdb smoke exists. The production gap audit
  machine-checks that wording and rejects stale operand-image overclaims.
- The deploy wrapper defaults to `values-prod.yaml` through
  `DEPLOY_PROFILE=prod`. Rendering dev or exhaustive profiles remains
  available, but `MODE=install` refuses any non-production or custom values file
  unless `ALLOW_ALPHA_INSTALL=1` is set explicitly for that run. The deploy
  check and production gap audit enforce the production-safe default and the
  non-production install guard.
- The disaster-recovery runbook now states that it is a release prerequisite
  and operational checklist, not production evidence by itself. It keeps
  `FEATURE: MR9` alpha until live multi-region failover, PITR restore, backup
  artifact restore, sidecar readiness, and conflict-policy evidence is measured
  against real runtime infrastructure. The production gap audit machine-checks
  that guardrail.
- Custom component READMEs, CRD/catalog docs, benchmark docs, and image
  overview docs now carry a shared production boundary: unless a feature is
  explicitly `Status: production-ready` in `docs/ai-blaise/NEW_FEATURES.md`,
  listed surfaces are alpha contracts and deterministic canonical reports,
  benchmark targets, or local runtime models are CI artifacts or planning
  scaffolding, not production evidence. The production gap audit
  machine-checks that every custom boundary doc preserves the wording.
- The cohabitation docs, TS6 patch metadata, and opt-in kind smoke now state
  that `citus.cohabit_extensions` is a deployment-level trust contract, not
  production evidence. Static patch checks, pure Rust acceptance models, and
  default contract-mode smoke output remain non-production until a live
  Citus+TimescaleDB cohabitation run records the operand image digest, command
  log, and CI or VM evidence in the audit. The production gap audit
  machine-checks that boundary.
- The Helm image helper now supports digest-pinned images, and
  `values-prod.yaml` sets `global.requireImageDigest: true` so production
  rendering fails unless the operator and pool images are supplied by immutable
  `sha256:` digests. `scripts/citus-scale/deploy.sh` accepts
  `OPERATOR_IMAGE_DIGEST` and `POOL_IMAGE_DIGEST` for production
  render/install, while `ALLOW_MUTABLE_IMAGE_TAGS=1` is an explicit local/dev
  escape hatch. The kind production smoke sets that escape hatch through Helm
  only for locally loaded test images; that proves runtime behavior, not
  release image pinning. GitOps sync fails closed until the release branch or
  deployment overlay supplies those digests.
- The Rust app image build script now writes
  `artifacts/ai-blaise-image-digests.tsv` with repository, image, tag, digest,
  package, binary, and push status. Release pushes fail if Docker does not
  report an immutable repo digest for a pushed image, giving production
  render/install a concrete source for `OPERATOR_IMAGE_DIGEST` and
  `POOL_IMAGE_DIGEST`.
- The Makefile live-smoke targets now set `REQUIRE_DOCKER=1` for pool proxy,
  SQL extension, real TimescaleDB bridge, and primary/standby observability
  replication smokes. Direct scripts may still skip for exploratory local use,
  but the documented `gate-close` release path fails closed if Docker is
  unavailable.
- The shared Rust runtime Dockerfile now accepts explicit default command args.
  Service images still default to `serve`, while the `citusctl` image defaults
  to `plan inspect cluster`. The kind production smoke runs a Kubernetes Job from
  the built `citusctl` image and requires the expected plan output, so the tool
  image is executed as part of D13 evidence rather than merely built and loaded.
- The Makefile release gate now runs `image-check` and `deploy-check` directly.
  The `deploy-check` target sets `REQUIRE_HELM=1`, so missing Helm is a release
  gate failure instead of a skipped rendered-chart evidence path.
- `TS18` remains alpha until real Citus+TimescaleDB cohabitation runs without a
  stubbed distribution entrypoint and records image digest, command log, and CI
  or VM evidence. The current TimescaleDB smoke is contract/runtime evidence for
  the bridge-state SQL surface, not production evidence for distributed
  cohabitation.
- The D8 deploy wrapper install path is now live-gated: the `values-prod.yaml`
  phase of `kind-production-smoke.sh` installs through
  `scripts/citus-scale/deploy.sh MODE=install` instead of bypassing the wrapper.
  The optional tools Deployment remains dev-only; production evidence executes
  the built `citusctl` image through a smoke Job. The Argo application is a
  GitOps render contract, not live controller evidence.
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
- Production Helm render/install must use immutable operator and pool image
  digests; local kind smokes that disable the digest requirement are runtime
  smoke evidence only, not release image-pinning evidence.
- Every production-promoted SQL runtime smoke must be part of the GitHub image
  workflow, `gate-close`, and static production gap audit guards; Makefile
  release smoke targets must set `REQUIRE_DOCKER=1` so missing Docker is a
  failure, not a skipped evidence path.
- The local release gate must run the same image and deploy contract checks as
  GitHub; rendered Helm checks must use `REQUIRE_HELM=1` so missing Helm is a
  failure, not a skipped evidence path.
- Production-ready Timescale/Citus claims require a real cohabitation run; stubs
  are acceptable only for alpha contract/runtime evidence and must be called out
  as such.
- Every custom boundary doc must keep the shared production boundary for
  deterministic contracts, benchmark targets, and local runtime models.

## Whole-Repo Production Readiness Audit

The deployment corrections above close the most dangerous false-positive path:
the chart now proves real Rust app images, real pods, sidecar probes, and live
SQL through the pool. The broader repository is still not production-ready as a
whole.

The current feature inventory contains 240 source `FEATURE:` markers and 161
feature headings in `docs/ai-blaise/NEW_FEATURES.md`. Six narrow headings are
`Status: production-ready` because they have live VM/GitHub evidence: `D13`
for the production runtime image matrix, `O4` for the shared sidecar
health/readiness/metrics runtime, `O1` for the installable
`pg_stat_statements` percentile view, `O2` for the installable local activity
stats view, `O3` for the installable replication-lag view against a real
streaming standby, and `R4` for the installable idle transaction detection SQL
surface. `TS18` remains alpha until real Citus+TimescaleDB cohabitation is
verified without a stubbed distribution entrypoint. The other
155 feature headings remain
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
SQL traffic, and live pod probe traffic. It also rejects custom boundary docs
that omit the shared production boundary for deterministic canonical reports,
benchmark targets, and local runtime models.

Production Helm values must also keep alpha sidecars disabled by default.
`values-prod.yaml` can carry replica/resource intent for those components, but
`ci/ai-blaise/deploy-check.sh` rejects production values that enable any alpha
sidecar before the corresponding feature is promoted with measured production
evidence.
