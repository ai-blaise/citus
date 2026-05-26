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
  structured-log schema coverage, typed JSON record validation for every runtime
  sidecar, real PostgreSQL ingestion through all generated `sidecar_*_log` typed
  views, pool data/admin port separation, pool live-SQL smoke coverage, and SQL
  bridge-state smoke coverage.
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
  process, GraphQL can execute `graphql.resolve(...)` against live
  `pg_graphql` when `AI_BLAISE_GRAPHQL_LIVE_EXECUTION=1` is set and still
  registers subscription transport state, and edge-functions exposes registry,
  trigger, invocation, and UDS-callback runtime surfaces. The
  `ci/ai-blaise/api-trio-runtime-smoke.sh` boots all three services and verifies
  live TCP readiness plus API-specific behavior. `graphql-pggraphql-live-smoke.sh`
  is the API3 production data-plane proof: it runs a PostgreSQL image with
  `pg_graphql`, creates an RLS-protected `public.account` table, starts the real
  GraphQL sidecar in live execution mode, posts tenant-scoped `/graphql/v1`
  queries, verifies `graphql.resolve(...)` returns only the caller tenant row,
  verifies the opposite tenant row is hidden by PostgreSQL RLS, and checks
  database URL/JWT secret material is not returned. EF1 has separate live
  inline-Deno process evidence through `edge-functions-deno-live-smoke.sh`, EF5
  has sidecar-owned scheduled/CDC trigger dispatch evidence in that same smoke,
  and EF4 has separate live PostgreSQL UDS callback evidence through
  `edge-functions-db-callback-uds-smoke.sh`. Durable GraphQL subscription
  fan-out, multi-worker GraphQL planning, Bun user-code execution,
  queue/broker delivery, live CDC slot tailing, and Kubernetes deployment remain
  outside this proof unless covered by their own feature evidence.
- The bundled-extension docs and operand-image README now explicitly state that
  `FEATURE: Bundle1` is not production-ready as a whole. The PG17 source-build
  path has targeted live evidence for feasible PGDG-missing extensions, and the
  pg_cron cohabitation smoke is production evidence for the TS19 clock-reservation
  path in a real PG17 Citus+pg_cron boot, including SQL-visible reservation,
  scheduled worker execution, and missing-allowlist fail-closed behavior. The
  TS20 role/configuration classifier is now also proved through SQL-visible Citus
  UDFs in that live server. The complete operand initdb contract
  remains alpha until the plrust upstream PG17 blocker and full-bundle image
  smoke are closed. The source-build subset now has a structured lockfile and
  contract checker that cross-validates manifest rows, Dockerfile pins/labels,
  smoke coverage, tracked evidence, and docs; source-build image labels state
  `source-build-subset-no-complete-initdb` so the evidence cannot be mistaken
  for the complete initdb path. The production gap audit rejects the old
  operand-image overclaim until that full-bundle evidence exists.
- Production values now keep alpha runtime/security intent controls disabled by
  default. The deploy check and production gap audit reject production values
  that enable protocol pipelining, PG18 `io_uring`,
  or release attestations before those controls are rendered, enforced, and
  verified end to end. A9 vector-provider secret binding and Sec7/Sec8
  External Secrets and TLS now have a separate live kind proof for controller
  reconciliation, Secret mounts, RBAC denial, and TLS 1.3 mTLS enforcement,
  while cloud provider auth and rotation remain outside the claim.
- Sec13 pool CIDR access control is now enforced by the live pool data path and
  rendered by Helm. The pool rejects PostgreSQL clients outside
  `AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST` before connecting upstream, exposes
  `ai_blaise_citus_pool_rejected_connections_total`, renders a matching
  NetworkPolicy for clusters with NetworkPolicy-capable CNI enforcement, and is
  proven by Docker plus kind smokes that verify allowed and denied SQL traffic.
- Operator RBAC now enumerates the ai-blaise CRD resources instead of using a
  wildcard grant, and runtime ServiceAccounts still receive no Secret API
  grants. A9/Sec7/Sec8 production evidence from
  `ci/ai-blaise/security-external-secrets-tls-live-smoke.sh` installs External
  Secrets Operator chart `0.10.7`, reconciles fake-provider ExternalSecrets
  from deterministic ExternalSecret manifest shape into real Kubernetes Secrets, verifies runtime Secret API reads are denied,
  proves `ai-blaise-vector-provider-openai` stays reference-only while its
  reconciled API key is hashed into the evidence file,
  mounts TLS Secret-reference material into pods, proves TLS 1.3 mTLS success, and proves
  no-client-cert and TLS 1.2 clients fail. Cloud provider authentication,
  cert-manager integration, production rotation SLOs, service-mesh policy, and
  every application protocol path remain outside the Sec7/Sec8 claim. The
  security supply-chain smoke validates the narrow SBOM/cosign metadata contract
  for digest-pinned fixture image refs, `.spdx.json` SBOM paths,
  `.sigstore.json` cosign bundles, SLSA provenance predicate metadata, and
  mutable-image/malformed-SBOM fail-closed behavior. Sec9 release-artifact
  readiness for the registry-backed
  generation/sign/verify flow is now proven by
  `ci/ai-blaise/security-sbom-cosign-live-smoke.sh`: it starts a local OCI
  registry, pushes a digest-pinned ai-blaise Citus image, generates an SPDX 2.3
  SBOM with Syft, signs and verifies the image digest with Cosign, verifies SPDX
  and SLSA provenance attestations, and verifies the `.sigstore.json` SBOM
  bundle. Kubernetes admission enforcement, public registry publication,
  keyless transparency-log policy, cert-manager issuance, and External Secrets
  controller reconciliation remain separate alpha boundaries. The deploy check
  and production gap audit reject wildcard CRD resources or Secret permissions
  in the operator role and require both security supply-chain gates.
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
  fork into `timescale/timescaledb-ha:pg17-ts2.27`, starts PostgreSQL with
  `shared_preload_libraries=timescaledb,citus` and
  `citus.cohabit_extensions=timescaledb`, creates real `citus`,
  `timescaledb`, and `ai_blaise_citus` extensions, verifies real
  `pg_dist_partition` rows, records observed PostgreSQL/TimescaleDB/Citus
  versions, and executes TS1/TS2/TS3/TS4/TS5/TS12 apply functions without
  defining a Citus stub. The evidence is explicitly scoped as
  `entrypoints-and-catalog-state-only`: TS6 and TS18 are therefore
  production-ready narrow startup/load/apply guard surfaces. TS7 now has live
  Kubernetes controller/status reconciliation evidence through
  `ci/ai-blaise/operator-hypertable-live-smoke.sh` for that same bounded bridge
  surface. The broader distributed Timescale feature entries remain alpha until
  multi-worker fanout, background policy execution, continuous aggregate
  refresh, and rebalance behavior are proven end to end.
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
- `TS19` is production-ready for the bounded pg_cron clock-reservation path.
  The smoke builds a PG17 image with PGDG `pg_cron`, this Citus fork, and
  `ai_blaise_citus`, boots with `shared_preload_libraries=pg_cron,citus` and
  `citus.cohabit_extensions=pg_cron`, creates real extensions, verifies
  `pg_catalog.citus_cohabit_clock_tick_reserved()` is true, waits for a scheduled
  pg_cron worker to insert evidence rows using `citus_get_node_clock()`, records
  `artifacts/pg-cron-cohabitation-evidence.tsv`, and verifies the missing-
  allowlist path leaves the reservation false and fails closed. This does not
  make `pg_cron` a trusted hook-chain coextension, and does not make broad
  Bundle1 cohabitation production-ready. The same smoke now separately proves
  the TS20 SQL-visible C API role/configuration classifier boundary by recording
  `citus_cohabit_pg_cron_role`, `citus_cohabit_pg_cron_configured`,
  `citus_cohabit_timescaledb_role`, `citus_cohabit_pg_partman_role`,
  `citus_cohabit_unknown_role`, and the negative configured=false path. TS20
  SQL-visible C API proof remains limited to role/configuration classification.
