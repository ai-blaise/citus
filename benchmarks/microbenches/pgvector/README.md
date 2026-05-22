# MB3: pgvector microbench

**FEATURE: MB3**
**Status seed**: production-ready (microbench scaffold + baseline; refined post-first-measured-run)

1k INSERT of 768-dim vectors followed by 1k IVFFlat ANN lookups.

## Operation measured

`ivfflat_insert_then_lookup_qps` — initial baseline `2000` qps with p95 `5` ms and p99 `15` ms.
Canonical workload size: `1000` rows (quick mode runs at one-tenth that
size to fit the CI smoke under the shared 60 s budget).

## Baseline source

pgvector benchmarks: IVFFlat 768-dim lookup ~2k qps single connection.

Baseline values in `baseline.json` are seeded from upstream-published numbers
so the first run has a comparison target. They are refined to the
3-worker kind cluster reality after the nightly `ci-microbench` workflow lands
its first measured `benchmarks/results/microbench-mb3-release.json`.

## Files

- `setup.sql` — creates the extension and the test fixtures. Idempotent.
- `bench.sql` — runs the measured workload; reads `:row_count` from psql.
- `bench.sh` — wraps `setup.sql` + `bench.sql` with timing, soft-passes when
  psql or the Postgres endpoint is unavailable, and writes
  `benchmarks/results/microbench-mb3-${BENCH_RESULT_TAG}.json`.
- `baseline.json` — recorded expected baseline; the comparator script fails
  if a measured run regresses by more than 10%.

## Running

Quick (CI smoke, ~1 s):

```sh
BENCH_QUICK=1 bash benchmarks/microbenches/pgvector/bench.sh
```

Full (nightly, against a 3-worker kind cluster):

```sh
BENCH_QUICK=0 BENCH_RESULT_TAG=release \
  bash benchmarks/microbenches/pgvector/bench.sh
```

Aggregate runner:

```sh
bash benchmarks/microbenches/run-all.sh
bash benchmarks/microbenches/compare-to-baseline.sh
```
