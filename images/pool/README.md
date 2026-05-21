# pool image

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Container image for the shard-aware pool.

Build with `scripts/citus-scale/build-app-images.sh`. The current production
command is `serve`, which proxies PostgreSQL TCP traffic on
`AI_BLAISE_POOL_LISTEN_ADDR` and exposes `/healthz`, `/readyz`, and `/metrics`
on `AI_BLAISE_POOL_ADMIN_ADDR`. `AI_BLAISE_POOL_UPSTREAM_ADDR` is required so
the pool fails closed instead of pretending to be ready without a backend.
