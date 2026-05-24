#!/usr/bin/env bash
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"

# FEATURE: PM3 PM4

expected_header=$'records\tpromoted\tobservations\taudit_events\tidempotent_replays\tretry_attempts\tfailed_commands\tregression_violations\tsql_contract_commands'
expected_row=$'1\t1\t1\t8\t1\t1\t1\t1\t5'

output="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-plan-runtime-canonical)"

if ! printf '%s\n' "${output}" | grep -Fqx "${expected_header}"; then
  echo "companion plan runtime canonical header mismatch" >&2
  printf '%s\n' "${output}" >&2
  exit 1
fi

if ! printf '%s\n' "${output}" | grep -Fqx "${expected_row}"; then
  echo "companion plan runtime canonical row mismatch" >&2
  printf '%s\n' "${output}" >&2
  exit 1
fi
