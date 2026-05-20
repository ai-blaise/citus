# companion

Rust `pgrx` companion extension for SQL surfaces that coordinate Citus,
TimescaleDB, bundled extensions, and sidecars.

Initial critical modules: `advanced_planner`, `auth`, `citus_timescale`,
`db_doctor`, `extension_catalog`, `geo_distributed`, `graph_bridge`,
`index_advisor`, `jsonschema_bridge`, `ledger`, `lsp_metadata`, `migration`,
`observability`, `ops_contracts`, `plan_freeze`, `router_assist`,
`schema_jobs`, `search_bridge`, `tenants`, `toolkit_distributed`, `vector`,
and `webhooks`.

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
- `ExtensionContract` for the V2 bundled and optional extension surfaces,
  including `FEATURE: A7`, `FEATURE: Search1`, `FEATURE: G1`,
  `FEATURE: JS1`, `FEATURE: PM1`, `FEATURE: IA1`, `FEATURE: O7`,
  `FEATURE: Sec3`, and `FEATURE: WF1`
- `AdvancedPlannerContract` for remaining planner, tiering, time-travel,
  cursor, savepoint, regional, backup, federation, storage, and research-guard
  surfaces, including `FEATURE: T4`, `FEATURE: T10`, `FEATURE: T11`,
  `FEATURE: T13`, `FEATURE: T14`, `FEATURE: TS10`, `FEATURE: TS11`,
  `FEATURE: S1`, `FEATURE: S3`, `FEATURE: S8`, `FEATURE: S12`,
  `FEATURE: MR3`, `FEATURE: MR6`, `FEATURE: B4`, and `FEATURE: Sto2`
- `OperationsReadinessContract` for Helm install, wrapper, runbook, MCP,
  security, realtime client, io_uring, and protocol pipeline gates, including
  `FEATURE: D7`, `FEATURE: D8`, `FEATURE: D9`, `FEATURE: D10`,
  `FEATURE: D11`, `FEATURE: MR9`, `FEATURE: RT5`, `FEATURE: S7`,
  `FEATURE: A9`, `FEATURE: Sec7`, `FEATURE: Sec8`, `FEATURE: Sec9`,
  `FEATURE: Sec13`, `FEATURE: T6`, and `FEATURE: T7`
- `COMPANION_FEATURE_STATUSES` as the canonical status table exposed by the
  pgrx `companion_feature_status()` function and by the SQL fallback extension
  packaged in the operand image.

Default `cargo test -p ai_blaise_citus_companion` runs pure Rust validation.
The `pg18` feature exposes the first pgrx SQL-callable companion Timescale
bridge functions.
