#!/usr/bin/env bash
# sysbench OLTP suite for the V2 performance acceptance gate.
#
# Workloads (run in order):
#   - oltp_read_only
#   - oltp_write_only
#   - oltp_read_write
#   - oltp_point_select
#
# Each workload writes a result row to
# `benchmarks/results/sysbench-<workload>-<BENCH_RESULT_TAG>.json`.

set -euo pipefail

HARNESS_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=../common/lib.sh
source "${HARNESS_DIR}/../common/lib.sh"

WORKLOADS=(oltp_read_only oltp_write_only oltp_read_write oltp_point_select)

mode="quick"
[[ "${BENCH_QUICK}" == "0" ]] && mode="full"

# Tables used by sysbench; we distribute them via create_distributed_table on
# the `id` primary key so OLTP shards across the cluster.
: "${BENCH_TABLES:=4}"
: "${BENCH_TABLE_SIZE:=10000}"

bench_log "sysbench: mode=${mode} workloads=${WORKLOADS[*]} tables=${BENCH_TABLES}"

run_distribute() {
  if ! bench_psql_available; then
    bench_log "sysbench: psql unavailable, skipping create_distributed_table"
    return 0
  fi

  if ! PGPASSWORD="${BENCH_PGPASSWORD}" psql \
      -h "${BENCH_PGHOST}" -p "${BENCH_PGPORT}" -U "${BENCH_PGUSER}" \
      -d "${BENCH_PGDATABASE}" -c "SELECT 1" >/dev/null 2>&1; then
    bench_log "sysbench: Postgres endpoint unreachable, skipping distribution"
    return 0
  fi

  bench_log "sysbench: distributing sbtest1..sbtest${BENCH_TABLES} via create_distributed_table"
  for ((i = 1; i <= BENCH_TABLES; i++)); do
    PGPASSWORD="${BENCH_PGPASSWORD}" psql \
      -h "${BENCH_PGHOST}" -p "${BENCH_PGPORT}" -U "${BENCH_PGUSER}" \
      -d "${BENCH_PGDATABASE}" \
      -c "SELECT create_distributed_table('sbtest${i}', 'id');" \
      >/dev/null 2>&1 || true
  done
}

run_workload() {
  local workload="$1"
  local out="${BENCH_RESULTS_ROOT}/sysbench-${workload}-${BENCH_RESULT_TAG}.json"

  if ! bench_require_or_quick_pass sysbench "sysbench (${workload})"; then
    cat >"${out}" <<JSON
{
  "workload": "${workload}",
  "tps": 0,
  "latency_ms_p95": 0,
  "duration_s": ${BENCH_DURATION_SECS},
  "mode": "${mode}",
  "note": "scaffold-only: sysbench not installed"
}
JSON
    bench_log "sysbench: ${workload} scaffold result -> ${out}"
    return 0
  fi

  local log="${BENCH_RESULTS_ROOT}/sysbench-${workload}-${BENCH_RESULT_TAG}.log"

  # `prepare` is idempotent enough that quick mode can rerun it; full mode
  # expects a long-lived dataset and runs prepare separately.
  if [[ "${BENCH_QUICK}" == "1" ]]; then
    sysbench "${workload}" \
      --db-driver=pgsql \
      --pgsql-host="${BENCH_PGHOST}" \
      --pgsql-port="${BENCH_PGPORT}" \
      --pgsql-user="${BENCH_PGUSER}" \
      --pgsql-password="${BENCH_PGPASSWORD}" \
      --pgsql-db="${BENCH_PGDATABASE}" \
      --tables="${BENCH_TABLES}" \
      --table-size="${BENCH_TABLE_SIZE}" \
      prepare >/dev/null 2>&1 || true
  fi

  set +e
  sysbench "${workload}" \
    --db-driver=pgsql \
    --pgsql-host="${BENCH_PGHOST}" \
    --pgsql-port="${BENCH_PGPORT}" \
    --pgsql-user="${BENCH_PGUSER}" \
    --pgsql-password="${BENCH_PGPASSWORD}" \
    --pgsql-db="${BENCH_PGDATABASE}" \
    --tables="${BENCH_TABLES}" \
    --table-size="${BENCH_TABLE_SIZE}" \
    --threads="${BENCH_CLIENTS}" \
    --time="${BENCH_DURATION_SECS}" \
    --report-interval=2 \
    --rate=0 \
    run >"${log}" 2>&1
  local status=$?
  set -e

  if [[ ${status} -ne 0 ]]; then
    if [[ "${BENCH_QUICK}" == "1" ]]; then
      bench_log "sysbench: ${workload} run failed (status ${status}); recording scaffold result"
      cat >"${out}" <<JSON
{
  "workload": "${workload}",
  "tps": 0,
  "latency_ms_p95": 0,
  "duration_s": ${BENCH_DURATION_SECS},
  "mode": "${mode}",
  "note": "scaffold-only: sysbench run exited ${status}"
}
JSON
      return 0
    fi
    bench_die "sysbench: ${workload} run failed (status ${status}); see ${log}"
  fi

  local tps p95
  tps="$(awk -F'[: ]+' '/transactions:/ {print $4; exit}' "${log}" || true)"
  p95="$(awk -F'[: ]+' '/95th percentile:/ {print $3; exit}' "${log}" || true)"
  tps="${tps:-0}"
  p95="${p95:-0}"

  cat >"${out}" <<JSON
{
  "workload": "${workload}",
  "tps": ${tps},
  "latency_ms_p95": ${p95},
  "duration_s": ${BENCH_DURATION_SECS},
  "mode": "${mode}"
}
JSON
  bench_log "sysbench: ${workload} tps=${tps} p95=${p95}ms -> ${out}"
}

run_distribute

for workload in "${WORKLOADS[@]}"; do
  run_workload "${workload}"
done

bench_log "sysbench: done"
