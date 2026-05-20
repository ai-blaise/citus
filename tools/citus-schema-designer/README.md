# citus-schema-designer

Schema designer fork target based on DrawDB.

The current Rust contract covers `FEATURE: M9` and `FEATURE: D6` by validating
schema models and rendering overlay layers for distribution columns,
hypertables, search indexes, webhooks, and live shard placements.
Use `cargo run -p ai_blaise_citus_schema_designer -- run-canonical` to emit the
deterministic overlay-layer TSV report.
