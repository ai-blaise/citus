#!/usr/bin/env bash
# Trial-loop helper for chaos scenarios.
#
# Each chaos scenario records a single per-trial JSON; this helper runs the
# scenario N times (default 5) and emits a per-scenario summary JSON with the
# pass count, success rate, max recovery_p99_ms across trials, and a
# pass/fail verdict against the 4/5 minimum.
#
# Usage:
#   source _trials.sh
#   chaos_run_trials kill-coordinator "${SCENARIO_BODY[@]}"
#
# In practice the helper is invoked as a runner: it executes the scenario
# script CHAOS_TRIALS times, collects each per-trial JSON, and writes a
# trial-summary JSON the harness driver can grep for.

set -euo pipefail

SCENARIO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../../common/lib.sh
source "${SCENARIO_DIR}/../../common/lib.sh"

: "${CHAOS_TRIALS:=5}"
: "${CHAOS_TRIALS_MIN_PASS:=4}"
export CHAOS_TRIALS CHAOS_TRIALS_MIN_PASS

# chaos_run_trials <scenario_name> <scenario_script>
#
# Runs <scenario_script> CHAOS_TRIALS times, each time tagging the per-trial
# result file with a `-trial<N>` suffix. Writes
# `chaos-<scenario_name>-trials-<BENCH_RESULT_TAG>.json` with the aggregate.
chaos_run_trials() {
  local scenario_name="$1"
  local scenario_script="$2"
  local original_tag="${BENCH_RESULT_TAG}"
  local pass_count=0
  local fail_count=0
  local max_recovery=0
  local trial_files=()

  for trial in $(seq 1 "${CHAOS_TRIALS}"); do
    BENCH_RESULT_TAG="${original_tag}-trial${trial}"
    export BENCH_RESULT_TAG
    bench_log "chaos: ${scenario_name}: trial ${trial}/${CHAOS_TRIALS}"

    if "${scenario_script}"; then
      pass_count=$(( pass_count + 1 ))
    else
      fail_count=$(( fail_count + 1 ))
    fi

    local trial_file="${BENCH_RESULTS_ROOT}/chaos-${scenario_name}-${BENCH_RESULT_TAG}.json"
    if [[ -s "${trial_file}" ]]; then
      trial_files+=("${trial_file}")
      local recovery
      recovery=$(python3 -c "import json,sys; print(int(json.load(open(sys.argv[1])).get('recovery_p99_ms', 0)))" \
        "${trial_file}" 2>/dev/null || echo 0)
      if (( recovery > max_recovery )); then
        max_recovery=${recovery}
      fi
    fi
  done

  BENCH_RESULT_TAG="${original_tag}"
  export BENCH_RESULT_TAG

  local verdict="pass"
  if (( pass_count < CHAOS_TRIALS_MIN_PASS )); then
    verdict="fail"
  fi

  local mode="quick"
  [[ "${BENCH_QUICK}" == "0" ]] && mode="full"

  local out="${BENCH_RESULTS_ROOT}/chaos-${scenario_name}-trials-${BENCH_RESULT_TAG}.json"
  cat >"${out}" <<JSON
{
  "scenario": "${scenario_name}",
  "trials_total": ${CHAOS_TRIALS},
  "trials_pass": ${pass_count},
  "trials_fail": ${fail_count},
  "min_pass_required": ${CHAOS_TRIALS_MIN_PASS},
  "verdict": "${verdict}",
  "max_recovery_p99_ms": ${max_recovery},
  "mode": "${mode}"
}
JSON
  bench_log "chaos: ${scenario_name}: trials -> ${out} (${pass_count}/${CHAOS_TRIALS} pass)"

  if [[ "${verdict}" == "fail" && "${BENCH_QUICK}" == "0" ]]; then
    return 1
  fi
  return 0
}
