# ai-blaise Images

Image directories under this tree build the Citus operand image, sidecars,
pooler, operator, and tool containers.

`images/citus-pg-overlay` builds the Postgres operand with the ai-blaise SQL
extension and extension manifest. `images/rust-runtime` is the shared Rust app
runtime for the operator, pool, sidecars, and tools; use
`scripts/citus-scale/build-app-images.sh` to build the full matrix.
