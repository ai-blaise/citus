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
  smoke, the real Citus+TimescaleDB cohabitation smoke, and the
  primary/standby observability replication smoke. The production gap audit
  rejects regressions that leave a promoted runtime smoke out of those gates.
- The bundled-extension docs and operand-image README now explicitly state that
  `FEATURE: Bundle1` is not production-ready as a whole. The PG17 source-build
  path has targeted live evidence for feasible PGDG-missing extensions, but the
  complete operand initdb contract remains alpha until the plrust upstream PG17
  blocker and full-bundle image smoke are closed. The production gap audit
  rejects the old operand-image overclaim until that full-bundle evidence exists.
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
  remains alpha. The deploy check and production gap audit reject wildcard CRD
  resources or Secret permissions in the operator role.
- Production/default Helm profiles now render only the operator ServiceAccount;
  controller-grade operator ClusterRole and ClusterRoleBinding resources are
  gated behind the explicit alpha `operator.controllerRbac.enabled` flag and
  rendered by `values-exhaustive.yaml` only for non-production contract
  coverage. Deploy checks and the production gap audit reject those RBAC
  resources in default/prod renders.
- The Kubernetes production smoke now runs three live Helm profiles in kind.
  The explicit `values-exhaustive.yaml` image-matrix profile still proves every
  Rust app image can serve probes and pool SQL traffic, the default
  `values.yaml` profile proves direct Helm installs fail closed to the
  production-safe operator/pool surface, and a separate `values-prod.yaml`
  profile proves that production values install with operator/pool replicas, no
  alpha sidecar or tools deployments, no controller-grade operator RBAC,
  monitoring CRDs present, and live SQL through the pool.
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
- The TS6 cohabitation source changes are now integrated into the fork. The
  patch files remain as rebase/reference artifacts, and
  `ci/ai-blaise/patches-check.sh` accepts either clean application to an
  upstream-like tree or clean reverse application when the patch is already
  integrated. `ci/ai-blaise/timescale-cohabitation-smoke.sh` builds this Citus
  fork into `timescale/timescaledb:latest-pg17`, starts PostgreSQL with
  `shared_preload_libraries=timescaledb,citus` and
  `citus.cohabit_extensions=timescaledb`, creates real `citus`,
  `timescaledb`, and `ai_blaise_citus` extensions, verifies real
  `pg_dist_partition` rows, and executes TS1/TS2/TS3/TS4/TS5/TS12 apply
  functions without defining a Citus stub. TS6 and TS18 are therefore
  production-ready narrow surfaces; the broader distributed Timescale feature
  entries remain alpha until multi-worker fanout, rebalance, and operator
  reconciliation are proven end to end.
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
- `TS18` now has real Citus+TimescaleDB cohabitation evidence without a stubbed
  distribution entrypoint. The VM run built
  `ai-blaise-citus-timescale-cohabitation:local` from
  `timescale/timescaledb:latest-pg17`, installed this Citus fork and the
  `ai_blaise_citus` SQL extension, created real `citus`, `timescaledb`, and
  `ai_blaise_citus` extensions, inserted through a real Citus distributed
  table, and then executed the bridge apply functions against the cohabiting
  server. The generated evidence file is
  `artifacts/timescale-cohabitation-evidence.tsv`.
- The D8 deploy wrapper install path is now live-gated: the `values-prod.yaml`
  phase of `kind-production-smoke.sh` installs through
  `scripts/citus-scale/deploy.sh MODE=install` instead of bypassing the wrapper.
  The optional tools Deployment remains dev-only; production evidence executes
  the built `citusctl` image through a smoke Job. The Argo application is a
  GitOps render contract, not live controller evidence.
- The O5 register entry and shared sidecar README now describe only the
  implemented sidecar deployment contract. They explicitly state that tracing
  and OpenTelemetry export, configuration loading, and PostgreSQL connection
  helpers are not implemented, and the production gap audit rejects
  reintroduced claims until real runtime code and live evidence exist.
- The D7 direct Helm install path now fails closed by default. `values.yaml`
  requires immutable operator and pool image digests, disables alpha sidecars,
  disables the optional tools Deployment, and disables alpha runtime/security
  intent. The old exhaustive alpha profile moved to the explicit
  `values-exhaustive.yaml` file, and the kind smoke installs both that explicit
  image-matrix profile and the default production-safe chart profile.
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
- Alpha wording cleanup now also covers the former addendum entries and tool
  READMEs. Schema visualization, plan-freeze, PostgREST, storage, D10, O5, and
  O12 wording uses versioned, operator, release, or measured-evidence language
  instead of stable/live/production phrasing for alpha contracts.
- The production gap audit now explicitly checks that the
  `timescale-cohabitation-smoke` Makefile target runs with `REQUIRE_DOCKER=1`,
  matching the live cohabitation script and GitHub image workflow guardrails.