- `TS18` now has real Citus+TimescaleDB cohabitation evidence without a stubbed
  distribution entrypoint. The VM run built
  `ai-blaise-citus-timescale-cohabitation:local` from
  `timescale/timescaledb-ha:pg17-ts2.27`, installed this Citus fork and the
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
- The O5 register entry and shared sidecar README now promote only the
  bounded sidecar controller apply path after live kind evidence.
  `ci/ai-blaise/sidecar-controller-live-smoke.sh` builds real operator and
  realtime sidecar containers, pushes digest-pinned images to a local registry,
  applies the generated Sidecar CRD, runs the operator in apply mode with
  scoped RBAC, creates the generated Deployment and Service, patches
  `sidecars/status`, serves `/healthz`, `/readyz`, and `/metrics` through the
  generated Service, and rejects a mutable image tag before Deployment
  creation. Tracing/OpenTelemetry export, configuration loading, PostgreSQL
  helpers, autoscaling/rollout policy, and all sidecar app semantics beyond the
  realtime probe container remain outside the O5 production claim.
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
- A2/A3/A4/A5/A6/A8 vectorizer production-ready evidence is bounded to the
  local Rust sidecar runtime: mock-provider queue processing, explicit
  live-provider opt-in policy, CRD-derived provider/model/dimension contract
  enforcement, fail-closed manual request validation, PostgreSQL-backed queue,
  budget and usage tables, health/readiness/drain endpoints, metrics, and the
  Docker PostgreSQL smoke. It does not claim real external embedding-provider
  calls, GPU inference, production-scale queue throughput, tenant billing
  integration, Kubernetes admission/webhook enforcement for every provider
  model, or broad semantic-search correctness.
- O14 trace-context propagation is now production-ready for the bounded
  pool-to-PostgreSQL-to-companion SQL path and sidecar HTTP ingress visibility.
  `otel-trace-propagation-smoke.sh` installs `ai_blaise_citus` in a real
  PostgreSQL container, routes libpq traffic through the pool, verifies
  `trace.parent`, `companion.current_traceparent`,
  `companion.project_traceparent_from_application_name(...)`, pool tap metrics,
  absent-trace counters, and the live shared sidecar `/tracez` endpoint. The
  optional kind/Jaeger mode remains a synthetic correlation harness; this does
  not claim automatic OTLP span export from every component or dashboard/SLO
  certification.
- D9 canary upgrade runbook is now production-ready for the local companion SQL
  extension upgrade/rollback path. `canary-upgrade-rollback-smoke.sh` starts a
  real PostgreSQL container, installs `ai_blaise_citus` at `0.1.0`, upgrades to
  `0.1.1`, records `companion_internal.extension_upgrade_events`, rolls back to
  `0.1.0`, and proves the 0.1.1 event table and recorder are absent after
  rollback. `upgrade-rollback-guardrails.sh` keeps the reverse manifest row,
  Dockerfile packaging, runbook, release docs, Make target, and workflow wiring
  fail-closed. This does not claim full upstream Citus upgrade-matrix evidence,
  operand image release certification, or human production promotion.
- D10 release hardening is now production-ready for the fail-closed runbook and
  release-record contract. `release-hardening-runbook-smoke.sh` executes the
  companion `run-release-hardening-canonical` report, verifies 19 release gates
  and 10 required release-record fields, reruns runbook command and docs
  evidence checks, requires `production-readiness-check.sh production-release`
  to block while alpha features remain, verifies D10 is not itself a blocker,
  and renders a release record with source revision, digest-manifest
  requirement, audit/check status, alpha scope, rollback checkpoint requirement,
  and owner signoff requirement. This does not certify a release candidate,
  perform human owner signoff, or execute the separate D9 canary
  upgrade/rollback drill.
- Alpha wording cleanup now also covers the former addendum entries and tool
  READMEs. Schema visualization, plan-freeze, PostgREST, storage, and O12
  wording uses versioned, operator, release, or measured-evidence language
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
  `timescale/timescaledb-ha:pg17-ts2.28`,
  `timescale/timescaledb-ha:pg17-ts2.28.0`, or
  `timescale/timescaledb-ha:pg17-ts2.28.1` image. This does not promote
  TS 2.28 to production-ready; a published 2.28 image fails the matrix until
  any `unknown` hook rows are measured and updated.
- The pool proxy smoke now opens a raw PostgreSQL protocol client through the
  real pool `serve` data port, sends two simple-query frames without waiting
  for the first result, verifies ordered rows from a `postgres:17` backend, and
  promotes only the `FEATURE: T7` simple-query data-plane pipelining boundary;
  extended-query batching and broader shard-aware pool routing remain alpha.

- The pool routing/security canonical smoke covers bounded production-ready
  contracts for T9/T12/R10, and MR5 now has a bounded live data-plane proof in
  `ci/ai-blaise/pool-geoip-live-smoke.sh`: two real `postgres:17-bookworm`
  regional replicas, static `AI_BLAISE_POOL_GEO_*` configuration,
  `geoip_pool_route_selected_region=us-east-1`, default-region fallback,
  `ai_blaise_citus_pool_geo_routes_total`,
  `ai_blaise_citus_pool_geo_fallback_routes_total`, and invalid-CIDR
  fail-closed startup. This is still not evidence for live canary mirroring,
  managed GeoIP databases, Region-CR synchronization, hot-swap reloads,
  rustls listener/session-resumption traffic, analytical sidecar query
  execution, cross-region/WAN behavior, edge-replica traffic, or Kubernetes
  traffic.
- R1/R5/R9/Search8 cold-tier local file materialization is production-ready for
  the bounded sidecar local `file://` runtime. `sidecar-coldtier-runtime-smoke.sh`
  runs the real `ai_blaise_citus_sidecar_coldtier` binary through
  `run-runtime-canonical` and `run-local-file-materialization-canonical`, writes
  four deterministic artifacts under `/tmp/ai-blaise-coldtier`, and verifies
  `coldtier_local_file_materialization=passed`, `local_file_materialized=true`,
  `materialized_artifact_count=4`, `materialized_bytes=1408`,
  `materialized_layer_files=2`, `search_indexes_materialized=2`,
  `planner_routes_refreshed=1`, `cold_tier_reads=1`,
  `object_store_io_attempted=false`, and `citus_cold_read_serving=false`. This
  is not evidence for S3/GCS/Azure object-store writes, pageserver deployment,
  Citus cold-read serving, distributed query planner integration,
  operator/Kubernetes scheduling, production object-store lifecycle, or real
  Tantivy/LanceDB query execution.
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
  state-file-only cleanup guardrails. `citusctl-k8s-apply-live-smoke.sh` now
  proves the M8 Kubernetes manifest plan/apply path against a live kind
  cluster: real `kubectl apply --dry-run=server`, deterministic `k8s-apply-*`
  plan id, apply-time plan-id match rejection, real `kubectl apply`, resource
  verification with `kubectl get -f`, idempotent reapply evidence, malformed
  manifest rejection, and `k8s-manifest-apply.audit.tsv` append evidence. This
  is not evidence for Docker/kind lifecycle orchestration by the CLI,
  migrations, backups, PITR, WAL replay, multi-step Citus data-plane rollout
  semantics, or production cluster lifecycle management beyond applying the
  supplied manifest.
