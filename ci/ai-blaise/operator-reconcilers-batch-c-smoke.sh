#!/usr/bin/env bash
set -euo pipefail

expected=$'ai-blaise-citus-repack-weekly-orders	pg_repack	5	users-add-display-name	8	write_only	update_origin_differs	apply_remote_if_newer	3	ai-blaise-citus-sidecar-primary-realtime	2	4'

cargo test -q -p ai_blaise_citus_operator

output="$(cargo run -q -p ai_blaise_citus_operator -- run-reconcile-plans-batch-c)"
if [[ "${output}" != "${expected}" ]]; then
  echo "Batch C reconcile plan contract changed." >&2
  echo "Expected: ${expected}" >&2
  echo "Actual: ${output}" >&2
  exit 1
fi

grep -Fq "FEATURE: R7" operator/src/reconcile/scheduled_repack.rs
grep -Fq "FEATURE: C4" operator/src/reconcile/conflict_policy.rs
grep -Fq "FEATURE: C5" operator/src/reconcile/conflict_policy.rs
grep -Fq "FEATURE: C9" operator/src/reconcile/migration.rs
grep -Fq "FEATURE: M3" operator/src/reconcile/migration.rs
grep -Fq "FEATURE: M14" operator/src/reconcile/migration.rs
grep -Fq "FEATURE: O5" operator/src/reconcile/sidecar.rs

printf "operator_reconcilers_batch_c	%s
" "${output}"
