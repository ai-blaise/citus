# pool image

Container image for the shard-aware pool.

Build with `scripts/citus-scale/build-app-images.sh`. The current production
command is `serve`, which proxies PostgreSQL TCP traffic on
`AI_BLAISE_POOL_LISTEN_ADDR` and exposes `/healthz`, `/readyz`, and `/metrics`
on `AI_BLAISE_POOL_ADMIN_ADDR`. `AI_BLAISE_POOL_UPSTREAM_ADDR` is required so
the pool fails closed instead of pretending to be ready without a backend.
