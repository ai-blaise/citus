#!/usr/bin/env bash
# Shared helpers for chaos scenarios. Sourced by each scenarios/*.sh.

set -euo pipefail

SCENARIO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../../common/lib.sh
source "${SCENARIO_DIR}/../../common/lib.sh"

# Cluster identifier; the operator creates a `CitusCluster` named "primary"
# inside the `ai-blaise-citus` namespace for the production-readiness smoke.
: "${CHAOS_NAMESPACE:=ai-blaise-citus}"
: "${CHAOS_CLUSTER:=primary}"
: "${CHAOS_WORKERS:=3}"
: "${CHAOS_RECOVERY_BUDGET_MS:=5000}"
: "${CHAOS_TRAFFIC_ERROR_BUDGET:=0.05}"

chaos_kubectl_available() {
  command -v kubectl >/dev/null 2>&1
}

chaos_cluster_reachable() {
  chaos_kubectl_available || return 1
  kubectl version --client >/dev/null 2>&1 || return 1
  kubectl get ns "${CHAOS_NAMESPACE}" >/dev/null 2>&1 || return 1
  return 0
}

# Each scenario writes a single result JSON via this helper so the chaos
# driver's combiner can pick the file up by filename pattern.
chaos_write_scenario_result() {
  local scenario="$1"
  local error_rate="$2"
  local recovery_ms="$3"
  local data_intact="$4"
  local note="${5:-}"

  local mode="quick"
  [[ "${BENCH_QUICK}" == "0" ]] && mode="full"

  local out="${BENCH_RESULTS_ROOT}/chaos-${scenario}-${BENCH_RESULT_TAG}.json"
  cat >"${out}" <<JSON
{
  "scenario": "${scenario}",
  "traffic_error_rate": ${error_rate},
  "recovery_p99_ms": ${recovery_ms},
  "data_intact": ${data_intact},
  "mode": "${mode}",
  "note": "${note}"
}
JSON
  bench_log "chaos: ${scenario} -> ${out}"
}

# Returns 0 if the scenario can execute (kubectl + cluster reachable), 1 if
# we should record a quick-mode scaffold result and exit cleanly.
chaos_can_execute_or_scaffold() {
  local scenario="$1"

  if chaos_cluster_reachable; then
    return 0
  fi

  if [[ "${BENCH_QUICK}" == "1" ]]; then
    chaos_write_scenario_result \
      "${scenario}" \
      0 \
      0 \
      true \
      "scaffold-only: no kubectl cluster"
    return 1
  fi

  bench_die "chaos: ${scenario} requires a reachable kubectl cluster"
}
