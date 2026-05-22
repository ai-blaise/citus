# Benchmarks

The harness scaffolding lives under `benchmarks/` (TPC-C, sysbench, Timescale
ingest, chaos). Per-run JSON outputs land in `benchmarks/results/`; aggregated
release-bound baselines land in `benchmarks/baselines/<date>-baseline.json` and
are referenced from `e2e/src/release_gates.rs` (see
`PERFORMANCE_BASELINE_PATH`). The CI smoke runner is
`ci/ai-blaise/benchmark-smoke.sh`; the nightly aggregate workflow is
`.github/workflows/ci-baseline-nightly.yml`.

The first round of measured baselines (2026-05-22) was collected on the
`experiment-playground` 2-core / 7 GB-RAM VM. The constrained-host numbers are
recorded honestly: TPC-C (pgbench TPC-B fallback) and Timescale ingest exceed
their gate-10 targets; sysbench OLTP read/write and read-only fall short of the
2 000 / 5 000 TPS targets because the VM has only two CPUs to share between the
database and the load generator. Production-grade hosts must re-baseline before
claiming the unwaived targets; the gate-10 + gate-11 entries in
`release_gates.rs` carry explicit `with_waiver` reasons for each constrained
scenario so the canonical report still reflects what was measured.

Constrained-host and quick-mode numbers are benchmark targets, not production
evidence, until measured runs from a production-grade host (≥ 8 vCPU, a
dedicated load-generator host, and a 3-worker Citus + Timescale cohabit) land
in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md` and the corresponding feature
status flips in `docs/ai-blaise/NEW_FEATURES.md`. The constrained-host entries
here close the gate-10 + gate-11 evidence loop for the canonical release-gate
report; they do not promote the unwaived targets to a production claim.

## Harnesses

| Surface              | Directory                         | Quick-mode entry                              | Driver                          |
| -------------------- | --------------------------------- | --------------------------------------------- | ------------------------------- |
| OLTP TPC-C           | `benchmarks/tpcc/`                | `make -f Makefile.ai-blaise bench-tpcc`       | benchbase + pgbench fallback    |
| sysbench OLTP        | `benchmarks/sysbench/`            | `make -f Makefile.ai-blaise bench-sysbench`   | stock `sysbench` binary         |
| Timescale ingest     | `benchmarks/timescale-ingest/`    | `make -f Makefile.ai-blaise bench-timescale-ingest` | `psql COPY` driver        |
| Kubernetes chaos     | `benchmarks/chaos/`               | `make -f Makefile.ai-blaise bench-chaos`      | `kubectl` + `tc` + `NetworkPolicy` |

All four harnesses share `benchmarks/common/lib.sh` for the quick-mode toggle,
results-directory layout, and the soft-skip pattern when a driver binary or
target cluster is missing.

## Gate-10 / gate-11 acceptance thresholds

Gates 10 and 11 in `e2e/src/release_gates.rs` enforce a narrow subset of the
threshold matrix below. These are the numbers the canonical report compares
each measured baseline against; the full "ambitious" alpha targets sit underneath
as long-term goals.

| Gate / metric                            | Target            | Source constant in `release_gates.rs`           |
| ---------------------------------------- | ----------------- | ----------------------------------------------- |
| TPC-C tpmC                               | > 5 000           | `PERFORMANCE_TARGET_TPCC_TPM_C`                 |
| sysbench OLTP read/write TPS             | > 2 000           | `PERFORMANCE_TARGET_SYSBENCH_RW_TPS`            |
| sysbench OLTP read-only TPS              | > 5 000           | `PERFORMANCE_TARGET_SYSBENCH_RO_TPS`            |
| Timescale compressed ingest rows/s       | > 100 000         | `PERFORMANCE_TARGET_TIMESCALE_INGEST_ROWS_PER_S` |
| Chaos recovery p99 (each scenario)       | < 5 000 ms        | `CHAOS_RECOVERY_P99_MS`                         |

## Ambitious "alpha" thresholds (release-cycle stretch goals)

The wider production-grade table below is what the harness families aim for
once running on a beefier host; not enforced by the V2 acceptance gate today.

| Harness            | Metric                        | Threshold (alpha)              |
| ------------------ | ----------------------------- | ------------------------------ |
| TPC-C              | tpmC                          | > 5 000 on a 3-worker kind cluster |
| TPC-C              | p99 latency                   | < 250 ms                       |
| TPC-C              | error rate                    | < 0.5%                         |
| sysbench (RO)      | TPS                           | > 20 000                       |
| sysbench (RO)      | p95 latency                   | < 5 ms                         |
| sysbench (WO)      | TPS                           | > 8 000                        |
| sysbench (WO)      | p95 latency                   | < 15 ms                        |
| sysbench (RW)      | TPS                           | > 12 000                       |
| sysbench (RW)      | p95 latency                   | < 10 ms                        |
| sysbench (Point)   | TPS                           | > 50 000                       |
| sysbench (Point)   | p95 latency                   | < 2 ms                         |
| Timescale ingest   | rows/s (compressed)           | > 10 000 000                   |
| Timescale ingest   | compression ratio             | > 6x                           |
| Timescale ingest   | lag (insert -> queryable)     | < 5 s                          |
| Chaos              | pool error rate during fault  | < 5%                           |
| Chaos              | recovery p99                  | < 5 000 ms                     |
| Chaos              | lost commits                  | 0                              |

## Recorded baselines

### 2026-05-22 (constrained host: experiment-playground VM, 2 cores / 7 GB RAM)

Source baseline: [`benchmarks/baselines/2026-05-22-baseline.json`](https://github.com/ai-blaise/citus/blob/main/benchmarks/baselines/2026-05-22-baseline.json).
Main SHA: `0b366b2973`. Driver versions: PostgreSQL 17.10 + TimescaleDB 2.27.1
(single-node), pgbench 17.10, sysbench 1.0.20, Python 3.11.

| Harness                                  | Metric            | Recorded          | Gate-10/11 target | Status                       |
| ---------------------------------------- | ----------------- | ----------------- | ----------------- | ---------------------------- |
| TPC-C (pgbench TPC-B fallback)           | tpmC              | 31 166            | > 5 000           | exceeds target               |
| TPC-C (pgbench TPC-B fallback)           | p99 latency       | 15.4 ms           | < 250 ms          | exceeds target               |
| sysbench OLTP read-only                  | TPS               | 491               | > 5 000           | constrained-host waiver      |
| sysbench OLTP read-only                  | p95 latency       | 27.2 ms           | < 5 ms            | constrained-host waiver      |
| sysbench OLTP write-only                 | TPS               | 1 440             | (no gate target)  | informational                |
| sysbench OLTP write-only                 | p95 latency       | 11.5 ms           | (no gate target)  | informational                |
| sysbench OLTP read-write                 | TPS               | 343               | > 2 000           | constrained-host waiver      |
| sysbench OLTP read-write                 | p95 latency       | 50.1 ms           | < 10 ms           | constrained-host waiver      |
| sysbench OLTP point-select               | TPS               | 9 707             | (no gate target)  | informational                |
| sysbench OLTP point-select               | p95 latency       | 3.7 ms            | (no gate target)  | informational                |
| Timescale ingest                         | rows/s            | 216 252           | > 100 000         | exceeds target               |
| Chaos: kill-coordinator                  | recovery p99      | scaffold-only     | < 5 000 ms        | waiver: no kind on 2-core VM |
| Chaos: kill-worker                       | recovery p99      | scaffold-only     | < 5 000 ms        | waiver: no kind on 2-core VM |
| Chaos: network-partition                 | recovery p99      | scaffold-only     | < 5 000 ms        | waiver: no kind on 2-core VM |
| Chaos: disk-full                         | recovery p99      | scaffold-only     | < 5 000 ms        | waiver: no kind on 2-core VM |
| Chaos: slow-disk                         | recovery p99      | scaffold-only     | < 5 000 ms        | waiver: no kind on 2-core VM |

VM environment characterization:

- Host: GCP `e2-standard-2` (2 vCPU AMD EPYC Milan-class, 7 GB RAM, 1.2 TB
  NVMe), Debian 12, Linux 6.1.0-47-cloud-amd64.
- Postgres + Timescale runs inside a single container
  (`ai-blaise-citus-timescale-cohabitation-2-27:local`); sysbench and pgbench
  run on the same VM, so the load generator competes with the database for
  CPU. This is intentional for the harness wiring shake-out; the production
  baseline must run database and driver on separate hosts.
- `kind` is not installed on this VM and a 3-worker kind cluster is infeasible
  in 7 GB RAM with two cores; chaos scenarios degrade to scaffold-only
  results. The nightly CI workflow uses GitHub-hosted runners (also 2-core);
  real failover baselines require the production smoke cluster in
  `deploy/k8s/`.

### Where the next baseline will come from

`ci-baseline-nightly.yml` runs the same harness sequence against a Timescale
17 container on a GitHub-hosted runner every night and uploads
`benchmarks/results/*.json` as an artifact. Promoting a nightly artifact to a
committed `benchmarks/baselines/<date>-baseline.json` is a manual step today;
the entry in `release_gates.rs::PERFORMANCE_BASELINE_PATH` must be updated in
lock-step with that promotion (the
`performance_baseline_path_points_at_committed_evidence` unit test enforces
the alignment).

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
```

Full-mode results are attached to the release record and tracked in
`docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`.

## Documentation site evidence

The mkdocs site at `https://ai-blaise.github.io/citus/` builds on every PR via
`.github/workflows/ci-docs-build.yml` and is published from `gh-pages` on push
to main via `.github/workflows/ci-docs-publish.yml`. See
[Releasing](RELEASING.md) for the one-time GitHub Pages source setting.

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
