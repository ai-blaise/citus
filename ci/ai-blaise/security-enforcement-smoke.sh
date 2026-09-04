#!/usr/bin/env bash
set -euo pipefail

expected=$'6	5	5	20	5	7	5	6	6	6'
output="$(cargo run -q -p ai_blaise_citus_operator -- run-security-canonical)"

if ! printf '%s\n' "${output}" | grep -Fqx "${expected}"; then
  echo "operator security enforcement runner did not emit expected TSV row" >&2
  echo "Expected: ${expected}" >&2
  echo "Actual output:" >&2
  printf '%s\n' "${output}" >&2
  exit 1
fi
