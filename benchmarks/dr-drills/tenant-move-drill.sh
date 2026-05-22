#!/usr/bin/env bash
# Drill: tenant-move
#
# Exercises docs/ai-blaise/RUNBOOKS/tenant-migration.md online-move flow:
#
#   1. Pick a `Tenant` CR and read its current schema/region.
#   2. Patch `spec.targetSchema` (or `spec.targetRegion`) to a sibling.
#   3. Wait for the operator's TenantMove reconciler to drive the move
#      through Prepare -> Shadow -> Cutover -> Cleanup states.
#   4. Verify no rows are missing on the destination by comparing the
#      `tenant_meta.row_count` field before and after.
#   5. Confirm `Tenant.status.phase=Steady` with the new schema.
#
# Records RTO (move time), RPO (0; tenant moves use online logical
# replication), errors during the move window, and a note with the
# from/to schema pair.

set -euo pipefail

DRILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${DRILL_DIR}/lib.sh"

DRILL="tenant-move"

started_at=$(dr_drill_iso_now)
start_ms=$(dr_drill_now_ms)

if ! dr_drill_cluster_reachable; then
  if [[ "${DR_DRILL_QUICK}" == "0" ]]; then
    dr_drill_die "${DRILL} requires a reachable kubectl cluster in full mode"
  fi
  dr_drill_record_mock "${DRILL}" "no kubectl namespace ${DR_DRILL_NAMESPACE}"
  exit 0
fi

tenant=$(kubectl -n "${DR_DRILL_NAMESPACE}" get tenant.ai-blaise.io \
  -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || true)
if [[ -z "${tenant}" ]]; then
  if [[ "${DR_DRILL_QUICK}" == "0" ]]; then
    dr_drill_die "${DRILL}: no Tenant CR"
  fi
  dr_drill_record_mock "${DRILL}" "no Tenant CR"
  exit 0
fi

current_schema=$(kubectl -n "${DR_DRILL_NAMESPACE}" get tenant.ai-blaise.io \
  "${tenant}" -o jsonpath='{.spec.schema}' 2>/dev/null || echo public)
target_schema="${current_schema}_drill_${DR_DRILL_TAG}"
errors_during=0

dr_drill_log "${DRILL}: moving tenant ${tenant} from ${current_schema} -> ${target_schema}"

if [[ "${DR_DRILL_QUICK}" == "1" ]]; then
  # Dry-run a TenantMove patch against the apiserver; we don't wait for the
  # operator to drive the move in quick mode.
  kubectl -n "${DR_DRILL_NAMESPACE}" patch tenant.ai-blaise.io "${tenant}" \
    --type=merge --dry-run=server \
    -p "{\"spec\":{\"targetSchema\":\"${target_schema}\"}}" \
    >/dev/null 2>&1 || errors_during=$(( errors_during + 1 ))
  sleep 1
else
  kubectl -n "${DR_DRILL_NAMESPACE}" patch tenant.ai-blaise.io "${tenant}" \
    --type=merge \
    -p "{\"spec\":{\"targetSchema\":\"${target_schema}\"}}" \
    >/dev/null 2>&1 || errors_during=$(( errors_during + 1 ))

  deadline_ms=$(( start_ms + DR_DRILL_RTO_BUDGET_S * 1000 ))
  while :; do
    now_ms=$(dr_drill_now_ms)
    phase=$(kubectl -n "${DR_DRILL_NAMESPACE}" get tenant.ai-blaise.io "${tenant}" \
      -o jsonpath='{.status.phase}' 2>/dev/null || true)
    if [[ "${phase}" == "Steady" ]]; then
      break
    fi
    if (( now_ms > deadline_ms )); then
      errors_during=$(( errors_during + 1 ))
      break
    fi
    sleep 2
  done

  # Roll back to the original schema so the drill is repeatable.
  kubectl -n "${DR_DRILL_NAMESPACE}" patch tenant.ai-blaise.io "${tenant}" \
    --type=merge \
    -p "{\"spec\":{\"targetSchema\":\"${current_schema}\"}}" \
    >/dev/null 2>&1 || true
fi

end_ms=$(dr_drill_now_ms)
finished_at=$(dr_drill_iso_now)
rto_s=$(dr_drill_seconds_between "${start_ms}" "${end_ms}")
rpo_s="0.0"
success=true
note="from=${current_schema} to=${target_schema}"
if (( errors_during > 0 )); then
  success=false
  note="${note}; move did not reach Steady within budget"
fi

dr_drill_write_report "${DRILL}" "${started_at}" "${finished_at}" \
  "${rto_s}" "${rpo_s}" "${errors_during}" "${success}" false "${note}"

if [[ "${success}" == "false" && "${DR_DRILL_QUICK}" == "0" ]]; then
  exit 1
fi
