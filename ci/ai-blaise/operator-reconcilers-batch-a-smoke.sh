#!/usr/bin/env bash
set -euo pipefail

source "${HOME}/.cargo/env" 2>/dev/null || true

output="$(cargo run -q -p ai_blaise_citus_operator -- run-reconcilers-batch-a)"
expected_header=$'tenant_apply_steps\ttenant_sql_steps\tregion_apply_steps\tregion_sql_steps\tsurvival_goal_apply_steps\tbackup_apply_steps\tbackup_status_endpoints\tsurvival_topology_key\tbackup_archive_scheme'
expected_row=$'5\t3\t4\t2\t4\t4\t2\ttopology.kubernetes.io/region\ts3'

if ! printf '%s\n' "${output}" | grep -Fqx "${expected_header}"; then
  echo "operator reconcilers Batch A smoke missing expected header" >&2
  printf '%s\n' "${output}" >&2
  exit 1
fi

if ! printf '%s\n' "${output}" | grep -Fqx "${expected_row}"; then
  echo "operator reconcilers Batch A smoke missing expected row" >&2
  echo "Expected: ${expected_row}" >&2
  echo "Actual output:" >&2
  printf '%s\n' "${output}" >&2
  exit 1
fi

cargo test -q -p ai_blaise_citus_operator