- B5 time-travel intent now has a real `citusctl` binary smoke for the bounded
  validation surface. `citusctl-time-travel-intent-smoke.sh` proves strict
  RFC3339 UTC timestamp parsing, calendar validation, ahead-of-now target
  rejection,
  explicit `--max-staleness-seconds` enforcement, deterministic `time-travel-*`
  plan ids, apply-time plan-id match rejection, TSV/JSON output, and
  `time-travel-intent.audit.tsv` append evidence. This is not evidence for
  follower reads, backup-backed query replay, closed-timestamp MVCC reads,
  Citus executor integration, or production query execution.
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
- S5 Raft per shard group is production-ready for the bounded sidecar
  consensus/transport component. `sidecar-raft-smoke.sh` proves deterministic
  election, AppendEntries replication, quorum commit, durable log replay,
  snapshot watermarking, and live multi-process HTTP transport by starting
  three separate `ai_blaise_citus_sidecar_raft serve` OS processes on loopback
  ports, electing `worker-a` through `/raft/campaign`, committing
  `networked-placement-intent` through `/raft/propose`, verifying all voters
  report the same leader/term/commit index/last-log index/payload through
  `/raft/status`, and proving follower proposals plus malformed
  `/raft/message` bodies fail closed. This does not claim operator-driven
  membership changes, CNPG failover execution, Citus placement synchronization,
  WAN latency/partition behavior, or Kubernetes reconciliation.
- S9 closed-timestamp follower-read gating is production-ready for the bounded
  HLC sidecar gate. `sidecar-hlc-smoke.sh` starts the real
  `ai_blaise_citus_sidecar_hlc serve` process, waits for `/readyz`, verifies
  `/closed_ts`, advances the local clock through `/clock/tick`, merges peer
  clock evidence through `/clock/observe`, confirms the peer appears in the
  published closed timestamp, proves `/follower_read` serves an `AS OF` exactly
  at the closed timestamp, proves an `AS OF` newer than the closed timestamp is
  rejected with HTTP 409, and verifies unknown peers fail closed. This does not
  claim MVCC snapshot execution, replica query routing, planner integration,
  stale-read SQL syntax, cross-region clock discipline, or Kubernetes
  reconciliation.
- MR6 closed-timestamp time-travel gate is production-ready for the same bounded
  live HLC data plane: timestamp/staleness intent validation, `/closed_ts`
  publication, `/clock/tick` local advancement, `/clock/observe` peer evidence,
  exact-closed `AS OF` follower-read serving, newer-than-closed HTTP 409
  rejection, and unknown-peer fail-closed behavior. The smoke emits
  `closed_timestamp_time_travel_gate=passed`,
  `follower_read_as_of_closed_served=true`,
  `follower_read_newer_than_closed_rejected=true`, and
  `closed_ts_peer_exchange_observed=true`. This does not claim MVCC snapshot
  execution, replica query routing, stale-read SQL syntax, planner integration,
  cross-region clock discipline, or Kubernetes reconciliation.
- Edge1 bounded-staleness edge read gating is production-ready for the bounded
  HLC sidecar admission-control surface. `sidecar-hlc-smoke.sh` starts the real
  `ai_blaise_citus_sidecar_hlc serve` process with
  `AI_BLAISE_HLC_EDGE_REPLICAS`, waits for `/readyz`, verifies `/closed_ts`,
  advances `/clock/tick`, observes peer evidence through `/clock/observe`, and
  exercises `/edge_read`. The live smoke proves exact-closed edge read serving,
  newer-than-closed HTTP 409 rejection, too-stale HTTP 409 rejection,
  replica/edge-region mismatch HTTP 409 rejection, and unknown-edge-region HTTP
  409 rejection. The smoke emits `edge_bounded_staleness_gate=passed`,
  `edge_read_as_of_closed_served=true`,
  `edge_read_newer_than_closed_rejected=true`,
  `edge_read_too_stale_rejected=true`,
  `edge_read_replica_mismatch_rejected=true`, and
  `edge_unknown_region_rejected=true`. This does not claim edge replica
  provisioning, POP/WAN network deployment, SQL/MVCC snapshot execution,
  planner integration, data-plane query routing, failover automation, or
  Kubernetes traffic.
- S3 clone-node fast scale-out is production-ready for the bounded live Citus
  physical-replica clone promotion path. `clone-node-live-smoke.sh` starts a
  real Citus coordinator and primary worker, creates distributed
  `public.s3_orders`, bootstraps a PostgreSQL physical streaming replica clone
  with `pg_basebackup`, verifies the clone is in recovery, executes
  `citus_add_clone_node` and `citus_promote_clone_and_rebalance` through
  companion-rendered SQL, waits for Citus catch-up and `pg_promote`, and proves
  `clone_rows_preserved=20`, `clone_sum_preserved=5060`,
  `clone_role_after_promote=primary`, `clone_active_after_promote=true`,
  `clone_should_have_shards_after_promote=true`,
  `clone_shard_placements_after=2`, and `primary_shard_placements_after=2`.
  This does not claim Kubernetes clone orchestration, CSI snapshot based
  cloning, automatic capacity policy, WAN/cross-region clone operation,
  service/DNS retargeting, or production traffic cutover.
- MR3 regional row placement is production-ready for the bounded live
  multi-worker Citus explicit-key placement path. `regional-placement-live-smoke.sh`
  now preserves the S8/S12 catalog/tablespace phase and adds a second phase that
  starts a real coordinator plus `us-east-1` and `eu-west-1` workers, creates
  `public.mr3_orders`, isolates `us-east-1:tenant-a` and `eu-west-1:tenant-b`
  with `isolate_tenant_to_new_shard`, moves the EU shard with
  `citus_move_shard_placement`, and proves `mr3_shards_isolated=true`,
  `mr3_citus_move_shard_placement_executed=true`,
  `mr3_worker_placement_enforced=true`, `mr3_matched_region_count=2`, and
  `mr3_rows_preserved=true`. This does not claim WAN/multi-region network
  execution, Kubernetes operator reconciliation, automatic repartition
  scheduling, regional traffic routing, GeoIP routing, or regional failover;
  MR9 remains alpha for survival drills.
- T5 parallel commit transaction status is production-ready for the bounded
  networked transaction-status sidecar API and SQL contract.
  `parallel-commits-smoke.sh` proves staging, finalize, and modeled fast-path
  step count; `schema-txn-runtime-smoke.sh` drives the real txn-status HTTP
  server through stage -> wait -> ack -> commit with malformed/unknown-field
  rejection; `sql-extension-smoke.sh` installs `companion.txn_stage`/
  `companion.txn_finalize` into real PostgreSQL; and
  `txn-status-networked-raft-smoke.sh` starts three separate
  `ai_blaise_citus_sidecar_raft serve` OS processes plus a real
  `ai_blaise_citus_sidecar_txn_status serve` process configured with
  `AI_BLAISE_TXN_RAFT_LEADER_ADDR`, elects `worker-a`, proves
  `stage:txn-live-raft-1:worker-a` is committed through the Raft log before
  the staged transaction is returned, proves wait decisions do not append
  terminal log entries, proves `commit:txn-live-raft-1` is committed before the
  sidecar reports committed, verifies every voter reaches commit index 2, and
  proves follower-backed replication failures fail closed without
  materialising a transaction record. This does not claim Citus distributed
  executor integration, PostgreSQL-core commit timestamp patch integration, or
  Kubernetes operator reconciliation.
