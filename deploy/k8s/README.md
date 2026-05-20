# deploy/k8s

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
start with `serve`. The operator and sidecars expose shared `/healthz`,
`/readyz`, and `/metrics` endpoints. The pool exposes PostgreSQL traffic on
its `postgres` port and admin probes on its separate `admin` port; its
readiness probe checks that `pool.upstream.host:pool.upstream.port` accepts TCP
connections.

`values.yaml`, `values-dev.yaml`, and `values-prod.yaml` deliberately list the
same sidecar names so environment overlays cannot silently drop a daemon from
the install path. `ci/ai-blaise/deploy-check.sh` enforces that list.
`ci/ai-blaise/pool-proxy-smoke.sh` verifies the pool data port by running a
real PostgreSQL query through `serve`, while CI requires Docker for that live
traffic gate.
`ci/ai-blaise/kind-production-smoke.sh` builds the real Rust image matrix,
loads it into kind, installs the Helm chart with a real PostgreSQL upstream,
and verifies SQL plus admin metrics through the pool service.
