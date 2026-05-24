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
9. Production Helm values previously enabled alpha runtime/security intent fields for
   protocol pipelining, PG18 `io_uring`, External Secrets, TLS, release
   attestations, and pool CIDR allowlists even though the chart did not yet
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
12a. Even after narrowing the resource list, the production/default Helm
     profiles still rendered controller-grade operator ClusterRole and
     ClusterRoleBinding resources despite the current operator `serve` path not
     running a Kubernetes controller.
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
28. `FEATURE: O5` and the shared sidecar README referred to OpenTelemetry
    traces, configuration, and PostgreSQL connection helpers even though the
    runtime has no trace emission, collector wiring, OTEL dependency,
    configuration loader, or PostgreSQL helper module. That made an alpha
    deployment-contract entry read like broader runtime implementation existed.
29. `FEATURE: O6` and `FEATURE: O10` were promoted as chart observability
    surfaces, but the deploy guard mostly grepped marker strings. Invalid
    embedded Grafana JSON, missing panel expressions, or an unguarded pool
    error-rate denominator could slip through without a parsed dashboard or
    alert-expression contract.
30. Several alpha feature entries still used production-sounding language such
    as stable production queries, multi-tenant production usage, live
    operations, and live dashboard views. Even when future-prerequisite framed,
    those phrases made alpha contracts read closer to promoted runtime behavior
    than their status allowed.
31. The first alpha wording guard covered only a narrow set of standalone
    feature headings. Other alpha headings, former addendum entries, and tool
    READMEs still used stable/live/production phrasing for contracts that are
    not promoted runtime behavior.
32. The Makefile target for the real Timescale/Citus cohabitation smoke already
    failed closed with `REQUIRE_DOCKER=1`, but the production gap audit did not
    explicitly enforce that target contract. A future edit could have weakened
    the local release gate while the audit still checked only the script and
    GitHub workflow paths.
33. The TS6 reference patch still said cohabitation promotion required an exact
    image digest and command log, while the executable smoke recorded an image
    identity but not the commit and command path in its evidence file. The
    wording and evidence schema needed to agree so a production audit cannot
    point at evidence the script does not actually produce.
34. `FEATURE: T15` had a real byte-transparent pool proxy and a deterministic
    pipeline policy contract, but no smoke proved that pipelined PostgreSQL
    client frames could traverse the pool data port before the client waited
    for the first response. That left the narrow pool pipelining claim short of
    wire-protocol production evidence.
35. `FEATURE: Auth2` had Rust/sidecar claim-shape contracts, but no installable
    SQL runtime for setting or reading session claims. That meant tenant-aware
    SQL surfaces could not be promoted without a real Postgres extension smoke.
36. `FEATURE: D4`, `FEATURE: M5`, and `FEATURE: TS8` had useful in-memory
    analyzer tests and a canonical TSV emitter, but no file-backed CLI smoke
    over real migration SQL and metadata. That left IDE diagnostics and
    quick-fix claims short of production evidence for an executable user
    surface.
37. `FEATURE: Sec1` had a Rust policy-plan contract and Auth2 had live session
    claims, but the extension still lacked installable SQL predicates that a
    real PostgreSQL RLS policy could execute under a non-superuser role.
38. `FEATURE: Sec5` and `FEATURE: Sec6` described ledger transfer and HMAC
    seal plans, but the extension did not install append-only ledger tables,
    hash-chain verification, or pgcrypto-backed seal functions that a real
    database could execute.
39. The MCP JSON-RPC tools had production-looking process smokes but only
    validation-only tool behavior. The corrective boundary is `MCP4`: a narrow
    production-ready `tools/citus-mcp` read-only database execution runtime
    backed by the maintained PostgreSQL driver, native TLS support, read-only
    transactions, bounded result materialization, row/timeout ceilings,
    `EXPLAIN ANALYZE` rejection, and a real PostgreSQL smoke.
40. Release and PR integration status could be checked only by reading several
    separate local gates plus GitHub check pages. That left room for stale V2
    command counts, missing production evidence, toy or alpha overclaims,
    missing benchmark formatting, or broad matrix failures to be overlooked
    while still claiming release readiness.
41. Benchmark smokes emitted JSON artifacts and docs listed SLO targets, but
    the production path lacked one thresholded, fail-closed evidence checker.
    Scaffold results, missing baselines, or missing driver data could therefore
    look like benchmark coverage unless a human inspected the artifacts.

42. `FEATURE: A10` and `FEATURE: A11` previously described AI SQL
    intent surfaces without an installable fail-closed SQL contract. A10 and
    A11 remain alpha: the corrected boundary is `sql-intent-fail-closed-only`,
    with no live provider call, no real streaming provider chunks, and no
    generated-query execution.

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
- The Kubernetes production-values lane now separates strict render evidence
  from live runtime evidence. `ci/ai-blaise/live-k8s-e2e.sh` rejects strict
  real-mode renders that use mutable/latest images or alpha sidecars, while
  `ci/ai-blaise/k8s-production-values-live-smoke.sh` proves a real kind
  StatefulSet, Ready pod, and in-cluster SQL client Job through Service DNS with
  an immutable image. It writes `claim_boundary=postgres_substrate_only` and
  does not claim live Rust app-image, operator, pool, or Citus data-plane
  behavior unless the exact command-center chart and digest-pinned Citus images
  are supplied.
- CI checks now assert the real image matrix, `serve` support, Helm probe
  contracts, live sidecar probe coverage, loopback `serve` probes and metrics
  for the operator, shared runtime, every sidecar, and the pool admin surface,
  structured-log schema coverage and typed JSON record validation for every
  runtime sidecar, pool data/admin port separation, pool live-SQL smoke coverage,
  and SQL bridge-state smoke coverage.
- The SQL extension smoke now attaches stdin to `psql`, preloads and creates
  `pg_stat_statements`, verifies live percentile rows through
  `companion_pg_stat_statements_p95`, opens a real idle-in-transaction
  backend, and requires `companion_idle_transactions(...)` to detect it.
