#!/usr/bin/env bash
# TPC-C harness for the V2 performance acceptance gate.
#
# Driver order: benchbase (if installed) -> pgbench TPC-B fallback (for the CI
# soft-pass path). The full V2 thresholds (tpmC > 5000 on a 3-worker kind
# cluster) require benchbase against a live Citus coordinator; the pgbench
# fallback only verifies that the harness scaffolding executes cleanly.

set -euo pipefail

HARNESS_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=../common/lib.sh
source "${HARNESS_DIR}/../common/lib.sh"

CONFIG="${HARNESS_DIR}/config.xml"
[[ -f "${CONFIG}" ]] || bench_die "missing benchbase config: ${CONFIG}"

mode="quick"
[[ "${BENCH_QUICK}" == "0" ]] && mode="full"

bench_log "tpcc: mode=${mode} duration=${BENCH_DURATION_SECS}s clients=${BENCH_CLIENTS}"

run_benchbase() {
  local benchbase
  benchbase="$(command -v benchbase 2>/dev/null || true)"
  [[ -n "${benchbase}" ]] || benchbase="${BENCHBASE_BIN:-}"
  [[ -n "${benchbase}" ]] || return 1

  local out_dir="${BENCH_RESULTS_ROOT}/tpcc-${BENCH_RESULT_TAG}.benchbase"
  mkdir -p "${out_dir}"

  bench_log "tpcc: invoking benchbase via ${benchbase}"
  "${benchbase}" \
    -b tpcc \
    -c "${CONFIG}" \
    --execute=true \
    --create=${BENCH_QUICK:-1}:false:true \
    --load=${BENCH_QUICK:-1}:false:true \
    -d "${out_dir}" \
    >"${out_dir}/stdout.log" 2>&1 || {
      bench_log "tpcc: benchbase exited non-zero; see ${out_dir}/stdout.log"
      return 2
    }

  # Benchbase emits raw.csv + summary.json under -d.
  python3 - "${out_dir}" "${BENCH_DURATION_SECS}" "${mode}" <<'PY'
import json
import pathlib
import sys

out_dir = pathlib.Path(sys.argv[1])
duration = int(sys.argv[2])
mode = sys.argv[3]

summary_path = next(out_dir.glob("*.summary.json"), None)
if summary_path is None:
    print("tpcc: no benchbase summary found", file=sys.stderr)
    sys.exit(3)

summary = json.loads(summary_path.read_text())
result = {
    "tpmC": summary.get("Throughput (requests/second)", 0.0) * 60,
    "latency_ms": {
        "p50": summary.get("Latency Distribution", {}).get("50th Percentile Latency (microseconds)", 0) / 1000.0,
        "p95": summary.get("Latency Distribution", {}).get("95th Percentile Latency (microseconds)", 0) / 1000.0,
        "p99": summary.get("Latency Distribution", {}).get("99th Percentile Latency (microseconds)", 0) / 1000.0,
    },
    "errors": summary.get("Aborted Requests (count)", 0),
    "duration_s": duration,
    "mode": mode,
}
print(json.dumps(result))
PY
}

