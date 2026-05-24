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

expect_tsv_field() {
  local description="$1"
  local actual="$2"
  local expected="$3"
  if [[ "${actual}" != "${expected}" ]]; then
    echo "${description}: expected ${expected}, got ${actual}" >&2
    exit 1
  fi
}

parse_tsv_row() {
  local payload="$1"
  local expected_header=$'mode\tcluster\taction\tstate_path\tplan_id\tdry_run\tchanged\tstate_written\tstate_removed\taudit_record_written\tbefore_status\tafter_status\tbefore_generation\tafter_generation\tsteps\tcleanup_guard\tevidence_boundary'
  local header row
  header="$(printf '%s\n' "${payload}" | sed -n '1p')"
  row="$(printf '%s\n' "${payload}" | sed -n '2p')"
  if [[ "${header}" != "${expected_header}" ]]; then
    echo "unexpected dev lifecycle TSV header" >&2
    echo "${header}" >&2
    exit 1
  fi
  IFS=$'\t' read -r mode cluster action state_path plan_id dry_run changed state_written state_removed audit_record_written before_status after_status before_generation after_generation steps cleanup_guard evidence_boundary <<< "${row}"
}

missing_state_stdout="$(mktemp -t ai-blaise-citusctl-missing-state.XXXXXX.out)"
missing_state_stderr="$(mktemp -t ai-blaise-citusctl-missing-state.XXXXXX.err)"
bad_plan_stdout="$(mktemp -t ai-blaise-citusctl-bad-plan.XXXXXX.out)"
bad_plan_stderr="$(mktemp -t ai-blaise-citusctl-bad-plan.XXXXXX.err)"
state_dir="$(mktemp -d -t ai-blaise-citusctl-dev-lifecycle.XXXXXX)"
cleanup() {
  rm -f "${missing_state_stdout}" "${missing_state_stderr}" "${bad_plan_stdout}" "${bad_plan_stderr}"
  rm -rf "${state_dir}"
}
trap cleanup EXIT

state_path="${state_dir}/dev-lifecycle.state"
audit_path="${state_dir}/dev-lifecycle.audit.tsv"

if run_citusctl plan dev up --format json >"${missing_state_stdout}" 2>"${missing_state_stderr}"; then
  cat "${missing_state_stdout}" >&2
  echo "citusctl plan dev up --format json unexpectedly succeeded without --state-dir" >&2
  exit 1
fi
if ! grep -Fq "citusctl: state_dir must not be empty" "${missing_state_stderr}"; then
  cat "${missing_state_stdout}" >&2
  cat "${missing_state_stderr}" >&2
  echo "missing --state-dir failure did not report the guarded state_dir error" >&2
  exit 1
fi

if run_citusctl apply "not ok" dev up --state-dir "${state_dir}" --format tsv >"${bad_plan_stdout}" 2>"${bad_plan_stderr}"; then
  cat "${bad_plan_stdout}" >&2
  echo "citusctl apply accepted an unstable plan id" >&2
  exit 1
fi
if ! grep -Fq "citusctl: plan_id must be stable ascii and non-empty" "${bad_plan_stderr}"; then
  cat "${bad_plan_stdout}" >&2
  cat "${bad_plan_stderr}" >&2
  echo "unstable plan id failure did not report the guarded plan_id error" >&2
  exit 1
fi

plan_json="$(run_citusctl plan dev up --state-dir "${state_dir}" --format json)"
expected_plan_json='{"action":"up","after_generation":1,"after_status":"running","audit_record_written":false,"before_generation":0,"before_status":"absent","changed":true,"cleanup_guard":"state-file-only-no-recursive-delete","cluster":"dev-citus","dry_run":true,"evidence_boundary":"local-state-file-only","mode":"plan","plan_id":null,"state_path":"'"${state_path}"'","state_removed":false,"state_written":false,"steps":3}'
if [[ "${plan_json}" != "${expected_plan_json}" ]]; then
  echo "unexpected plan dev up JSON" >&2
  echo "expected: ${expected_plan_json}" >&2
  echo "actual:   ${plan_json}" >&2
  exit 1
fi
if [[ -e "${state_path}" || -e "${audit_path}" ]]; then
  echo "plan dev up wrote local runtime state or audit files" >&2
  exit 1
fi

apply_up="$(run_citusctl apply plan-dev-up-1 dev up --state-dir "${state_dir}" --format tsv)"
parse_tsv_row "${apply_up}"
expect_tsv_field "apply up mode" "${mode}" "apply"
expect_tsv_field "apply up action" "${action}" "up"
expect_tsv_field "apply up plan id" "${plan_id}" "plan-dev-up-1"
expect_tsv_field "apply up dry_run" "${dry_run}" "false"
expect_tsv_field "apply up changed" "${changed}" "true"
expect_tsv_field "apply up state_written" "${state_written}" "true"
expect_tsv_field "apply up state_removed" "${state_removed}" "false"
expect_tsv_field "apply up audit_record_written" "${audit_record_written}" "true"
expect_tsv_field "apply up before_status" "${before_status}" "absent"
expect_tsv_field "apply up after_status" "${after_status}" "running"
expect_tsv_field "apply up steps" "${steps}" "7"
expect_tsv_field "apply up evidence_boundary" "${evidence_boundary}" "local-state-file-only"
if [[ ! -s "${state_path}" || ! -s "${audit_path}" ]]; then
  echo "apply dev up did not write the expected local state and audit files" >&2
  exit 1
