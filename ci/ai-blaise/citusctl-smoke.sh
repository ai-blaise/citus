#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

run_citusctl() {
  cargo run -q -p ai_blaise_citusctl -- "$@"
}

expect_output() {
  local description="$1"
  local expected="$2"
  shift 2

  local actual
  actual="$(run_citusctl "$@")"
  if [[ "${actual}" != "${expected}" ]]; then
    echo "unexpected citusctl output for ${description}" >&2
    echo "expected: ${expected}" >&2
    echo "actual:   ${actual}" >&2
    exit 1
  fi
}

missing_plan_stdout="$(mktemp -t ai-blaise-citusctl-missing-plan.XXXXXX.out)"
missing_plan_stderr="$(mktemp -t ai-blaise-citusctl-missing-plan.XXXXXX.err)"
cleanup() {
  rm -f "${missing_plan_stdout}" "${missing_plan_stderr}" "${bad_plan_stdout:-}" "${bad_plan_stderr:-}"
}
trap cleanup EXIT

if run_citusctl apply >"${missing_plan_stdout}" 2>"${missing_plan_stderr}"; then
  cat "${missing_plan_stdout}" >&2
  cat "${missing_plan_stderr}" >&2
  echo "citusctl apply without a plan id unexpectedly succeeded" >&2
  exit 1
fi

if ! grep -Fq "citusctl: plan_id must not be empty" "${missing_plan_stderr}"; then
  cat "${missing_plan_stdout}" >&2
  cat "${missing_plan_stderr}" >&2
  echo "citusctl apply without a plan id did not report the guarded plan_id error" >&2
  exit 1
fi

bad_plan_stdout="$(mktemp -t ai-blaise-citusctl-bad-plan.XXXXXX.out)"
bad_plan_stderr="$(mktemp -t ai-blaise-citusctl-bad-plan.XXXXXX.err)"
if run_citusctl apply "not ok" inspect cluster >"${bad_plan_stdout}" 2>"${bad_plan_stderr}"; then
  cat "${bad_plan_stdout}" >&2
  echo "citusctl apply accepted an unstable plan id" >&2
  exit 1
fi
if ! grep -Fq "citusctl: plan_id must be stable ascii and non-empty" "${bad_plan_stderr}"; then
  cat "${bad_plan_stdout}" >&2
  cat "${bad_plan_stderr}" >&2
  echo "citusctl apply did not fail closed on an unstable plan id" >&2
  exit 1
fi

expect_output \
  "plan inspect cluster" \
  "citusctl inspect destructive=false requires_plan_id=true steps=3" \
  plan inspect cluster

expect_output \
  "plan apply manifest" \
  "citusctl apply destructive=true requires_plan_id=true steps=3" \
  plan apply external/citus-cluster/values-prod.yaml

expect_output \
  "apply plan id manifest" \
  "citusctl apply destructive=true requires_plan_id=true steps=5" \
  apply plan-123 apply external/citus-cluster/values-prod.yaml

wal_fixture_dir="$(mktemp -d -t ai-blaise-citusctl-wal.XXXXXX)"
wal_fixture="${wal_fixture_dir}/fixture.env"
cat >"${wal_fixture}" <<'FIXTURE'
source_uri=s3://citus-wal/prod
timeline=0000000100000000000000A1
start_time=2026-05-21T09:00:00Z
end_time=2026-05-21T11:00:00Z
segments=3
FIXTURE
trap 'rm -f "${missing_plan_stdout}" "${missing_plan_stderr}" "${bad_plan_stdout:-}" "${bad_plan_stderr:-}"; rm -rf "${wal_fixture_dir}"' EXIT

expect_output \
  "wal replay fixture json" \
  '{"actions":["validate_source","inspect_fixture","bound_target_time","render_replay_plan"],"end_time":"2026-05-21T11:00:00Z","segments":3,"source_uri":"s3://citus-wal/prod","start_time":"2026-05-21T09:00:00Z","target_time":"2026-05-21T10:00:00Z","timeline":"0000000100000000000000A1"}' \
  plan wal-replay s3://citus-wal/prod 2026-05-21T10:00:00Z --fixture "${wal_fixture}" --json

wal_bad_stdout="${wal_fixture_dir}/bad.out"
wal_bad_stderr="${wal_fixture_dir}/bad.err"
if run_citusctl plan wal-replay s3://citus-wal/prod 2026-05-21T12:00:00Z --fixture "${wal_fixture}" --json >"${wal_bad_stdout}" 2>"${wal_bad_stderr}"; then
  cat "${wal_bad_stdout}" >&2
  cat "${wal_bad_stderr}" >&2
  echo "citusctl wal-replay accepted a target_time outside the fixture range" >&2
  exit 1
fi
if ! grep -Fq "citusctl: unknown target_time: outside fixture range" "${wal_bad_stderr}"; then
  cat "${wal_bad_stdout}" >&2
  cat "${wal_bad_stderr}" >&2
  echo "citusctl wal-replay did not fail closed on an out-of-range target_time" >&2
  exit 1
fi

if run_citusctl plan wal-replay https://example.invalid/wal 2026-05-21T10:00:00Z --fixture "${wal_fixture}" --json >"${wal_bad_stdout}" 2>"${wal_bad_stderr}"; then
  cat "${wal_bad_stdout}" >&2
  cat "${wal_bad_stderr}" >&2
  echo "citusctl wal-replay accepted an unsupported source_uri" >&2
  exit 1
fi
if ! grep -Fq "citusctl: unknown source_uri: https://example.invalid/wal" "${wal_bad_stderr}"; then
  cat "${wal_bad_stdout}" >&2
  cat "${wal_bad_stderr}" >&2
  echo "citusctl wal-replay did not reject unsupported source_uri schemes" >&2
  exit 1
fi

echo "ai_blaise_citusctl plan-id smoke passed"
