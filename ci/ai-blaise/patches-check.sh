#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
default_series_list=("patches/series" "patches/postgres/series")

if [[ $# -gt 0 ]]; then
  series_list=("$@")
else
  series_list=()
  for candidate in "${default_series_list[@]}"; do
    if [[ -f "${repo_root}/${candidate}" ]]; then
      series_list+=("${candidate}")
    fi
  done
fi

if [[ ${#series_list[@]} -eq 0 ]]; then
  echo "no patch series files found (looked for: ${default_series_list[*]})" >&2
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

verify_postgres_patch_format() {
  local patch_path="$1"
  if [[ ! -s "${patch_path}" ]]; then
    echo "postgres patch is empty: ${patch_path}" >&2
    return 1
  fi
  if ! grep -q '^diff --git ' "${patch_path}"; then
    echo "postgres patch missing 'diff --git' header: ${patch_path}" >&2
    return 1
  fi
  if ! grep -q '^--- a/' "${patch_path}"; then
    echo "postgres patch missing '--- a/' line: ${patch_path}" >&2
    return 1
  fi
  if ! grep -q '^+++ b/' "${patch_path}"; then
    echo "postgres patch missing '+++ b/' line: ${patch_path}" >&2
    return 1
  fi
  if ! grep -q '^@@ ' "${patch_path}"; then
    echo "postgres patch missing hunk header '@@ ': ${patch_path}" >&2
    return 1
  fi
  if ! grep -q '^FEATURE: ' "${patch_path}"; then
    echo "postgres patch missing 'FEATURE:' marker: ${patch_path}" >&2
    return 1
  fi
  return 0
}

for series in "${series_list[@]}"; do
  series_path="${repo_root}/${series}"

  if [[ ! -f "${series_path}" ]]; then
    echo "patch series not found: ${series}" >&2
    exit 1
  fi

  series_dir="$(dirname "${series}")"
  mode="citus"
  if [[ "${series_dir}" == "patches/postgres" ]]; then
    mode="postgres"
  fi

  echo "checking series ${series} (mode=${mode})"

  while IFS= read -r patch || [[ -n "${patch}" ]]; do
    patch="${patch%%#*}"
    patch="${patch%"${patch##*[![:space:]]}"}"
    patch="${patch#"${patch%%[![:space:]]*}"}"

    if [[ -z "${patch}" ]]; then
      continue
    fi

    patch_path="${series_dir}/${patch}"
    if [[ ! -f "${patch_path}" ]]; then
      echo "patch listed in ${series} does not exist: ${patch}" >&2
      exit 1
    fi

    echo "checking ${patch_path}"
    if [[ "${mode}" == "postgres" ]]; then
      # Postgres-core patches target upstream PG source, which is not in this
      # repo; verify diff format only. The PG-build pipeline (see
      # images/citus-pg-overlay/Dockerfile) applies these to PG source before
      # Citus is compiled.
      verify_postgres_patch_format "${patch_path}"
    else
      if git apply --reverse --check --whitespace=error "${patch_path}" >/dev/null 2>&1; then
        echo "already integrated ${patch_path}"
      elif git apply --check --whitespace=error "${patch_path}" >/dev/null 2>&1; then
        git apply --whitespace=error "${patch_path}"
      else
        git apply --check --whitespace=error "${patch_path}"
      fi
    fi
  done < "${worktree}/${series}"
done
