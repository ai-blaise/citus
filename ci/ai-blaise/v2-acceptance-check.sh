#!/usr/bin/env bash
set -euo pipefail

expected=$'15\t15\t3\tfalse\t45\t4500\t800\t1000\t4000\t180\t100\t2000\t3\t3\t3\trelease-14.0'
output="$(cargo run -q -p ai_blaise_citus_e2e --bin release_gate_report)"

if ! printf '%s\n' "${output}" | grep -Fqx "${expected}"; then
  echo "release gate report did not emit expected TSV row" >&2
  echo "Expected: ${expected}" >&2
  echo "Actual output:" >&2
  printf '%s\n' "${output}" >&2
  exit 1
fi

# The production gap audit treats this as modeled acceptance. Passing this
# check is a release prerequisite, not production evidence for alpha features.
bash ci/ai-blaise/upstream-merge-dry.sh
bash ci/ai-blaise/production-readiness-check.sh
bash ci/ai-blaise/docs-evidence-boundary-check.sh
