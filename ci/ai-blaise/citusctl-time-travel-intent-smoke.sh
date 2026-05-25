#!/usr/bin/env bash
set -euo pipefail

# FEATURE: B5
# Real citusctl time-travel intent proof: strict UTC parsing, deterministic
# staleness-window validation, plan-id-gated apply, and local audit append.

if [[ -f "${HOME}/.cargo/env" ]]; then
  source "${HOME}/.cargo/env"
fi

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

run_citusctl() {
  cargo run -q -p ai_blaise_citusctl -- "$@"
}

expect_contains() {
  local description="$1"
  local haystack="$2"
  local needle="$3"
  if [[ "${haystack}" != *"${needle}"* ]]; then
    echo "${description} did not contain ${needle}" >&2
    echo "${haystack}" >&2
    exit 1
  fi
}

extract_json_string() {
  local key="$1"
  sed -n "s/.*\"${key}\":\"\\([^\"]*\\)\".*/\\1/p"
}

state_dir="$(mktemp -d -t ai-blaise-citusctl-time-travel.XXXXXX)"
bad_stdout="$(mktemp -t ai-blaise-citusctl-time-travel-bad.XXXXXX.out)"
bad_stderr="$(mktemp -t ai-blaise-citusctl-time-travel-bad.XXXXXX.err)"
cleanup() {
  rm -rf "${state_dir}"
  rm -f "${bad_stdout}" "${bad_stderr}"
}
trap cleanup EXIT

target_time="2026-05-24T00:00:00Z"
now_time="2026-05-24T00:00:30Z"
max_staleness=60

plan_json="$(run_citusctl plan time-travel "${target_time}" --now "${now_time}" --max-staleness-seconds "${max_staleness}" --state-dir "${state_dir}" --format json)"
expect_contains "time-travel plan json" "${plan_json}" '"mode":"plan"'
expect_contains "time-travel plan json" "${plan_json}" '"accepted":true'
expect_contains "time-travel plan json" "${plan_json}" '"age_seconds":30'
expect_contains "time-travel plan json" "${plan_json}" '"audit_record_written":false'
expect_contains "time-travel plan json" "${plan_json}" '"evidence_boundary":"time-travel-intent-validation-only"'
plan_id="$(printf '%s\n' "${plan_json}" | extract_json_string plan_id)"
if [[ -z "${plan_id}" || "${plan_id}" != time-travel-* ]]; then
  echo "time-travel plan did not include deterministic plan id" >&2
  echo "${plan_json}" >&2
  exit 1
fi

apply_tsv="$(run_citusctl apply "${plan_id}" time-travel "${target_time}" --now "${now_time}" --max-staleness-seconds "${max_staleness}" --state-dir "${state_dir}" --format tsv)"
expected_header=$'mode\tplan_id\ttarget_time\tnow\tage_seconds\tmax_staleness_seconds\taccepted\tdry_run\taudit_record_written\tsteps\tevidence_boundary'
actual_header="$(printf '%s\n' "${apply_tsv}" | sed -n '1p')"
if [[ "${actual_header}" != "${expected_header}" ]]; then
  echo "unexpected time-travel TSV header" >&2
  echo "${actual_header}" >&2
  exit 1
fi
expect_contains "time-travel apply tsv" "${apply_tsv}" $'\t30\t60\ttrue\tfalse\ttrue\t4\ttime-travel-intent-validation-only'

audit_path="${state_dir}/time-travel-intent.audit.tsv"
if [[ ! -s "${audit_path}" ]]; then
  echo "time-travel apply did not write time-travel-intent.audit.tsv" >&2
  exit 1
fi
if ! grep -Fq "${plan_id}" "${audit_path}" || ! grep -Fq "time-travel-intent-validation-only" "${audit_path}"; then
  cat "${audit_path}" >&2
  echo "time-travel audit row missing plan/evidence boundary" >&2
  exit 1
fi

if run_citusctl apply wrong-plan time-travel "${target_time}" --now "${now_time}" --max-staleness-seconds "${max_staleness}" --state-dir "${state_dir}" --format json >"${bad_stdout}" 2>"${bad_stderr}"; then
  cat "${bad_stdout}" >&2
  echo "time-travel apply accepted a mismatched plan id" >&2
  exit 1
fi
if ! grep -Fq "plan_id does not match current time-travel intent plan" "${bad_stderr}"; then
  cat "${bad_stdout}" >&2
  cat "${bad_stderr}" >&2
  echo "time-travel mismatched plan id did not fail closed" >&2
  exit 1
fi

for invalid_time in \
  "2026-05-24 00:00:00" \
  "2026-02-29T00:00:00Z" \
  "2026-05-24T00:00:60Z"; do
  if run_citusctl plan time-travel "${invalid_time}" --now "${now_time}" --max-staleness-seconds "${max_staleness}" --state-dir "${state_dir}" --format json >"${bad_stdout}" 2>"${bad_stderr}"; then
    cat "${bad_stdout}" >&2
    echo "time-travel accepted invalid UTC timestamp ${invalid_time}" >&2
    exit 1
  fi
  if ! grep -Fq "target_time must be an RFC3339 UTC timestamp" "${bad_stderr}"; then
    cat "${bad_stdout}" >&2
    cat "${bad_stderr}" >&2
    echo "invalid timestamp ${invalid_time} did not report strict UTC error" >&2
    exit 1
  fi
done

if run_citusctl plan time-travel "2026-05-23T23:59:00Z" --now "${now_time}" --max-staleness-seconds "${max_staleness}" --state-dir "${state_dir}" --format json >"${bad_stdout}" 2>"${bad_stderr}"; then
  cat "${bad_stdout}" >&2
  echo "time-travel accepted an out-of-window timestamp" >&2
  exit 1
fi
if ! grep -Fq "older than max_staleness_seconds 60" "${bad_stderr}"; then
  cat "${bad_stdout}" >&2
  cat "${bad_stderr}" >&2
  echo "out-of-window timestamp did not report staleness guard" >&2
  exit 1
fi

if run_citusctl plan time-travel "2026-05-24T00:00:31Z" --now "${now_time}" --max-staleness-seconds "${max_staleness}" --state-dir "${state_dir}" --format json >"${bad_stdout}" 2>"${bad_stderr}"; then
  cat "${bad_stdout}" >&2
  echo "time-travel accepted a future timestamp" >&2
  exit 1
fi
if ! grep -Fq "must not be in the future" "${bad_stderr}"; then
  cat "${bad_stdout}" >&2
  cat "${bad_stderr}" >&2
  echo "future timestamp did not report future-target guard" >&2
  exit 1
fi

echo "B5	${plan_id}	${target_time}	${now_time}	30	${max_staleness}	time-travel-intent-validation-only"
echo "citusctl-time-travel-intent-smoke passed"
