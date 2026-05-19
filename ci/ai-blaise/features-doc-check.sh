#!/usr/bin/env bash
set -euo pipefail

base="${BASE_SHA:-}"
head="${HEAD_SHA:-HEAD}"

if [[ -z "${base}" ]]; then
  if git rev-parse --verify origin/main >/dev/null 2>&1; then
    base="origin/main"
  else
    base="$(git rev-list --max-parents=0 HEAD | tail -1)"
  fi
fi

feature_paths='^(companion/src/|sidecar/[^/]+/src/|pool/src/|operator/src/crds/|patches/|tools/[^/]+/src/)'

added_files="$(git diff --name-only --diff-filter=A "${base}" "${head}" \
  | grep -E "${feature_paths}" || true)"

if [[ -z "${added_files}" ]]; then
  exit 0
fi

if git diff --name-only "${base}" "${head}" \
  | grep -qx 'docs/ai-blaise/NEW_FEATURES.md'; then
  exit 0
fi

echo "feature-bearing files were added without updating docs/ai-blaise/NEW_FEATURES.md" >&2
echo "${added_files}" >&2
exit 1
