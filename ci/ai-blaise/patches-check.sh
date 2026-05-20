#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
series="${1:-patches/series}"
series_path="${repo_root}/${series}"

if [[ ! -f "${series_path}" ]]; then
  echo "patch series not found: ${series}" >&2
  exit 1
fi

tmp_parent="$(mktemp -d)"
worktree="${tmp_parent}/worktree"

cleanup() {
  git -C "${repo_root}" worktree remove --force "${worktree}" >/dev/null 2>&1 || true
  rm -rf "${tmp_parent}"
}
trap cleanup EXIT

git -C "${repo_root}" worktree add --detach --quiet "${worktree}" HEAD

rm -rf "${worktree}/patches"
mkdir -p "${worktree}/patches"
cp -R "${repo_root}/patches/." "${worktree}/patches/"

cd "${worktree}"

while IFS= read -r patch || [[ -n "${patch}" ]]; do
  patch="${patch%%#*}"
  patch="${patch%"${patch##*[![:space:]]}"}"
  patch="${patch#"${patch%%[![:space:]]*}"}"

  if [[ -z "${patch}" ]]; then
    continue
  fi

  patch_path="patches/${patch}"
  if [[ ! -f "${patch_path}" ]]; then
    echo "patch listed in ${series} does not exist: ${patch}" >&2
    exit 1
  fi

  echo "checking ${patch_path}"
  git apply --check --whitespace=error "${patch_path}"
  git apply --whitespace=error "${patch_path}"
done < "${worktree}/${series}"