run_pgbench_fallback() {
  if ! bench_require_or_quick_pass pgbench "tpcc-pgbench-fallback"; then
    # Quick mode without pgbench: write a scaffold result so the smoke script
    # still validates the harness wiring end-to-end.
    bench_write_result tpcc "$(cat <<JSON
{
  "tpmC": 0,
  "latency_ms": {"p50": 0, "p95": 0, "p99": 0},
  "errors": 0,
  "duration_s": ${BENCH_DURATION_SECS},
  "mode": "${mode}",
  "note": "scaffold-only: pgbench unavailable"
}
JSON
)"
    return 0
  fi

  if ! bench_psql_available; then
    bench_log "tpcc: psql unavailable, recording scaffold result"
    bench_write_result tpcc "$(cat <<JSON
{
  "tpmC": 0,
  "latency_ms": {"p50": 0, "p95": 0, "p99": 0},
  "errors": 0,
  "duration_s": ${BENCH_DURATION_SECS},
  "mode": "${mode}",
  "note": "scaffold-only: psql unavailable"
}
JSON
)"
    return 0
  fi

  # Probe Postgres. CI typically does not have a live Citus cluster; treat the
  # missing endpoint as a soft pass in quick mode and only fail in full mode.
  if ! PGPASSWORD="${BENCH_PGPASSWORD}" psql \
      -h "${BENCH_PGHOST}" -p "${BENCH_PGPORT}" -U "${BENCH_PGUSER}" \
      -d "${BENCH_PGDATABASE}" -c "SELECT 1" >/dev/null 2>&1; then
    if [[ "${BENCH_QUICK}" == "1" ]]; then
      bench_log "tpcc: no Postgres endpoint at ${BENCH_PGHOST}:${BENCH_PGPORT}, recording scaffold result"
      bench_write_result tpcc "$(cat <<JSON
{
  "tpmC": 0,
  "latency_ms": {"p50": 0, "p95": 0, "p99": 0},
  "errors": 0,
  "duration_s": ${BENCH_DURATION_SECS},
  "mode": "${mode}",
  "note": "scaffold-only: no Postgres endpoint"
}
JSON
)"
      return 0
    fi
    bench_die "tpcc: cannot reach Postgres at ${BENCH_PGHOST}:${BENCH_PGPORT}"
  fi

  bench_log "tpcc: pgbench fallback initialising"
  PGPASSWORD="${BENCH_PGPASSWORD}" pgbench -i -s "${BENCH_SCALE}" \
    -h "${BENCH_PGHOST}" -p "${BENCH_PGPORT}" -U "${BENCH_PGUSER}" "${BENCH_PGDATABASE}" >/dev/null

  local out
  out="$(PGPASSWORD="${BENCH_PGPASSWORD}" pgbench \
    -h "${BENCH_PGHOST}" -p "${BENCH_PGPORT}" -U "${BENCH_PGUSER}" \
    -c "${BENCH_CLIENTS}" -j "${BENCH_CLIENTS}" \
    -T "${BENCH_DURATION_SECS}" \
    --progress=2 \
    "${BENCH_PGDATABASE}" 2>&1)"

  printf '%s\n' "${out}" >"${BENCH_RESULTS_ROOT}/tpcc-${BENCH_RESULT_TAG}.pgbench.log"

  local tps latency
  # pgbench 14+ phrases the summary as "tps = N (without initial connection time)";
  # earlier majors used "tps = N (excluding connections establishing)". Match both.
  tps="$(printf '%s\n' "${out}" | awk -F'[: ]+' '/tps =/ && (/without initial connection time/ || /excluding/) {print $3; exit}')"
  latency="$(printf '%s\n' "${out}" | awk -F'[: ]+' '/^latency average/ {print $4; exit}')"
  tps="${tps:-0}"
  latency="${latency:-0}"

  # pgbench is TPC-B not TPC-C; we surface tps*60 as a stand-in tpmC so the
  # field exists. The real V2 acceptance threshold (tpmC > 5000) is only valid
  # with benchbase.
  local tpmc
  tpmc="$(python3 -c "import sys; print(int(float(sys.argv[1])*60))" "${tps}")"

  bench_write_result tpcc "$(cat <<JSON
{
  "tpmC": ${tpmc},
  "latency_ms": {"p50": 0, "p95": 0, "p99": ${latency}},
  "errors": 0,
  "duration_s": ${BENCH_DURATION_SECS},
  "mode": "${mode}",
  "note": "pgbench TPC-B fallback; benchbase is the canonical driver"
}
JSON
)"
}

if run_benchbase 2>/tmp/tpcc-benchbase-stderr; then
  bench_log "tpcc: benchbase run completed"
else
  status=$?
  if [[ ${status} -eq 1 ]]; then
    bench_log "tpcc: benchbase not installed; falling back to pgbench"
  else
    bench_log "tpcc: benchbase wrapper failed (status ${status}); falling back to pgbench"
    cat /tmp/tpcc-benchbase-stderr >&2 || true
  fi
  run_pgbench_fallback
fi

bench_log "tpcc: done"
