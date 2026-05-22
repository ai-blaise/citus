# benchmarks/timescale-ingest

Timescale ingest harness for the V2 performance acceptance gate (gate 10).

## Driver

[`ingest.py`](ingest.py) drives `psql COPY ... FROM STDIN` into a hypertable
managed by the Timescale CRD reconciler (`FEATURE: TS7`). The COPY stream
emits `(ts, series_id, value)` triples, fanned out across `BENCH_SERIES`
distinct series IDs.

When `timescaledb_parallel_copy` is installed and configured, set
`BENCH_INGEST_DRIVER=parallel-copy` to use the parallel loader; the default
psql COPY path is intentionally light so the harness runs on the 2-core
experiment VM.

## Target

The full V2 target is 10M rows/s compressed insert against a 3-worker kind
cluster with `chunk_time_interval = 1h`. Quick-mode CI smoke only verifies
that the harness can connect to Postgres, create the hypertable, and write a
result JSON.

## Cohabitation profile

The harness assumes the Timescale extension is loaded via the Citus
cohabit-mode bridge documented in `docs/ai-blaise/COHABITATION.md`. The
`bench_metric` table is created locally on the coordinator; for a distributed
hypertable run, set up `create_distributed_hypertable` ahead of time via the
operator (see `ai_blaise_citus_operator` Hypertable CRD).

## Acceptance thresholds (alpha)

| Metric                    | Threshold (alpha) |
| ------------------------- | ----------------- |
| rows/s (compressed)       | > 10,000,000      |
| compression ratio         | > 6x              |
| lag (insert -> queryable) | < 5 s             |

The compression ratio surface in quick mode is a placeholder; real numbers
come from `timescaledb_information.hypertable_compression_stats` in full runs.

## Run

```sh
# Quick mode (CI smoke):
make -f Makefile.ai-blaise bench-timescale-ingest

# Full release run:
cd benchmarks/timescale-ingest
make full BENCH_DURATION_SECS=600 BENCH_ROWS=10000000 BENCH_SERIES=1024
```

## Result file

`benchmarks/results/timescale-ingest-<BENCH_RESULT_TAG>.json` with schema:

```json
{
  "rows_per_s": 1234567,
  "compression_ratio": 6.4,
  "lag_ms": 320,
  "duration_s": 600,
  "mode": "release"
}
```