- The image workflow and `gate-close` now run every promoted SQL runtime
  smoke: the plain PostgreSQL extension smoke, the real TimescaleDB bridge
  smoke, the real Citus+TimescaleDB cohabitation smoke, and the
  primary/standby observability replication smoke. The production gap audit
  rejects regressions that leave a promoted runtime smoke out of those gates.
- The API trio sidecars now have runtime front doors instead of canonical-only
  binaries: PostgREST renders config/OpenAPI and can spawn the configured child
  process, GraphQL renders the `graphql.resolve(...)` pg_graphql boundary and
  registers subscription transport state, and edge-functions exposes registry,
  trigger, invocation, and UDS-callback runtime surfaces. The new
  `ci/ai-blaise/api-trio-runtime-smoke.sh` boots all three services and verifies
  live TCP readiness plus API-specific behavior. These features remain alpha
  where the docs still require a live Postgres/PostgREST/pg_graphql/Deno/Bun
  deployment before production promotion.
- The bundled-extension docs and operand-image README now explicitly state that
  `FEATURE: Bundle1` is not production-ready as a whole. The PG17 source-build
  path has targeted live evidence for feasible PGDG-missing extensions, and the
  pg_cron cohabitation smoke is subset evidence only for a real PG17
  Citus+pg_cron boot, SQL-visible detection, job registration, and
  missing-allowlist fail-closed behavior. The complete operand initdb contract
  remains alpha until the plrust upstream PG17 blocker and full-bundle image
  smoke are closed. The source-build subset now has a structured lockfile and
  contract checker that cross-validates manifest rows, Dockerfile pins/labels,
  smoke coverage, tracked evidence, and docs; source-build image labels state
  `source-build-subset-no-complete-initdb` so the evidence cannot be mistaken
  for the complete initdb path. The production gap audit rejects the old
  operand-image overclaim until that full-bundle evidence exists.
- Production values now keep alpha runtime/security intent controls disabled by
  default. The deploy check and production gap audit reject production values
  that enable protocol pipelining, PG18 `io_uring`, External Secrets, TLS,
  or release attestations before those controls are rendered, enforced, and
  verified end to end.
- Sec13 pool CIDR access control is now enforced by the live pool data path and
  rendered by Helm. The pool rejects PostgreSQL clients outside
  `AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST` before connecting upstream, exposes
  `ai_blaise_citus_pool_rejected_connections_total`, renders a matching
  NetworkPolicy for clusters with NetworkPolicy-capable CNI enforcement, and is
  proven by Docker plus kind smokes that verify allowed and denied SQL traffic.
- Operator RBAC now enumerates the ai-blaise CRD resources instead of using a
  wildcard grant, and it no longer grants Secret access while secret binding
  remains alpha. The operator security runner now fails closed when pool or
  sidecar runtime Secret refs are not backed by ExternalSecret binding metadata,
  validates deterministic ExternalSecret manifest and TLS Secret-reference
  shape, and still requires TLS 1.3, client certificates, restricted security
  contexts, and no Secret RBAC grants. The new security supply-chain smoke also
  validates the narrow SBOM/cosign metadata contract for digest-pinned fixture
  image refs, `.spdx.json` SBOM paths, `.sigstore.json` cosign bundles, SLSA
  provenance predicate metadata, and mutable-image/malformed-SBOM fail-closed
  behavior. This does not publish SBOMs, does not sign images, does not
  verify a registry signature, enforce admission policy, prove cert-manager
  issuance, or prove External Secrets controller reconciliation; those remain
  alpha. The deploy
  check and production gap audit reject wildcard CRD resources or Secret
  permissions in the operator role and require the security supply-chain gate.
- Production/default Helm profiles now render only the operator ServiceAccount;
  controller-grade operator ClusterRole and ClusterRoleBinding resources are
  gated behind the explicit alpha `operator.controllerRbac.enabled` flag and
  rendered by `values-exhaustive.yaml` only for non-production contract
  coverage. Deploy checks and the production gap audit reject those RBAC
  resources in default/prod renders.
- After the chart fold, full Helm profiles and Argo application behavior are
  command-center-owned release evidence. This repo now verifies the Citus-side
  contract: strict real-mode chart renders must be digest-pinned and alpha-free,
  and the self-contained VM smoke proves only the live kind/SQL-service substrate
  path without pretending to install the unpublished command-center release
  profile.
- Argo application pruning, self-heal, namespace creation, and production
  profile selection remain command-center deployment concerns. They must be
  proven in that repository before they can be cited as release-controller
  evidence for this overlay.
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
- Multi-region placement/survival hardening on 2026-05-24 adds VM evidence via
  `ci/ai-blaise/operator-multiregion-contracts-smoke.sh` for deterministic
  operator contracts only: strict region label/zone/tablespace validation,
  duplicate region inventory rejection, `RegionalRowPlacementPlan` admission
  checks for declared regions, distribution key, replication factor, and
  `topology.kubernetes.io/region` spread, plus canonical output with
  `live_k8s_exercised=false`. `MR3`, `MR5`, and `MR9` remain alpha; this is not
  evidence for live row movement, GeoIP pool routing, cross-region client
  traffic, DNS cutover, regional failover, PITR restore, or backup artifact
  restore.
- The backup/PITR runbooks now have a restore-depth gate backed by
  `ci/ai-blaise/dr-restore-depth-check.sh`. The gate validates fail-closed
  restore configuration, read-only branch-before-restore policy, destructive
  plan-id requirements, two-operator approval evidence, KMS evidence, WAL
  archive continuity, PITR target/replay evidence, and tenant/placement/ledger/
  search validation query evidence. With `REQUIRE_DOCKER=1`, it also runs a
  real PostgreSQL PITR smoke using `pg_basebackup`, WAL archiving,
  `recovery_target_time`, and restored-row verification.
- B1/B3/B4/B6 backup sidecar production-ready evidence is bounded to the
  local sidecar runtime: strict config validation, WAL-G command materialization
  and execution against deterministic local fakes, HTTP status/control paths,
  scheduler state, retention/failure accounting, metrics, PITR job records, and
  queryable branch read-only probes. It does not claim live S3/GCS/Azure
  credentials, managed WAL-G object-store success, Kubernetes CronJob execution,
  or live production-cluster PITR.
