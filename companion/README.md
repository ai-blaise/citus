# companion

Rust `pgrx` companion extension for SQL surfaces that coordinate Citus,
TimescaleDB, bundled extensions, and sidecars.

Initial critical modules: `citus_timescale`, `db_doctor`,
`geo_distributed`, `graph_bridge`, `index_advisor`, `jsonschema_bridge`,
`ledger`, `lsp_metadata`, `migration`, `observability`, `plan_freeze`,
`auth`, `router_assist`, `schema_jobs`, `search_bridge`, `tenants`,
`toolkit_distributed`, and `webhooks`.

## Current Surface

- `DistributedHypertablePlan` for `FEATURE: TS1`
- `VectorizerDefinition` SQL-plan rendering for `FEATURE: A1`
- SQL-plan rendering for `FEATURE: TS2`, `FEATURE: TS3`, `FEATURE: TS4`, and
  `FEATURE: TS12`
- `TimeRangeShardPrunerPlan` for `FEATURE: TS5`
- `LspMetadataViewPlan` for `FEATURE: D4`, `FEATURE: M5`, and `FEATURE: TS8`
- `SearchIndexDistributedPlan`, `HybridRankPlan`, and `RerankerPlan` for
  `FEATURE: Search2`, `FEATURE: Search3`, and `FEATURE: Search9`
- `GraphDistributionPlan` for `FEATURE: G2`, `FEATURE: G3`, and
  `FEATURE: API4`
- `JsonSchemaDistributedPlan` for `FEATURE: JS2` and `FEATURE: M13`
- `GeoDistributionPlan` and `GeoPruningPlan` for `FEATURE: Geo2` and
  `FEATURE: Geo3`
- `DbDoctorPlan`, `CohabitPreflightPlan`, and `DbDoctorReport` for
  `FEATURE: TS9` and `FEATURE: M7`
- `PlanFreezePlan` and `PlanRegressionSample` for `FEATURE: PM3` and
  `FEATURE: PM4`
- `IndexAdvisorPlan` for `FEATURE: IA3`
- `LedgerTransferPlan`, `LedgerSealPlan`, and `LedgerChain` for
  `FEATURE: Sec5` and `FEATURE: Sec6`
- `MigrationPlan` for `FEATURE: M1` and `FEATURE: M11`
- `WebhookRegistrationPlan` for `FEATURE: WH2`
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
- `ToolkitDistributedPlan` for `FEATURE: T8`, `FEATURE: TS13`,
  `FEATURE: TS14`, `FEATURE: TS15`, `FEATURE: TS16`, `FEATURE: TS17`, and
  `FEATURE: L9`

Default `cargo test -p ai_blaise_citus_companion` runs pure Rust validation.
The `pg18` feature exposes the first pgrx SQL-callable companion Timescale
bridge functions.
