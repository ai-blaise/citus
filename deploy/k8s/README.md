# deploy/k8s

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Slim Kubernetes install surface for ai-blaise/citus. This chart owns only the
ai-blaise overlay components and CRDs; third-party platform charts remain
externally operated.

The initial Helm chart lives at `deploy/k8s/helm/citus-overlay` and includes
operator, pool, all Rust sidecars, optional tools, and CRD packaging contracts.
It does not vendor CNPG, monitoring, secrets, ingress, storage, or backup
platform charts; those remain external platform responsibilities.

The chart also carries ai-blaise-owned observability artifacts: Grafana
dashboard ConfigMaps and optional `PrometheusRule` alerts. These resources are
plain Kubernetes/monitoring objects and assume the platform already provides
the matching controllers.

Rust workloads run the real image matrix built by
`scripts/citus-scale/build-app-images.sh`. The operator, pool, and sidecars
start with `serve`; the `citusctl` tool image defaults to `plan inspect
cluster` and is executed by the kind production smoke as a Kubernetes Job. The
operator and sidecars expose shared `/healthz`, `/readyz`, and `/metrics`
endpoints. The pool exposes PostgreSQL traffic on its `postgres` port and admin
probes on its separate `admin` port; its readiness probe checks that
`pool.upstream.host:pool.upstream.port` accepts TCP connections. The pool
PostgreSQL data port enforces `pool.networkPolicy.cidrAllowlist` in the live
proxy through `AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST`, and the chart renders a
matching NetworkPolicy for clusters with NetworkPolicy-capable CNI support.
Release image builds set `PUSH=true` and write
`artifacts/ai-blaise-image-digests.tsv`; production Helm values consume the
operator and pool `sha256:` digests from that manifest.
The production/default operator runtime needs only its ServiceAccount for pod
identity and probes. Controller-grade ClusterRole/Binding output is behind the
explicit alpha `operator.controllerRbac.enabled` flag; the exhaustive profile
renders that RBAC for contract coverage, still enumerating ai-blaise resources
and avoiding Secret access while External Secrets integration remains alpha.

`values.yaml`, `values-dev.yaml`, `values-exhaustive.yaml`, and
`values-prod.yaml` deliberately list the same sidecar names so environment
overlays cannot silently drop a daemon from the install path.
`ci/ai-blaise/deploy-check.sh` enforces that list. The default `values.yaml`
profile is production-safe: it requires immutable operator/pool image digests
and keeps alpha sidecars, tools, and alpha runtime/security intent disabled.
Use `values-exhaustive.yaml` only for explicit non-production image-matrix
coverage.
`ci/ai-blaise/pool-proxy-smoke.sh` verifies the pool data port by running a
real PostgreSQL query through `serve`, while CI requires Docker for that live
traffic gate. The smoke also proves CIDR-denied SQL traffic is rejected and
reported through pool metrics.
`ci/ai-blaise/kind-production-smoke.sh` builds the real Rust image matrix,
loads it into kind, installs the explicit `values-exhaustive.yaml` image-matrix
profile with a real PostgreSQL upstream, verifies SQL plus admin metrics
through the pool service, proves the pool CIDR deny path with live Kubernetes
traffic, and executes the built `citusctl` image. It also installs the default
`values.yaml` profile with direct Helm and the production `values-prod.yaml`
profile through the deploy wrapper, where alpha sidecars and tools remain
disabled. For `FEATURE: O6` and `FEATURE: O10`, the same smoke requires the
live dashboard ConfigMap and PrometheusRule resources to contain the expected
dashboard JSON payloads and alert names after each Helm profile is installed.
The Argo application targets the `main` release branch, uses
`values-prod.yaml`, creates the target namespace, and prunes stale rendered
resources so GitOps deploys the same production profile that the smoke
verifies without leaving disabled alpha workloads behind.

The human deploy wrapper is production-safe by default as well:
`scripts/citus-scale/deploy.sh` defaults `DEPLOY_PROFILE=prod` to
`values-prod.yaml`. Use `DEPLOY_PROFILE=dev` or `DEPLOY_PROFILE=exhaustive`
for rendering non-production profiles, and set `ALLOW_ALPHA_INSTALL=1` only
when intentionally installing dev, exhaustive, or custom values that may enable
alpha sidecars or alpha runtime/security intent.

Production values require immutable operator and pool image digests. Set
`OPERATOR_IMAGE_DIGEST=sha256:...` and `POOL_IMAGE_DIGEST=sha256:...` when
using the deploy wrapper, or set `operator.image.digest` and `pool.image.digest`
directly with Helm. `ALLOW_MUTABLE_IMAGE_TAGS=1` is only for local/dev smoke
work with locally loaded images. The Argo production app fails closed until the
release branch or deployment overlay supplies those digests.

The Makefile release gate runs `ci/ai-blaise/deploy-check.sh` with
`REQUIRE_HELM=1`, so rendered chart checks fail closed when Helm is unavailable
instead of being skipped as exploratory evidence.

The production-values smoke installs through `scripts/citus-scale/deploy.sh
MODE=install`, which live-gates the human deploy wrapper. The optional
`tools` Deployment is a dev-only rendered contract until separately promoted;
the production smoke executes the built `citusctl` image as a Job instead. The
Argo application manifest is a GitOps render contract, not live controller
evidence by itself.
