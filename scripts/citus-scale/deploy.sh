#!/usr/bin/env bash
set -euo pipefail

# FEATURE: D8
# FEATURE: O6
#
# After the 2026-05-22 chart fold into ai-blaise/command-center, the deploy
# wrapper that used to render the in-tree Helm chart is no longer applicable
# in this repository. The chart lives at:
#
#   https://github.com/ai-blaise/command-center/tree/main/helm/charts/citus-cluster
#
# To render or install:
#
#   helm template citus-cluster \
#     <command-center-checkout>/helm/charts/citus-cluster \
#     --values <command-center-checkout>/helm/charts/citus-cluster/values-prod.yaml \
#     --set global.operatorImageDigest=sha256:... \
#     --set global.poolImageDigest=sha256:...
#
# Argo CD takes over via gitops/apps/13-citus-cluster.yaml in command-center.

cat <<EOF >&2
ai-blaise/citus deploy wrapper retired on 2026-05-22: chart folded into
ai-blaise/command-center/helm/charts/citus-cluster/. Use the command-center
chart directly with Helm or via Argo CD against
gitops/apps/13-citus-cluster.yaml in that repo.
EOF
exit 64
