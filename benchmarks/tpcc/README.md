# benchmarks/tpcc

TPC-C harness for the V2 performance acceptance gate (gate 10).

## Driver

The canonical driver is [benchbase](https://github.com/cmu-db/benchbase). The
harness invokes `benchbase -b tpcc -c config.xml`. The config is parameterised
on `BENCH_PGHOST`, `BENCH_PGPORT`, `BENCH_PGUSER`, `BENCH_PGPASSWORD`,
`BENCH_PGDATABASE`, `BENCH_SCALE`, `BENCH_CLIENTS`, `BENCH_DURATION_SECS`, and
`BENCH_WARMUP_SECS`.

If benchbase is not on `PATH`, the harness falls back to a `pgbench` TPC-B run
against the same endpoint, recording the result with a `note` flag so the
result file is unambiguously a fallback rather than a real TPC-C measurement.
The fallback is only used for CI smoke; full V2 acceptance requires benchbase.

## Schema distribution

[`schema/distribute.sql`](schema/distribute.sql) applies the Citus-aware
distribution after benchbase loads the base tables. Warehouse-keyed tables are
co-located via `colocate_with => 'warehouse'`, the item catalogue is a
reference table.

## Acceptance thresholds (alpha)

| Metric                | Threshold (alpha)             |
| --------------------- | ----------------------------- |
| tpmC                  | > 5000 on a 3-worker kind cluster |
| p99 latency (ms)      | < 250 ms                      |
| Error rate            | < 0.5%                        |

Thresholds are tuned iteratively as full runs land. Promotion to
production-ready requires entries in
`docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`.

## Run

```sh
# Quick mode (CI smoke, ~10s):
make -f Makefile.ai-blaise bench-tpcc

# Full release run:
cd benchmarks/tpcc
make full BENCH_DURATION_SECS=600 BENCH_CLIENTS=32 BENCH_SCALE=10
```

## Result file

`benchmarks/results/tpcc-<BENCH_RESULT_TAG>.json` with schema:

```json
{
  "tpmC": 5234,
  "latency_ms": {"p50": 12.4, "p95": 78.1, "p99": 215.0},
  "errors": 0,
  "duration_s": 600,
  "mode": "release"
}
```
