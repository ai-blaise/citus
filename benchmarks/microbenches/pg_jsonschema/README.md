# MB15: pg_jsonschema microbench

**FEATURE: MB15**
**Status seed**: production-ready (microbench scaffold + baseline; refined post-first-measured-run)

Validate 10k JSONB rows against a fixed JSON schema with jsonschema_is_valid.

## Operation measured

`jsonb_validate_per_s` — initial baseline `50000` qps with p95 `0.5` ms and p99 `2` ms.
Canonical workload size: `10000` rows (quick mode runs at one-tenth that
size to fit the CI smoke under the shared 60 s budget).

## Baseline source

pg_jsonschema upstream: ~50k validations/s on simple schemas.

Baseline values in `baseline.json` are seeded from upstream-published numbers
so the first run has a comparison target. They are refined to the
3-worker kind cluster reality after the nightly `ci-microbench` workflow lands
its first measured `benchmarks/results/microbench-mb15-release.json`.

## Files

- `setup.sql` — creates the extension and the test fixtures. Idempotent.
- `bench.sql` — runs the measured workload; reads `:row_count` from psql.
- `bench.sh` — wraps `setup.sql` + `bench.sql` with timing, soft-passes when
  psql or the Postgres endpoint is unavailable, and writes
  `benchmarks/results/microbench-mb15-${BENCH_RESULT_TAG}.json`.
- `baseline.json` — recorded expected baseline; the comparator script fails
  if a measured run regresses by more than 10%.

## Running

Quick (CI smoke, ~1 s):

```sh
BENCH_QUICK=1 bash benchmarks/microbenches/pg_jsonschema/bench.sh
```

Full (nightly, against a 3-worker kind cluster):

```sh
BENCH_QUICK=0 BENCH_RESULT_TAG=release \
  bash benchmarks/microbenches/pg_jsonschema/bench.sh
```

Aggregate runner:

```sh
bash benchmarks/microbenches/run-all.sh
bash benchmarks/microbenches/compare-to-baseline.sh
```