- The Timescale/Citus cohabitation smoke evidence file now records the Git SHA,
  stable Docker image identity, base image reference, command path, preload
  libraries, and cohabitation allowlist. The TS6 reference patch and docs now
  use that same evidence contract.
- The pool proxy smoke now opens a raw PostgreSQL protocol client through the
  real pool `serve` data port, sends two simple-query frames without waiting
  for the first result, verifies ordered rows from a `postgres:17` backend, and
  keeps the broader shard-aware and `FEATURE: T7` pipeline contract alpha.
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
- The `citus-lsp` CLI now has a direct executable smoke for the narrow
  `FEATURE: D4`, `FEATURE: M5`, and `FEATURE: TS8` file-backed diagnostic
  surface. The smoke runs `citus-lsp analyze --metadata <metadata.tsv> --sql
  <migration.sql>` against a real SQL file and metadata TSV, verifies
  diagnostics and quick-fix actions, verifies distributed hypertable bridge
  suppression, and verifies bad or missing metadata fails closed. This is not
  evidence for JSON-RPC language-server protocol integration, editor
  transport, workspace indexing, automatic file rewrites, or full PostgreSQL
  grammar coverage.
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
- Live default `values.yaml` and explicit `values-prod.yaml` Helm rollouts that
  keep alpha workloads disabled while the production operator and pool
  deployments become available and serve SQL/admin traffic.
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
- Production-ready Timescale/Citus claims require a real cohabitation run; the
  stubbed Timescale bridge smoke remains useful contract evidence, but promoted
  TS6/TS18 evidence must come from the non-stubbed cohabitation smoke.
- Production-ready observability chart claims require parsed Grafana JSON,
  exact panel/PromQL contracts, live installed ConfigMap/PrometheusRule
  resources, and guarded pool error-rate expressions.
- Alpha feature docs, former addendum entries, and tool READMEs must not use
  production-sounding wording for unpromoted contracts; use versioned, runtime,
  tenant-workload, release-hardening, or operator-workflow language until
  measured production evidence supports a status promotion.
- Every custom boundary doc must keep the shared production boundary for
  deterministic contracts, benchmark targets, and local runtime models.
- Pool pipelining production evidence must include a raw PostgreSQL
  wire-protocol smoke that sends multiple simple-query frames through the real
  pool data port before reading the first result; psql request/response pacing
  alone is not sufficient evidence for `FEATURE: T15`.
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
- D2 production evidence is limited to the real `citusctl` CLI apply-mode
  plan-id guard and command-summary smoke. It must not be cited as evidence for
  full mutating apply execution, manifest reconciliation, migrations, backups,
  PITR, WAL replay, or dev cluster lifecycle.
- D4/M5/TS8 production evidence is limited to the file-backed `citus-lsp`
  diagnostic and quick-fix action CLI over supported SQL migration statements.
  It must not be cited as evidence for JSON-RPC editor integration, workspace
  indexing, automatic rewrites, or full PostgreSQL grammar coverage.

## Whole-Repo Production Readiness Audit

The deployment corrections above close the most dangerous false-positive path:
the chart now proves real Rust app images, real pods, sidecar probes, and live
SQL through the pool. The broader repository is still not production-ready as a
whole.

