# citus-admin

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Admin UI fork target based on WhoDB.

The current Rust contract covers `FEATURE: D5` by validating admin routes and
confirmation-gated mutating actions.
Use `cargo run -p ai_blaise_citus_admin -- run-canonical` to emit the
deterministic route/action TSV report.
