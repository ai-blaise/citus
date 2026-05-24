#!/usr/bin/env bash
set -euo pipefail

output="$(cargo run -q -p ai_blaise_citus_operator -- run-endpointslice-retarget-canonical)"

require_line() {
  local expected="$1"
  if ! printf '%s\n' "${output}" | grep -Fqx "${expected}"; then
    echo "EndpointSlice retarget smoke missing expected line: ${expected}" >&2
    printf '%s\n' "${output}" >&2
    exit 1
  fi
}

require_line $'phase\tstatus\tgeneration\tselected\tendpoints\tslice'
require_line $'initial\tactive\t1\tprimary\t1\tai-blaise-realtime-retarget'
require_line $'primary_failed\tactive\t1\tstandby\t1\tai-blaise-realtime-retarget'
require_line $'all_failed\tfail-closed\t1\tnone\t0\tai-blaise-realtime-retarget'

grep -Fq 'kind: EndpointSlice' <<<"${output}"
grep -Fq 'kubernetes.io/service-name: ai-blaise-realtime' <<<"${output}"
grep -Fq 'ai-blaise.citus/retarget-selected: none' <<<"${output}"
grep -Fq 'endpoints: []' <<<"${output}"
grep -Fq '"selector": null' <<<"${output}"
grep -Fq '"ai-blaise.citus/retarget-selected": "none"' <<<"${output}"