The current feature inventory contains 275 source `FEATURE:` markers and 275
feature headings in `docs/ai-blaise/NEW_FEATURES.md`. 125 narrow headings
are `Status: production-ready`. The authoritative per-feature list is
`docs/ai-blaise/NEW_FEATURES.md`; the examples below group evidence boundaries
rather than duplicating every feature id. The group includes `D7`
for the production-safe default Helm install, `D8` for the production-safe
deploy wrapper, `D13` for the production runtime image matrix, `O4` for the
shared sidecar health/readiness/metrics runtime, `O1` for the installable
`pg_stat_statements` percentile view, `O2` for the installable local activity
stats view, `O3` for the installable replication-lag view against a real
streaming standby, `O6` for the live-installed Grafana dashboard ConfigMap,
`O10` for the live-installed PrometheusRule alert bundle, and `R4` for the
installable idle transaction detection SQL surface, `TS6` for the integrated
trusted hook-coextension source path under real Timescale/Citus cohabitation,
`TS18` for the installable bridge-state SQL surface under real Timescale/Citus
cohabitation, `Sec13` for pool CIDR access control with live allowed and
denied SQL traffic proof, plus `T15` for raw PostgreSQL simple-query
pipelining through the real pool proxy data port, plus `Auth2` for installable
SQL session-claim helpers under a real PostgreSQL extension smoke, plus `D2`
for the real `citusctl` apply-mode plan-id guard, plus `D4`, `M5`, and `TS8`
for the file-backed `citus-lsp` diagnostic and quick-fix CLI, plus `Sec1` for
installable SQL tenant RLS helper predicates, plus `Sec5` and `Sec6` for the
append-only SQL ledger and pgcrypto HMAC seal runtime, plus `Sec2` for the
installable HS256 SQL JWT verifier, plus `S6` and `S13` for installable SQL
placement-generation and shard-index routing helpers, plus `PM3` and `PM4`
for installable SQL plan-freeze and regression-policy helpers, plus `M1`,
`M11`, `IA3`, and `WH2` for installable SQL migration, online type-change,
index-advisor, and webhook trigger queue helpers, plus `Search2`, `Search3`,
`Search9`, `G2`, `G3`, `API4`, `JS2`, `M13`, `Geo2`, and `Geo3` for
installable SQL search, graph, GraphQL metadata, JSON schema, and geo helper
runtimes, plus `A1`, `TS9`, `M7`, `T8`, `L9`, `TS13`, `TS14`, `TS15`, `TS16`,
and `TS17` for installable SQL vectorizer, cohabitation doctor, and Toolkit
aggregate plan helper runtimes, plus `C10`, `M2`, `S14`, `TO3`, `TO4`, and
`TO5` for installable SQL schema-job and tenant lifecycle helper runtimes, plus
`A7`, `A12`, `C11`, `C12`, `C13`, `EF6`, `F2`, `F5`, `G1`, `Geo1`, `IA1`,
`IA2`, `JS1`, `L11`, `M6`, `M10`, `M12`, `MR7`, `O7`, `O8`, `O9`, `O11`,
`O12`, `PM1`, `PM2`, `R6`, `R11`, `Search1`, `Search4`, `Search5`,
`Search6`, `Sec3`, `Sec4`, `Sec10`, `Sec11`, `Sec14`, `Sec15`, and `WF1`
for the installable SQL extension catalog runtime that records required,
optional, integration-target, preload, feature-coverage, and hard-block
extension contracts, plus `S2` for the operator-owned `ShardGroupReconcilePlan`
and `CitusClusterReconcilePlan` plan-builders that render the canonical SQL
apply plan (`set_shard_count`, `set_shard_replication_factor`,
`create_distributed_table`, optional `update_distributed_table_colocation`,
and a `pg_dist_shard` post-condition guard) plus Kubernetes-style
topology-spread constraints and the CloudNativePG cluster manifest from the
canonical `CitusClusterSpec` and `ShardGroupSpec` under `cargo test -p
ai_blaise_citus_operator` and `cargo run -p ai_blaise_citus_operator --
run-reconcile-plans`, while live in-cluster reconciliation (a Kubernetes
controller loop that watches the CRDs and updates `.status`) remains gated
behind the alpha `operator.controllerRbac.enabled` profile because the
operator runtime currently exposes only health/readiness/metrics and
plan-builder helpers, plus `MCP4` for the narrow `tools/citus-mcp` read-only
database execution runtime against real PostgreSQL with native TLS driver
support, read-only transactions, row/timeout bounds, tenant schema denial, and
destructive-tool denial, with `EXPLAIN ANALYZE` rejected so explain requests
do not execute the explained statement. The MCP entries `MCP1`, `MCP2`,
`MCP3`, and `D11` now remain alpha for the broader workflow: they have real
stdio and HTTP JSON-RPC process smokes, obvious cross-schema request denial,
and exhaustive-profile Kubernetes sidecar traffic proof, while `MCP4` covers
only read-only database execution for `tools/citus-mcp`. Authentication,
mutating database execution, Kubernetes tool execution, and production sidecar
enablement remain alpha, and production values keep the MCP sidecar disabled
until those contracts are implemented and live-gated. `TS19` and `TS20` remain
alpha: TS19 has a patch-level clock reservation but no live Citus+pg_cron boot
evidence yet, and TS20 has deterministic companion detection proof but no live
patched-Citus C API caller yet. The other 150 feature headings remain
`Status: alpha`. There are no remaining source-only feature markers: the
former V2 addendum rows were promoted to alpha feature headings with
deterministic executable evidence. This is acceptable for catalog integrity,
but it is not a production claim for the full feature plan.
Every feature heading now has an explicit Executable, CI, Acceptance, SQL
runtime, or SQL extension reference line. Those references are alpha contract
evidence unless the entry is also marked `Status: production-ready`; they keep
the catalog auditable, but they are not independently sufficient for production
signoff.

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
`values-prod.yaml` can carry replica/resource intent for those components, but
`ci/ai-blaise/deploy-check.sh` rejects production values that enable any alpha
sidecar before the corresponding feature is promoted with measured production
evidence.

- Current inventory: contains 273 source `feature:` markers and 273 feature headings; 125 narrow headings are `Status: production-ready`; the other 148 feature headings remain `Status: alpha`.
