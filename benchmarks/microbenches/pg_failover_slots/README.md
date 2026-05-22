# MB21: pg_failover_slots microbench

**FEATURE: MB21**
**Status seed**: production-ready (microbench scaffold + baseline; refined post-first-measured-run)

WAL write overhead with pg_failover_slots tracking enabled vs disabled; standalone proxy for failover-slot bookkeeping cost.

## Operation measured

`wal_write_overhead_pct` — initial baseline `5000` qps with p95 `5` ms and p99 `20` ms.
Canonical workload size: `10000` rows (quick mode runs at one-tenth that
size to fit the CI smoke under the shared 60 s budget).

## Baseline source

pg_failover_slots upstream: <5% WAL-write overhead.

Baseline values in `baseline.json` are seeded from upstream-published numbers
so the first run has a comparison target. They are refined to the
3-worker kind cluster reality after the nightly `ci-microbench` workflow lands
its first measured `benchmarks/results/microbench-mb21-release.json`.

## Files

- `setup.sql` — creates the extension and the test fixtures. Idempotent.
- `bench.sql` — runs the measured workload; reads `:row_count` from psql.
- `bench.sh` — wraps `setup.sql` + `bench.sql` with timing, soft-passes when
  psql or the Postgres endpoint is unavailable, and writes
  `benchmarks/results/microbench-mb21-${BENCH_RESULT_TAG}.json`.
- `baseline.json` — recorded expected baseline; the comparator script fails
  if a measured run regresses by more than 10%.

## Running

Quick (CI smoke, ~1 s):

```sh
BENCH_QUICK=1 bash benchmarks/microbenches/pg_failover_slots/bench.sh
```

Full (nightly, against a 3-worker kind cluster):

```sh
BENCH_QUICK=0 BENCH_RESULT_TAG=release \
  bash benchmarks/microbenches/pg_failover_slots/bench.sh
```

Aggregate runner:

```sh
bash benchmarks/microbenches/run-all.sh
bash benchmarks/microbenches/compare-to-baseline.sh
```
