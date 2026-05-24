#!/usr/bin/env bash
set -euo pipefail

report="$(cargo run -q -p ai_blaise_citus_companion --bin companion_runtime_depth_a -- run-canonical)"

printf '%s
' "${report}"

grep -Fq $'features\tfeature_ids\tmigration_phases\tmigration_sql_batches' <<<"${report}"
grep -Fq $'5\tM1,M11,R6,C4,C5\t6\t4\t9\t12\t3\t1\t7\t7\t2\t14' <<<"${report}"

printf 'companion runtime depth A smoke passed\n'
