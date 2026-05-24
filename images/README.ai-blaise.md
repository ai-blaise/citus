# ai-blaise Images

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

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
sidecars, and tools; use `scripts/citus-scale/build-app-images.sh` to build and
optionally push the release app image matrix. Production promotion requires
`artifacts/ai-blaise-image-digests.tsv` with immutable repo digest rows for the
exact images installed by the command-center chart.
