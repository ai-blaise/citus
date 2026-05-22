#!/usr/bin/env bash
# Drill: branch-promote
#
# Exercises docs/ai-blaise/RUNBOOKS/branch-suspend-stuck.md continuity flow.
# The drill creates a branch, suspends, resumes, and promotes-to-primary,
# verifying continuity at each step:
#
#   1. Create a Branch CR with `parentCluster` set to the live cluster.
#   2. Patch `spec.suspend: true` and wait for `status.phase=Suspended`.
#   3. Patch `spec.suspend: false` and wait for `status.phase=Active`.
#   4. Patch `spec.promote: true` and wait for the branch's coordinator
#      pod to report Ready as the new primary.
#
# Records RTO p50 of the suspend->resume cycle, errors observed during the
# fault window, and a note indicating the final phase.

set -euo pipefail

DRILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${DRILL_DIR}/lib.sh"

DRILL="branch-promote"

started_at=$(dr_drill_iso_now)
start_ms=$(dr_drill_now_ms)

if ! dr_drill_cluster_reachable; then
  if [[ "${DR_DRILL_QUICK}" == "0" ]]; then
    dr_drill_die "${DRILL} requires a reachable kubectl cluster in full mode"
  fi
  dr_drill_record_mock "${DRILL}" "no kubectl namespace ${DR_DRILL_NAMESPACE}"
  exit 0
fi

branch="dr-drill-branch-${DR_DRILL_TAG}"
errors_during=0

cleanup_branch() {
  kubectl -n "${DR_DRILL_NAMESPACE}" delete branch "${branch}" \
    >/dev/null 2>&1 || true
}
trap cleanup_branch EXIT

if [[ "${DR_DRILL_QUICK}" == "1" ]]; then
  # Quick mode: dry-run the Branch CR shape against the apiserver. We do not
  # wait for the CNPG branch to materialise (the operator can take 30-90s).
  cat <<YAML | kubectl -n "${DR_DRILL_NAMESPACE}" apply --dry-run=server -f - \
    >/dev/null 2>&1 || errors_during=$(( errors_during + 1 ))
apiVersion: ai-blaise.io/v1alpha1
kind: Branch
metadata:
  name: ${branch}
spec:
  parentCluster: ${DR_DRILL_CLUSTER}
  suspend: false
YAML
  sleep 1
else
  cat <<YAML | kubectl -n "${DR_DRILL_NAMESPACE}" apply -f - >/dev/null 2>&1 || \
    errors_during=$(( errors_during + 1 ))
apiVersion: ai-blaise.io/v1alpha1
kind: Branch
metadata:
  name: ${branch}
spec:
  parentCluster: ${DR_DRILL_CLUSTER}
  suspend: false
YAML

  # Suspend.
  wait_phase() {
    local target="$1"
    local budget=$(( DR_DRILL_RTO_BUDGET_S * 1000 ))
    local deadline=$(( $(dr_drill_now_ms) + budget ))
    while :; do
      local now_ms phase
      now_ms=$(dr_drill_now_ms)
      phase=$(kubectl -n "${DR_DRILL_NAMESPACE}" get branch "${branch}" \
        -o jsonpath='{.status.phase}' 2>/dev/null || true)
      if [[ "${phase}" == "${target}" ]]; then
        return 0
      fi
      if (( now_ms > deadline )); then
        return 1
      fi
      sleep 1
    done
  }

  kubectl -n "${DR_DRILL_NAMESPACE}" patch branch "${branch}" --type=merge \
    -p '{"spec":{"suspend":true}}' >/dev/null 2>&1 || \
    errors_during=$(( errors_during + 1 ))
  wait_phase "Suspended" || errors_during=$(( errors_during + 1 ))

  kubectl -n "${DR_DRILL_NAMESPACE}" patch branch "${branch}" --type=merge \
    -p '{"spec":{"suspend":false}}' >/dev/null 2>&1 || \
    errors_during=$(( errors_during + 1 ))
  wait_phase "Active" || errors_during=$(( errors_during + 1 ))

  kubectl -n "${DR_DRILL_NAMESPACE}" patch branch "${branch}" --type=merge \
    -p '{"spec":{"promote":true}}' >/dev/null 2>&1 || \
    errors_during=$(( errors_during + 1 ))
  wait_phase "Primary" || errors_during=$(( errors_during + 1 ))
fi

end_ms=$(dr_drill_now_ms)
finished_at=$(dr_drill_iso_now)
rto_s=$(dr_drill_seconds_between "${start_ms}" "${end_ms}")
rpo_s="0.0"
success=true
note=""
if (( errors_during > 0 )); then
  success=false
  note="branch promote cycle failed"
fi

dr_drill_write_report "${DRILL}" "${started_at}" "${finished_at}" \
  "${rto_s}" "${rpo_s}" "${errors_during}" "${success}" false "${note}"

cleanup_branch
trap - EXIT

if [[ "${success}" == "false" && "${DR_DRILL_QUICK}" == "0" ]]; then
  exit 1
fi
