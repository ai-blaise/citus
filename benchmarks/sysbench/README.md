# benchmarks/sysbench

sysbench OLTP suite for the V2 performance acceptance gate (gate 10).

## Workloads

| Workload          | Read/Write split |
| ----------------- | ---------------- |
| `oltp_read_only`  | 100% reads       |
| `oltp_write_only` | 100% writes      |
| `oltp_read_write` | 70/30            |
| `oltp_point_select` | PK point reads |

## Driver

Stock `sysbench` (Debian: `apt-get install -y sysbench`). On a stripped-down VM
or CI runner without sysbench installed, quick-mode falls back to writing a
scaffold result JSON so the harness wiring stays exercisable.

## Schema distribution

`run-suite.sh` calls `create_distributed_table('sbtestN', 'id')` for each
`sbtestN` table after the sysbench `prepare` step. Sharding on the primary key
gives even distribution for point reads and writes; quick mode runs with
`BENCH_TABLES=4` and `BENCH_TABLE_SIZE=10000`.

## Acceptance thresholds (alpha)

| Workload          | TPS threshold | p95 latency threshold |
| ----------------- | ------------- | --------------------- |
| `oltp_read_only`  | > 20000       | < 5 ms                |
| `oltp_write_only` | > 8000        | < 15 ms               |
| `oltp_read_write` | > 12000       | < 10 ms               |
| `oltp_point_select` | > 50000     | < 2 ms                |

## Run

```sh
# Quick mode (CI smoke):
make -f Makefile.ai-blaise bench-sysbench

# Full release run:
BENCH_QUICK=0 BENCH_DURATION_SECS=300 BENCH_CLIENTS=16 \
  BENCH_TABLES=16 BENCH_TABLE_SIZE=1000000 \
  ./run-suite.sh
```

## Result files

`benchmarks/results/sysbench-<workload>-<BENCH_RESULT_TAG>.json`, one per
workload, with schema:

```json
{
  "workload": "oltp_read_write",
  "tps": 12345.6,
  "latency_ms_p95": 8.4,
  "duration_s": 300,
  "mode": "release"
}
```
