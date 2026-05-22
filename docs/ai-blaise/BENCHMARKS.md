# Benchmarks

Benchmark results are recorded here per release.

These entries are benchmark targets, not production evidence, until measured
VM/container output is attached to the relevant release record and promoted in
`docs/ai-blaise/NEW_FEATURES.md`. The harness scaffolding lives under
`benchmarks/` (TPC-C, sysbench, Timescale ingest, chaos, plus the 26
per-bundled-extension microbenches under `benchmarks/microbenches/`) and the
CI smoke runners are `ci/ai-blaise/benchmark-smoke.sh` and
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`. Promotion to
production-ready requires entries in
`docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md` and a live 3-worker Citus
cluster recording.

## Harnesses

| Surface              | Directory                         | Quick-mode entry                              | Driver                          |
| -------------------- | --------------------------------- | --------------------------------------------- | ------------------------------- |
| OLTP TPC-C           | `benchmarks/tpcc/`                | `make -f Makefile.ai-blaise bench-tpcc`       | benchbase + pgbench fallback    |
| sysbench OLTP        | `benchmarks/sysbench/`            | `make -f Makefile.ai-blaise bench-sysbench`   | stock `sysbench` binary         |
| Timescale ingest     | `benchmarks/timescale-ingest/`    | `make -f Makefile.ai-blaise bench-timescale-ingest` | `psql COPY` driver        |
| Kubernetes chaos     | `benchmarks/chaos/`               | `make -f Makefile.ai-blaise bench-chaos`      | `kubectl` + `tc` + `NetworkPolicy` |
| Bundled-ext microbenches | `benchmarks/microbenches/<ext>/` | `make -f Makefile.ai-blaise microbench-smoke` | `psql` driver + per-ext SQL fixtures |

All harnesses share `benchmarks/common/lib.sh` for the quick-mode toggle,
results-directory layout, and the soft-skip pattern when a driver binary or
target cluster is missing.

## Per-extension microbenches (26 always-on bundled extensions)

Each of the 26 always-on bundled extensions ships a microbenchmark under
`benchmarks/microbenches/<ext>/` with four files: `setup.sql` (extension +
fixtures), `bench.sql` (measured workload, accepts `:row_count`), `bench.sh`
(timing wrapper that emits a single-line JSON record), and `baseline.json`
(seeded expected baseline with `regression_threshold_pct=10`). The aggregate
runner is `benchmarks/microbenches/run-all.sh`; the
baseline-comparison gate is `benchmarks/microbenches/compare-to-baseline.sh`.
Nightly full-row-count runs are driven by `.github/workflows/ci-microbench.yml`.

| MB ID | Extension | Operation                              | Initial baseline (qps unless noted)  | Workload size |
| ----- | --------- | -------------------------------------- | ------------------------------------ | ------------- |
| MB1   | timescaledb       | hypertable_insert_rows_per_s         | 50,000 rows/s                        | 100k rows / 7d |
| MB2   | citus             | distributed_insert_rows_per_s        | 30,000 rows/s                        | 100k rows      |
| MB3   | pgvector          | ivfflat_insert_then_lookup_qps       | 2,000                                | 1k vectors x 768d |
| MB4   | pg_cron           | job_schedule_overhead_ms             | 200 schedule/s                       | 100 jobs        |
| MB5   | pg_partman        | child_partition_create_ms            | 50 partitions/s                      | 100 partitions  |
| MB6   | pgaudit           | audited_insert_overhead_pct          | 8,500 rows/s (<= 15% overhead)       | 10k rows        |
| MB7   | pgsodium          | libsodium_encrypt_rows_per_s         | 5,000                                | 1k rows         |
| MB8   | postgresql-hll    | hll_add_agg_ms                       | 200,000                              | 100k distinct   |
| MB9   | postgresql-topn   | topn_add_agg_ms                      | 150,000                              | 100k rows       |
| MB10  | tdigest           | tdigest_percentile_ms                | 100,000                              | 100k samples    |
| MB11  | pgnodemx          | pgnodemx_cpu_invocation_us           | 5,000                                | 1k calls        |
| MB12  | postgis           | st_dwithin_qps                       | 4,000                                | 100k POINT      |
| MB13  | pg_search         | bm25_insert_index_lookup_qps         | 3,000                                | 100k docs       |
| MB14  | pg_graphql        | graphql_join_qps                     | 1,500                                | 10k rows joined |
| MB15  | pg_jsonschema     | jsonb_validate_per_s                 | 50,000                               | 10k JSONB       |
| MB16  | age               | cypher_path_qps                      | 800                                  | 1k-node graph   |
| MB17  | plrust            | plrust_function_call_us              | 200,000                              | 10k calls       |
| MB18  | plv8              | plv8_function_call_us                | 100,000                              | 10k calls       |
| MB19  | pg_uuidv7         | uuidv7_generations_per_s             | 1,000,000                            | 100k UUIDs      |
| MB20  | pg_repack         | repack_table_seconds                 | ~10 s end-to-end                     | 100k rows       |
| MB21  | pg_failover_slots | wal_write_overhead_pct               | 5,000 inserts/s (<= 5% overhead)     | 10k rows        |
| MB22  | pg_warm           | warm_throughput_mb_per_s             | ~1 GB/s                              | 100k-row proxy  |
| MB23  | pgcrypto          | pgp_sym_encrypt_rows_per_s           | 15,000                               | 10k rows        |
| MB24  | pg_trgm           | trigram_similarity_qps               | 5,000                                | 100k rows       |
| MB25  | citext            | citext_lookup_qps                    | 20,000                               | 100k rows       |
| MB26  | rum               | rum_fts_index_build_lookup_qps       | 4,000                                | 100k docs       |

Baseline values are seeded from upstream-published numbers (sources cited
inside each microbench's `README.md` and `baseline.json`). They are refined
to the 3-worker kind-cluster reality after the first nightly `ci-microbench`
run lands its measured aggregate.

## Initial harness target thresholds (alpha)

| Harness            | Metric                        | Threshold (alpha)              |
| ------------------ | ----------------------------- | ------------------------------ |
| TPC-C              | tpmC                          | > 5000 on a 3-worker kind cluster |
| TPC-C              | p99 latency                   | < 250 ms                       |
| TPC-C              | error rate                    | < 0.5%                         |
| sysbench (RO)      | TPS                           | > 20000                        |
| sysbench (RO)      | p95 latency                   | < 5 ms                         |
| sysbench (WO)      | TPS                           | > 8000                         |
| sysbench (WO)      | p95 latency                   | < 15 ms                        |
| sysbench (RW)      | TPS                           | > 12000                        |
| sysbench (RW)      | p95 latency                   | < 10 ms                        |
| sysbench (Point)   | TPS                           | > 50000                        |
| sysbench (Point)   | p95 latency                   | < 2 ms                         |
| Timescale ingest   | rows/s (compressed)           | > 10,000,000                   |
| Timescale ingest   | compression ratio             | > 6x                           |
| Timescale ingest   | lag (insert -> queryable)     | < 5 s                          |
| Chaos              | pool error rate during fault  | < 5%                           |
| Chaos              | recovery p99                  | < 5000 ms                      |
| Chaos              | lost commits                  | 0                              |

The first wave of measured runs is expected to fall short of these thresholds;
thresholds are tuned across release cycles.

## Run procedure

### Quick mode (CI smoke)

```sh
make -f Makefile.ai-blaise bench-smoke
```

Each harness caps at `BENCH_DURATION_SECS=10` and writes a JSON result under
`benchmarks/results/`. When a driver binary or the target endpoint is
unavailable, the harness writes a scaffold result (note flag set) and
soft-passes so the smoke remains green on the 2-core experiment VM.

### Full mode (nightly / release)

```sh
# Provision a 3-worker kind cluster first:
make -f Makefile.ai-blaise kind-production-smoke

