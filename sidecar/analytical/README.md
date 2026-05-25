# sidecar/analytical

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Analytical sidecar based on the `pg_lake` sidecar pattern, with DataFusion,
Iceberg, Arrow, Parquet, and object-store backends.

Current implemented surface:

- `AnalyticalSidecarPlan`
- `LakehouseReadPlan`
- `DataFusionPushdownPlan`
- `IcebergSnapshotCommitPlan`
- `FederatedCatalog`
- `DuckDbExtensionCatalog`
- `MotherDuckConnector`
- `AnalyticalRuntime`
- `AnalyticalRuntimePolicy`
- `AnalyticalRuntimeReport`
- `canonical_analytical_execution_plan()`
- `canonical_analytical_runtime_report()`
- `cargo run -p ai_blaise_citus_sidecar_analytical -- run-canonical`
- `cargo run -p ai_blaise_citus_sidecar_analytical -- run-runtime-canonical`
- `cargo run -p ai_blaise_citus_sidecar_analytical -- serve`
- `bash ci/ai-blaise/sidecar-analytical-smoke.sh`

These contracts cover `FEATURE: L1`, `FEATURE: L2`, `FEATURE: L3`,
`FEATURE: L4`, `FEATURE: L5`, `FEATURE: L6`, `FEATURE: L8`, `FEATURE: L12`,
and `FEATURE: L13`. The runtime flow validates mirror/object-store alignment,
DataFusion pushdown shape preservation, Iceberg snapshot commit reporting,
federated catalog publication, DuckDB extension loading, MotherDuck session
accounting, and logical-replication mirror materialization counters.

Current runtime executes a bounded local DataFusion query over an Arrow
`RecordBatch` and reports `external_io_attempted=false`,
`query_engine_executed=true`, `datafusion_output_rows=2`,
`projection_pushdown_executed=true`, `filter_pushdown_executed=true`,
`limit_pushdown_executed=true`, and
`evidence_boundary=local-datafusion-recordbatch-only`. This is production
evidence only for `FEATURE: L2` and `FEATURE: L4` under that local in-process
runtime boundary. The smoke also starts the loopback probe server and verifies
health, readiness, metrics, and drain behavior. `FEATURE: L1`, `FEATURE: L3`,
`FEATURE: L5`, `FEATURE: L6`, `FEATURE: L8`, `FEATURE: L12`, and
`FEATURE: L13` remain alpha; this is not production evidence for pg_lake,
DuckDB, MotherDuck, Iceberg commits, object-store IO, logical-replication
mirror materialization, Kubernetes traffic, or Citus planner integration.