- The broader Raft/HLC/transaction-status triad still has executable sidecar
  runtime evidence without overclaiming full distributed-database integration:
  `topology-consensus-smoke.sh` proves S4 coordinator-less pool admission, S5
  fail-closed placement/member validation, and S9/MR6 closed-timestamp
  follower-read serve/reject gates. S4 coordinator-less topology mode is
  production-ready only for the bounded Citus MX worker-entry and pool-entry
  smoke in `ci/ai-blaise/coordinatorless-mx-live-smoke.sh`: a real
  three-node Citus topology, `start_metadata_sync_to_node`, worker-side
  `Custom Scan (Citus Adaptive)` with `Task Count: 1`, `worker_entry_sum=550`,
  and a real pool proxy pointed at the metadata-synced worker returning
  `pool_worker_entry_sum=550`. This does not claim coordinator bootstrap
  removal, does not claim dynamic shard-aware pool routing, does not claim
  multi-shard plan-leader execution, does not claim Kubernetes reconciliation,
  and does not claim WAN or cross-region behavior. MR6 is promoted only for
  the live closed-timestamp time-travel gate and not for SQL/MVCC execution.
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
  missing-claim errors, introspection denial, subscription-boundary responses,
  and the bounded API6 OpenAPI document path. `postgrest-live-data-plane-smoke.sh`
  is the production data-plane proof for API1/API2/API5: it runs a Citus-capable
  PostgreSQL container, creates the real `citus` extension, distributes
  `public.orders`, asserts `pg_dist_partition`, creates the security-invoker
  `api.orders` view, launches the official PostgREST 12.2.12 binary through the
  sidecar `run-live-postgrest` supervisor, starts the sidecar proxy with
  `AI_BLAISE_POSTGREST_UPSTREAM`, and verifies authenticated GET/POST traffic plus
  tenant RLS isolation and secret non-disclosure end to end.
  `graphql-pggraphql-live-smoke.sh` is the matching production data-plane proof
  for API3 live `pg_graphql` query execution and tenant RLS through the GraphQL
  sidecar. `edge-functions-deno-live-smoke.sh` is the EF1 and EF5 production
  proof for explicit opt-in inline Deno execution and sidecar-owned trigger
  dispatch: it boots the real sidecar, verifies live mode fails closed unless
  `AI_BLAISE_EDGE_RUNTIME_EXECUTION=1` and `AI_BLAISE_DENO_BIN` are supplied,
  executes inline user code in a real Deno process, checks `status=executed`,
  `execution_mode=live`, `user_code_executed=true`, and
  `runtime_response_json`, proves default environment access is denied by Deno
  permissions, verifies runtime timeout requests return HTTP 504, rejects
  excessive runtime stdout, and dispatches scheduled plus CDC trigger events
  through `/triggers/scheduled` and `/triggers/cdc` into live Deno functions.
  `edge-functions-bun-live-smoke.sh` is the EF2 production Bun proof: it runs the
  same sidecar with `AI_BLAISE_EDGE_RUNTIME_EXECUTION=1` and `AI_BLAISE_BUN_BIN`,
  executes inline Bun user code, verifies `runtime_env_cleared=true`, timeout and
  stdout guards, and dispatches scheduled plus CDC trigger events into live Bun
  functions. `edge-functions-db-callback-uds-smoke.sh` is the EF4 production data-plane
  proof: it runs a real `postgres:17`
  container with a mounted `.s.PGSQL.5432` socket, enables
  `AI_BLAISE_EDGE_DB_CALLBACK_EXECUTION=1`, registers a callback-enabled
  function, proves disabled execution and unsafe multi-statement SQL fail
  closed, executes one insert through the PostgreSQL Unix socket, and verifies
  the inserted row plus `db_callback_rows=1`. Bun DB-callback integration,
  user-code initiated callback RPC, queue/broker delivery, live CDC slot
  tailing, distributed trigger fan-out, durable retry/DLQ, package
  installation, non-inline source fetching, and Kubernetes deployment remain
  alpha-scoped until those live data-plane paths are proven.
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
- The T2 placement-generation UDF contract now exposes
  `pg_catalog.citus_placement_generation()` in fresh-install and 15.0 upgrade SQL,
  keeps versioned UDF snapshots in sync, guards the companion query and upstream
  patch artifact with `ci/ai-blaise/placement-generation-udf-contract-smoke.sh`,
  and closes the live patched-Citus runtime boundary in
  `ci/ai-blaise/pg-cron-cohabitation-smoke.sh`. That live PG17 smoke creates two
  real distributed tables, records `placement_generation_initial`,
  `placement_generation_after_first_distribution`,
  `placement_generation_after_second_distribution`, and
  `placement_generation_placements`, asserts monotonic counter advancement from
  the installed `pg_catalog.citus_placement_generation()` UDF while Citus creates
  placement metadata, and proves `GUC_REPORT`/ParameterStatus emission with
  `citus_shard_count_parameter_status` after `SET citus.shard_count TO 7`. This
  makes T2 production-ready for the bounded Citus patch surface; it still does
  not claim production latency, rebalance throughput, or unpublished pool
  data-plane serving traffic under real tenant load.
- The Citus quilt now promotes the bounded `FEATURE: T3` and `FEATURE: T4`
  patch surfaces with live VM evidence. `ci/ai-blaise/router-patch-smoke.sh`
  verifies patch applicability against upstream `release-14.0`, builds the
  integrated Citus source, boots a PG17 Docker runtime from this fork, proves the
  SQL-registered fast-path-router locality probe against a real single-shard
  distributed table, and writes measured non-scaffold results to
  `benchmarks/citus-patches/results/0004-router-planner-hotpath.json` and
  `benchmarks/citus-patches/results/0006-fast-path-router-skip.json`. The claim
  is intentionally bounded to source-integrated T3/T4 patch behavior, SQL-visible
  locality probing, and local planner-hot-path measurement; broad multi-region
  coordinator-less serving and fleet planner latency remain separate release
  performance work.
- The companion advanced-planner runtime smoke expands `FEATURE: T4`, the
  adjacent advanced-planner contract set, and the T10/T11/T13/T14 contract
  budgets into deterministic runtime-boundary scenarios. It still verifies
  duplicate-feature rejection, unknown-scenario rejection, invalid budget
  rejection, and live-distributed-execution overclaim rejection through
  `ci/ai-blaise/companion-advanced-planner-smoke.sh`. `FEATURE: T10` and
  `FEATURE: T11` now also have bounded live Citus SQL evidence in
  `ci/ai-blaise/bulk-distsql-live-smoke.sh`: a real distributed table, the
  companion-rendered `FETCH 4096`, `bulk_fetch_rows_returned=4096`,
  `Custom Scan (Citus Adaptive)`, `citus_adaptive_plan_observed=true`,
  `citus_task_count_observed=1`, and `worker_task_budget=16`. This is not
  evidence for a custom PostgreSQL wire-protocol implementation, adaptive
  backpressure, an optimizer rewrite engine, worker-plan injection,
  multi-worker fanout, distributed cursor/savepoint cleanup, or Kubernetes
  traffic.
