#!/usr/bin/env bash
# Trial-mode driver for the chaos scenarios.
#
# Wraps benchmarks/chaos/run.sh by running each scenario CHAOS_TRIALS times
# (default 5) and asserting the per-scenario pass count meets
# CHAOS_TRIALS_MIN_PASS (default 4). Each trial's per-scenario JSON is kept
# alongside the trial-summary JSON so the chaos gate sees both the raw and
# aggregate views.
#
# Exit codes:
#   0   every scenario met the 4/5 minimum (full mode) or quick mode swallowed
#       any per-trial failures.
#   1   one or more scenarios did not meet the 4/5 minimum in full mode.

set -euo pipefail

HARNESS_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=../common/lib.sh
source "${HARNESS_DIR}/../common/lib.sh"
# shellcheck source=scenarios/_trials.sh
source "${HARNESS_DIR}/scenarios/_trials.sh"

SCENARIOS=(
  "kill-coordinator"
  "kill-worker"
  "network-partition"
  "disk-full"
  "slow-disk"
)

mode="quick"
[[ "${BENCH_QUICK}" == "0" ]] && mode="full"

bench_log "chaos-trials: mode=${mode} scenarios=${#SCENARIOS[@]} trials=${CHAOS_TRIALS}"

failed=0
for scenario_name in "${SCENARIOS[@]}"; do
  scenario_script="${HARNESS_DIR}/scenarios/${scenario_name}.sh"
  if ! chaos_run_trials "${scenario_name}" "${scenario_script}"; then
    failed=$(( failed + 1 ))
  fi
done

# Aggregate trial summaries into a single chaos-trials-<tag>.json file.
python3 - "${BENCH_RESULTS_ROOT}" "${BENCH_RESULT_TAG}" "${mode}" \
  "${CHAOS_TRIALS}" "${CHAOS_TRIALS_MIN_PASS}" <<'PY'
import json
import pathlib
import sys

results_root, tag, mode, trials, min_pass = sys.argv[1:6]
results_dir = pathlib.Path(results_root)
combined = {
    "mode": mode,
    "trials_total": int(trials),
    "min_pass_required": int(min_pass),
    "scenarios": [],
}
for path in sorted(results_dir.glob(f"chaos-*-trials-{tag}.json")):
    combined["scenarios"].append(json.loads(path.read_text()))

out = results_dir / f"chaos-trials-{tag}.json"
out.write_text(json.dumps(combined, indent=2) + "\n")
print(f"chaos-trials: combined -> {out}")
PY

if (( failed > 0 )); then
  bench_log "chaos-trials: ${failed} scenarios below ${CHAOS_TRIALS_MIN_PASS}/${CHAOS_TRIALS}"
  if [[ "${mode}" == "full" ]]; then
    exit 1
  fi
fi

bench_log "chaos-trials: done"
