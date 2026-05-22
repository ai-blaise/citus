#!/usr/bin/env bash
# CI smoke runner for the four V2 benchmark harnesses.
#
# Each harness is invoked in quick mode (BENCH_QUICK=1, BENCH_DURATION_SECS=10).
# Harnesses are designed to soft-pass when their driver binaries (benchbase,
# sysbench, timescaledb_parallel_copy) or the target Postgres / Kubernetes
# endpoint are unavailable, so this script provides one consistent exit code
# for the CI overlay even on a stripped-down runner.
#
# Time budget: ~60s total. Each harness is bounded by BENCH_DURATION_SECS
# plus startup overhead.

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "${repo_root}"

export BENCH_QUICK=1
export BENCH_DURATION_SECS="${BENCH_DURATION_SECS:-10}"
export BENCH_WARMUP_SECS="${BENCH_WARMUP_SECS:-2}"
export BENCH_CLIENTS="${BENCH_CLIENTS:-2}"
export BENCH_SCALE="${BENCH_SCALE:-1}"
export BENCH_RESULT_TAG="${BENCH_RESULT_TAG:-quick}"

results_dir="${repo_root}/benchmarks/results"
mkdir -p "${results_dir}"

run_step() {
  local label="$1"
  shift
  echo "[benchmark-smoke] >>> ${label}"
  local start
  start=$(date +%s)
  if ! "$@"; then
    echo "[benchmark-smoke] ${label} failed" >&2
    return 1
  fi
  local elapsed=$(( $(date +%s) - start ))
  echo "[benchmark-smoke] ${label} ok (${elapsed}s)"
}

run_step "tpcc" bash benchmarks/tpcc/run.sh
run_step "sysbench" bash benchmarks/sysbench/run-suite.sh
run_step "timescale-ingest" python3 benchmarks/timescale-ingest/ingest.py --quick
run_step "chaos" bash benchmarks/chaos/run.sh

# Assert each harness produced a result JSON. The shape is verified by the
# harness itself; here we just gate on file existence so the CI overlay has
# something concrete to grep for.
expected=(
  "${results_dir}/tpcc-${BENCH_RESULT_TAG}.json"
  "${results_dir}/timescale-ingest-${BENCH_RESULT_TAG}.json"
  "${results_dir}/chaos-${BENCH_RESULT_TAG}.json"
)
for path in "${expected[@]}"; do
  if [[ ! -s "${path}" ]]; then
    echo "[benchmark-smoke] missing result: ${path}" >&2
    exit 1
  fi
done

# Sysbench writes one result per workload; assert at least one exists.
if ! ls "${results_dir}"/sysbench-*-"${BENCH_RESULT_TAG}".json >/dev/null 2>&1; then
  echo "[benchmark-smoke] missing sysbench results" >&2
  exit 1
fi

echo "[benchmark-smoke] ok"