# Run each harness in full mode; results land in benchmarks/results/.
cd benchmarks/tpcc && make full BENCH_DURATION_SECS=600 BENCH_CLIENTS=32 BENCH_SCALE=10
cd ../sysbench && BENCH_QUICK=0 BENCH_DURATION_SECS=300 BENCH_CLIENTS=16 BENCH_TABLES=16 BENCH_TABLE_SIZE=1000000 ./run-suite.sh
cd ../timescale-ingest && make full BENCH_DURATION_SECS=600 BENCH_ROWS=10000000 BENCH_SERIES=1024
cd ../chaos && BENCH_QUICK=0 CHAOS_NAMESPACE=ai-blaise-citus CHAOS_CLUSTER=primary ./run.sh

# 26 per-extension microbenches against the same kind cluster.
BENCH_QUICK=0 BENCH_RESULT_TAG=release \
  bash benchmarks/microbenches/run-all.sh
BENCH_QUICK=0 BENCH_RESULT_TAG=release \
  bash benchmarks/microbenches/compare-to-baseline.sh
```

Full-mode results are attached to the release record and tracked in
`docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`.

## Other measured surfaces (separate runners)

The TPC-C / sysbench / ingest / chaos harnesses cover gates 10 and 11. The
remaining release gates are exercised by other runners:

- distributed BM25 search latency: `cargo run -p ai_blaise_citus_e2e --bin
  release_gate_report` reports the modeled p95.
- Vectorizer lag (`FEATURE: A2`): `cargo run -p
  ai_blaise_citus_sidecar_vectorizer -- run-canonical`.
- HTAP cross-tier query latency: covered by the latency gate in
  `e2e/src/release_gates.rs`.
- Failover recovery time: chaos `kill-coordinator` scenario above.
