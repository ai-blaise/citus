# ADR 0004: Fork pg_lake as the Analytical Substrate

## Status

Accepted (2026-05-21)

## Context

The overlay needs an analytical engine that speaks Postgres wire,
addresses Iceberg / Delta / Parquet through object storage, and runs
out-of-process so an OOM in the analytical engine does not take down
the OLTP backend. TigerData's sunset of Hypercore TAM (2.21 → 2.22)
and the broader industry shift toward FDW-style external columnar
engines validate the out-of-process direction. The choice is whether
to build the analytical engine from scratch or fork an Apache 2.0
upstream that already integrates with Postgres.

## Decision

Fork `Snowflake-Labs/pg_lake` (Apache 2.0) as the analytical substrate
and rewrite its `pgduck_server` component in Rust under
`sidecar/analytical`. The Rust rewrite uses `datafusion 48.x` as the
primary engine, `duckdb-rs` as an alternate engine for workloads where
DuckDB's vectorized executor wins, `iceberg-rust 0.9.1` as the table
format, `arrow-rs` for in-memory representation,
`object_store`/`opendal` for S3/GCS/Azure access, `parquet` for the
file format, and `delta-kernel-rs` for Delta tables. The sidecar speaks
the Postgres wire protocol on `localhost:5332`; Citus shards reach it
through pg_lake's `pg_lake_table` FDW. Crash-isolation is enforced by
the process boundary.

## Alternatives considered

- DuckDB as the primary engine (`duckdb-rs` only). Rejected as primary
  but kept as alternate — DuckDB's single-process model and its lack
  of distributed execution make it the wrong base when we want to fan
  out across pods. DataFusion's `SessionContext` plus
  `iceberg-datafusion` slots into a sidecar fleet more cleanly.
- DataFusion only, no DuckDB. Rejected — DuckDB still wins on a
  meaningful slice of tight aggregate workloads, and the `duckdb-rs`
  binding is small enough to keep alongside.
- Vendor pg_duckdb (Hydra) instead of pg_lake. Rejected — `pg_duckdb`
  embeds DuckDB inside Postgres, defeating the crash-isolation goal,
  and its Iceberg story is weaker.
- Build an analytical engine from scratch on Arrow. Rejected — years
  of work to reach pg_lake's current Iceberg / Parquet / wire-protocol
  coverage.

## Consequences

- Positive: Iceberg becomes a first-class table format. Citus
  distributed tables and Iceberg tables coexist; the pool's HTAP
  classifier (§6.4) routes analytical queries to
  `sidecar/analytical` and OLTP queries to Postgres.
- Positive: the analytical process is crash-isolated. OOM, panic, or
  long GC in DataFusion cannot destabilize the Postgres backend.
- Positive: object storage is the durable layer for the analytical
  side, so the cold-tier sidecar (`coldtier/`) and the analytical
  sidecar share `object_store` and Parquet plumbing.
- Negative: cross-engine queries (joining a Citus distributed table
  against an Iceberg table) pay an FDW hop and lose some pushdown.
  Mitigation: the pool's classifier prefers to land both sides on the
  analytical engine when the analytical share is large.
- Risks: DataFusion's optimizer is younger than Postgres planner;
  surprising plans will surface. Mitigation: a query-shape regression
  fixture set in `benchmarks/` and a plan-pinning facility in the
  classifier.

## References

- Plan §6.3.1 (`sidecar/analytical`)
- Plan §4.9 (HTAP without dual-write)
- `Snowflake-Labs/pg_lake` — Apache 2.0 upstream
- `apache/datafusion` 48.x release notes
- `apache/iceberg-rust` 0.9.1
- `delta-io/delta-kernel-rs`
