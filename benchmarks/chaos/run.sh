#!/usr/bin/env bash
# Chaos harness driver for the V2 chaos acceptance gate (gate 11).
#
# Executes each scenario script under `scenarios/`, collects the resulting
# per-scenario JSON, and writes a combined `chaos-<tag>.json`. Each scenario
# is responsible for emitting its own JSON via `bench_write_result`.

set -euo pipefail

HARNESS_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=../common/lib.sh
source "${HARNESS_DIR}/../common/lib.sh"

SCENARIOS=(
  "${HARNESS_DIR}/scenarios/kill-coordinator.sh"
  "${HARNESS_DIR}/scenarios/kill-worker.sh"
  "${HARNESS_DIR}/scenarios/network-partition.sh"
  "${HARNESS_DIR}/scenarios/disk-full.sh"
  "${HARNESS_DIR}/scenarios/slow-disk.sh"
)

mode="quick"
[[ "${BENCH_QUICK}" == "0" ]] && mode="full"

bench_log "chaos: mode=${mode} scenarios=${#SCENARIOS[@]}"

for scenario in "${SCENARIOS[@]}"; do
  name="$(basename "${scenario}" .sh)"
  bench_log "chaos: running ${name}"
  if ! "${scenario}"; then
    if [[ "${BENCH_QUICK}" == "1" ]]; then
      bench_log "chaos: ${name} failed; quick mode treats as soft pass"
    else
      bench_die "chaos: ${name} failed in full mode"
    fi
  fi
done

# Combine per-scenario JSON into a single summary so the smoke gate has one
# artifact to grep for.
python3 - "${BENCH_RESULTS_ROOT}" "${BENCH_RESULT_TAG}" "${mode}" <<'PY'
import json
import pathlib
import sys

results_root, tag, mode = sys.argv[1:4]
results_dir = pathlib.Path(results_root)
combined = {"mode": mode, "scenarios": []}
for path in sorted(results_dir.glob(f"chaos-*-{tag}.json")):
    name = path.stem.replace(f"chaos-", "").replace(f"-{tag}", "")
    if name == "" or name == tag:
        continue
    combined["scenarios"].append({"scenario": name, **json.loads(path.read_text())})

out = results_dir / f"chaos-{tag}.json"
out.write_text(json.dumps(combined, indent=2) + "\n")
print(f"chaos: combined summary -> {out}")
PY

bench_log "chaos: done"