- `FEATURE: TS10` and `FEATURE: TS11` now have bounded live Citus+Timescale
  evidence in `ci/ai-blaise/timescale-advanced-live-smoke.sh`: the real
  cohabitation image creates a Citus-distributed hypertable, builds and
  refreshes a two-level continuous aggregate hierarchy, observes
  `hierarchical_cagg_count=2` and `hierarchical_cagg_daily_rows=4`, sets
  `compression_segmentby_columns=2`, and materializes
  `segmentby_bloom_rows=16` companion bloom rows with bit/hash parameters
  `2048:3`. This is not evidence for native Timescale bloom filters, planner
  integration, compressed-chunk scan pruning, multi-worker fanout, automated
  refresh scheduling, false-positive-rate calibration, or Kubernetes traffic.
- `FEATURE: S1` now has bounded live Citus shard-split evidence in
  `ci/ai-blaise/shard-split-live-smoke.sh`: a real Citus server with
  `wal_level=logical`, a distributed `public.s1_orders` table, the
  companion-rendered `isolate_tenant_to_new_shard` call, shard-count growth
  `split_shard_count_before=4` to `split_shard_count_after=6`,
  `split_tenant_rows_preserved=10`, `split_tenant_shard_changed=true`, and
  `split_isolated_range_exact=true`. This is not evidence for an automated
  policy scheduler, threshold telemetry, rollback automation, multi-node
  movement, autonomous rebalancing, cross-table cascade coverage beyond the
  tested Citus call, or Kubernetes traffic.
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
- D1/M8 evidence now has two production-ready real-binary paths. The D1 dev
  lifecycle path is limited to local state-file runtime behind explicit
  `--state-dir` invocations: dry-run plan rendering, stable plan-id validation,
  deterministic JSON/TSV output, idempotent up/down state transitions, local
  audit append, and state-file-only cleanup. The M8 Kubernetes manifest path is
  limited to `citusctl plan/apply apply <manifest> --namespace ... --state-dir
  ... --format json|tsv`: server-side dry-run, deterministic plan id,
  apply-time plan-id match guard, real `kubectl apply`, `kubectl get -f`
  verification, and `k8s-manifest-apply.audit.tsv` append evidence. This
  evidence must not be cited for Docker/kind startup performed by the CLI,
  migrations, backups, PITR, WAL replay, Postgres/Citus data-plane health,
  extension-service orchestration, or production cluster lifecycle management
  beyond applying the supplied manifest.
- B5 time-travel intent evidence is limited to the real `citusctl` CLI
  validation and audit path:
  `citusctl plan/apply time-travel <target_time> --now ... --max-staleness-seconds
  ... --state-dir ... --format json|tsv`. It proves strict UTC calendar
  validation, stale/ahead-of-now rejection, plan-id-gated apply, and
  `time-travel-intent.audit.tsv` append evidence only. It must not be cited for
  follower-read execution, backup-backed query replay, closed-timestamp MVCC
  reads, Citus executor integration, or production query execution.
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
- R7 repack hardening now adds fail-closed strategy selection, a deterministic
  dry-run execution report for `sidecar/repack`, and a sidecar-owned live
  `pg_repack` command. The Docker VM smoke records `dry_run=false`,
  `executed=true`, and `evidence_boundary=live-pg-repack-execution` after
  executing `pg_repack` against a real PostgreSQL 17 table. This is production
  evidence for a single local PostgreSQL target plus operator plan rendering. It
  must not be cited as production evidence for PostgreSQL 19 `REPACK
  CONCURRENTLY`, Kubernetes-scheduled repack execution, or Citus shard fanout
  across workers.
- Analytical/lakehouse hardening now includes bounded live local DataFusion
  execution for `FEATURE: L2` and `FEATURE: L4`. The sidecar analytical smoke
  runs a real in-process DataFusion query over an Arrow `RecordBatch`, records
  `query_engine_executed=true`, `datafusion_output_rows=2`,
  `datafusion_output_total=3000`, `projection_pushdown_executed=true`,
  `filter_pushdown_executed=true`, `limit_pushdown_executed=true`, and
  `evidence_boundary=local-datafusion-recordbatch-only`, while also verifying
  the analytical loopback probe server health/readiness/metrics/drain path.
  This remains `external_io_attempted=false` and must not be cited as
  production evidence for pg_lake, object-store IO, Iceberg/Parquet/Delta file
  reads beyond the separately promoted local Parquet file path, Iceberg commits,
  DuckDB, MotherDuck, Citus planner integration, Kubernetes traffic, or
  benchmarked analytical performance.
- `FEATURE: L3` now has bounded local Parquet read evidence.
  `ci/ai-blaise/sidecar-analytical-parquet-read-smoke.sh` runs
  `run-local-parquet-read-canonical`, writes a real local Parquet file with
  `ArrowWriter`, registers it through DataFusion `ParquetReadOptions`, and
  queries the file with projection, filter, ordering, and limit. The smoke
  requires `parquet_lakehouse_read_live=passed`,
  `l3_local_parquet_file_created=true`,
  `l3_datafusion_parquet_read_executed=true`, `l3_source_rows=4`,
  `l3_source_total=5500`, `l3_datafusion_output_rows=2`,
  `l3_datafusion_output_total=3000`, and
  `local-datafusion-parquet-file-only`. The claim is bounded to local Parquet
  file materialization and local DataFusion Parquet reads. It is not evidence
  for Iceberg runtime reads, Delta runtime reads, object-store IO, pg_lake,
  MotherDuck, Citus planner integration, warehouse federation, or Kubernetes
  traffic; the smoke requires `object_store_io_attempted=false`,
  `iceberg_runtime_exercised=false`, `delta_runtime_exercised=false`,
  `pg_lake_runtime_exercised=false`, `motherduck_session_exercised=false`, and
  `kubernetes_traffic_exercised=false`.
- `FEATURE: L5` now has bounded local Iceberg-style snapshot commit evidence.
  `ci/ai-blaise/sidecar-analytical-iceberg-snapshot-smoke.sh` runs
  `run-local-iceberg-snapshot-commit-canonical`, writes a local manifest JSON,
  a local metadata JSON, and a `current-snapshot.txt` pointer using temp-file
  plus atomic rename and fsync, then reads the artifacts back. The smoke
  requires `iceberg_snapshot_commit_live=passed`,
  `l5_local_metadata_written=true`, `l5_local_manifest_written=true`,
  `l5_current_pointer_committed=true`, `l5_prepare_lsn_recorded=true`,
  `l5_snapshot_metadata_round_tripped=true`, `atomic_rename_used=true`,
  `fsync_executed=true`, and
  `local-iceberg-snapshot-metadata-commit-only`. The claim is bounded to local
  prepare-LSN metadata commit artifacts. It is not evidence for live Iceberg
  catalog commits, object-store IO, a Citus prepare hook, multi-writer conflict
  detection, warehouse federation, or Kubernetes traffic; the smoke requires
  `iceberg_catalog_commit_exercised=false`, `object_store_io_attempted=false`,
  `citus_prepare_hook_exercised=false`,
  `multi_writer_conflict_detection_exercised=false`,
  `warehouse_federation_exercised=false`, and `kubernetes_traffic_exercised=false`.
