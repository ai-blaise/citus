#!/usr/bin/env bash
# Focused D1/M8 citusctl dev lifecycle smoke.
#
# Exercises the real CLI against the local state-file runtime. This is a
# deterministic dev lifecycle boundary only: no Kubernetes cluster, Docker
# Compose stack, Citus data plane, or mutating manifest apply is attempted.

set -euo pipefail

if [[ -f "${HOME}/.cargo/env" ]]; then
  source "${HOME}/.cargo/env"
fi

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

run_citusctl() {
  cargo run -q -p ai_blaise_citusctl -- "$@"
}

echo "==> citusctl-dev-lifecycle-smoke: plan dev up summary"
plan_up="$(run_citusctl plan dev up)"
echo "${plan_up}"
if [[ "${plan_up}" != "citusctl dev destructive=false requires_plan_id=true steps=3" ]]; then
  echo "unexpected plan dev up summary" >&2
  exit 1
fi

echo "==> citusctl-dev-lifecycle-smoke: apply dev down summary"
apply_down="$(run_citusctl apply plan-dev-down dev down)"
echo "${apply_down}"
if [[ "${apply_down}" != "citusctl dev destructive=false requires_plan_id=true steps=5" ]]; then
  echo "unexpected apply dev down summary" >&2
  exit 1
fi

echo "==> citusctl-dev-lifecycle-smoke: runtime canonical"
runtime="$(run_citusctl run-dev-lifecycle-canonical)"
echo "${runtime}"
header="$(printf '%s\n' "${runtime}" | sed -n '1p')"
expected_header=$'state_dir	plan_up_steps	apply_up_changed	idempotent_up_changed	apply_down_changed	idempotent_down_changed	final_state_present	cleanup_guard	evidence_boundary'
if [[ "${header}" != "${expected_header}" ]]; then
  echo "unexpected dev lifecycle canonical header" >&2
  exit 1
fi
row="$(printf '%s\n' "${runtime}" | sed -n '2p')"
IFS=$'	' read -r state_dir plan_up_steps apply_up_changed idempotent_up_changed apply_down_changed idempotent_down_changed final_state_present cleanup_guard evidence_boundary <<< "${row}"
if [[ "${plan_up_steps}" != "3" || "${apply_up_changed}" != "true" || "${idempotent_up_changed}" != "false" ]]; then
  echo "dev up lifecycle idempotency evidence mismatch" >&2
  exit 1
fi
if [[ "${apply_down_changed}" != "true" || "${idempotent_down_changed}" != "false" || "${final_state_present}" != "false" ]]; then
  echo "dev down cleanup evidence mismatch" >&2
  exit 1
fi
if [[ "${cleanup_guard}" != "state-file-only-no-recursive-delete" || "${evidence_boundary}" != "local-state-file-only" ]]; then
  echo "dev lifecycle boundary fields mismatch" >&2
  exit 1
fi
if [[ -e "${state_dir}/dev-lifecycle.state" ]]; then
  echo "dev lifecycle state file leaked after canonical down" >&2
  exit 1
fi

cargo test -q -p ai_blaise_citusctl --all-targets

echo "citusctl-dev-lifecycle-smoke passed"
