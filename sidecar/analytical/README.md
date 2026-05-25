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
- `cargo run -p ai_blaise_citus_sidecar_analytical -- run-local-parquet-read-canonical`
- `cargo run -p ai_blaise_citus_sidecar_analytical -- run-local-iceberg-snapshot-commit-canonical`
- `cargo run -p ai_blaise_citus_sidecar_analytical -- run-logical-mirror-materialization-from-stdin`
- `cargo run -p ai_blaise_citus_sidecar_analytical -- run-duckdb-extension-catalog-canonical`
- `cargo run -p ai_blaise_citus_sidecar_analytical -- run-federation-catalog-publication-canonical`
- `cargo run -p ai_blaise_citus_sidecar_analytical -- serve`
- `bash ci/ai-blaise/sidecar-analytical-smoke.sh`
- `bash ci/ai-blaise/sidecar-analytical-parquet-read-smoke.sh`
- `bash ci/ai-blaise/sidecar-analytical-iceberg-snapshot-smoke.sh`
- `REQUIRE_DOCKER=1 bash ci/ai-blaise/sidecar-analytical-mirror-live-smoke.sh`
- `REQUIRE_DOCKER=1 bash ci/ai-blaise/sidecar-analytical-duckdb-extension-live-smoke.sh`
- `bash ci/ai-blaise/sidecar-analytical-federation-catalog-live-smoke.sh`

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
health, readiness, metrics, and drain behavior.

`FEATURE: L3` has separate bounded production evidence:
`sidecar-analytical-parquet-read-smoke.sh` runs
`run-local-parquet-read-canonical`, writes a real local Parquet file with
`ArrowWriter`, registers that file through DataFusion `ParquetReadOptions`, and
queries it with projection, filter, ordering, and limit. The smoke requires
`parquet_lakehouse_read_live=passed`, `l3_local_parquet_file_created=true`,
`l3_datafusion_parquet_read_executed=true`, `l3_source_rows=4`,
`l3_source_total=5500`, `l3_datafusion_output_rows=2`,
`l3_datafusion_output_total=3000`, and
`evidence_boundary=local-datafusion-parquet-file-only`. This is production
evidence only for local Parquet file materialization and local DataFusion
Parquet reads; it does not cover Iceberg runtime reads, Delta runtime reads,
object-store IO, pg_lake, MotherDuck, Citus planner integration, or Kubernetes
traffic.

`FEATURE: L5` has separate bounded production evidence:
`sidecar-analytical-iceberg-snapshot-smoke.sh` runs
`run-local-iceberg-snapshot-commit-canonical`, writes a local manifest JSON, a
local Iceberg-style metadata JSON, and a `current-snapshot.txt` pointer using
temp-file plus atomic rename and fsync. The smoke reads the artifacts back and
requires `iceberg_snapshot_commit_live=passed`,
`l5_local_metadata_written=true`, `l5_local_manifest_written=true`,
`l5_current_pointer_committed=true`, `l5_prepare_lsn_recorded=true`,
`l5_snapshot_metadata_round_tripped=true`, `atomic_rename_used=true`,
`fsync_executed=true`, and
`evidence_boundary=local-iceberg-snapshot-metadata-commit-only`. This is
production evidence only for a local prepare-LSN metadata commit primitive; it
does not cover live Iceberg catalog commits, object-store IO, a Citus prepare
hook, multi-writer conflict detection, warehouse federation, or Kubernetes
traffic.

`FEATURE: L8` has separate bounded production evidence:
`sidecar-analytical-mirror-live-smoke.sh` starts a real PostgreSQL 17 container
with `wal_level=logical`, creates a `test_decoding` logical slot, inserts three
rows into `public.l8_orders`, consumes `pg_logical_slot_get_changes`, runs
`run-logical-mirror-materialization-from-stdin`, writes a local TSV mirror
artifact, registers that `.tsv` artifact through `CsvReadOptions`, and queries
it through DataFusion. The live smoke
requires `logical_mirror_live=passed`, `l8_test_decoding_slot_consumed=true`,
`l8_materialized_rows=3`, `l8_materialized_total=6000`, and
`l8_datafusion_mirror_query_executed=true`.

`FEATURE: L12` has separate bounded production evidence:
`sidecar-analytical-duckdb-extension-live-smoke.sh` runs the canonical DuckDB
extension allow-list against a pinned real DuckDB container and requires
`duckdb_extension_catalog_live=passed`, `l12_extensions_installed=2`,
`l12_extensions_loaded=2`, and `l12_duckdb_extensions_catalog_queried=true`.

`FEATURE: L6` has separate bounded production evidence:
`sidecar-analytical-federation-catalog-live-smoke.sh` runs
`run-federation-catalog-publication-canonical`, writes the v1 JSON federation
catalog for Databricks, Snowflake, Trino, and Spark, validates the JSON, serves it
over loopback HTTP, fetches it with `curl`, and byte-compares the fetched payload
with the generated artifact. The smoke requires
`federation_catalog_publication_live=passed`, `l6_catalog_version=v1`,
`l6_catalog_count=4`,
`l6_federation_targets=databricks,snowflake,trino,spark`,
`l6_local_catalog_artifact_created=true`, `l6_local_http_catalog_served=true`,
and `evidence_boundary=local-federation-catalog-artifact-http-only`.

`FEATURE: L1` and `FEATURE: L13` remain alpha. The L3 local Parquet path
remains explicitly bounded: `external_io_attempted=false`,
`object_store_io_attempted=false`, `iceberg_runtime_exercised=false`,
`delta_runtime_exercised=false`, `pg_lake_runtime_exercised=false`,
`motherduck_session_exercised=false`, and `kubernetes_traffic_exercised=false`.
The L5 local snapshot path is bounded: `iceberg_catalog_commit_exercised=false`,
`object_store_io_attempted=false`, `citus_prepare_hook_exercised=false`,
`multi_writer_conflict_detection_exercised=false`, and
`kubernetes_traffic_exercised=false`.
The L8 mirror path remains explicitly bounded:
`object_store_io_attempted=false`, `long_running_slot_tailing=false`,
`checkpoint_persistence_exercised=false`, and `kubernetes_traffic_exercised=false`.
The L12 DuckDB path is also bounded: `pg_duckdb_runtime_exercised=false`,
`motherduck_session_exercised=false`, `object_store_io_attempted=false`, and
`extension_repository_mirror_verified=false`. The L6 federation catalog path is
bounded to local artifact publication and loopback HTTP serving:
`external_warehouse_connections_attempted=false`, `object_store_io_attempted=false`,
and `catalog_auth_exercised=false`. These are not production evidence for
pg_lake, pg_duckdb inside PostgreSQL, MotherDuck, Iceberg commits, live Snowflake,
live Trino, live Spark, live Databricks, warehouse connections, catalog
authentication, object-store catalog reads, F3 warehouse federation, Kubernetes
traffic, Citus planner integration, or a long-running logical-replication mirror
daemon.
