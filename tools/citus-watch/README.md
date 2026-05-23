# citus-watch

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Ratatui dashboard for cluster, pool, and sidecar status.

The current Rust contract covers `FEATURE: O13` and `FEATURE: D12` by
validating data sources and panels for cluster topology, shards, hypertables,
Citus-aware EXPLAIN, rebalance status, vectorizer backlog, search indexes,
tenants, and branches.
The runtime path now renders a deterministic dashboard frame from the shared
tools snapshot TSV, including pool readiness, vectorizer backlog, shard, and
tenant signals plus the query plan each panel will execute.

Use `cargo run -p ai_blaise_citus_watch -- run-canonical` to emit the
deterministic dashboard TSV report.
Use `cargo run -p ai_blaise_citus_watch -- render-frame --snapshot <snapshot.tsv>`
to render the dashboard frame.
