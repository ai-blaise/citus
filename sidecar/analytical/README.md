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
- `canonical_analytical_execution_plan()`
- `cargo run -p ai_blaise_citus_sidecar_analytical -- run-canonical`

These contracts cover `FEATURE: L1`, `FEATURE: L2`, `FEATURE: L3`,
`FEATURE: L4`, `FEATURE: L5`, `FEATURE: L6`, `FEATURE: L8`, `FEATURE: L12`,
and `FEATURE: L13`.
