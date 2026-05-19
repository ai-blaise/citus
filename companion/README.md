# companion

Rust `pgrx` companion extension for SQL surfaces that coordinate Citus,
TimescaleDB, bundled extensions, and sidecars.

Initial critical module: `citus_timescale`.

## Current Surface

- `DistributedHypertablePlan` for `FEATURE: TS1`
- `TimeRangeShardPrunerPlan` for `FEATURE: TS5`
- policy plan shapes for distributed compression, retention, reorder, and
  continuous aggregate refresh

Default `cargo test -p ai_blaise_citus_companion` runs pure Rust validation.
The `pg18` feature is reserved for PostgreSQL 18 `pgrx` packaging.
