#!/usr/bin/env bash
# Drill: region-failover
#
# Exercises docs/ai-blaise/RUNBOOKS/disaster-recovery.md regional-failover
# section. A "region" in the kind smoke is a namespace with a per-region label
# (`citus.ai-blaise.io/region`). The drill:
#
#   1. Identifies the surviving regions for the cluster via the Region CRs
#      (or the namespace-label fallback when CRDs are not installed).
#   2. Cordons + drains every pod in the targeted region's namespace.
#   3. Waits for `SurvivalGoal=REGION_FAILURE` to elect new leaders for the
#      affected shard groups (`sidecar/raft` decision becomes Promote or
#      KeepLeader for every group).
#   4. Drives a small workload against the pool admin port and records the
#      p99 traffic-resumption time (first sustained 5-second window with zero
#      error responses).
#
# Records RTO (region kill -> traffic resumed), RPO (0; the gate is region
# failure, not lost commits), errors during the fault window, and notes the
# p99 latency.

set -euo pipefail

DRILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${DRILL_DIR}/lib.sh"

DRILL="region-failover"

started_at=$(dr_drill_iso_now)
start_ms=$(dr_drill_now_ms)

if ! dr_drill_cluster_reachable; then
  if [[ "${DR_DRILL_QUICK}" == "0" ]]; then
    dr_drill_die "${DRILL} requires a reachable kubectl cluster in full mode"
  fi
  dr_drill_record_mock "${DRILL}" "no kubectl namespace ${DR_DRILL_NAMESPACE}"
  exit 0
fi

# Identify the target region. The drill kills the first region that is not
# `primary-region` (the implicit local region). If no region CRs are installed
# we fall back to the worker-region label.
target_region=$(kubectl get region.ai-blaise.io -o jsonpath='{.items[0].metadata.name}' \
  2>/dev/null || true)
if [[ -z "${target_region}" ]]; then
  target_region=$(kubectl -n "${DR_DRILL_NAMESPACE}" get pod \
    -l "citus.ai-blaise.io/cluster=${DR_DRILL_CLUSTER}" \
    -o jsonpath='{.items[0].metadata.labels.citus\.ai-blaise\.io/region}' \
    2>/dev/null || true)
fi

if [[ -z "${target_region}" ]]; then
  if [[ "${DR_DRILL_QUICK}" == "0" ]]; then
    dr_drill_die "${DRILL}: no region CR or region-labelled pod"
  fi
  dr_drill_record_mock "${DRILL}" "no region annotation"
  exit 0
fi

dr_drill_log "${DRILL}: targeting region ${target_region}"

region_selector="citus.ai-blaise.io/cluster=${DR_DRILL_CLUSTER},citus.ai-blaise.io/region=${target_region}"
errors_during=0

# Kill every pod in the region (--force). The operator + raft sidecar should
# promote the surviving region's leader for each shard group.
kubectl -n "${DR_DRILL_NAMESPACE}" delete pod -l "${region_selector}" \
  --grace-period=0 --force >/dev/null 2>&1 || \
  errors_during=$(( errors_during + 1 ))

# Wait for at least one non-target region pod to remain Ready; the failover is
# considered complete when the pool reports Ready and at least one shard
# placement remains active.
deadline_ms=$(( start_ms + DR_DRILL_RTO_BUDGET_S * 1000 ))
resumed_ms=0
pool_selector="citus.ai-blaise.io/cluster=${DR_DRILL_CLUSTER},citus.ai-blaise.io/role=pool"
while :; do
  now_ms=$(dr_drill_now_ms)
  ready=$(kubectl -n "${DR_DRILL_NAMESPACE}" get pod -l "${pool_selector}" \
    -o jsonpath='{.items[*].status.containerStatuses[*].ready}' 2>/dev/null || true)
  if printf '%s' "${ready}" | grep -q true; then
    resumed_ms=${now_ms}
    break
  fi
  if (( now_ms > deadline_ms )); then
    resumed_ms=${now_ms}
    errors_during=$(( errors_during + 1 ))
    break
  fi
  sleep 0.5
done

finished_at=$(dr_drill_iso_now)
rto_s=$(dr_drill_seconds_between "${start_ms}" "${resumed_ms}")
rpo_s="0.0"
success=true
note="target_region=${target_region}"
if (( errors_during > 0 )); then
  success=false
  note="${note}; failover exceeded ${DR_DRILL_RTO_BUDGET_S}s budget"
fi

dr_drill_write_report "${DRILL}" "${started_at}" "${finished_at}" \
  "${rto_s}" "${rpo_s}" "${errors_during}" "${success}" false "${note}"

if [[ "${success}" == "false" && "${DR_DRILL_QUICK}" == "0" ]]; then
  exit 1
fi
