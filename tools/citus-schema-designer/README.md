# citus-schema-designer

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Schema designer fork target based on DrawDB.

The current Rust contract covers `FEATURE: M9` and `FEATURE: D6` by validating
schema models and rendering overlay layers for distribution columns,
hypertables, search indexes, webhooks, and operator shard placements.
Use `cargo run -p ai_blaise_citus_schema_designer -- run-canonical` to emit the
deterministic overlay-layer TSV report.