- Custom component READMEs, CRD/catalog docs, benchmark docs, and image
  overview docs now carry a shared production boundary: unless a feature is
  explicitly `Status: production-ready` in `docs/ai-blaise/NEW_FEATURES.md`,
  listed surfaces are alpha contracts and deterministic canonical reports,
  benchmark targets, or local runtime models are CI artifacts or planning
  scaffolding, not production evidence. The production gap audit
  machine-checks that every custom boundary doc preserves the wording.
- The TS6 cohabitation source changes are now integrated into the fork. The
  patch files remain as rebase/reference artifacts, and
  `ci/ai-blaise/patches-check.sh` accepts either clean application to an
  upstream-like tree or clean reverse application when the patch is already
  integrated. `ci/ai-blaise/timescale-cohabitation-smoke.sh` builds this Citus
  fork into `timescale/timescaledb:latest-pg17`, starts PostgreSQL with
  `shared_preload_libraries=timescaledb,citus` and
  `citus.cohabit_extensions=timescaledb`, creates real `citus`,
  `timescaledb`, and `ai_blaise_citus` extensions, verifies real
  `pg_dist_partition` rows, records observed PostgreSQL/TimescaleDB/Citus
  versions, and executes TS1/TS2/TS3/TS4/TS5/TS12 apply functions without
  defining a Citus stub. The evidence is explicitly scoped as
  `entrypoints-and-catalog-state-only`: TS6 and TS18 are therefore
  production-ready narrow startup/load/apply guard surfaces, while the broader
  distributed Timescale feature entries remain alpha until multi-worker fanout,
  background policy execution, continuous aggregate refresh, rebalance, and
  operator reconciliation are proven end to end.
- The Helm image helper now supports digest-pinned images, and the default
  `values.yaml` profile and `values-prod.yaml` both set
  `global.requireImageDigest: true` so production rendering fails unless the
  operator and pool images are supplied by immutable `sha256:` digests.
  `scripts/citus-scale/deploy.sh` accepts
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
  SQL extension, real TimescaleDB bridge, real Citus+TimescaleDB
  cohabitation, and primary/standby observability replication smokes. Direct
  scripts may still skip for exploratory local use, but the documented
  `gate-close` release path fails closed if Docker is unavailable.
- The shared Rust runtime Dockerfile now accepts explicit default command args.
  Service images still default to `serve`, while the `citusctl` image defaults
  to `plan inspect cluster`. The kind production smoke runs a Kubernetes Job from
  the built `citusctl` image and requires the expected plan output, so the tool
  image is executed as part of D13 evidence rather than merely built and loaded.
- The Makefile release gate now runs `image-check` and `deploy-check` directly.
  The `deploy-check` target sets `REQUIRE_HELM=1`, so missing Helm is a release
  gate failure instead of a skipped rendered-chart evidence path.
- `TS19` and `TS20` gained a narrow real pg_cron cohabitation smoke, but they
  remain alpha as whole features. The smoke builds a PG17 image with PGDG
  `pg_cron`, this Citus fork, and `ai_blaise_citus`, boots with
  `shared_preload_libraries=pg_cron,citus` and
  `citus.cohabit_extensions=pg_cron`, creates real extensions, registers a cron
  job, records `artifacts/pg-cron-cohabitation-evidence.tsv`, and verifies the
  SQL-visible cohabit detector fails closed when the allowlist entry is absent.
  It does not prove the unexposed TS19 in-shmem clock-reservation flag,
  long-running pg_cron worker execution, the TS20 C API being called by a live C
  extension, or broad production cohabitation.
- `TS18` now has real Citus+TimescaleDB cohabitation evidence without a stubbed
  distribution entrypoint. The VM run built
  `ai-blaise-citus-timescale-cohabitation:local` from
  `timescale/timescaledb:latest-pg17`, installed this Citus fork and the
  `ai_blaise_citus` SQL extension, created real `citus`, `timescaledb`, and
  `ai_blaise_citus` extensions, inserted through a real Citus distributed
  table, and then executed the bridge apply functions against the cohabiting
  server. The generated evidence file is
  `artifacts/timescale-cohabitation-evidence.tsv` and includes
  `timescaledb_extversion`, `citus_extversion`, `real_citus_distribution=true`,
  `stubbed_citus_distribution=false`, bridge feature counts, and
  `policy_execution_scope=entrypoints-and-catalog-state-only` so the evidence
  cannot be mistaken for full TimescaleDB policy execution or planner
  correctness.
- The D8 deploy wrapper install path now fails closed in this repo and points to
  command-center for the chart. Live Citus app-container install evidence must
  run through the external chart with strict digest-pinned values; the branch-local
  self-contained kind smoke covers only the production-values Kubernetes harness
  and SQL Service path (`claim_boundary=postgres-substrate-only`). The optional
  tools Deployment remains dev-only, and the Argo application is a GitOps render
  contract until a live controller run is recorded in the chart-owning repository.
- The O5 register entry and shared sidecar README now describe only the
  implemented sidecar deployment contract. They explicitly state that tracing
  and OpenTelemetry export, configuration loading, and PostgreSQL connection
  helpers are not implemented, and the production gap audit rejects
  reintroduced claims until real runtime code and live evidence exist.
- The D7 direct Helm install path now fails closed at the Citus-side harness
  boundary. Strict real-mode renders require immutable `@sha256` images and no
  alpha sidecar workloads, while the actual production chart profiles remain in
  command-center. The kind smoke no longer treats an exhaustive local profile as
  production evidence from this repository.
- The observability dashboard and alert templates now query
  `ai_blaise_sidecar_ready`, the metric emitted by the sidecar runtime, and
  the live kind production smoke requires the installed dashboard ConfigMap and
  PrometheusRule resources to contain the production dashboard JSON payloads
  and alert names in every Helm profile installed by the smoke.
