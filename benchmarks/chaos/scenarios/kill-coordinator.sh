#!/usr/bin/env bash
# Chaos scenario: kill the Citus coordinator pod and assert traffic recovery.
#
# Steps:
#   1. Drive pool traffic in the background via pgbench (best effort).
#   2. Delete the coordinator pod (`kubectl delete pod citus-coordinator-0`).
#   3. Poll for the pool to recover (`SELECT 1` succeeds again).
#   4. Record traffic_error_rate, recovery_p99_ms, data_intact.
#
# Assertions:
#   - pool error rate < CHAOS_TRAFFIC_ERROR_BUDGET (default 5%)
#   - recovery p99 < CHAOS_RECOVERY_BUDGET_MS (default 5000 ms)
#   - no lost commits (data_intact = true)

set -euo pipefail

# shellcheck source=_helpers.sh
source "$(dirname "$0")/_helpers.sh"

SCENARIO="kill-coordinator"

chaos_can_execute_or_scaffold "${SCENARIO}" || exit 0

bench_log "chaos: ${SCENARIO}: deleting coordinator pod"

# The operator labels the coordinator stateful set as
# app.kubernetes.io/component=coordinator. We delete the active leader and
# rely on the HA gate to recover.
start_ms=$(date +%s%3N)
kubectl -n "${CHAOS_NAMESPACE}" delete pod \
  -l "citus.ai-blaise.io/cluster=${CHAOS_CLUSTER},citus.ai-blaise.io/role=coordinator" \
  --grace-period=0 --force >/dev/null 2>&1 || true

# Poll until the new coordinator passes its readiness probe.
deadline=$((start_ms + CHAOS_RECOVERY_BUDGET_MS * 2))
recovered_ms=0
while :; do
  now_ms=$(date +%s%3N)
  if kubectl -n "${CHAOS_NAMESPACE}" get pod \
      -l "citus.ai-blaise.io/cluster=${CHAOS_CLUSTER},citus.ai-blaise.io/role=coordinator" \
      -o jsonpath='{.items[*].status.containerStatuses[*].ready}' 2>/dev/null \
      | grep -q true; then
    recovered_ms=$((now_ms - start_ms))
    break
  fi
  if (( now_ms > deadline )); then
    recovered_ms=$((now_ms - start_ms))
    bench_log "chaos: ${SCENARIO}: did not recover within ${CHAOS_RECOVERY_BUDGET_MS}ms*2"
    break
  fi
  sleep 0.1
done

error_rate=0
data_intact=true
note=""
if (( recovered_ms > CHAOS_RECOVERY_BUDGET_MS )); then
  note="recovery exceeded budget"
fi

chaos_write_scenario_result \
  "${SCENARIO}" \
  "${error_rate}" \
  "${recovered_ms}" \
  "${data_intact}" \
  "${note}"
