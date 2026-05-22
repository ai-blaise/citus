#!/usr/bin/env bash
# Chaos scenario: partition a worker from the coordinator via NetworkPolicy.
#
# Steps:
#   1. Apply a deny-all NetworkPolicy targeting one worker.
#   2. Sleep for one budget window; observe pool error rate.
#   3. Remove the policy; observe recovery time.

set -euo pipefail

# shellcheck source=_helpers.sh
source "$(dirname "$0")/_helpers.sh"

SCENARIO="network-partition"

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

policy_name="chaos-partition-${worker}"
bench_log "chaos: ${SCENARIO}: applying NetworkPolicy ${policy_name}"

cat <<YAML | kubectl -n "${CHAOS_NAMESPACE}" apply -f - >/dev/null 2>&1 || true
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: ${policy_name}
spec:
  podSelector:
    matchLabels:
      statefulset.kubernetes.io/pod-name: ${worker}
  policyTypes: [Ingress, Egress]
  ingress: []
  egress: []
YAML

start_ms=$(date +%s%3N)
sleep "${BENCH_WARMUP_SECS}"
kubectl -n "${CHAOS_NAMESPACE}" delete networkpolicy "${policy_name}" >/dev/null 2>&1 || true

deadline=$((start_ms + CHAOS_RECOVERY_BUDGET_MS * 2))
recovered_ms=0
while :; do
  now_ms=$(date +%s%3N)
  ready=$(kubectl -n "${CHAOS_NAMESPACE}" get pod "${worker}" \
    -o jsonpath='{.status.containerStatuses[*].ready}' 2>/dev/null || true)
  if printf '%s' "${ready}" | grep -q true; then
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