- `FEATURE: L7`, `FEATURE: R3`, and `FEATURE: R8` now have bounded live
  Citus columnar evidence. `ci/ai-blaise/columnar-tiering-live-smoke.sh` starts
  a real Citus coordinator and worker, installs `citus_columnar`, creates
  `public.columnar_orders` with `USING columnar`, distributes it with
  `create_distributed_table('public.columnar_orders', 'tenant_id', shard_count => 4)`,
  inserts 12 rows totaling 3024, and executes the companion-rendered
  `run-columnar-tiering-sql-canonical` guard. The smoke also checks a real
  `EXPLAIN` for Citus adaptive execution plus `ColumnarScan` and connects
  directly to the worker to require `r3_worker_access_method=columnar` and
  preserved worker rows. The required live markers are
  `columnar_tiering_live=passed`, `l7_distributed_columnar_table=true`,
  `l7_columnar_access_method=true`, `l7_columnar_query_rows=12`,
  `l7_columnar_query_total=3024`, `l7_citus_custom_scan_executed=true`,
  `l7_columnar_scan_executed=true`, `r3_worker_columnstore_policy_live=true`,
  `r3_worker_access_method=columnar`, and
  `r8_non_hypertable_cold_columnar_path=true`. The claim is bounded to live
  Citus columnar table creation, distributed read execution, worker-local
  columnar verification, and non-hypertable catalog checks. It is not evidence
  for cost-model tier selection, automatic tier movement, workload-routing
  rewrites, background schedulers, object-store cold reads, hypertable
  conversion, or Kubernetes traffic; the smoke requires
  `cost_model_selection_exercised=false`,
  `automatic_tier_movement_executed=false`, `workload_routing_exercised=false`,
  and `kubernetes_traffic_exercised=false`.
- `FEATURE: L10` now has bounded live Citus cross-tier query evidence.
  `ci/ai-blaise/cross-tier-query-live-smoke.sh` starts a real Citus
  coordinator and worker, installs `citus_columnar`, creates one distributed hot
  row table and two distributed columnar tables (`public.l10_warm_orders` and
  `public.l10_cold_orders`), inserts deterministic rows, executes the
  companion-rendered `run-cross-tier-query-sql-canonical` guard, and checks a
  real `EXPLAIN` for Citus adaptive execution plus warm/cold `ColumnarScan`
  nodes. Required markers include `cross_tier_query_live=passed`,
  `l10_hot_tier_rows=4`, `l10_warm_tier_rows=4`, `l10_cold_tier_rows=4`,
  `l10_cross_tier_rows=12`, `l10_cross_tier_total=6678`,
  `l10_citus_custom_scan_executed=true`, and
  `l10_columnar_scan_executed=true`. The production-ready claim is bounded to
  companion-rendered read-only `UNION ALL` composition and rollup preservation
  over live distributed row plus columnar tables. It is not evidence for
  automatic workload routing, automatic arbitrary-SQL query rewrites,
  cost-model tier selection, object-store cold reads, background tier movement,
  or Kubernetes traffic; the smoke requires
  `automatic_workload_routing_exercised=false`,
  `automatic_query_rewrite_exercised=false`,
  `cost_model_selection_exercised=false`,
  `object_store_cold_read_exercised=false`, and
  `kubernetes_traffic_exercised=false`.
- `FEATURE: L8` now has separate bounded live logical-replication mirror
  materialization evidence. `ci/ai-blaise/sidecar-analytical-mirror-live-smoke.sh`
  starts PostgreSQL 17 with `wal_level=logical`, creates a `test_decoding` slot,
  inserts rows into `public.l8_orders`, consumes `pg_logical_slot_get_changes`,
  runs `run-logical-mirror-materialization-from-stdin`, writes a local TSV mirror
  artifact, and verifies `logical_mirror_live=passed`,
  `l8_test_decoding_slot_consumed=true`, `l8_materialized_rows=3`,
  `l8_materialized_total=6000`, and `l8_datafusion_mirror_query_executed=true`.
  The claim is bounded to local live logical decoding plus local TSV artifact
  materialization and DataFusion `.tsv` reads:
  `object_store_io_attempted=false`,
  `long_running_slot_tailing=false`, `checkpoint_persistence_exercised=false`,
  and `kubernetes_traffic_exercised=false` are required. It is not evidence for
  object-store mirror writes, a long-running logical-replication mirror daemon,
  exactly-once checkpoint persistence, Citus distributed mirror routing, or
  Kubernetes traffic.
- `FEATURE: L12` now has separate bounded live DuckDB extension-catalog
  evidence. `ci/ai-blaise/sidecar-analytical-duckdb-extension-live-smoke.sh`
  runs `run-duckdb-extension-catalog-canonical`, verifies `INSTALL httpfs`,
  `LOAD httpfs`, `INSTALL iceberg`, and `LOAD iceberg`, then runs the pinned
  DuckDB container
  `duckdb/duckdb@sha256:ddc7ffc382dfd3f8213ac3d29435a7ce0ea4446fb3fc966a57a28d39b46174b1`.
  The smoke executes real DuckDB extension installation/loading, queries
  `duckdb_extensions()`, and requires `duckdb_extension_catalog_live=passed`,
  `l12_extensions_installed=2`, `l12_extensions_loaded=2`, and
  `l12_duckdb_extensions_catalog_queried=true`. The claim is bounded to that
  pinned DuckDB extension-catalog path: `pg_duckdb_runtime_exercised=false`,
  `motherduck_session_exercised=false`, `object_store_io_attempted=false`, and
  `extension_repository_mirror_verified=false` are required. It is not evidence
  for pg_duckdb inside PostgreSQL, MotherDuck cloud sessions, object-store reads,
  warehouse federation, or an internally mirrored DuckDB extension repository.
- `FEATURE: L6` now has separate bounded local federation-catalog publication
  evidence. `ci/ai-blaise/sidecar-analytical-federation-catalog-live-smoke.sh`
  runs `run-federation-catalog-publication-canonical`, writes the v1 JSON
  catalog artifact for Databricks, Snowflake, Trino, and Spark, validates it via
  `json.load`, serves it over loopback HTTP, fetches it with `curl`, and requires
  byte equality with the generated artifact. The smoke requires
  `federation_catalog_publication_live=passed`, `l6_catalog_version=v1`,
  `l6_catalog_count=4`,
  `l6_federation_targets=databricks,snowflake,trino,spark`,
  `l6_local_catalog_artifact_created=true`, `l6_local_http_catalog_served=true`,
  and `local-federation-catalog-artifact-http-only`. The claim is not evidence
  for live Snowflake, live Trino, live Spark, live Databricks, warehouse
  connections, catalog authentication, object-store catalog reads, F3 warehouse
  federation, or Kubernetes traffic; the report requires
  `external_warehouse_connections_attempted=false`,
  `object_store_io_attempted=false`, and `catalog_auth_exercised=false`.
  `FEATURE: L1` and `FEATURE: L13` remain alpha.
- Agentmemory checkpointing for this slice used the scaleable-database-infra
  service at `http://127.0.0.1:3911` and did not edit or erase the backing
  memory file directly.

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

