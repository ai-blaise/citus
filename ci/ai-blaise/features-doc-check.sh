#!/usr/bin/env bash
set -euo pipefail

# FEATURE: D10

repo="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
validator=""
while (($#)); do
  case "$1" in
    --repo|--feature-register-bin)
      if (($# < 2)); then
        echo "$1 requires a path" >&2
        exit 2
      fi
      if [[ "$1" == "--repo" ]]; then repo="$2"; else validator="$2"; fi
      shift 2
      ;;
    *)
      echo "usage: $0 [--repo PATH] [--feature-register-bin PATH]" >&2
      exit 2
      ;;
  esac
done
if [[ -n "${validator}" ]]; then
  validator="$(cd -- "$(dirname -- "${validator}")" && pwd -P)/$(basename -- "${validator}")"
fi
cd -- "${repo}"

if [[ -n "${validator}" ]]; then
  "${validator}" check-source-coverage --repo .
else
  cargo run --locked --quiet -p ai_blaise_feature_register -- check-source-coverage
fi

base="${BASE_SHA:-}"
head="${HEAD_SHA:-}"
if [[ "${base}" =~ ^0+$ ]]; then base=""; fi
if [[ -z "${base}" ]]; then
  remote_base="origin/${GITHUB_BASE_REF:-main}"
  if git rev-parse --verify --end-of-options "${remote_base}^{commit}" >/dev/null 2>&1; then
    base="${remote_base}"
  elif git rev-parse --verify 'origin/main^{commit}' >/dev/null 2>&1; then
    base=origin/main
  else
    base=HEAD
  fi
fi
base_oid="$(git rev-parse --verify --end-of-options "${base}^{commit}")"

changed_paths="$(mktemp)"
trap 'rm -f "${changed_paths}"' EXIT
if [[ -n "${head}" ]]; then
  head_oid="$(git rev-parse --verify --end-of-options "${head}^{commit}")"
  git diff --name-only -z "${base_oid}" "${head_oid}" -- >"${changed_paths}"
else
  git diff --name-only -z "${base_oid}" -- >"${changed_paths}"
  git ls-files --others --exclude-standard -z >>"${changed_paths}"
fi

inventory_changed=false
feature_changes=()
while IFS= read -r -d '' path; do
  case "${path}" in
    docs/features.tsv) inventory_changed=true ;;
    companion/src/*|sidecar/*/src/*|pool/src/*|operator/src/*|e2e/src/*|patches/*|tools/*/src/*)
      feature_changes+=("${path}")
      ;;
  esac
done <"${changed_paths}"

if ((${#feature_changes[@]} > 0)) && [[ "${inventory_changed}" != true ]]; then
  echo "feature-bearing files changed without updating docs/features.tsv:" >&2
  printf '  %q\n' "${feature_changes[@]}" >&2
  exit 1
fi
printf 'feature_doc_change_check\tpassed\tfeature_paths=%s\tinventory_changed=%s\n' \
  "${#feature_changes[@]}" "${inventory_changed}"
