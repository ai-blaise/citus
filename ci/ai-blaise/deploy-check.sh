#!/usr/bin/env bash
set -euo pipefail

# FEATURE: D8

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

# CI-safe by default: render/lint when CHART_DIR or COMMAND_CENTER_DIR is
# supplied, otherwise report the external chart dependency explicitly. Release
# jobs can set REQUIRE_HELM=1 or REQUIRE_CHART=1 to fail closed on a missing
# command-center checkout.
export LIVE_K8S_MODE="${LIVE_K8S_MODE:-dry-run}"
export REQUIRE_CHART="${REQUIRE_CHART:-${REQUIRE_HELM:-0}}"
export REQUIRE_HTTP="${REQUIRE_HTTP:-0}"
export REQUIRE_SQL="${REQUIRE_SQL:-0}"

exec bash ci/ai-blaise/live-k8s-e2e.sh
