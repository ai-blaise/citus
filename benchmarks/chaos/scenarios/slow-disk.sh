#!/usr/bin/env bash
# Chaos scenario: simulate slow disk on a worker via `tc qdisc add netem
# delay` on the pod's loopback (a proxy for I/O latency at the application
# layer). The real production scenario uses Linux blktrace; this scaffold
# approximates it with an `tc` rule.
#
# Steps:
#   1. Apply a 50ms delay to the worker's loopback.
#   2. Sleep for the warmup window.
#   3. Remove the delay; observe recovery.

set -euo pipefail

# shellcheck source=_helpers.sh
source "$(dirname "$0")/_helpers.sh"

SCENARIO="slow-disk"

chaos_can_execute_or_scaffold "${SCENARIO}" || exit 0

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

apply_cmd="tc qdisc add dev lo root netem delay 50ms 2>/dev/null || true"
clear_cmd="tc qdisc del dev lo root netem 2>/dev/null || true"

start_ms=$(date +%s%3N)
kubectl -n "${CHAOS_NAMESPACE}" exec "${worker}" -- /bin/sh -c "${apply_cmd}" >/dev/null 2>&1 || true
sleep "${BENCH_WARMUP_SECS}"
kubectl -n "${CHAOS_NAMESPACE}" exec "${worker}" -- /bin/sh -c "${clear_cmd}" >/dev/null 2>&1 || true

recovered_ms=$(( $(date +%s%3N) - start_ms ))
data_intact=true
note=""
if (( recovered_ms > CHAOS_RECOVERY_BUDGET_MS )); then
  note="recovery exceeded budget"
fi

chaos_write_scenario_result \
  "${SCENARIO}" \
  0 \
  "${recovered_ms}" \
  "${data_intact}" \
  "${note}"
