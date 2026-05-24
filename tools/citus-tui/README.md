# citus-tui

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Terminal UI fork target based on rainfrog.

The current Rust contract covers `FEATURE: D3` by validating required live
panels, read-only actions, and safe-mode blocking for mutating workflows. The
runtime path now renders deterministic terminal frames from the shared tools
snapshot TSV and previews operator actions with safe-mode and `CONFIRM`
guards.

Use `cargo run -p ai_blaise_citus_tui -- run-canonical` to emit the
deterministic session TSV report.
Use `cargo run -p ai_blaise_citus_tui -- render-frame --snapshot <snapshot.tsv> --panel shards`
to render a concrete frame.
