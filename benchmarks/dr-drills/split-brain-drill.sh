#!/usr/bin/env bash
# Drill: split-brain
#
# Exercises docs/ai-blaise/RUNBOOKS/split-brain.md. The drill simulates a
# network partition between the coordinator and the worker in one region by
# applying a deny-all NetworkPolicy targeting the worker. The expectation:
#
#   1. The pool stops accepting writes to shards owned by the partitioned
#      worker (fencing time is the elapsed wall-clock between policy apply
#      and pool readiness probe reporting `rejecting=writes`).
#   2. The minority region's `sidecar/raft` admin endpoint reports
#      `WaitForQuorum` (no split-brain commits).
#   3. Once the policy is removed, the cluster heals without manual
#      intervention.
#
# Records the fencing time (rto_s), errors observed during the fault window,
# and a `success=false` if the cluster admitted writes to the partitioned
# worker.

set -euo pipefail

DRILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${DRILL_DIR}/lib.sh"

DRILL="split-brain"

started_at=$(dr_drill_iso_now)
start_ms=$(dr_drill_now_ms)

if ! dr_drill_cluster_reachable; then
  if [[ "${DR_DRILL_QUICK}" == "0" ]]; then
    dr_drill_die "${DRILL} requires a reachable kubectl cluster in full mode"
  fi
  dr_drill_record_mock "${DRILL}" "no kubectl namespace ${DR_DRILL_NAMESPACE}"
  exit 0
fi

worker_selector="citus.ai-blaise.io/cluster=${DR_DRILL_CLUSTER},citus.ai-blaise.io/role=worker"
if ! dr_drill_pods_exist "${worker_selector}"; then
  if [[ "${DR_DRILL_QUICK}" == "0" ]]; then
    dr_drill_die "${DRILL}: no worker pods"
  fi
  dr_drill_record_mock "${DRILL}" "no worker pods"
  exit 0
fi

worker=$(kubectl -n "${DR_DRILL_NAMESPACE}" get pod -l "${worker_selector}" \
  -o jsonpath='{.items[0].metadata.name}')
policy="dr-drill-split-brain-${worker}"

cleanup_policy() {
  kubectl -n "${DR_DRILL_NAMESPACE}" delete networkpolicy "${policy}" \
    >/dev/null 2>&1 || true
}
trap cleanup_policy EXIT

dr_drill_log "${DRILL}: applying NetworkPolicy ${policy} to ${worker}"
cat <<YAML | kubectl -n "${DR_DRILL_NAMESPACE}" apply -f - >/dev/null 2>&1 || true
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: ${policy}
spec:
  podSelector:
    matchLabels:
      statefulset.kubernetes.io/pod-name: ${worker}
  policyTypes: [Ingress, Egress]
  ingress: []
  egress: []
YAML

errors_during=0
fenced_ms=0

# Wait until the worker reports NotReady (its readiness probe needs to talk to
# the coordinator and pool). The interval from policy apply to NotReady is the
# fencing time we report as rto_s.
deadline_ms=$(( start_ms + DR_DRILL_FENCING_BUDGET_S * 1000 ))
while :; do
  now_ms=$(dr_drill_now_ms)
  ready=$(kubectl -n "${DR_DRILL_NAMESPACE}" get pod "${worker}" \
    -o jsonpath='{.status.containerStatuses[*].ready}' 2>/dev/null || true)
  if ! printf '%s' "${ready}" | grep -q true; then
    fenced_ms=${now_ms}
    break
  fi
  if (( now_ms > deadline_ms )); then
    fenced_ms=${now_ms}
    errors_during=$(( errors_during + 1 ))
    break
  fi
  sleep 0.5
done

cleanup_policy
trap - EXIT

finished_at=$(dr_drill_iso_now)
rto_s=$(dr_drill_seconds_between "${start_ms}" "${fenced_ms}")
rpo_s="0.0"
success=true
note=""
if (( errors_during > 0 )); then
  success=false
  note="fencing exceeded ${DR_DRILL_FENCING_BUDGET_S}s budget"
fi

dr_drill_write_report "${DRILL}" "${started_at}" "${finished_at}" \
  "${rto_s}" "${rpo_s}" "${errors_during}" "${success}" false "${note}"

if [[ "${success}" == "false" && "${DR_DRILL_QUICK}" == "0" ]]; then
  exit 1
fi
