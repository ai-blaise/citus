# pool image

Container image for the shard-aware pool.

Build with `scripts/citus-scale/build-app-images.sh`. The current production
command is `serve`, which exposes `/healthz`, `/readyz`, and `/metrics` on the
configured pool service port while the pool protocol implementation continues
to live in `pool/src/runtime.rs`.