- The observability deploy check now parses the embedded Grafana dashboard JSON
  from the Helm template, requires the exact dashboard files, panel titles, and
  PromQL target expressions, and rejects unguarded pool request-rate division.
  The dashboard and alert rule both use a guarded pool error-rate denominator,
  and the alert rule also requires positive request traffic before firing.
- O2 and R4 production-ready wording now matches the implemented SQL runtime:
  O2 is local-node activity stats with a compatibility alias, and R4 is
  idle-transaction detection only, not cancellation or termination.
- Alpha feature entries now avoid production-sounding phrases for unpromoted
  surfaces such as vectorizer accounting, pool pipelining, plan freeze, MCP
  tenant scoping, and citus-watch dashboard/TUI contracts. The production gap
  audit rejects the stale phrases so alpha contracts stay visibly
  non-production.
- A2/A3/A4/A5/A6 vectorizer production-ready evidence is bounded to the local
  Rust sidecar runtime: mock-provider queue processing, explicit live-provider
  opt-in policy, fail-closed manual request validation, PostgreSQL-backed queue,
  budget and usage tables, health/readiness/drain endpoints, metrics, and the
  Docker PostgreSQL smoke. It does not claim real external embedding-provider
  calls, GPU inference, production-scale queue throughput, tenant billing
  integration, or broad semantic-search correctness.
- Alpha wording cleanup now also covers the former addendum entries and tool
  READMEs. Schema visualization, plan-freeze, PostgREST, storage, D10, O5, and
  O12 wording uses versioned, operator, release, or measured-evidence language
  instead of stable/live/production phrasing for alpha contracts.
- Worker Tools Runtime evidence from 2026-05-24 tightens the already-promoted
  D3/D5/D6/D12/M9/O13 snapshot-backed boundary: the shared tools runtime now
  rejects duplicate snapshot identities and vectorizer/realtime tenant
  references to unknown tenants under cargo tests and `tools-ui-runtime-smoke`.
  Live database sessions, browser embedding, continuous terminal event loops,
  and mutating execution remain alpha.
- The production gap audit now explicitly checks that the
  `timescale-cohabitation-smoke` Makefile target runs with `REQUIRE_DOCKER=1`,
  matching the live cohabitation script and GitHub image workflow guardrails.
- The Timescale/Citus cohabitation smoke evidence file now records the Git SHA,
  stable Docker image identity, base image reference, command path, preload
  libraries, and cohabitation allowlist. The TS6 reference patch and docs now
  use that same evidence contract.
- The TS-version matrix gate now pins TimescaleDB minor-line image tags under
  `tests/cohab-matrix/`, requires 2.27 to run live, and records 2.28 as
  `skip-with-note` only because the VM registry probe on 2026-05-24 found no
  `2.28-pg17`, `2.28.0-pg17`, or `2.28.1-pg17` image. This does not promote
  TS 2.28 to production-ready; a published 2.28 image fails the matrix until
  any `unknown` hook rows are measured and updated.
- The pool proxy smoke now opens a raw PostgreSQL protocol client through the
  real pool `serve` data port, sends two simple-query frames without waiting
  for the first result, verifies ordered rows from a `postgres:17` backend, and
  promotes only the `FEATURE: T7` simple-query data-plane pipelining boundary;
  extended-query batching and broader shard-aware pool routing remain alpha.

- The pool routing/security canonical smoke now covers bounded production-ready
  contracts for T9/T12/R10 plus alpha-only MR5 parser/report evidence:
  fail-closed mirror rule parsing and deterministic sampling reports,
  fail-closed HTAP feature-report parsing, fail-closed GeoIP CIDR/replica-table
  parsing with fallback reports, and TLS ticket rotation reports with redacted
  fingerprints. This is not evidence for live canary mirroring, managed GeoIP
  databases, rustls listener/session-resumption traffic, analytical sidecar
  query execution, live multi-region read routing, or Kubernetes traffic.
- The SQL extension now installs `FEATURE: Auth2` session-claim helpers that
  set and read `uid`, `role`, `tenant_id`, and optional JWT ID through custom
  GUCs. The PostgreSQL extension smoke proves valid claims and empty-claim
  rejection against a real `postgres:17` container while JWT issuance, pool
  authentication, and token-cache behavior remain alpha. Sec2 JWT verification
  has a separate SQL runtime boundary.
- The `citusctl` CLI now has a direct executable smoke for the narrow
  `FEATURE: D2` apply-mode plan-id guard. The smoke requires `citusctl apply`
  without a plan ID to fail closed and verifies valid plan/apply summaries from
  the real binary. This is not evidence for mutating cluster apply execution,
  manifest reconciliation, migrations, backups, PITR, WAL replay, or dev
  cluster lifecycle.
- The `citusctl` CLI now also has a direct executable smoke for the narrow
  WF2 fixture-backed WAL replay debugger plan. The smoke creates a
  local WAL fixture, requires exact deterministic JSON from `citusctl plan
  wal-replay ... --fixture ... --json`, and verifies unsupported source URI
  schemes plus out-of-range target times fail closed. This is not evidence for
  real WAL segment inspection, PostgreSQL `pg_walinspect` execution,
  restore/replay mutation, or production cluster operations.
- The `citusctl` CLI now has direct executable smokes for bounded plan/apply
  and dev lifecycle behavior. `citusctl-smoke.sh` proves the narrow D2
  apply-mode plan-id guard, including fail-closed unstable plan IDs, and
  `citusctl-dev-lifecycle-smoke.sh` proves local D1/M8 `plan/apply dev`
  runtime behavior through explicit `--state-dir` real-binary invocations:
  dry-run plan rendering, deterministic JSON/TSV output, stable plan-id apply
  validation, idempotent up/down state handling, local audit append, and
  state-file-only cleanup guardrails. This is not evidence for mutating
  Kubernetes apply execution, manifest reconciliation, migrations, backups,
  PITR, WAL replay, Docker/kind startup, or a live Citus data plane.
