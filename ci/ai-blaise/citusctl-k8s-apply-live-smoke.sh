#!/usr/bin/env bash
set -euo pipefail

# FEATURE: M8
# Live citusctl plan/apply proof: the real CLI performs a server-side
# Kubernetes dry-run, requires the apply plan id to match the rendered manifest
# plan, mutates a live kind cluster with kubectl apply, verifies the resource,
# and appends k8s-manifest-apply.audit.tsv evidence.

if [[ -f "${HOME}/.cargo/env" ]]; then
  source "${HOME}/.cargo/env"
fi

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

cluster="${M8_KIND_CLUSTER:-ai-blaise-m8-citusctl-live}"
namespace="${M8_NAMESPACE:-ai-blaise-m8}"
node_image="${M8_KIND_NODE_IMAGE:-kindest/node:v1.30.0}"
evidence_file="${M8_EVIDENCE:-artifacts/citusctl-k8s-apply-live-evidence.tsv}"
keep_cluster="${KEEP_KIND_CLUSTER:-0}"
work=""

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "$1 is required for M8 citusctl Kubernetes apply live smoke" >&2
    exit 1
  fi
}

cleanup() {
  if [[ -n "${work}" ]]; then
    rm -rf "${work}"
  fi
  if [[ "${keep_cluster}" != "1" ]]; then
    kind delete cluster --name "${cluster}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

require_tool cargo
require_tool docker
require_tool kind
require_tool kubectl
require_tool sed

mkdir -p "$(dirname "${evidence_file}")"
kind delete cluster --name "${cluster}" >/dev/null 2>&1 || true
kind create cluster --name "${cluster}" --image "${node_image}" --wait 120s >/dev/null
kubectl config use-context "kind-${cluster}" >/dev/null
kubectl create namespace "${namespace}" >/dev/null

work="$(mktemp -d -t ai-blaise-citusctl-k8s.XXXXXX)"
state_dir="${work}/state"
manifest="${work}/m8-configmap.yaml"
bad_manifest="${work}/bad-configmap.yaml"
plan_stderr="${work}/plan.err"
bad_plan_stdout="${work}/bad-plan.out"
bad_plan_stderr="${work}/bad-plan.err"

cat >"${manifest}" <<EOF
apiVersion: v1
kind: ConfigMap
metadata:
  name: ai-blaise-citusctl-live
data:
  feature: M8
  evidence: live-kubernetes-manifest-apply
EOF

cat >"${bad_manifest}" <<EOF
apiVersion: v1
kind: ConfigMap
metadata:
data:
  feature: M8
EOF

run_citusctl() {
  cargo run -q -p ai_blaise_citusctl -- "$@"
}

extract_json_string() {
  local key="$1"
  sed -n "s/.*\"${key}\":\"\\([^\"]*\\)\".*/\\1/p"
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

plan_json="$(run_citusctl plan apply "${manifest}" --namespace "${namespace}" --state-dir "${state_dir}" --format json 2>"${plan_stderr}")"
expect_contains "plan json" "${plan_json}" '"mode":"plan"'
expect_contains "plan json" "${plan_json}" '"dry_run":true'
expect_contains "plan json" "${plan_json}" '"audit_record_written":false'
expect_contains "plan json" "${plan_json}" '"resources":["configmap/ai-blaise-citusctl-live"]'
expect_contains "plan json" "${plan_json}" '"evidence_boundary":"live-kubernetes-manifest-apply"'
plan_id="$(printf '%s\n' "${plan_json}" | extract_json_string plan_id)"
if [[ -z "${plan_id}" || "${plan_id}" != k8s-apply-* ]]; then
  echo "plan output did not include a deterministic k8s-apply plan_id" >&2
  echo "${plan_json}" >&2
  exit 1
fi

if run_citusctl apply wrong-plan-id apply "${manifest}" --namespace "${namespace}" --state-dir "${state_dir}" --format json >"${bad_plan_stdout}" 2>"${bad_plan_stderr}"; then
  cat "${bad_plan_stdout}" >&2
  echo "citusctl apply accepted a mismatched Kubernetes manifest plan id" >&2
  exit 1
fi
if ! grep -Fq "plan_id does not match current Kubernetes manifest plan" "${bad_plan_stderr}"; then
  cat "${bad_plan_stdout}" >&2
  cat "${bad_plan_stderr}" >&2
  echo "mismatched plan id failure did not report the guarded Kubernetes plan error" >&2
  exit 1
fi

apply_json="$(run_citusctl apply "${plan_id}" apply "${manifest}" --namespace "${namespace}" --state-dir "${state_dir}" --format json)"
expect_contains "apply json" "${apply_json}" '"mode":"apply"'
expect_contains "apply json" "${apply_json}" '"applied":true'
expect_contains "apply json" "${apply_json}" '"changed":true'
expect_contains "apply json" "${apply_json}" '"audit_record_written":true'
expect_contains "apply json" "${apply_json}" '"resources":["configmap/ai-blaise-citusctl-live"]'

feature_value="$(kubectl -n "${namespace}" get configmap ai-blaise-citusctl-live -o jsonpath='{.data.feature}')"
if [[ "${feature_value}" != "M8" ]]; then
  echo "live Kubernetes ConfigMap data mismatch: ${feature_value}" >&2
  exit 1
fi

reapply_json="$(run_citusctl apply "${plan_id}" apply "${manifest}" --namespace "${namespace}" --state-dir "${state_dir}" --format json)"
expect_contains "reapply json" "${reapply_json}" '"applied":true'
expect_contains "reapply json" "${reapply_json}" '"changed":false'

if run_citusctl plan apply "${bad_manifest}" --namespace "${namespace}" --state-dir "${state_dir}" --format json >"${bad_plan_stdout}" 2>"${bad_plan_stderr}"; then
  cat "${bad_plan_stdout}" >&2
  echo "citusctl accepted a malformed Kubernetes manifest" >&2
  exit 1
fi
if ! grep -Fq "invalid Kubernetes manifest" "${bad_plan_stderr}"; then
  cat "${bad_plan_stdout}" >&2
  cat "${bad_plan_stderr}" >&2
  echo "malformed manifest failure did not report the guarded manifest error" >&2
  exit 1
fi

audit_path="${state_dir}/k8s-manifest-apply.audit.tsv"
if [[ ! -s "${audit_path}" ]]; then
  echo "citusctl apply did not write k8s-manifest-apply.audit.tsv" >&2
  exit 1
fi
audit_records="$(($(wc -l <"${audit_path}" | tr -d ' ') - 1))"
if [[ "${audit_records}" != "2" ]]; then
  echo "expected exactly two Kubernetes apply audit records" >&2
  cat "${audit_path}" >&2
  exit 1
fi
if ! grep -Fq "live-kubernetes-manifest-apply" "${audit_path}"; then
  echo "audit log missing live Kubernetes evidence boundary" >&2
  cat "${audit_path}" >&2
  exit 1
fi

{
  printf 'feature_id\tcluster\tnamespace\tplan_id\tresource\tapply_changed\treapply_changed\taudit_records\tevidence_boundary\n'
  printf 'M8\t%s\t%s\t%s\tconfigmap/ai-blaise-citusctl-live\ttrue\tfalse\t%s\tlive-kubernetes-manifest-apply\n' \
    "${cluster}" "${namespace}" "${plan_id}" "${audit_records}"
} >"${evidence_file}"

cat "${evidence_file}"
echo "citusctl-k8s-apply-live-smoke passed"
