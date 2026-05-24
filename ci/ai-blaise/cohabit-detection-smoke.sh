#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

output="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-cohabit-detection-canonical)"
printf '%s
' "${output}"

grep -Fq $'timescaledb	trusted-hook	true	true	true	true	ok' <<<"${output}"
grep -Fq $'pg_cron	clock-worker	true	true	true	true	ok' <<<"${output}"
grep -Fq $'pg_partman	partition-manager	false	false	true	true	ok' <<<"${output}"
grep -Fq $'summary	detected=3	ready=3	hard_failures=0	unsupported=0' <<<"${output}"

echo "cohabit extension detection smoke passed"