- The `citus-lsp` CLI now has direct executable smokes for the narrow
  `FEATURE: D4`, `FEATURE: M5`, and `FEATURE: TS8` file-backed diagnostic
  surface. The smoke runs `citus-lsp analyze --metadata <metadata.tsv> --sql
  <migration.sql>` against a real SQL file and metadata TSV, verifies
  diagnostics and quick-fix actions, verifies distributed hypertable bridge
  suppression, and verifies bad or missing metadata fails closed. It also
  drives file-backed LSP-style `Content-Length` JSON-RPC stdio frames through
  `citus-lsp serve-stdio --metadata <metadata.tsv>` for initialize, opened-file
  publish diagnostics, pull diagnostics, malformed JSON, unknown methods, and
  unopened-document failure. This is not evidence for editor transport,
  workspace indexing, automatic file rewrites, live metadata refresh, or full
  PostgreSQL grammar coverage.
- The Raft/HLC/transaction-status triad now has executable sidecar runtime
  evidence: `sidecar-raft-smoke.sh` proves deterministic election,
  AppendEntries replication, quorum commit, durable log replay, and snapshot
  watermarking; `topology-consensus-smoke.sh` proves S4 coordinator-less pool
  admission, S5 fail-closed placement/member validation, and S9/MR6 closed
  timestamp follower-read serve/reject gates; HLC runtime canonical output
  proves peer clock exchange and closed-timestamp derivation;
  `parallel-commits-smoke.sh` proves staging, finalize, and modeled fast-path
  step count; `schema-txn-runtime-smoke.sh` drives the real txn-status HTTP
  server through stage -> wait -> ack -> commit with malformed/unknown-field
  rejection; and `sql-extension-smoke.sh` installs `companion.txn_stage`/
  `companion.txn_finalize` into real PostgreSQL. S4, S5, S9, MR6, and T5
  remain alpha for the broader distributed-database behavior until networked
  multi-process Raft, MVCC follower-read execution, PostgreSQL-core patch
  integration, Citus executor integration, pool routing, and Kubernetes
  operator reconciliation are live-gated.
- The schema-job sidecar now has an explicit runtime-boundary smoke for the
  narrow C10/M2 sidecar surface. `schema-txn-runtime-smoke.sh` runs the real
  binary canonical worker output, controller advance/wait/rollback output,
  manifest validator, unsafe SQL/apply-boundary rejection, malformed JSON
  rejection, and loopback probe behavior. This is still not evidence for live
  Kubernetes reconciliation, lock orchestration, dual-write triggers, or actual
  DDL/backfill execution workers beyond the SQL catalog state machine already
  proven by `sql-extension-smoke.sh`.
- The auto-API sidecars and edge-functions Rust boundary now have bounded
  process/socket smokes for their canonical API contracts. The shared smoke
  builds the real PostgREST, GraphQL, and edge-functions binaries, runs their
  canonical TSV commands directly, starts each `serve` process on loopback,
  verifies health/readiness/metrics/drain behavior persists across HTTP
  requests, and verifies unknown commands plus empty listen addresses fail
  closed. `graphql-postgrest-runtime-smoke.sh` adds focused GraphQL/PostgREST
  evidence for runtime dependency validation, malformed input handling,
  PostgREST route method rejection, secret-backed config rendering, GraphQL
  missing-claim errors, introspection denial, and subscription-boundary
  responses. The dedicated `sidecar-edge-functions-runtime-smoke.sh`
  additionally proves edge-functions plan-only status, env-secret/path/JSON
  validation, payload and timeout ceilings, unknown-function handling, and
  fail-closed rejection of external Deno/Bun execution requests. This is not
  evidence for table-backed PostgREST request serving, live `pg_graphql`
  execution, external Deno/Bun user-code execution, real PostgreSQL UDS callback
  execution, queue/broker dispatch, live CDC tailing, or Kubernetes deployment;
  the GraphQL/PostGREST sidecar feature headings and EF1, EF2, EF4, and EF5
  remain alpha until those live data-plane paths are proven.
- The SQL extension now installs `FEATURE: Sec1` RLS helper predicates:
  `companion_tenant_id_matches(...)` and `companion_require_tenant_id()`. The
  PostgreSQL extension smoke creates a real row-level security policy over a
  tenant table, switches to a non-superuser role, verifies tenant-a and
  tenant-b sessions see only their own rows, verifies `WITH CHECK` rejects a
  cross-tenant insert, and verifies missing tenant claims fail closed. This is
  not evidence for automatic policy generation, pool authentication, or
  auto-API integration. Sec2 JWT verification is separate evidence.
- The SQL extension now installs `FEATURE: Sec2` HS256 JWT verification
  helpers: base64url encode/decode, audience matching, and
  `companion_verify_jwt_hs256(...)`. The PostgreSQL extension smoke builds a
  signed token inside the database, verifies issuer, audience, expiration,
  not-before, subject, role, tenant, and JWT ID claims, feeds the result into
  Auth2 session claims, and proves bad signatures, wrong audiences, expired
  tokens, and missing tenant claims fail closed. This is not evidence for
  JWKS/RSA/ECDSA key discovery, Auth1 token issuance, pool authentication,
  Auth3 token-cache behavior, external secret resolution, or key rotation.
- The auth sidecar now ships real `FEATURE: Auth1`, `FEATURE: Auth4`, and
  `FEATURE: Auth5` runtime boundaries. The auth smoke starts the real binary
  with an explicit signing secret, verifies live health/readiness/metrics
  responses, registers users, logs in, verifies and introspects JWTs, refreshes
  sessions, logs out, proves revoked JWTs fail closed, exercises TOTP login,
  enforces max-attempt TOTP lockout, proves WebAuthn routes fail closed with
  `501`, validates the OIDC login/callback pre-exchange boundary with a stub
  provider config, proves bad provider/redirect/nonce/replay callbacks fail
  closed, and applies the auth schema migration against `postgres:17` when
  `REQUIRE_DOCKER=1`. This is not evidence for RS256/JWKS discovery, external
  IdP token exchange, ID-token verification, account linking, WebAuthn
  ceremonies, key rotation, persistent runtime loading from the auth schema, or
  pool data-plane token authentication.
