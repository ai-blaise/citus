#!/usr/bin/env bash
# benchmarks/microbenches/run-all.sh
#
# Iterates every microbench directory under benchmarks/microbenches/,
# invokes its bench.sh, and writes an aggregate report to
# benchmarks/microbenches/results-<timestamp>.json.
#
# Quick mode (default) runs each bench at one-tenth row count so CI smoke
# stays inside the bench-smoke 60 s budget. Full mode (BENCH_QUICK=0) runs the
# canonical row counts and is nightly/release-only.

set -euo pipefail

HARNESS_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "${HARNESS_DIR}/../.." && pwd)"
# shellcheck source=../common/lib.sh
source "${REPO_ROOT}/benchmarks/common/lib.sh"

mb_dirs=()
while IFS= read -r -d '' dir; do
  mb_dirs+=("${dir}")
done < <(find "${HARNESS_DIR}" -mindepth 1 -maxdepth 1 -type d -print0 | sort -z)

if [[ "${#mb_dirs[@]}" -eq 0 ]]; then
  bench_die "run-all.sh: no microbench directories under ${HARNESS_DIR}"
fi

ts="$(date -u +%Y%m%dT%H%M%SZ)"
aggregate="${HARNESS_DIR}/results-${ts}.json"

# Build the aggregate report as a single JSON array. We track failures so the
# wrapping CI job can fail loudly while still producing a complete report.
entries=()
failures=0

for dir in "${mb_dirs[@]}"; do
  bench="${dir}/bench.sh"
  if [[ ! -x "${bench}" ]]; then
    bench_log "run-all: skipping ${dir} (no bench.sh)"
    continue
  fi

  ext_name="$(basename "${dir}")"
  bench_log "run-all: running ${ext_name}"
  if ! out="$(bash "${bench}" 2>&1 | tail -1)"; then
    bench_log "run-all: ${ext_name} bench.sh exited non-zero"
    failures=$((failures + 1))
    out='{"ext":"'"${ext_name}"'","error":"bench.sh failed"}'
  fi

  # bench.sh prints the JSON line to stdout (last line). Capture and tag.
  entries+=("${out}")
done

# Emit the aggregate. Single JSON object with `results` array, `count`, and
# the run timestamp; downstream tooling (compare-to-baseline.sh) reads this.
{
  printf '{\n'
  printf '  "ts": "%s",\n' "${ts}"
  printf '  "count": %d,\n' "${#entries[@]}"
  printf '  "failures": %d,\n' "${failures}"
  printf '  "mode": "%s",\n' "${BENCH_QUICK:-1}"
  printf '  "results": [\n'
  for i in "${!entries[@]}"; do
    if [[ $i -eq $((${#entries[@]} - 1)) ]]; then
      printf '    %s\n' "${entries[$i]}"
    else
      printf '    %s,\n' "${entries[$i]}"
    fi
  done
  printf '  ]\n'
  printf '}\n'
} >"${aggregate}"

bench_log "run-all: aggregate -> ${aggregate} (count=${#entries[@]} failures=${failures})"
printf '%s\n' "${aggregate}"

# Keep a stable alias for the most recent run so downstream scripts can
# find the report without parsing the timestamp.
cp "${aggregate}" "${HARNESS_DIR}/results-latest.json"

exit "${failures}"
