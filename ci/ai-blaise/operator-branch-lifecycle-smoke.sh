#!/usr/bin/env bash
set -euo pipefail

cargo test -q -p ai_blaise_citus_operator branch

output="$(cargo run -q -p ai_blaise_citus_operator -- run-branch-lifecycle-canonical)"
expected=$'action	from_phase	to_phase	steps	source	target	snapshot_ready	target_ready	active_sessions	pending_migrations
apply	pending	ready	7	prod-us-east	branch-review	true	true	0	0
suspend	ready	suspended	6	prod-us-east	branch-review	true	true	0	0
promote	ready	promoted	9	prod-us-east	branch-review	true	true	0	0'

if [[ "${output}" != "${expected}" ]]; then
  echo "Branch lifecycle canonical contract changed." >&2
  echo "Expected:" >&2
  printf '%s
' "${expected}" >&2
  echo "Actual:" >&2
  printf '%s
' "${output}" >&2
  exit 1
fi

grep -Fq "FEATURE: C6" operator/src/crds/branch.rs
grep -Fq "FEATURE: C7" operator/src/crds/branch.rs
grep -Fq "FEATURE: C8" operator/src/crds/branch.rs

grep -Fq "SnapshotNotReady" operator/src/crds/branch.rs
grep -Fq "SuspendedPromotionBlocked" operator/src/crds/branch.rs
grep -Fq "PendingMigrations" operator/src/crds/branch.rs

printf "operator_branch_lifecycle	%s
" "${output//$'
'/$'|'}"