fi

idempotent_up="$(run_citusctl apply plan-dev-up-2 dev up --state-dir "${state_dir}" --format tsv)"
parse_tsv_row "${idempotent_up}"
expect_tsv_field "idempotent up changed" "${changed}" "false"
expect_tsv_field "idempotent up state_written" "${state_written}" "false"
expect_tsv_field "idempotent up audit_record_written" "${audit_record_written}" "true"
expect_tsv_field "idempotent up before_status" "${before_status}" "running"
expect_tsv_field "idempotent up after_status" "${after_status}" "running"

down="$(run_citusctl apply plan-dev-down-1 dev down --state-dir "${state_dir}" --format tsv)"
parse_tsv_row "${down}"
expect_tsv_field "down action" "${action}" "down"
expect_tsv_field "down changed" "${changed}" "true"
expect_tsv_field "down state_written" "${state_written}" "false"
expect_tsv_field "down state_removed" "${state_removed}" "true"
expect_tsv_field "down audit_record_written" "${audit_record_written}" "true"
expect_tsv_field "down before_status" "${before_status}" "running"
expect_tsv_field "down after_status" "${after_status}" "absent"
if [[ -e "${state_path}" ]]; then
  echo "dev down left the tracked state file behind" >&2
  exit 1
fi
if [[ ! -s "${audit_path}" ]]; then
  echo "dev down removed the audit log despite the state-file-only cleanup guard" >&2
  exit 1
fi

idempotent_down="$(run_citusctl apply plan-dev-down-2 dev down --state-dir "${state_dir}" --format tsv)"
parse_tsv_row "${idempotent_down}"
expect_tsv_field "idempotent down changed" "${changed}" "false"
expect_tsv_field "idempotent down state_removed" "${state_removed}" "false"
expect_tsv_field "idempotent down audit_record_written" "${audit_record_written}" "true"
expect_tsv_field "idempotent down before_status" "${before_status}" "absent"
expect_tsv_field "idempotent down after_status" "${after_status}" "absent"

if [[ "$(wc -l <"${audit_path}" | tr -d ' ')" != "5" ]]; then
  echo "expected audit header plus four apply records" >&2
  cat "${audit_path}" >&2
  exit 1
fi
if ! grep -Fq $'plan-dev-down-1\tdev-citus\tdown\trunning\tabsent\ttrue\tfalse\ttrue\tlocal-state-file-only' "${audit_path}"; then
  echo "dev down audit record missing expected state-file-only evidence" >&2
  cat "${audit_path}" >&2
  exit 1
fi

echo "==> citusctl-dev-lifecycle-smoke: legacy summaries and canonical report"
summary="$(run_citusctl plan dev up)"
if [[ "${summary}" != "citusctl dev destructive=false requires_plan_id=true steps=3" ]]; then
  echo "unexpected plan dev up summary" >&2
  exit 1
fi
apply_summary="$(run_citusctl apply plan-dev-down dev down)"
if [[ "${apply_summary}" != "citusctl dev destructive=false requires_plan_id=true steps=5" ]]; then
  echo "unexpected apply dev down summary" >&2
  exit 1
fi
runtime="$(run_citusctl run-dev-lifecycle-canonical)"
canonical_header="$(printf '%s\n' "${runtime}" | sed -n '1p')"
expected_canonical_header=$'state_dir\tplan_up_steps\tapply_up_changed\tidempotent_up_changed\tapply_down_changed\tidempotent_down_changed\tfinal_state_present\tcleanup_guard\tevidence_boundary'
if [[ "${canonical_header}" != "${expected_canonical_header}" ]]; then
  echo "unexpected dev lifecycle canonical header" >&2
  exit 1
fi
canonical_row="$(printf '%s\n' "${runtime}" | sed -n '2p')"
IFS=$'\t' read -r canonical_state_dir plan_up_steps apply_up_changed idempotent_up_changed apply_down_changed idempotent_down_changed final_state_present cleanup_guard evidence_boundary <<< "${canonical_row}"
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
if [[ -e "${canonical_state_dir}/dev-lifecycle.state" ]]; then
  echo "dev lifecycle state file leaked after canonical down" >&2
  exit 1
fi

cargo test -q -p ai_blaise_citusctl --all-targets

echo "citusctl-dev-lifecycle-smoke passed"
