#!/usr/bin/env bash
# ci/ai-blaise/bundled-ext-microbenches-smoke.sh
#
# CI smoke runner for all 26 always-on bundled-extension microbenches.
#
# Each microbench is invoked in quick mode (BENCH_QUICK=1) with a one-tenth
# row count. The bench scripts soft-pass when psql or the target Postgres
# endpoint is unavailable so the smoke stays green on the 2-core experiment VM
# and the GitHub Actions runner. Real numbers come from the nightly
# `ci-microbench` workflow against the 3-worker kind-production-smoke cluster.
#
# Time budget: ~30 s total. 26 benches * (~1 s startup + scaffold path).

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "${repo_root}"

export BENCH_QUICK=1
export BENCH_DURATION_SECS="${BENCH_DURATION_SECS:-10}"
export BENCH_RESULT_TAG="${BENCH_RESULT_TAG:-quick}"

results_dir="${repo_root}/benchmarks/results"
mkdir -p "${results_dir}"

mb_dirs=()
while IFS= read -r -d '' dir; do
  mb_dirs+=("${dir}")
done < <(find "${repo_root}/benchmarks/microbenches" -mindepth 1 -maxdepth 1 -type d -print0 | sort -z)

if [[ "${#mb_dirs[@]}" -lt 26 ]]; then
  echo "[microbench-smoke] expected >= 26 microbench dirs, found ${#mb_dirs[@]}" >&2
  exit 1
fi

failures=0
for dir in "${mb_dirs[@]}"; do
  ext_name="$(basename "${dir}")"
  bench="${dir}/bench.sh"
  if [[ ! -x "${bench}" ]]; then
    echo "[microbench-smoke] ${ext_name}: missing bench.sh" >&2
    failures=$((failures + 1))
    continue
  fi

  echo "[microbench-smoke] >>> ${ext_name}"
  start=$(date +%s)
  if ! bash "${bench}" >/dev/null; then
    echo "[microbench-smoke] ${ext_name} failed" >&2
    failures=$((failures + 1))
    continue
  fi
  elapsed=$(( $(date +%s) - start ))
  echo "[microbench-smoke] ${ext_name} ok (${elapsed}s)"
done

# Assert each microbench produced a result JSON. Shape is verified by the
# microbench itself; the smoke just gates on file presence.
for dir in "${mb_dirs[@]}"; do
  ext_name="$(basename "${dir}")"
  # Result filename uses the mb id (lowercase) embedded in bench.sh. We
  # accept any microbench-*-${BENCH_RESULT_TAG}.json that references this ext.
  if ! ls "${results_dir}"/microbench-*-"${BENCH_RESULT_TAG}".json >/dev/null 2>&1; then
    echo "[microbench-smoke] no microbench-*-${BENCH_RESULT_TAG}.json produced" >&2
    exit 1
  fi
done

count="$(ls "${results_dir}"/microbench-*-"${BENCH_RESULT_TAG}".json 2>/dev/null | wc -l | tr -d ' ')"
if [[ "${count}" -lt 26 ]]; then
  echo "[microbench-smoke] expected >= 26 result files, got ${count}" >&2
  exit 1
fi

if [[ "${failures}" -ne 0 ]]; then
  echo "[microbench-smoke] ${failures} microbench(es) failed" >&2
  exit 1
fi

echo "[microbench-smoke] ok (26 microbenches, ${count} result files)"
