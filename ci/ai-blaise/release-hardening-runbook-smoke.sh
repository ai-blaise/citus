#!/usr/bin/env bash
set -euo pipefail

# FEATURE: D10
# Verify release rejection and inventory coverage, not release qualification.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

bash ci/ai-blaise/runbook-command-check.sh
bash ci/ai-blaise/docs-evidence-boundary-check.sh
bash ci/ai-blaise/production-gap-audit.sh >/dev/null

release_check="$(mktemp)"
trap 'rm -f "${release_check}"' EXIT

if bash ci/ai-blaise/production-readiness-check.sh production-release >"${release_check}"; then
  echo "production release unexpectedly passed without a trusted current-source evidence verifier" >&2
  exit 1
else
  release_status=$?
fi
if [[ "${release_status}" != "1" ]]; then
  echo "release-gap check must return unqualified exit 1, not an input or execution error (${release_status})" >&2
  exit 1
fi

python3 - "${release_check}" <<'PY'
import csv
from pathlib import Path
import sys

with Path(sys.argv[1]).open(encoding="utf-8", newline="") as report_file:
    actual = list(csv.reader(report_file, delimiter="\t"))
with Path("docs/features.tsv").open(encoding="utf-8", newline="") as register_file:
    features = list(csv.DictReader(register_file, delimiter="\t"))

columns = [
    "id", "title", "kind", "disposition", "canonical_ids", "maturity",
    "historical_claimed_status", "scope", "blockers",
]
expected = [
    ["feature_release_gaps", "blocked"],
    ["release_evidence_verifier", "unimplemented"],
    ["feature_release_gap_columns", *columns],
]
for feature in features:
    expected.append([
        "feature_release_gap",
        *(feature["claimed_status" if key == "historical_claimed_status" else key]
          for key in columns),
    ])
if not features or actual != expected:
    raise SystemExit("release-gap report must exactly preserve every inventory row and its blockers")
print(f"release_report_rows={len(features)}")
PY

printf 'release_hardening_runbook=passed\n'
printf 'production_release_blocked=true\n'
printf 'release_evidence_verifier=unimplemented\n'
printf 'claim_boundary=rejection-contract-only\n'
