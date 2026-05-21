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
  rm -f "${missing_plan_stdout}" "${missing_plan_stderr}"
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

expect_output \
  "plan inspect cluster" \
  "citusctl inspect destructive=false requires_plan_id=true steps=3" \
  plan inspect cluster

expect_output \
  "plan apply manifest" \
  "citusctl apply destructive=true requires_plan_id=true steps=3" \
  plan apply deploy/k8s/helm/citus-overlay/values-prod.yaml

expect_output \
  "apply plan id manifest" \
  "citusctl apply destructive=true requires_plan_id=true steps=5" \
  apply plan-123 apply deploy/k8s/helm/citus-overlay/values-prod.yaml

echo "ai_blaise_citusctl plan-id smoke passed"