- The SQL extension now installs narrow `FEATURE: S6` and `FEATURE: S13`
  router helper runtimes. S6 persists placement-generation counters and
  local-placement worker names, verifies generation advancement and shard-zero
  failure in the PostgreSQL smoke, and does not claim Citus metadata
  synchronization, pool cache invalidation, rebalance hooks, planner
  invalidation, or operator placement changes. S13 exposes deterministic hash
  and bounded numeric range shard-index helpers, verifies out-of-range and
  zero-shard failures in the PostgreSQL smoke, and does not claim dynamic shard
  creation, Citus router integration, operator rebalancing, pool data-plane
  routing, or distributed range metadata propagation.
- The Citus quilt now carries `FEATURE: T3` and `FEATURE: T4` patch artifacts
  for the coordinator-skip locality probe and hashed router-planner placement
  intersection. The current evidence is patch applicability, companion
  router-assist tests, and `ci/ai-blaise/router-patch-smoke.sh`, which records
  portable algorithm-smoke output under `benchmarks/results/`. This is not live
  Citus performance evidence; full planner CPU and pool latency claims remain
  alpha until a real Citus build and multi-worker measurement are recorded.
- The companion advanced-planner runtime smoke now expands `FEATURE: T4`,
  `FEATURE: T10`, `FEATURE: T11`, `FEATURE: T13`, `FEATURE: T14`, and the
  adjacent advanced-planner contract set into deterministic runtime-boundary
  scenarios. It verifies duplicate-feature rejection, unknown-scenario
  rejection, invalid budget rejection, and live-distributed-execution overclaim
  rejection through `ci/ai-blaise/companion-advanced-planner-smoke.sh`. This is
  contract/runtime-boundary evidence only; protocol execution, Citus physical
  pushdown, distributed cursor/savepoint cleanup, and live multi-worker planner
  measurements remain alpha until separately measured.
- The SQL extension now installs narrow `FEATURE: PM3` and `FEATURE: PM4`
  plan-management runtimes. PM3 persists frozen query hashes, plan XML, hint
  set names, and promotion policy thresholds, and PM4 evaluates latency/cost
  regression policies with a sample log. The PostgreSQL smoke verifies visible
  plan-freeze state, policy storage, a violating sample, a non-violating
  sample, recorded samples, and fail-closed missing/empty identifiers. This is
  not evidence for planner enforcement, hint injection, pg_hint_plan/sr_plan
  integration, auto-promotion workers, distributed plan capture, plan XML
  validation, automatic production-plan replacement, query capture, workload
  baselining, or distributed planner integration.
- The SQL extension now installs `FEATURE: Sec5` and `FEATURE: Sec6` ledger
  runtime helpers: append-only ledger entry and seal tables,
  `companion_internal.ledger_transfer(...)`,
  `companion_ledger_chain_valid()`, and `companion_ledger_seal(...)`. The
  PostgreSQL extension smoke installs `pgcrypto`, appends chained ledger
  transfers, verifies chain validity, rejects a missing previous hash, seals a
  transfer with HMAC-SHA256, verifies the seal through
  `companion_ledger_entries`, rejects direct mutation/deletion through
  append-only triggers, and rejects unsupported HMAC algorithms. This is not
  evidence for multi-party accounting workflows, external ledger backends,
  external secret resolution, key rotation, hardware-backed signing, tenant
  workflow authorization, or migration/operator integration.

- The SQL extension now installs the bounded `FEATURE: Sto2` storage metadata
  runtime: `storage.file_attachment` is a jsonb domain with fail-closed
  validation for object shape, bucket names, traversal-safe object keys,
  content type, bounded non-negative `size_bytes`, lowercase 64-hex SHA-256,
  and optional object metadata. The PostgreSQL extension smoke also proves the
  constructor/accessors/URI helper and `storage.file_attachment_refs` tenant and
  owner metadata persistence. This is not evidence for object storage
  upload/download, retention automation, malware scanning, pool or RLS
  authorization, or sidecar integration.

## Verification Standard

- Benchmark evidence now has a checked-in threshold manifest and a reusable
  checker. Quick smoke runs call `ci/ai-blaise/performance-evidence-check.sh`
  in exploratory mode, while release promotion must run
  `make -f Makefile.ai-blaise performance-evidence-release-check` with
  `PERF_EVIDENCE_MODE=release BENCH_RESULT_TAG=release`. Release mode fails
  closed on missing artifacts, scaffold notes, missing baselines, malformed
  JSON, and SLO/capacity threshold misses without rerunning the expensive
  benchmark jobs.

Rule 10 completion for this branch requires local and VM verification of:

- Rust formatting and compile/test gates for all changed packages.
- SQL extension smoke against a real Postgres container.
- SQL smoke commands must be fed into the Postgres container with stdin
  attached; static image checks reject the old false-positive pattern.
- Strict Helm render validation for any external command-center chart supplied
  to the Citus-side live harness: mutable/latest image refs, placeholder/local
  production images, `imagePullPolicy: Always`, and alpha sidecar render leaks
  fail closed when `PRODUCTION_VALUES_STRICT=1`.
- A real VM kind deployment through
  `ci/ai-blaise/k8s-production-values-live-smoke.sh`: generated Helm
  `values-production.yaml`, immutable `@sha256` operand image, alpha sidecars
  disabled, Kubernetes readiness waits, an in-cluster SQL client Job that
  reaches PostgreSQL through Service DNS, captured Helm/kubectl/log/image
  evidence artifacts, and `claim_boundary=postgres_substrate_only`.
- Full command-center/Citus app-container release evidence still requires the
  exact release chart values and immutable Citus image digests. The
  self-contained VM live smoke is Kubernetes production-values substrate
  evidence, not proof of unpublished Citus app behavior, operator reconciliation,
  pool routing, Citus data-plane semantics, or multi-component command-center
  readiness.
- Every production-promoted SQL runtime smoke must be part of the GitHub image
  workflow, `gate-close`, and static production gap audit guards; Makefile
  release smoke targets must set `REQUIRE_DOCKER=1` so missing Docker is a
  failure, not a skipped evidence path.
- The local release gate must run the same image and deploy contract checks as
  GitHub; rendered Helm checks must use `REQUIRE_HELM=1` so missing Helm is a
  failure, not a skipped evidence path.
