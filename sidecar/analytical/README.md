# sidecar/analytical

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
- `AnalyticalRuntimeReport`
- `canonical_analytical_execution_plan()`
- `canonical_analytical_runtime_report()`
- `cargo run -p ai_blaise_citus_sidecar_analytical -- run-canonical`
- `cargo run -p ai_blaise_citus_sidecar_analytical -- run-runtime-canonical`

These contracts cover `FEATURE: L1`, `FEATURE: L2`, `FEATURE: L3`,
`FEATURE: L4`, `FEATURE: L5`, `FEATURE: L6`, `FEATURE: L8`, `FEATURE: L12`,
and `FEATURE: L13`. The runtime flow validates mirror/object-store alignment,
DataFusion pushdown shape preservation, Iceberg snapshot commit reporting,
federated catalog publication, DuckDB extension loading, MotherDuck session
accounting, and logical-replication mirror materialization counters.