Worker D CDC/realtime production evidence from 2026-05-23 and the C2 DDL
capture follow-up from 2026-05-24 add `C1`, `C2`, `C3`, `WH3`, `RT1`, `RT2`,
`RT3`, `RT4`, and `RT5` to the narrow production-ready set. The evidence is
limited to the CDC/realtime sidecar runtime boundary: wal2json ingest, pgoutput
logical-frame decoder boundary, checkpoint/ack state, health/readiness/metrics,
PII anonymization before sink encoding, file/in-memory DLQ records,
schema-qualified row-event delivery, DDL stream-table parsing from a live
PostgreSQL event-trigger capture harness, raw WebSocket Phoenix channel join,
presence, tenant/topic filtering, `postgres_changes` fan-out, and CDC-to-realtime
Unix-domain-socket bridging under `cargo test -p ai_blaise_citus_sidecar_cdc`,
`cargo test -p ai_blaise_citus_sidecar_realtime`,
`ci/ai-blaise/sidecar-cdc-smoke.sh`, and
`ci/ai-blaise/sidecar-realtime-smoke.sh`. The C2 DDL claim is bounded to a live
`postgres:17-bookworm` harness that creates `cdc.ddl_events`, installs
`CREATE EVENT TRIGGER ai_blaise_capture_ddl`, captures
`CREATE TABLE public.cdc_schema_smoke`, and verifies `ddl_events_total`,
`ddl_stream_table`, `command_tag`, `object_schema`, `object_identity`, and
per-event `ddl_event` JSON through the same `/ingest` runtime path; managed
broker delivery, multi-node Kubernetes traffic, and long-running logical
replication slot tailing remain outside this promoted evidence. The realtime claim is bounded to
`runtime_boundary=single-node-raw-ws-cdc-ingest` with
`websocket_network_exercised=true`, `browser_client_exercised=false`,
`cdc_tailing_integrated=false`, `multi_node_pubsub=false`, and
`kubernetes_traffic_exercised=false`; browser client behavior, WebSocket
extension negotiation, live CDC tailing, multi-node pubsub, and Kubernetes
traffic are not promoted by this evidence. External managed broker operations
(NATS auth/TLS/JetStream, GCP Pub/Sub IAM/live publish, Kafka/Kinesis managed
client operation) remain alpha unless covered by their own feature entry.

Worker CDC-Sinks production evidence from 2026-05-24 adds `C14` and `C15` to the narrow production-ready set. The evidenced boundary is strict local NATS subject/server URL and Pub/Sub project/topic validation, deterministic NATS `PUB` and Pub/Sub `messages.publish` frame encoding, serve-runtime/canonical stdout exposure, and DLQ retry accounting for live NATS dispatch failures under `cargo test -p ai_blaise_citus_sidecar_shared -p ai_blaise_citus_sidecar_cdc` and `ci/ai-blaise/sidecar-cdc-smoke.sh`. Managed NATS auth/TLS/JetStream and live GCP Pub/Sub auth/IAM/topic operations remain alpha.
R2 scale-to-zero compute is production-ready for the bounded Kubernetes
Deployment compute scale-down primitive. `operator-branch-lifecycle-smoke.sh`
proves the operator suspend plan moves `ready` to `suspended` in six steps and
includes `ScaleTargetComputeToZero`;
`REQUIRE_DOCKER=1 ci/ai-blaise/operator-branch-scale-to-zero-live-smoke.sh`
creates a real kind cluster, applies a one-replica `branch-review` Deployment,
executes `kubectl scale deployment/branch-review --replicas=0`, and verifies
`branch_scale_to_zero_live=passed`, `kubernetes_deployment_scaled_to_zero=true`,
`spec_replicas_after_scale=0`, `observed_replicas_after_scale=0`,
`active_sessions_fail_closed=true`, and `pending_migrations_fail_closed=true`.
This does not claim CSI `VolumeSnapshot` creation, PVC cloning, full branch
suspend/resume reconciliation, Service/DNS retargeting, traffic cut-over, or
branch promotion.

PGC1/PGC2 production evidence from 2026-05-25 promotes only the bounded PostgreSQL 17 patched-core runtime path. `REQUIRE_DOCKER=1 bash ci/ai-blaise/postgres-core-patches-live-smoke.sh` builds `images/citus-pg-overlay/Dockerfile.pgcore-patches`, clones PostgreSQL `REL_17_10`, applies `patches/postgres/series`, compiles Citus against the patched `pg_config`, installs the smoke-only `ai_blaise_pgc_probe` extension, runs `initdb`, starts PostgreSQL with `shared_preload_libraries=citus` and `track_commit_timestamp=on`, creates both `citus` and `ai_blaise_pgc_probe`, verifies `pgc_logical_clock_hook_executed=true`, verifies a `SubTransactionIdSetCommitTsData` override through `pg_xact_commit_timestamp`, and verifies `pg_waldump` identifies `SUBTRANS_TS`. This does not claim live pgactive traffic, live Spock apply traffic, multi-node active-active conflict replay, PG18, or the full Bundle1 operand image.

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

Edge2 libsql read-tier research-guard evidence from 2026-05-25 promotes only the
fail-closed negative guard. `ci/ai-blaise/edge2-libsql-research-guard-smoke.sh`
runs the targeted companion test and `run-libsql-read-tier-guard-canonical`,
then verifies `edge2_libsql_research_guard_smoke`,
`guard_status=fail-closed`, `live_execution_claims=0`,
`replication_adapter_claimed=false`, `workload_isolation_claimed=false`, and
`production_query_routing_claimed=false`. The guard points at
`docs/ai-blaise/ADR/0009-libsql-read-tier-research-guard.md`, blocks the
`libsql production read tier` integration, and requires explicit promotion
evidence before any replacement ADR can enable it. This does not claim libsql
read-tier integration, a libsql replication adapter, workload isolation,
production query routing to libsql, operator reconciliation, or Kubernetes
traffic.



C4/C5 production evidence from 2026-05-24 promotes only the bounded conflict-policy metadata and taxonomy surface. `REQUIRE_DOCKER=1 ci/ai-blaise/operator-reconcilers-batch-c-smoke.sh` boots the live Citus overlay image through `CONFLICT_POLICY_IMAGE`, runs `run-conflict-policy-runtime-canonical`, applies the generated SQL, and verifies `conflict_policy_live_row` rows for `accounts-lww` (`update_origin_differs`/`apply_remote_if_newer`) and `accounts-merge` (`update_exists`/`merge_function` with `public.merge_remote_into_local`) plus `replication_conflict_status`. The same evidence is paired with `ci/ai-blaise/companion-runtime-depth-a-smoke.sh`, where `conflict_classes` is `7` and audit SQL targets `companion.replication_conflict_audit`. This does not claim live pgactive conflict traffic, does not claim live Spock apply traffic, and does not prove multi-node active-active replication, PGC1/PGC2 runtime activation, remote conflict replay, or a production replication apply worker.

F4 production evidence from 2026-05-25 promotes only bounded `postgres_fdw`
credential rotation. `REQUIRE_DOCKER=1 ci/ai-blaise/fdw-credential-rotation-live-smoke.sh`
starts two live `postgres:17-bookworm` containers, creates a real
`postgres_fdw` foreign server and user mapping, proves the original mapping can
read a remote table, changes the remote role password, proves the stale mapping
is rejected with `old_password_rejected=true`, executes the companion-rendered
`ALTER USER MAPPING` plan, and proves the rotated mapping reads successfully
with `new_password_succeeded=true`. The generated SQL uses
`:'fdw_new_password'`, calls `postgres_fdw_disconnect_all()`, and is checked
with `plan_secret_literals=false`. This does not claim managed secret backend
reconciliation, Kubernetes `ExternalSecret` updates, application connection
draining outside `postgres_fdw`, cross-region FDW topology changes, or
multi-tenant secret distribution.