- Production-ready Timescale/Citus claims require a real cohabitation run; the
  stubbed Timescale bridge smoke remains useful contract evidence and must keep
  its missing-Citus fail-closed assertion, but promoted TS6/TS18 evidence must
  come from the non-stubbed cohabitation smoke and remain bounded to
  entrypoint/catalog-state behavior, not full TimescaleDB runtime correctness.
- Production-ready observability chart claims require parsed Grafana JSON,
  exact panel/PromQL contracts, live installed ConfigMap/PrometheusRule
  resources, and guarded pool error-rate expressions.
- Alpha feature docs, former addendum entries, and tool READMEs must not use
  production-sounding wording for unpromoted contracts; use versioned, runtime,
  tenant-workload, release-hardening, or operator-workflow language until
  measured production evidence supports a status promotion.
- Every custom boundary doc must keep the shared production boundary for
  deterministic contracts, benchmark targets, and local runtime models.
- Pool pipelining production evidence for `FEATURE: T7` or `FEATURE: T15` must
  include a raw PostgreSQL wire-protocol smoke that sends multiple simple-query
  frames through the real pool data port before reading the first result; psql
  request/response pacing alone is not sufficient evidence.
- T1 settings-bucket production evidence is limited to live proxy startup
  parsing, tracked-GUC fingerprint accounting, borrow/release metrics, and a
  raw PostgreSQL smoke proving simultaneous `citus.enable_repartition_joins`
  clients observe distinct backend state. It must not be cited as proof of
  reusable transaction pooling, backend reset correctness, shard-aware routing,
  or broad transaction pooling semantics.
- MCP1/MCP2/MCP3 sidecar production evidence is limited to real stdio and HTTP
  JSON-RPC request/response behavior, `/healthz`, `/readyz`, `/metrics`, and
  `/drain`, exact tool registry listing, malformed-input resilience, and
  fail-closed database dependency errors. It must not be cited as evidence for
  a full external MCP service, token authentication, durable sessions,
  streaming remote transport, sidecar-owned live database execution, Kubernetes
  execution, or mutating tools; MCP4 database execution remains `tools/citus-mcp`.
- Auth2 production evidence is limited to installable SQL session-claim
  helpers. It must not be cited as evidence for Auth1 JWT issuance or Auth3
  token-cache behavior; Sec2 JWT verification has a separate SQL-runtime
  evidence boundary.
- Sec1 production evidence is limited to installable SQL tenant RLS helper
  predicates under a real PostgreSQL RLS policy. It must not be cited as
  evidence for automatic policy generation, pool authentication, or auto-API
  integration. Sec2 JWT verification does not expand the Sec1 helper claim.
- Sec2 production evidence is limited to the local SQL HS256 verifier:
  pgcrypto HMAC signature verification, issuer/audience/expiration/not-before
  validation, required Auth2-compatible claim extraction, and fail-closed bad
  signature, wrong-audience, expired-token, and missing-tenant rejection. It
  must not be cited as evidence for JWKS/RSA/ECDSA key discovery, Auth1 token
  issuance, pool authentication, Auth3 token-cache behavior, external secret
  resolution, or key rotation.
- Sec5/Sec6 production evidence is limited to the local SQL ledger runtime:
  append-only entries, append-only HMAC seals, hash-chain verification, and
  pgcrypto-backed seal calculation. It must not be cited as evidence for
  external ledger backends, external secret resolution, key rotation,
  hardware-backed signing, accounting workflow authorization, or
  migration/operator integration.
- D1/M8 dev lifecycle evidence is limited to the real `citusctl` CLI local
  state-file runtime behind explicit `--state-dir` invocations: dry-run plan
  rendering, stable plan-id validation, deterministic JSON/TSV output,
  idempotent up/down state transitions, local audit append, and
  state-file-only cleanup. M8 remains alpha outside that bounded D1 subpath.
  This evidence must not be cited for Docker/kind startup, Kubernetes
  deployment, Postgres/Citus data-plane health, extension-service
  orchestration, or production cluster lifecycle management.
- D2 production evidence is limited to the real `citusctl` CLI apply-mode
  plan-id guard and command-summary smoke. It must not be cited as evidence for
  full mutating apply execution, manifest reconciliation, migrations, backups,
  PITR, WAL replay, or dev cluster lifecycle.
- D4/M5/TS8 production evidence is limited to the file-backed `citus-lsp`
  diagnostic and quick-fix action CLI plus the file-backed JSON-RPC stdio
  diagnostic service over supported SQL migration statements. It must not be
  cited as evidence for editor integration, workspace indexing, automatic
  rewrites, live metadata refresh, or full PostgreSQL grammar coverage.
- PM3/PM4 production evidence now includes the deterministic
  `companion/src/plan_runtime.rs` execution path for idempotency replay,
  bounded retry, durable audit-event emission, promotion-policy evaluation,
  regression-candidate rejection, and unknown-plan failure handling. It must
  not be cited as evidence for planner hint enforcement, background
  auto-promotion workers, distributed workload baselining, external durable
  storage, or pg_hint_plan/sr_plan runtime integration.
- R7 repack hardening now adds fail-closed strategy selection and a deterministic
  dry-run execution report for `sidecar/repack`. The canonical smoke records
  `dry_run=true`, `executed=false`, and `evidence_boundary=dry-run-plan-only`, so
  it must not be cited as production evidence for live `pg_repack`, live
  PostgreSQL 19 `REPACK CONCURRENTLY`, or Kubernetes-scheduled repack execution.
- Analytical/lakehouse hardening now adds fail-closed runtime-policy checks and
  a deterministic canonical smoke for L1/L2/L3/L4/L5/L6/L8/L12/L13. The smoke
  also starts the analytical loopback probe server and verifies health,
  readiness, metrics, and drain behavior. It records
  `external_io_attempted=false`, `query_engine_executed=false`, and
  `evidence_boundary=deterministic-runtime-report-only`, so it must not be cited
  as production evidence for live DataFusion, DuckDB, MotherDuck, Iceberg
  commits, object-store IO, Kubernetes traffic, or Citus planner integration.
