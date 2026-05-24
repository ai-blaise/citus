#!/usr/bin/env bash
set -euo pipefail

# FEATURE: D13

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if [[ -z "${LIVE_K8S_MODE:-}" ]]; then
  if [[ "${REAL_K8S:-0}" == "1" ]]; then
    export LIVE_K8S_MODE=kind
  else
    export LIVE_K8S_MODE=dry-run
  fi
fi

case "${LIVE_K8S_MODE}" in
  dry-run)
    export REQUIRE_HTTP="${REQUIRE_HTTP:-0}"
    export REQUIRE_SQL="${REQUIRE_SQL:-0}"
    ;;
  real|kind)
    export REQUIRE_CHART="${REQUIRE_CHART:-1}"
    export REQUIRE_HTTP="${REQUIRE_HTTP:-1}"
    export REQUIRE_SQL="${REQUIRE_SQL:-1}"
    ;;
  *)
    echo "LIVE_K8S_MODE must be dry-run, real, or kind" >&2
    exit 2
    ;;
esac

exec bash ci/ai-blaise/live-k8s-e2e.sh
