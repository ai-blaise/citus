#!/usr/bin/env bash
set -euo pipefail

base="${BASE_SHA:-}"
head="${HEAD_SHA:-HEAD}"

if [[ -z "${base}" ]]; then
  remote_base="${GITHUB_BASE_REF:-main}"
  if git config --get remote.origin.url >/dev/null 2>&1; then
    git fetch -q origin "${remote_base}:refs/remotes/origin/${remote_base}" >/dev/null 2>&1 || true
    if [[ "${remote_base}" != "main" ]]; then
      git fetch -q origin main:refs/remotes/origin/main >/dev/null 2>&1 || true
    fi
  fi

  if git rev-parse --verify "origin/${remote_base}" >/dev/null 2>&1; then
    base="origin/${remote_base}"
  elif git rev-parse --verify origin/main >/dev/null 2>&1; then
    base="origin/main"
  else
    base="$(git rev-list --max-parents=0 HEAD | tail -1)"
  fi
fi

feature_paths='^(companion/src/|sidecar/[^/]+/src/|pool/src/|operator/src/crds/|e2e/src/|patches/|tools/[^/]+/src/)'
scan_paths=(
  companion
  sidecar
  pool
  operator
  e2e
  tools
  patches
)

source_ids="$(mktemp)"
doc_ids="$(mktemp)"
trap 'rm -f "${source_ids}" "${doc_ids}"' EXIT

extract_feature_ids() {
  if command -v rg >/dev/null 2>&1; then
    rg -No 'FEATURE: [A-Za-z][A-Za-z0-9]*' "$@" || true
  else
    grep -RhoE 'FEATURE: [A-Za-z][A-Za-z0-9]*' "$@" 2>/dev/null || true
  fi
}

extract_feature_ids "${scan_paths[@]}" \
  | sed -E 's/.*FEATURE: ([A-Za-z][A-Za-z0-9]*).*/\1/' \
  | sort -u >"${source_ids}"

extract_feature_ids docs/ai-blaise/NEW_FEATURES.md \
  | sed -E 's/.*FEATURE: ([A-Za-z][A-Za-z0-9]*).*/\1/' \
  | sort -u >"${doc_ids}"

missing_ids="$(comm -23 "${source_ids}" "${doc_ids}")"

if [[ -n "${missing_ids}" ]]; then
  echo "source FEATURE markers missing from docs/ai-blaise/NEW_FEATURES.md:" >&2
  echo "${missing_ids}" >&2
  exit 1
fi

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