- Agentmemory checkpointing for this depth-B slice was unavailable on the VM
  because `http://127.0.0.1:3911` refused connections; no memory files were
  edited or erased.

## Whole-Repo Production Readiness Audit

The deployment corrections above close the most dangerous false-positive path:
strict render checks now reject mutable image and alpha-sidecar leaks, and the
VM kind smoke proves a real production-values Kubernetes SQL Service substrate
path. Full Rust app-image, pool-routing, operator, Citus data-plane, and
command-center release-controller proof still requires the exact chart and
digest-pinned Citus images. The broader repository is still not production-ready
as a whole.

The current feature inventory is machine-derived by
`ci/ai-blaise/production-readiness-check.sh` and
`ci/ai-blaise/production-gap-audit.sh`. Do not restate source/heading/status
counts in prose: mutable totals are emitted on every run as
`source_feature_ids`, `feature_headings`, `production_ready`, and
`alpha_headings` fields in the `production_gap_audit` line, with the richer
`status_counts` map in the `production_readiness_audit` line. The scripts
compare source `FEATURE:` markers to the feature headings in
`docs/ai-blaise/NEW_FEATURES.md`, reject missing, extra, or duplicate headings,
require a status field for every heading, and preserve the production boundary
here without depending on hand-maintained inventory totals.

The promoted feature set is the set of `Status: production-ready` headings with
explicit production evidence in `docs/ai-blaise/NEW_FEATURES.md`; every other
heading remains `Status: alpha`. There are no manual source-only carve-outs:
any new source `FEATURE:` marker must land with a corresponding feature heading
and evidence line before the audit passes. This keeps the catalog auditable, but
alpha contract evidence is not independently sufficient for production signoff.

Worker D CDC/realtime production evidence from 2026-05-23 adds `C1`, `C3`,
`WH3`, `RT1`, `RT2`, `RT3`, `RT4`, and `RT5` to the narrow
production-ready set. The evidence is limited to the CDC/realtime sidecar
runtime boundary: wal2json ingest, pgoutput logical-frame decoder boundary,
checkpoint/ack state, health/readiness/metrics, PII anonymization before sink
encoding, file/in-memory DLQ records, raw WebSocket Phoenix channel join,
presence, tenant/topic filtering, `postgres_changes` fan-out, and CDC-to-realtime
Unix-domain-socket bridging under `cargo test -p ai_blaise_citus_sidecar_cdc`,
`cargo test -p ai_blaise_citus_sidecar_realtime`,
`ci/ai-blaise/sidecar-cdc-smoke.sh`, and
`ci/ai-blaise/sidecar-realtime-smoke.sh`. The realtime claim is bounded to
`runtime_boundary=single-node-raw-ws-cdc-ingest` with
`websocket_network_exercised=true`, `browser_client_exercised=false`,
`cdc_tailing_integrated=false`, `multi_node_pubsub=false`, and
`kubernetes_traffic_exercised=false`; browser client behavior, WebSocket
extension negotiation, live CDC tailing, multi-node pubsub, and Kubernetes
traffic are not promoted by this evidence. External managed broker operations
(NATS auth/TLS/JetStream, GCP Pub/Sub IAM/live publish, Kafka/Kinesis managed
client operation) remain alpha unless covered by their own feature entry.

Worker CDC-Sinks production evidence from 2026-05-24 adds `C14` and `C15` to the narrow production-ready set. The evidenced boundary is strict local NATS subject/server URL and Pub/Sub project/topic validation, deterministic NATS `PUB` and Pub/Sub `messages.publish` frame encoding, serve-runtime/canonical stdout exposure, and DLQ retry accounting for live NATS dispatch failures under `cargo test -p ai_blaise_citus_sidecar_shared -p ai_blaise_citus_sidecar_cdc` and `ci/ai-blaise/sidecar-cdc-smoke.sh`. Managed NATS auth/TLS/JetStream and live GCP Pub/Sub auth/IAM/topic operations remain alpha.
Branch lifecycle evidence for `C6`, `C7`, and `C8` is intentionally alpha. The
VM smoke `ci/ai-blaise/operator-branch-lifecycle-smoke.sh` and focused operator
unit tests verify only deterministic local contracts: branch source/target
validation, conservative storage and snapshot class admission guards,
storage-quantity validation, snapshot-class invariants, apply/suspend/promote
state transition planning, and fail-closed guards for readiness, active sessions,
pending migrations, write quiescence, replication catch-up, and suspend intent.
This is not production evidence for live CSI `VolumeSnapshot` creation, PVC
cloning, Kubernetes cluster materialization, StatefulSet scaling, traffic
cut-over, DNS/Service retargeting, or production branch promotion.




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
fails while any feature heading remains
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
`ci/ai-blaise/live-k8s-e2e.sh` enforces that boundary in strict real modes, and
`ci/ai-blaise/k8s-production-values-live-smoke.sh` proves the same guardrail in
a VM kind deployment with immutable images and live SQL service traffic. A
command-center render that enables an alpha sidecar, uses `latest`, or omits
`@sha256` image pinning is rejected before it can be cited as production
evidence.

The release gate monitor now centralizes the bounded integration contract for
production wording, executable evidence, V2 domain-command freshness,
benchmark Black formatting, image probe coverage, Docker/Postgres readiness,
and parallel matrix monitoring via `gh pr checks`. It is wired into
`gate-close` and the `release-gate-monitor` workflow, while the repository
remains not production-ready as a whole until production-release mode passes.

The Citus patch production integration audit keeps custom patch artifacts
`0004`, `0006`, `0007`, and `0008` explicitly not production-ready until their
measured gates exist. `ci/ai-blaise/citus-patch-production-audit.sh` fails
closed unless each artifact is listed in `patches/series`, future patch roster
entries stay documented as roster-only until artifacts land, and any production
claim has a measured non-scaffold result with thresholds in
`benchmarks/citus-patches/production-gates.json`. This is negative evidence for
the current branch, not a runtime signoff for those patch IDs.
