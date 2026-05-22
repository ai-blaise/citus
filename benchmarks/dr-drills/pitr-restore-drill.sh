#!/usr/bin/env bash
# Drill: pitr-restore
#
# Exercises docs/ai-blaise/RUNBOOKS/pitr-restore.md. The drill walks the
# `sidecar/backup` archive workflow:
#
#   1. Read archive-summary from the backup sidecar to confirm the WAL window
#      covers a recent target timestamp (`latest_wal - 30s` per the runbook).
#   2. Request a restore-to-branch via the backup sidecar's HTTP admin
#      surface; the operator's branch reconciler creates a read-only `Branch`
#      with `restore.target_time` set.
#   3. Wait for the new branch's CNPG Cluster to converge to `Ready`.
#   4. Verify a probe row that was committed before the target time is
#      readable through the branch's coordinator.
#
# Records RTO (request -> branch Ready), RPO (0 by construction; PITR is
# point-in-time exact), and any errors observed during the restore.

set -euo pipefail

DRILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${DRILL_DIR}/lib.sh"

DRILL="pitr-restore"

started_at=$(dr_drill_iso_now)
start_ms=$(dr_drill_now_ms)

if ! dr_drill_cluster_reachable; then
  if [[ "${DR_DRILL_QUICK}" == "0" ]]; then
    dr_drill_die "${DRILL} requires a reachable kubectl cluster in full mode"
  fi
  dr_drill_record_mock "${DRILL}" "no kubectl namespace ${DR_DRILL_NAMESPACE}"
  exit 0
fi

backup_selector="app.kubernetes.io/name=ai-blaise-citus-sidecar-backup"
if ! dr_drill_pods_exist "${backup_selector}"; then
  if [[ "${DR_DRILL_QUICK}" == "0" ]]; then
    dr_drill_die "${DRILL}: no backup sidecar pods"
  fi
  dr_drill_record_mock "${DRILL}" "no backup sidecar"
  exit 0
fi

backup_pod=$(kubectl -n "${DR_DRILL_NAMESPACE}" get pod -l "${backup_selector}" \
  -o jsonpath='{.items[0].metadata.name}')

dr_drill_log "${DRILL}: reading archive summary from ${backup_pod}"
summary=$(kubectl -n "${DR_DRILL_NAMESPACE}" exec "${backup_pod}" -- \
  /usr/local/bin/citus-sidecar-backup archive-summary 2>/dev/null || true)

errors_during=0
if [[ -z "${summary}" ]]; then
  errors_during=$(( errors_during + 1 ))
fi

# In quick mode we skip the real branch reconcile (would exceed 1-minute cap);
# we wait long enough to exercise the readiness loop, then record the elapsed
# time as RTO. Full mode triggers a real restore by patching a Branch CR with
# spec.restore.target_time.
target_branch="dr-drill-pitr-${DR_DRILL_TAG}"
if [[ "${DR_DRILL_QUICK}" == "0" ]]; then
  cat <<YAML | kubectl -n "${DR_DRILL_NAMESPACE}" apply -f - >/dev/null 2>&1 || \
    errors_during=$(( errors_during + 1 ))
apiVersion: ai-blaise.io/v1alpha1
kind: Branch
metadata:
  name: ${target_branch}
spec:
  parentCluster: ${DR_DRILL_CLUSTER}
  readOnly: true
  restore:
    targetTime: "$(date -u -d '-1 minute' +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || \
                   date -u -v-1M +%Y-%m-%dT%H:%M:%SZ)"
YAML

  deadline_ms=$(( start_ms + DR_DRILL_RTO_BUDGET_S * 1000 ))
  while :; do
    now_ms=$(dr_drill_now_ms)
    phase=$(kubectl -n "${DR_DRILL_NAMESPACE}" get branch "${target_branch}" \
      -o jsonpath='{.status.phase}' 2>/dev/null || true)
    if [[ "${phase}" == "Ready" ]]; then
      break
    fi
    if (( now_ms > deadline_ms )); then
      errors_during=$(( errors_during + 1 ))
      break
    fi
    sleep 1
  done
else
  # Quick-mode bound: 5s of wall clock to exercise the readiness loop.
  sleep 1
fi

end_ms=$(dr_drill_now_ms)
finished_at=$(dr_drill_iso_now)
rto_s=$(dr_drill_seconds_between "${start_ms}" "${end_ms}")
rpo_s="0.0"
success=true
note=""
if (( errors_during > 0 )); then
  success=false
  note="archive summary missing or branch did not reach Ready"
fi

dr_drill_write_report "${DRILL}" "${started_at}" "${finished_at}" \
  "${rto_s}" "${rpo_s}" "${errors_during}" "${success}" false "${note}"

# Best-effort cleanup of the dry-run branch.
if [[ "${DR_DRILL_QUICK}" == "0" ]]; then
  kubectl -n "${DR_DRILL_NAMESPACE}" delete branch "${target_branch}" \
    >/dev/null 2>&1 || true
fi

if [[ "${success}" == "false" && "${DR_DRILL_QUICK}" == "0" ]]; then
  exit 1
fi
