# companion

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Rust `pgrx` companion extension for SQL surfaces that coordinate Citus,
TimescaleDB, bundled extensions, and sidecars.

Initial critical modules: `advanced_planner`, `auth`, `citus_timescale`, `columnar_tiering`,
`db_doctor`, `domain_contracts`, `extension_catalog`, `geo_distributed`,
`graph_bridge`, `index_advisor`, `jsonschema_bridge`, `ledger`,
`lsp_metadata`, `migration`, `observability`, `ops_contracts`, `plan_freeze`,
`queue`, `replication_conflict`, `router_assist`, `runtime_depth_a`,
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
- `PlanFreezePlan`, `PlanRegressionSample`, and `PlanRuntime` for
  `FEATURE: PM3` and `FEATURE: PM4`; the runtime applies durable
  idempotency, bounded retry, promotion, regression, and audit contracts.
- `IndexAdvisorPlan` for `FEATURE: IA3`
- `LedgerTransferPlan`, `LedgerSealPlan`, and `LedgerChain` for
  `FEATURE: Sec5` and `FEATURE: Sec6`
- `MigrationPlan` and `MigrationRuntime` for `FEATURE: M1` and
  `FEATURE: M11`, including guarded expand/backfill/validate/cutover phase
  decisions.
- `DurableQueueRuntime` for the `FEATURE: R6` queue substrate, including
  idempotent enqueue, `FOR UPDATE SKIP LOCKED` lease SQL, visibility timeout,
  retry backoff, ack, and dead-letter transitions.
- `ReplicationConflictResolver` for `FEATURE: C4` and `FEATURE: C5`, including
  the seven conflict classes, monotonic-clock validation, origin-priority and
  home-region policies, audit SQL, and fail-closed ambiguous/apply-error paths.
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
- `canonical_domain_contracts_report` executes the deterministic companion
  evidence batch across vector, auth, graph, geo, advisor, JSON schema,
  migration, plan freeze, router, search, ledger, toolkit, tenant, and webhook
  contracts; `companion_contracts run-domain-contracts-canonical` emits the
  TSV summary used by CI.
- `canonical_plan_runtime_report` executes the PM3/PM4 companion runtime
  depth-B path with a deterministic promotion, an idempotency replay, a
  transient retry, a blocked regression candidate, and a failed unknown-plan
  guard; `companion_contracts run-plan-runtime-canonical` emits the TSV
  summary used by CI.
- `ExtensionContract` for the V2 bundled and optional extension surfaces,
  including `FEATURE: A7`, `FEATURE: Search1`, `FEATURE: G1`,
  `FEATURE: JS1`, `FEATURE: PM1`, `FEATURE: IA1`, `FEATURE: O7`,
  `FEATURE: Sec3`, and `FEATURE: WF1`;
  `companion_contracts run-extension-catalog-canonical` emits the deterministic
  catalog summary used by CI.
- `AdvancedPlannerContract` for remaining planner, tiering, time-travel,
  cursor, savepoint, regional, backup, federation, storage, and research-guard
  surfaces, including `FEATURE: T4`, `FEATURE: T10`, `FEATURE: T11`,
  `FEATURE: T13`, `FEATURE: T14`, `FEATURE: TS10`, `FEATURE: TS11`,
  `FEATURE: S1`, `FEATURE: S3`, `FEATURE: S8`, `FEATURE: S12`,
  `FEATURE: MR3`, `FEATURE: MR6`, `FEATURE: B4`, and `FEATURE: Sto2`;
  `companion_contracts run-advanced-planner-canonical` emits the deterministic
  execution summary used by CI.
- `canonical_advanced_planner_runtime_report` expands every advanced-planner
  surface into a deterministic runtime-boundary scenario, counts fail-closed
  duplicate/unknown/live-execution-claim checks, and keeps live distributed
  execution outside the claim. `companion_contracts
  run-advanced-planner-runtime-canonical` and
  `ci/ai-blaise/companion-advanced-planner-smoke.sh` emit the TSV evidence
  used by CI.
- `canonical_columnar_tiering_report` and
  `canonical_columnar_tiering_sql_plan` cover `FEATURE: L7`, `FEATURE: R3`,
  and `FEATURE: R8`. The live VM smoke
  `ci/ai-blaise/columnar-tiering-live-smoke.sh` creates a real distributed
  Citus `USING columnar` table, verifies `ColumnarScan`, checks worker-local
  `columnar` access method visibility, and records explicit nonclaims for
  cost-model tier selection, automatic tier movement, workload routing, and
  Kubernetes traffic.
- `OperationsReadinessContract` for Helm install, wrapper, runbook, MCP,
  security, realtime client, io_uring, and protocol pipeline gates, including
  `FEATURE: D7`, `FEATURE: D8`, `FEATURE: D9`, `FEATURE: D10`,
  `FEATURE: D11`, `FEATURE: MR9`, `FEATURE: RT5`, `FEATURE: S7`,
  `FEATURE: A9`, `FEATURE: Sec7`, `FEATURE: Sec8`, `FEATURE: Sec9`,
  `FEATURE: Sec13`, `FEATURE: T6`, and `FEATURE: T7`;
  `companion_contracts run-operations-canonical` emits the deterministic
  readiness summary used by CI.
- `COMPANION_FEATURE_STATUSES` as the canonical status table exposed by the
  pgrx `companion_feature_status()` function and by the SQL fallback extension
  packaged in the operand image.
- `canonical_companion_runtime_depth_a_report` executes the companion-owned
  migration, queue, and replication-conflict runtime evidence batch;
  `companion_runtime_depth_a run-canonical` emits the TSV summary used by the
  focused smoke script.

Default `cargo test -p ai_blaise_citus_companion` runs pure Rust validation.
Use `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- \
run-advanced-planner-canonical`, `cargo run -p ai_blaise_citus_companion \
--bin companion_contracts -- run-extension-catalog-canonical`, `cargo run -p \
ai_blaise_citus_companion --bin companion_contracts -- \
run-domain-contracts-canonical`, `cargo run -p ai_blaise_citus_companion \
--bin companion_contracts -- run-operations-canonical`, and `cargo run -p \
ai_blaise_citus_companion --bin companion_contracts -- \
run-plan-runtime-canonical` to emit TSV reports for the broad V2 companion
contracts.
The `pg18` feature exposes the first pgrx SQL-callable companion Timescale
bridge functions.
