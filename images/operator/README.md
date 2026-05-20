# operator image

Container image for the Rust kube-rs operator.

Build with `scripts/citus-scale/build-app-images.sh`. The production command is
`serve`, which exposes `/healthz`, `/readyz`, and `/metrics` on port 8080.
