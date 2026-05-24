#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

# The in-repo Helm chart was folded into ai-blaise/command-center on
# 2026-05-22. This Citus-side deploy check therefore validates the deployment
# guardrail contract that command-center must render for ai-blaise workloads.
bash ci/ai-blaise/k8s-guardrails-check.sh
