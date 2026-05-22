# MB22: pg_warm microbench

**FEATURE: MB22**
**Status seed**: production-ready (microbench scaffold + baseline; refined post-first-measured-run)

pg_prewarm a 100k-row table (proxy for the 10GB full-mode workload); reports warming throughput.

## Operation measured

`warm_throughput_mb_per_s` — initial baseline `1000` qps with p95 `1` ms and p99 `5` ms.
Canonical workload size: `100000` rows (quick mode runs at one-tenth that
size to fit the CI smoke under the shared 60 s budget).

## Baseline source

pg_prewarm upstream: ~1 GB/s warming on local NVMe.

Baseline values in `baseline.json` are seeded from upstream-published numbers
so the first run has a comparison target. They are refined to the
3-worker kind cluster reality after the nightly `ci-microbench` workflow lands
its first measured `benchmarks/results/microbench-mb22-release.json`.

## Files

- `setup.sql` — creates the extension and the test fixtures. Idempotent.
- `bench.sql` — runs the measured workload; reads `:row_count` from psql.
- `bench.sh` — wraps `setup.sql` + `bench.sql` with timing, soft-passes when
  psql or the Postgres endpoint is unavailable, and writes
  `benchmarks/results/microbench-mb22-${BENCH_RESULT_TAG}.json`.
- `baseline.json` — recorded expected baseline; the comparator script fails
  if a measured run regresses by more than 10%.

## Running

Quick (CI smoke, ~1 s):

```sh
BENCH_QUICK=1 bash benchmarks/microbenches/pg_warm/bench.sh
```

Full (nightly, against a 3-worker kind cluster):

```sh
BENCH_QUICK=0 BENCH_RESULT_TAG=release \
  bash benchmarks/microbenches/pg_warm/bench.sh
```

Aggregate runner:

```sh
bash benchmarks/microbenches/run-all.sh
bash benchmarks/microbenches/compare-to-baseline.sh
```
