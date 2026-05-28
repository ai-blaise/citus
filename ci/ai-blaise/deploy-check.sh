#!/usr/bin/env bash
set -euo pipefail

# FEATURE: D8

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

# The in-repo Helm chart was folded into ai-blaise/command-center on
# 2026-05-22. This Citus-side deploy check validates the deployment guardrail
# contract that command-center must preserve, then exercises the external-chart
# live harness in CI-safe dry-run mode unless callers request stricter behavior.
bash ci/ai-blaise/k8s-guardrails-check.sh

export LIVE_K8S_MODE="${LIVE_K8S_MODE:-dry-run}"
export REQUIRE_CHART="${REQUIRE_CHART:-${REQUIRE_HELM:-0}}"
export REQUIRE_HTTP="${REQUIRE_HTTP:-0}"
export REQUIRE_SQL="${REQUIRE_SQL:-0}"

# In dry-run mode the chart is rendered without supplying production image
# digests, so default to values-dev.yaml (requireImageDigest=false) to let
# the contract-only smoke complete. Callers who supply VALUES_FILES win.
if [[ "${LIVE_K8S_MODE}" == "dry-run" && -z "${VALUES_FILES:-}" ]]; then
  candidate_chart_dir="${CHART_DIR:-${COMMAND_CENTER_DIR:+${COMMAND_CENTER_DIR}/helm/charts/citus-cluster}}"
  if [[ -z "${candidate_chart_dir}" ]]; then
    for guess in \
      "${repo_root}/../command-center/helm/charts/citus-cluster" \
      "${repo_root}/../ai-blaise/command-center/helm/charts/citus-cluster"; do
      if [[ -f "${guess}/values-dev.yaml" ]]; then
        candidate_chart_dir="${guess}"
        break
      fi
    done
  fi
  if [[ -n "${candidate_chart_dir}" && -f "${candidate_chart_dir}/values-dev.yaml" ]]; then
    export VALUES_FILES="${candidate_chart_dir}/values-dev.yaml"
  fi
fi

bash ci/ai-blaise/live-k8s-e2e.sh
