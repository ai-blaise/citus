#!/usr/bin/env bash
set -euo pipefail

source "${HOME}/.cargo/env" 2>/dev/null || true

cargo test -q -p ai_blaise_citus_operator --lib region
cargo test -q -p ai_blaise_citus_operator --lib survival
cargo test -q -p ai_blaise_citus_operator --bin ai_blaise_citus_operator multiregion

output="$(cargo run -q -p ai_blaise_citus_operator -- run-multiregion-contracts-canonical)"
expected=$'surface\tstatus\tsteps\ttopology_key\tdeclared_regions\tleader_region\tlive_k8s_exercised\nregion\tready\t4\ttopology.kubernetes.io/zone\t2\tus-east-1\tfalse\nplacement\tready\t4\ttopology.kubernetes.io/region\t2\tus-east-1\tfalse\nsurvival\tready\t4\ttopology.kubernetes.io/region\t2\tus-east-1\tfalse'

if [[ "${output}" != "${expected}" ]]; then
  echo "multi-region contracts canonical output changed" >&2
  echo "Expected:" >&2
  printf '%s\n' "${expected}" >&2
  echo "Actual:" >&2
  printf '%s\n' "${output}" >&2
  exit 1
fi

grep -Fq "FEATURE: MR3" operator/src/reconcile/region.rs
grep -Fq "RegionalRowPlacementPlan" operator/src/reconcile/region.rs
grep -Fq "DuplicateRegionInventory" operator/src/reconcile/survival_goal.rs
grep -Fq "live_k8s_exercised" operator/src/main.rs

printf "operator_multiregion_contracts\t%s\n" "${output//$'\n'/$'|'}"
