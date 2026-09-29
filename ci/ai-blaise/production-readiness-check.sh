#!/usr/bin/env bash
set -euo pipefail

# FEATURE: D10

mode="${1:-audit}"
if [[ $# -gt 1 || ( "${mode}" != "audit" && "${mode}" != "production-release" ) ]]; then
  echo "usage: $0 [audit|production-release]" >&2
  exit 2
fi

if [[ "${mode}" == "production-release" ]]; then
  exec cargo run --locked --quiet -p ai_blaise_feature_register -- release-gaps
fi

cargo run --locked --quiet -p ai_blaise_feature_register -- check-source-coverage
bash ci/ai-blaise/docs-evidence-boundary-check.sh
printf 'production_readiness_audit\tmode=audit\tauthority=structural-only\trelease_qualification=unverified\n'
