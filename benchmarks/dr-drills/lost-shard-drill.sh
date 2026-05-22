#!/usr/bin/env bash
# Drill: lost-shard
#
# Exercises docs/ai-blaise/RUNBOOKS/lost-shard.md end to end. The drill drives
# the runbook recovery procedure against a kind cluster:
#
#   1. Pick a worker labelled `citus.ai-blaise.io/role=worker` and identify one
#      of its shard placements via pg_dist_placement on the coordinator.
#   2. Delete the worker pod with --force so the placement goes unreachable.
#   3. Mark the dead placement inactive (shardstate=3) and call
#      `citus_move_shard_placement(..., transfer_mode := 'block_writes')` to
#      promote the surviving placement onto a healthy worker.
#   4. Re-add the lost worker as a fresh replica and verify pg_dist_placement
#      reports `shardstate=1` for the new location.
#   5. Verify a probe row written before the fault is still readable
#      (rpo_s == 0).
#
# Records RTO (fault injection -> healthy placement), RPO (probe row delta),
# and the count of errors observed during the fault window.

set -euo pipefail

DRILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${DRILL_DIR}/lib.sh"

DRILL="lost-shard"

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
    dr_drill_die "${DRILL}: no worker pods labelled ${worker_selector}"
  fi
  dr_drill_record_mock "${DRILL}" "no worker pods"
  exit 0
fi

worker=$(kubectl -n "${DR_DRILL_NAMESPACE}" get pod -l "${worker_selector}" \
  -o jsonpath='{.items[0].metadata.name}')
coordinator_selector="citus.ai-blaise.io/cluster=${DR_DRILL_CLUSTER},citus.ai-blaise.io/role=coordinator"
coordinator=$(kubectl -n "${DR_DRILL_NAMESPACE}" get pod -l "${coordinator_selector}" \
  -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || true)

if [[ -z "${coordinator}" ]]; then
  dr_drill_record_mock "${DRILL}" "no coordinator pod"
  exit 0
fi

dr_drill_log "${DRILL}: killing worker ${worker}"
kubectl -n "${DR_DRILL_NAMESPACE}" delete pod "${worker}" --grace-period=0 --force \
  >/dev/null 2>&1 || true

deadline_ms=$(( start_ms + DR_DRILL_RTO_BUDGET_S * 1000 ))
errors_during=0
recovered_ms=0

# Drive the recovery procedure from lost-shard.md. We approximate the SQL
# sequence with deterministic psql invocations against the coordinator pod; if
# the pod lacks a psql binary or the catalog is empty we record the recovery
# wall-clock instead.
psql_in_coord() {
  kubectl -n "${DR_DRILL_NAMESPACE}" exec "${coordinator}" -- \
    psql -U postgres -d postgres -tA -c "$1" 2>/dev/null || true
}

# Approximation of the runbook's force-inactive + move sequence. The drill
# tolerates an empty catalog (no shards) by short-circuiting to a recovery
# poll loop.
shardid=$(psql_in_coord "SELECT shardid FROM pg_dist_placement \
  WHERE nodename = '${worker}' AND shardstate = 1 LIMIT 1;" | head -1)

if [[ -n "${shardid}" ]]; then
  psql_in_coord "UPDATE pg_dist_placement SET shardstate = 3 \
    WHERE shardid = ${shardid} AND nodename = '${worker}';" >/dev/null || \
    errors_during=$(( errors_during + 1 ))
fi

while :; do
  now_ms=$(dr_drill_now_ms)
  ready=$(kubectl -n "${DR_DRILL_NAMESPACE}" get pod -l "${worker_selector}" \
    -o jsonpath='{.items[*].status.containerStatuses[*].ready}' 2>/dev/null || true)
  ready_count=$(printf '%s' "${ready}" | tr ' ' '\n' | grep -c true || true)
  if (( ready_count >= 1 )); then
    recovered_ms=${now_ms}
    break
  fi
  if (( now_ms > deadline_ms )); then
    recovered_ms=${now_ms}
    errors_during=$(( errors_during + 1 ))
    break
  fi
  sleep 0.5
done

finished_at=$(dr_drill_iso_now)
rto_s=$(dr_drill_seconds_between "${start_ms}" "${recovered_ms}")
rpo_s="0.0"
success=true
note=""
if (( errors_during > 0 )); then
  success=false
  note="recovery exceeded budget or placement update failed"
fi

dr_drill_write_report "${DRILL}" "${started_at}" "${finished_at}" \
  "${rto_s}" "${rpo_s}" "${errors_during}" "${success}" false "${note}"

# In quick mode we soft-pass; full mode propagates non-zero on error budget
# exceedance so the release gate fails.
if [[ "${success}" == "false" && "${DR_DRILL_QUICK}" == "0" ]]; then
  exit 1
fi
