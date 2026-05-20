#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
upstream_remote="${UPSTREAM_REMOTE:-https://github.com/citusdata/citus.git}"
upstream_ref="${UPSTREAM_REF:-release-14.0}"
series="${SERIES:-patches/series}"
series_path="${repo_root}/${series}"

if [[ ! -f "${series_path}" ]]; then
  echo "patch series not found: ${series}" >&2
  exit 1
fi

tmp_parent="$(mktemp -d)"
worktree="${tmp_parent}/upstream-worktree"

cleanup() {
  git -C "${repo_root}" worktree remove --force "${worktree}" >/dev/null 2>&1 || true
  rm -rf "${tmp_parent}"
}
trap cleanup EXIT

git -C "${repo_root}" fetch --quiet "${upstream_remote}" "${upstream_ref}"
upstream_sha="$(git -C "${repo_root}" rev-parse FETCH_HEAD)"
git -C "${repo_root}" worktree add --detach --quiet "${worktree}" "${upstream_sha}"

rm -rf "${worktree}/patches"
mkdir -p "${worktree}/patches"
cp -R "${repo_root}/patches/." "${worktree}/patches/"

while IFS= read -r patch || [[ -n "${patch}" ]]; do
  patch="${patch%%#*}"
  patch="${patch%"${patch##*[![:space:]]}"}"
  patch="${patch#"${patch%%[![:space:]]*}"}"

  if [[ -z "${patch}" ]]; then
    continue
  fi

  patch_path="${worktree}/patches/${patch}"
  if [[ ! -f "${patch_path}" ]]; then
    echo "patch listed in ${series} does not exist: ${patch}" >&2
    exit 1
  fi

  echo "checking ${patch} against ${upstream_ref}@${upstream_sha}"
  git -C "${worktree}" apply --check --whitespace=error "${patch_path}"
  git -C "${worktree}" apply --whitespace=error "${patch_path}"
done <"${series_path}"

echo "upstream merge dry-run passed for ${upstream_ref}@${upstream_sha}"
