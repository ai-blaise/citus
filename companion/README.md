# companion

Rust `pgrx` companion extension for SQL surfaces that coordinate Citus,
TimescaleDB, bundled extensions, and sidecars.

Initial critical modules: `citus_timescale`, `observability`, `auth`,
`router_assist`, `schema_jobs`, and `tenants`.

## Current Surface

- `DistributedHypertablePlan` for `FEATURE: TS1`
- SQL-plan rendering for `FEATURE: TS2`, `FEATURE: TS3`, `FEATURE: TS4`, and
  `FEATURE: TS12`
- `TimeRangeShardPrunerPlan` for `FEATURE: TS5`
- policy plan shapes for distributed compression, retention, reorder, and
  continuous aggregate refresh
- `OperationsGuardrailPlan` for `FEATURE: O1`, `FEATURE: O2`, `FEATURE: O3`,
  and `FEATURE: R4`
- `TenantRlsPolicyPlan` and `JwtVerificationPlan` for `FEATURE: Sec1`,
  `FEATURE: Sec2`, and `FEATURE: Auth2`
- `ShardForValuePlan`, `PlacementGenerationQuery`, and `LocalPlacementCheck`
  for `FEATURE: S6` and `FEATURE: S13`
- `SchemaJobPlan` for `FEATURE: C10` and `FEATURE: M2`
- `TenantMovePlan`, `TenantQuotaPlan`, and `TenantArchivePlan` for
  `FEATURE: S14`, `FEATURE: TO3`, `FEATURE: TO4`, and `FEATURE: TO5`

Default `cargo test -p ai_blaise_citus_companion` runs pure Rust validation.
The `pg18` feature exposes the first pgrx SQL-callable companion Timescale
bridge functions.
