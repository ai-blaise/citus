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

bash ci/ai-blaise/live-k8s-e2e.sh
