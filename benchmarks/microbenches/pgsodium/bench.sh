#!/usr/bin/env bash
# MB7: pgsodium microbench.
#
# Encrypt 1k rows with crypto_secretbox using a single derived key.
#
# Reads :row_count from BENCH_ROW_COUNT (default 1000; quick mode falls
# back to a tenth of that). Emits a single-line JSON record to stdout and to
# benchmarks/results/microbench-mb7-${BENCH_RESULT_TAG:-quick}.json.

set -euo pipefail

HARNESS_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=../../common/lib.sh
source "${HARNESS_DIR}/../../common/lib.sh"

mb_id="MB7"
ext_name="pgsodium"
op_label="libsodium_encrypt_rows_per_s"
default_rows=1000

if [[ "${BENCH_QUICK}" == "1" ]]; then
  rows="${BENCH_ROW_COUNT:-$((default_rows / 10))}"
  [[ "${rows}" -lt 10 ]] && rows=10
else
  rows="${BENCH_ROW_COUNT:-${default_rows}}"
fi

out="${BENCH_RESULTS_ROOT}/microbench-mb7-${BENCH_RESULT_TAG}.json"

emit_scaffold() {
  local note="$1"
  cat >"${out}" <<JSON
{"ext":"${ext_name}","mb":"${mb_id}","op":"${op_label}","qps":0,"p95_ms":0,"p99_ms":0,"rows":${rows},"mode":"scaffold","ts":"$(date -u +%Y-%m-%dT%H:%M:%SZ)","note":"${note}"}
JSON
  bench_log "MB7: scaffold result -> ${out} (${note})"
  printf '%s\n' "$(cat "${out}")"
}

if ! bench_psql_available; then
  emit_scaffold "psql not on PATH"
  exit 0
fi

if ! PGPASSWORD="${BENCH_PGPASSWORD}" psql \
    -h "${BENCH_PGHOST}" -p "${BENCH_PGPORT}" -U "${BENCH_PGUSER}" \
    -d "${BENCH_PGDATABASE}" -c "SELECT 1" >/dev/null 2>&1; then
  emit_scaffold "Postgres endpoint unreachable"
  exit 0
fi

setup_log="${BENCH_RESULTS_ROOT}/microbench-mb7-${BENCH_RESULT_TAG}.setup.log"
bench_log "${BENCH_RESULTS_ROOT}/microbench-mb7-${BENCH_RESULT_TAG}.bench.log"

if ! PGPASSWORD="${BENCH_PGPASSWORD}" psql -v ON_ERROR_STOP=1 \
    -h "${BENCH_PGHOST}" -p "${BENCH_PGPORT}" -U "${BENCH_PGUSER}" \
    -d "${BENCH_PGDATABASE}" -f "${HARNESS_DIR}/setup.sql" \
    >"${setup_log}" 2>&1; then
  emit_scaffold "setup.sql failed; see ${setup_log}"
  exit 0
fi

bench_log_file="${BENCH_RESULTS_ROOT}/microbench-mb7-${BENCH_RESULT_TAG}.bench.log"

start_ns=$(date +%s%N)
if ! PGPASSWORD="${BENCH_PGPASSWORD}" psql -v ON_ERROR_STOP=1 \
    -h "${BENCH_PGHOST}" -p "${BENCH_PGPORT}" -U "${BENCH_PGUSER}" \
    -d "${BENCH_PGDATABASE}" \
    -v "row_count=${rows}" \
    -f "${HARNESS_DIR}/bench.sql" \
    >"${bench_log_file}" 2>&1; then
  emit_scaffold "bench.sql failed; see ${bench_log_file}"
  exit 0
fi
end_ns=$(date +%s%N)

elapsed_s=$(awk -v s="${start_ns}" -v e="${end_ns}" \
  'BEGIN { printf "%.6f", (e - s) / 1000000000 }')
qps=$(awk -v rows="${rows}" -v secs="${elapsed_s}" \
  'BEGIN { if (secs <= 0) print 0; else printf "%.2f", rows / secs }')
elapsed_ms=$(awk -v s="${elapsed_s}" 'BEGIN { printf "%.3f", s * 1000 }')

cat >"${out}" <<JSON
{"ext":"${ext_name}","mb":"${mb_id}","op":"${op_label}","qps":${qps},"p95_ms":${elapsed_ms},"p99_ms":${elapsed_ms},"rows":${rows},"mode":"measured","elapsed_s":${elapsed_s},"ts":"$(date -u +%Y-%m-%dT%H:%M:%SZ)"}
JSON
bench_log "MB7: measured qps=${qps} elapsed_s=${elapsed_s} rows=${rows} -> ${out}"
printf '%s\n' "$(cat "${out}")"
