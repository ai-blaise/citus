#!/usr/bin/env bash
# Shared helpers for benchmark harnesses.
#
# Sourced by the per-harness scripts (`benchmarks/tpcc/run.sh`,
# `benchmarks/sysbench/run-suite.sh`, etc.). The helpers exist so the harnesses
# share one notion of "quick mode", one error-handling style, and one report
# layout under `benchmarks/results/`.

set -euo pipefail

# Resolve repo root regardless of caller cwd.
BENCH_REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export BENCH_REPO_ROOT

BENCH_RESULTS_ROOT="${BENCH_REPO_ROOT}/benchmarks/results"
export BENCH_RESULTS_ROOT
mkdir -p "${BENCH_RESULTS_ROOT}"

# Quick mode keeps each harness under ~30 seconds so CI smoke can run them all
# inside a single GitHub-hosted job. Full benchmarks are nightly/release-only.
: "${BENCH_QUICK:=1}"
: "${BENCH_DURATION_SECS:=10}"
: "${BENCH_WARMUP_SECS:=2}"
: "${BENCH_CLIENTS:=2}"
: "${BENCH_SCALE:=1}"
: "${BENCH_RESULT_TAG:=quick}"
export BENCH_QUICK BENCH_DURATION_SECS BENCH_WARMUP_SECS BENCH_CLIENTS BENCH_SCALE BENCH_RESULT_TAG

# Postgres / Citus connection defaults. CI smoke targets a local container or
# an in-process stub; full benchmarks target a 3-worker kind cluster.
: "${BENCH_PGHOST:=127.0.0.1}"
: "${BENCH_PGPORT:=5432}"
: "${BENCH_PGUSER:=postgres}"
: "${BENCH_PGPASSWORD:=}"
: "${BENCH_PGDATABASE:=postgres}"
export BENCH_PGHOST BENCH_PGPORT BENCH_PGUSER BENCH_PGPASSWORD BENCH_PGDATABASE

bench_log() {
  printf '[%s] %s\n' "$(date -u +%H:%M:%SZ)" "$*"
}

bench_die() {
  printf '[bench] error: %s\n' "$*" >&2
  exit 1
}

bench_release_mode() {
  [[ "${AI_BLAISE_RELEASE_MODE:-0}" == "1" || "${BENCH_REQUIRE_MEASURED:-0}" == "1" ]]
}

# Some harness tools (sysbench, benchbase) are not installed in stripped-down
# CI/VM environments. We treat "tool missing" as a quick-mode soft pass so the
# scaffold is exercisable everywhere, while full nightly runs use a richer VM
# where every tool is present.
bench_require_or_quick_pass() {
  local tool="$1"
  local label="$2"

  if command -v "${tool}" >/dev/null 2>&1; then
    return 0
  fi

  if bench_release_mode; then
    bench_die "${label} requires '${tool}' on PATH in release mode"
  fi

  if [[ "${BENCH_QUICK}" == "1" ]]; then
    bench_log "skipping ${label}: '${tool}' not installed (quick mode)"
    return 1
  fi

  bench_die "${label} requires '${tool}' on PATH"
}

bench_write_result() {
  local harness="$1"
  local payload="$2"

  local out="${BENCH_RESULTS_ROOT}/${harness}-${BENCH_RESULT_TAG}.json"
  printf '%s\n' "${payload}" >"${out}"
  bench_log "result: ${out}"
}

bench_psql_available() {
  if command -v psql >/dev/null 2>&1; then
    return 0
  fi

  bench_release_mode && bench_die "psql is required on PATH in release mode"
  return 1
}

# All harnesses bail out at the first failure. The wrapping CI script then
# decides whether to treat an individual missing tool as a soft skip.
bench_on_exit() {
  local status=$?
  if [[ ${status} -ne 0 ]]; then
    bench_log "harness exited with status ${status}"
  fi
}
trap bench_on_exit EXIT
