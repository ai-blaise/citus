# benchmarks

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Benchmark harnesses for the V2 release gates. The full V2 plan (acceptance
gates 10 and 11) requires measured throughput and chaos evidence on a live
3-worker Citus cluster. The scaffolding in this directory is the executable
entry point for that evidence; thresholds remain alpha until full runs land in
`docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`.

## Harnesses

| Directory                                    | Surface              | Driver                       | Quick-mode entry                              |
| -------------------------------------------- | -------------------- | ---------------------------- | --------------------------------------------- |
| [`tpcc/`](tpcc/README.md)                    | OLTP TPC-C           | benchbase + pgbench fallback | `make -f Makefile.ai-blaise bench-tpcc`       |
| [`sysbench/`](sysbench/README.md)            | sysbench OLTP suite  | stock `sysbench` binary      | `make -f Makefile.ai-blaise bench-sysbench`   |
| [`timescale-ingest/`](timescale-ingest/README.md) | Timescale ingest | `psql` COPY + Python driver  | `make -f Makefile.ai-blaise bench-timescale-ingest` |
| [`chaos/`](chaos/README.md)                  | Kubernetes chaos     | `kubectl` + `iptables` shell | `make -f Makefile.ai-blaise bench-chaos`      |

The shared scaffolding lives in [`common/lib.sh`](common/lib.sh): result
directory layout, quick-mode toggles, soft-skip behaviour when a driver tool
is missing, and the Postgres connection defaults.

## Run modes

### Quick mode (CI smoke)

`ci/ai-blaise/benchmark-smoke.sh` runs every harness in quick mode with:

- duration: `BENCH_DURATION_SECS=10`
- clients: `BENCH_CLIENTS=2`
- scale: `BENCH_SCALE=1`

A quick-mode run that cannot reach a Postgres endpoint or a Kubernetes cluster
is treated as a soft pass; the harness still verifies that the script,
configuration, and any vendored tooling are syntactically valid. Quick mode is
the bar for the CI overlays, not the V2 acceptance threshold.

### Full mode (nightly / release)

Full mode targets a 3-worker `kind` cluster provisioned by
`ci/ai-blaise/kind-production-smoke.sh`. Each harness uses the documented full
defaults (see the per-harness `README.md`).

Set `BENCH_QUICK=0` and override `BENCH_DURATION_SECS`, `BENCH_CLIENTS`,
`BENCH_SCALE`, and `BENCH_RESULT_TAG=release-<n>` before invoking the harness.

## Results

Each harness writes JSON to
`benchmarks/results/<harness>-<BENCH_RESULT_TAG>.json`. Schemas:

- `tpcc-*.json`: `{tpmC, latency_ms: {p50,p95,p99}, errors, duration_s, mode}`
- `sysbench-*.json`: `{tps, latency_ms_p95, workload, duration_s, mode}`
- `timescale-ingest-*.json`: `{rows_per_s, compression_ratio, lag_ms, duration_s, mode}`
- `chaos-*.json`: `{scenario, traffic_error_rate, recovery_p99_ms, data_intact, mode}`

The `mode` field is `quick` for CI smoke and `release-<n>` for full runs.

## Thresholds

Initial acceptance thresholds (alpha) are documented in
`docs/ai-blaise/BENCHMARKS.md`. They are tuned iteratively; production-grade
thresholds require an entry in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
and a `Status: production-ready` promotion in
`docs/ai-blaise/NEW_FEATURES.md`.
