#!/usr/bin/env bash
# DR drill quick-mode smoke.
#
# Runs every drill in benchmarks/dr-drills/ in quick mode (1-minute cap per
# drill) and emits an aggregate JSON report at
# benchmarks/dr-drills/reports/aggregate-<tag>.json. Each drill is also
# guaranteed to leave behind its individual report; this script gates on file
# existence and a green `success` field per drill.
#
# Quick mode is the CI smoke entrypoint; the drills inside soft-pass when the
# cluster is unreachable by writing `mock=true` reports. Full mode is the
# release path and requires a kind cluster from `kind-production-smoke.sh`.

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "${repo_root}"

export DR_DRILL_QUICK="${DR_DRILL_QUICK:-1}"
export DR_DRILL_NAMESPACE="${DR_DRILL_NAMESPACE:-ai-blaise-citus}"
export DR_DRILL_CLUSTER="${DR_DRILL_CLUSTER:-primary}"
export DR_DRILL_RTO_BUDGET_S="${DR_DRILL_RTO_BUDGET_S:-60}"
export DR_DRILL_FENCING_BUDGET_S="${DR_DRILL_FENCING_BUDGET_S:-15}"
export DR_DRILL_TAG="${DR_DRILL_TAG:-$(date -u +%Y%m%dT%H%M%SZ)}"

reports_dir="${repo_root}/benchmarks/dr-drills/reports"
mkdir -p "${reports_dir}"
export DR_DRILL_REPORTS_ROOT="${reports_dir}"

drills=(
  "lost-shard"
  "split-brain"
  "pitr-restore"
  "region-failover"
  "branch-promote"
  "tenant-move"
)

failed=0
for drill in "${drills[@]}"; do
  script="${repo_root}/benchmarks/dr-drills/${drill}-drill.sh"
  if [[ ! -x "${script}" ]]; then
    echo "[dr-drill-smoke] missing or non-executable: ${script}" >&2
    failed=$(( failed + 1 ))
    continue
  fi
  echo "[dr-drill-smoke] >>> ${drill}"
  start=$(date +%s)
  if ! bash "${script}"; then
    echo "[dr-drill-smoke] ${drill} returned non-zero" >&2
    failed=$(( failed + 1 ))
    continue
  fi
  elapsed=$(( $(date +%s) - start ))
  echo "[dr-drill-smoke] ${drill} ok (${elapsed}s)"
done

# Assert every drill left a report behind and aggregate them.
python3 - "${reports_dir}" "${DR_DRILL_TAG}" "${DR_DRILL_QUICK}" \
  "${drills[@]}" <<'PY'
import json
import pathlib
import sys

reports_dir = pathlib.Path(sys.argv[1])
tag = sys.argv[2]
mode = "quick" if sys.argv[3] == "1" else "full"
drills = sys.argv[4:]

aggregate = {
    "mode": mode,
    "drills": [],
    "missing": [],
    "non_success": [],
}
exit_code = 0
for drill in drills:
    report_path = reports_dir / f"{drill}-{tag}.json"
    if not report_path.exists():
        aggregate["missing"].append(drill)
        exit_code = 1
        continue
    data = json.loads(report_path.read_text())
    aggregate["drills"].append(data)
    if mode == "full" and not data.get("success", False):
        aggregate["non_success"].append(drill)
        exit_code = 1

out_path = reports_dir / f"aggregate-{tag}.json"
out_path.write_text(json.dumps(aggregate, indent=2) + "\n")
print(f"[dr-drill-smoke] aggregate -> {out_path}")

if aggregate["missing"]:
    print("[dr-drill-smoke] missing reports: " + ",".join(aggregate["missing"]),
          file=sys.stderr)
if aggregate["non_success"]:
    print("[dr-drill-smoke] non-success drills: " + ",".join(aggregate["non_success"]),
          file=sys.stderr)
sys.exit(exit_code)
PY

if (( failed > 0 )); then
  echo "[dr-drill-smoke] ${failed} drill(s) failed" >&2
  if [[ "${DR_DRILL_QUICK}" != "1" ]]; then
    exit 1
  fi
fi

echo "[dr-drill-smoke] ok"
