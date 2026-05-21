# ai-blaise Images

Image directories under this tree contain the Postgres operand-image contract
and build the Rust sidecar, pooler, operator, and tool containers.

`images/citus-pg-overlay` is the `FEATURE: Bundle1` alpha contract for the
Postgres operand image. It copies the local `ai_blaise_citus` SQL fallback,
extension manifest, preload contract, and initdb order, but it is not
production evidence that the full required binary extension bundle is installed
in a runnable operand image. Release promotion for Bundle1 requires a real
operand image build/initdb smoke that proves required extension control files
and `CREATE EXTENSION` execution end to end.

`images/rust-runtime` is the shared Rust app runtime for the operator, pool,
sidecars, and tools; use `scripts/citus-scale/build-app-images.sh` to build the
production-verified app image matrix.
