# citus-admin

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Admin UI fork target based on WhoDB.

The current Rust contract covers `FEATURE: D5` by validating admin routes and
confirmation-gated mutating actions. The runtime path now reads the shared
tools snapshot TSV, renders concrete HTML for every guarded admin route, and
rejects mutating action previews unless the exact `CONFIRM` token is supplied.

Use `cargo run -p ai_blaise_citus_admin -- run-canonical` to emit the
deterministic route/action TSV report.
Use `cargo run -p ai_blaise_citus_admin -- render --snapshot <snapshot.tsv> --route /cluster/shards`
to render a route from a validated snapshot.