M4 production evidence from 2026-05-25 promotes only live schema drift
detection. `REQUIRE_DOCKER=1 ci/ai-blaise/schema-drift-live-smoke.sh` starts a
live `postgres:17-bookworm` container, creates an intentionally drifted
`public.accounts` table, executes the companion-rendered
`information_schema.columns` detector, and verifies rows for `missing_column`,
`type_mismatch`, `nullability_mismatch`, and `unexpected_column`. The same smoke
then fixes the table and proves `clean_schema_zero_drift=true`. This does not
claim remediation planning, DDL execution, operator apply behavior,
cross-database inventory fanout, or automatic migration generation.

R12 per-shard temperature ranking is production-ready only for a bounded
read-only Citus catalog ranking surface.
`REQUIRE_DOCKER=1 ci/ai-blaise/shard-temperature-ranking-live-smoke.sh` starts
the local Citus+Timescale cohabitation image with `shared_preload_libraries`,
creates `public.temperature_orders`, distributes it with Citus, inserts three
validated `public.ai_blaise_shard_temperature_samples` rows from real
`pg_dist_shard` shard ids, and executes the companion-rendered query. The smoke
verifies `shard_temperature_ranking_live=passed`,
`citus_pg_dist_shard_joined=true`, `temperature_scores_ranked=true`,
`hot_shards=1`, `warm_shards=1`, `cold_shards=1`,
`automatic_tier_movement=false`, and `coldtier_moves_executed=false`. This does
not claim telemetry collection. It does not claim automatic tier movement, does
not claim cold-tier artifact moves, does not claim Citus placement changes, and
does not claim distributed planner integration.

S8/S12 regional placement primitives are production-ready only for a bounded
read-only Citus/PostgreSQL catalog guard.
`REQUIRE_DOCKER=1 ci/ai-blaise/regional-placement-live-smoke.sh` starts the
local Citus+Timescale cohabitation image, creates real PostgreSQL tablespaces,
creates `public.locality_orders` and `public.locality_orders_eu` in those
tablespaces, distributes both tables with Citus on `locality_key`, and executes
the companion-rendered catalog query. The smoke verifies
`regional_placement_live=passed`, `locality_prefixed_pk_valid=true`,
`citus_distribution_present=true`, `region_tablespace_mappings_valid=true`,
`region_tablespace_count=2`, `automatic_rebalance_executed=false`,
`shard_movement_executed=false`, `worker_placement_enforced=false`, and
`multi_region_failover_exercised=false`. This does not claim key rewrites. It
does not claim foreign-key compatibility migration, does not claim production
tablespace creation, does not claim operator reconciliation, does not claim
worker-level shard placement enforcement, does not claim automatic rebalance,
does not claim shard movement, and does not claim multi-region failover.

T13/T14 transaction-state primitives are production-ready only for a bounded
single-node Citus distributed-table SQL transaction smoke.
`REQUIRE_DOCKER=1 ci/ai-blaise/transaction-state-live-smoke.sh` starts the local
Citus+Timescale cohabitation image, creates and distributes
`public.txn_state_orders`, inserts five rows, executes the companion-rendered
transaction SQL, and verifies `transaction_state_live=passed`,
`distributed_cursor_declared=true`, `cursor_fetch_batches=2`,
`cursor_rows_fetched=5`, `savepoint_rollback_verified=true`,
`count_after_insert=6`, `count_after_rollback=5`, `final_count=5`,
`citus_adaptive_plan_observed=true`, `citus_task_count_observed=1`,
`coordinator_failover_exercised=false`, `multi_worker_cleanup_exercised=false`,
and `wire_protocol_portal_exercised=false`. This does not claim PostgreSQL wire
protocol portal implementation. It does not claim multi-worker cursor cleanup,
does not claim cursor holdability across transactions, does not claim
coordinator restart recovery, does not claim distributed deadlock handling, and
does not claim Kubernetes transaction-drain behavior.


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

The Citus patch production integration audit now has measured gates for custom
patch artifacts `0004`, `0006`, `0007`, and `0008`. `0004` records the integrated
router-planner hot-path p95 and sample-count gate; `0006` records the live
single-shard fast-path-router locality probe with zero coordinator round trips
for the bounded pool contract; `0007` and `0008` retain the VM pg_cron
cohabitation and cohabit-detection results. `ci/ai-blaise/citus-patch-production-audit.sh` fails
closed unless each artifact is listed in `patches/series`, future patch roster
entries stay documented as roster-only until artifacts land, measured JSON uses
`mode: measured`, and declared thresholds in
`benchmarks/citus-patches/production-gates.json` pass. This remains negative
evidence for patch IDs without measured results, while measured JSON is required
for any patch-gate signoff.
Bundle1 production-ready evidence from 2026-05-26 promotes `FEATURE: Bundle1`
from alpha to production-ready for the `full-bundle-required-minus-plrust`
boundary. The new `bundle1-pgdg-runtime` Dockerfile stage layers PGDG and
TimescaleDB binary-package extensions on top of `postgres:17-bookworm`
(timescaledb-2-postgresql-17, postgresql-17-cron, postgresql-17-partman,
postgresql-17-pgaudit, postgresql-17-pgauditlogtofile, postgresql-17-pgvector,
postgresql-17-postgis-3, postgresql-17-repack, postgresql-17-rum,
postgresql-17-hll, postgresql-17-pg-failover-slots, postgresql-17-age,
postgresql-17-pg-uuidv7, postgresql-17-tdigest, postgresql-17-pgnodemx).
`bundle1-final-light` then layers in the source-built citus, pgsodium, topn,
pg_jsonschema, pg_graphql extensions, and `bundle1-final-full` adds pg_search
and plv8. The canonical `shared-preload-libraries.conf` now only references
actually-installed shared libraries (citus, timescaledb, pgaudit,
pgauditlogtofile, pgsodium, pg_cron, age, pg_failover_slots, pgnodemx) and the
`/docker-entrypoint-initdb.d/00-ai-blaise-extensions.sql` script runs
`CREATE EXTENSION` for every required Bundle1 extension at first container
start. The image labels record the new scope:
`ai-blaise.citus.bundle1.evidence-scope=full-bundle-required-minus-plrust` and
`ai-blaise.citus.bundle1.full-initdb-path=true`. The
`BUNDLE1_BUILD_IMAGE=1 REQUIRE_DOCKER=1` and
`BUNDLE1_BUILD_HEAVY=1` variants of
`ci/ai-blaise/sql-extension-smoke.sh` verify pg_extension catalog records
every required Bundle1 extension after initdb and record the proof in
`images/citus-pg-overlay/bundle1-source-build-evidence.tsv`. plrust has been
moved from `required` to `optional` in
`images/citus-pg-overlay/extension-manifest.tsv`; the plrust PG17 upstream
gap (upstream main still pg13-pg16 with pgrx 0.11.0 as of 2026-02-27) is
tracked separately under `FEATURE: EF6` and does not block the Bundle1
required-extension production-ready claim. This is not evidence for plrust
Rust UDFs, PG18 source-build of the heavy extensions, command-center release
chart certification, Kubernetes operand image release certification, or
production multi-region deployment correctness.
