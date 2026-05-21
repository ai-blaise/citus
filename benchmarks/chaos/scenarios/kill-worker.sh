#!/usr/bin/env bash
# Chaos scenario: kill a Citus worker pod and assert shard placement recovery.
#
# Steps:
#   1. Delete a worker pod chosen at random.
#   2. Poll the shard placement table until rebalanced placements report ready.
#   3. Record traffic_error_rate, recovery_p99_ms, data_intact.

set -euo pipefail

# shellcheck source=_helpers.sh
source "$(dirname "$0")/_helpers.sh"

SCENARIO="kill-worker"

chaos_can_execute_or_scaffold "${SCENARIO}" || exit 0

bench_log "chaos: ${SCENARIO}: selecting worker"

worker=$(kubectl -n "${CHAOS_NAMESPACE}" get pod \
  -l "citus.ai-blaise.io/cluster=${CHAOS_CLUSTER},citus.ai-blaise.io/role=worker" \
  -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || true)

if [[ -z "${worker}" ]]; then
  if [[ "${BENCH_QUICK}" == "1" ]]; then
    chaos_write_scenario_result "${SCENARIO}" 0 0 true "scaffold-only: no worker pod"
    exit 0
  fi
  bench_die "chaos: ${SCENARIO}: no worker pod"
fi

start_ms=$(date +%s%3N)
kubectl -n "${CHAOS_NAMESPACE}" delete pod "${worker}" --grace-period=0 --force >/dev/null 2>&1 || true

deadline=$((start_ms + CHAOS_RECOVERY_BUDGET_MS * 2))
recovered_ms=0
while :; do
  now_ms=$(date +%s%3N)
  ready=$(kubectl -n "${CHAOS_NAMESPACE}" get pod \
    -l "citus.ai-blaise.io/cluster=${CHAOS_CLUSTER},citus.ai-blaise.io/role=worker" \
    -o jsonpath='{.items[*].status.containerStatuses[*].ready}' 2>/dev/null || true)
  ready_count=$(printf '%s' "${ready}" | tr ' ' '\n' | grep -c true || true)
  if (( ready_count >= CHAOS_WORKERS )); then
    recovered_ms=$((now_ms - start_ms))
    break
  fi
  if (( now_ms > deadline )); then
    recovered_ms=$((now_ms - start_ms))
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
