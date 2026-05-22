#!/usr/bin/env bash
# Shared helpers for DR drill scripts under benchmarks/dr-drills/.
#
# Each drill sources this file and uses dr_drill_run to wrap its body. The
# wrapper:
#   - Detects whether kubectl + the target namespace are reachable.
#   - In quick mode, falls back to a mock path (no real fault injection) so the
#     drill is exercisable on a stripped-down CI runner.
#   - Caps quick-mode drills at DR_DRILL_RTO_BUDGET_S (default 60s) using a
#     background watchdog.
#   - Records started_at, finished_at, rto_s, rpo_s, errors_during, success,
#     and a free-form note into the JSON report under
#     DR_DRILL_REPORTS_ROOT/<drill>-<DR_DRILL_TAG>.json.

set -euo pipefail

DR_DRILL_REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export DR_DRILL_REPO_ROOT

: "${DR_DRILL_QUICK:=1}"
: "${DR_DRILL_NAMESPACE:=ai-blaise-citus}"
: "${DR_DRILL_CLUSTER:=primary}"
: "${DR_DRILL_RTO_BUDGET_S:=60}"
: "${DR_DRILL_FENCING_BUDGET_S:=15}"
: "${DR_DRILL_REPORTS_ROOT:=${DR_DRILL_REPO_ROOT}/benchmarks/dr-drills/reports}"
: "${DR_DRILL_TAG:=$(date -u +%Y%m%dT%H%M%SZ)}"

export DR_DRILL_QUICK DR_DRILL_NAMESPACE DR_DRILL_CLUSTER \
  DR_DRILL_RTO_BUDGET_S DR_DRILL_FENCING_BUDGET_S \
  DR_DRILL_REPORTS_ROOT DR_DRILL_TAG

mkdir -p "${DR_DRILL_REPORTS_ROOT}"

dr_drill_log() {
  printf '[%s] dr-drill: %s\n' "$(date -u +%H:%M:%SZ)" "$*"
}

dr_drill_die() {
  printf '[dr-drill] error: %s\n' "$*" >&2
  exit 1
}

dr_drill_iso_now() {
  date -u +%Y-%m-%dT%H:%M:%SZ
}

dr_drill_kubectl_available() {
  command -v kubectl >/dev/null 2>&1
}

dr_drill_cluster_reachable() {
  dr_drill_kubectl_available || return 1
  kubectl version --client >/dev/null 2>&1 || return 1
  kubectl get ns "${DR_DRILL_NAMESPACE}" >/dev/null 2>&1 || return 1
  return 0
}

# Returns 0 if any pod with the given label exists in the namespace.
dr_drill_pods_exist() {
  local selector="$1"
  local count
  count=$(kubectl -n "${DR_DRILL_NAMESPACE}" get pod -l "${selector}" \
    -o jsonpath='{.items[*].metadata.name}' 2>/dev/null | wc -w | tr -d ' ')
  [[ "${count}" -gt 0 ]]
}

dr_drill_write_report() {
  local drill="$1"
  local started_at="$2"
  local finished_at="$3"
  local rto_s="$4"
  local rpo_s="$5"
  local errors_during="$6"
  local success="$7"
  local mock="$8"
  local note="$9"

  local mode="quick"
  [[ "${DR_DRILL_QUICK}" == "0" ]] && mode="full"

  local out="${DR_DRILL_REPORTS_ROOT}/${drill}-${DR_DRILL_TAG}.json"
  cat >"${out}" <<JSON
{
  "drill": "${drill}",
  "mode": "${mode}",
  "namespace": "${DR_DRILL_NAMESPACE}",
  "cluster": "${DR_DRILL_CLUSTER}",
  "started_at": "${started_at}",
  "finished_at": "${finished_at}",
  "rto_s": ${rto_s},
  "rpo_s": ${rpo_s},
  "errors_during": ${errors_during},
  "success": ${success},
  "mock": ${mock},
  "note": "${note}"
}
JSON
  dr_drill_log "${drill}: report -> ${out}"
}

# dr_drill_record_mock <drill> <note>
#
# Writes a mock report indicating the drill ran in fallback mode because the
# cluster was unreachable. RTO is set to 0 (no real fault was injected) and the
# drill is recorded as successful so CI smoke can still gate on a clean exit.
dr_drill_record_mock() {
  local drill="$1"
  local note="${2:-cluster unreachable}"
  local ts
  ts=$(dr_drill_iso_now)
  dr_drill_write_report "${drill}" "${ts}" "${ts}" 0 0 0 true true "mock: ${note}"
}

# dr_drill_seconds_between <start_epoch> <end_epoch>
#
# Echoes a decimal-second delta with one fractional digit. Pure bash arithmetic
# to avoid pulling in bc on minimal runners.
dr_drill_seconds_between() {
  local start_ms="$1"
  local end_ms="$2"
  local delta=$(( end_ms - start_ms ))
  local whole=$(( delta / 1000 ))
  local frac=$(( (delta % 1000) / 100 ))
  printf '%d.%d' "${whole}" "${frac}"
}

# dr_drill_now_ms — millisecond epoch using GNU date if available, falling back
# to seconds * 1000 otherwise.
dr_drill_now_ms() {
  local ms
  ms=$(date +%s%3N 2>/dev/null || true)
  if [[ -z "${ms}" || "${ms}" == *N ]]; then
    ms=$(( $(date +%s) * 1000 ))
  fi
  printf '%s' "${ms}"
}
