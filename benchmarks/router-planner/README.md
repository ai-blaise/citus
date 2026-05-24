# benchmarks/router-planner

Portable smoke benchmark for the Citus router patch series.

`bench.py` compares the legacy nested-loop placement intersection with the
hashed endpoint lookup introduced by `patches/0004-hashtable-on-planner-hotpath.patch`
and checks the conservative coordinator-skip decision used by
`patches/0006-fast-path-router-no-coord-rt.patch`.

This benchmark is algorithm evidence. `ci/ai-blaise/router-patch-smoke.sh`
combines it with an integrated Citus build and, when `REQUIRE_DOCKER=1`, a live
single-node Citus SQL probe. It still does not replace a live multi-worker
cluster or release performance numbers.

```sh
python3 benchmarks/router-planner/bench.py --quick
```

The result is written to `benchmarks/results/router-planner-<BENCH_RESULT_TAG>.json`.
