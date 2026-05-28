#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

build_script="scripts/citus-scale/build-app-images.sh"
release_docs="docs/ai-blaise/RELEASING.md"
production_runbook="docs/ai-blaise/RUNBOOKS/production.md"
deploy_readme="deploy/README.md"
manifest="${RELEASE_DIGEST_MANIFEST:-artifacts/ai-blaise-image-digests.tsv}"
require_published="${REQUIRE_PUBLISHED_DIGESTS:-false}"

fail() {
  echo "release publishability check failed: $*" >&2
  exit 1
}

required_rows=(
  $'citus-operator\tai_blaise_citus_operator\tai_blaise_citus_operator\tserve'
  $'citus-pool\tai_blaise_citus_pool\tai_blaise_citus_pool\tserve'
  $'citus-sidecar-analytical\tai_blaise_citus_sidecar_analytical\tai_blaise_citus_sidecar_analytical\tserve'
  $'citus-sidecar-auth\tai_blaise_citus_sidecar_auth\tai_blaise_citus_sidecar_auth\tserve'
  $'citus-sidecar-backup\tai_blaise_citus_sidecar_backup\tai_blaise_citus_sidecar_backup\tserve'
  $'citus-sidecar-cdc\tai_blaise_citus_sidecar_cdc\tai_blaise_citus_sidecar_cdc\tserve'
  $'citus-sidecar-coldtier\tai_blaise_citus_sidecar_coldtier\tai_blaise_citus_sidecar_coldtier\tserve'
  $'citus-sidecar-edge-functions\tai_blaise_citus_sidecar_edge_functions\tai_blaise_citus_sidecar_edge_functions\tserve'
  $'citus-sidecar-graphql\tai_blaise_citus_sidecar_graphql\tai_blaise_citus_sidecar_graphql\tserve'
  $'citus-sidecar-hlc\tai_blaise_citus_sidecar_hlc\tai_blaise_citus_sidecar_hlc\tserve'
  $'citus-sidecar-mcp\tai_blaise_citus_sidecar_mcp\tai_blaise_citus_sidecar_mcp\tserve'
  $'citus-sidecar-postgrest\tai_blaise_citus_sidecar_postgrest\tai_blaise_citus_sidecar_postgrest\tserve'
  $'citus-sidecar-raft\tai_blaise_citus_sidecar_raft\tai_blaise_citus_sidecar_raft\tserve'
  $'citus-sidecar-realtime\tai_blaise_citus_sidecar_realtime\tai_blaise_citus_sidecar_realtime\tserve'
  $'citus-sidecar-repack\tai_blaise_citus_sidecar_repack\tai_blaise_citus_sidecar_repack\tserve'
  $'citus-sidecar-schema-job\tai_blaise_citus_sidecar_schema_job\tai_blaise_citus_sidecar_schema_job\tserve'
  $'citus-sidecar-storage\tai_blaise_citus_sidecar_storage\tai_blaise_citus_sidecar_storage\tserve'
  $'citus-sidecar-txn-status\tai_blaise_citus_sidecar_txn_status\tai_blaise_citus_sidecar_txn_status\tserve'
  $'citus-sidecar-vectorizer\tai_blaise_citus_sidecar_vectorizer\tai_blaise_citus_sidecar_vectorizer\tserve'
  $'citusctl\tai_blaise_citusctl\tai_blaise_citusctl\tplan inspect cluster'
)

expected_payload_for() {
  local repository="$1"
  local row repo package binary default_args
  for row in "${required_rows[@]}"; do
    IFS=$'\t' read -r repo package binary default_args <<< "${row}"
    if [[ "${repo}" == "${repository}" ]]; then
      printf '%s\t%s\t%s\n' "${package}" "${binary}" "${default_args}"
      return 0
    fi
  done
  return 1
}

tag_is_mutable() {
  case "$1" in
    latest | main | master | dev | test | local) return 0 ;;
    *) return 1 ;;
  esac
}

check_static_contract() {
  [[ -x "${build_script}" ]] || fail "missing executable ${build_script}"
  [[ -s "${release_docs}" ]] || fail "missing ${release_docs}"
  [[ -s "${production_runbook}" ]] || fail "missing ${production_runbook}"
  [[ -s "${deploy_readme}" ]] || fail "missing ${deploy_readme}"

  grep -Fq "LIST_IMAGES" "${build_script}" || fail "${build_script} must expose LIST_IMAGES=true"
  grep -Fq "PUSH=true requires IMAGE_REGISTRY" "${build_script}" || fail "${build_script} must require explicit registry for pushes"
  grep -Fq "PUSH=true requires TAG" "${build_script}" || fail "${build_script} must require explicit tag for pushes"
  grep -Fq "release image tag must not be mutable" "${build_script}" || fail "${build_script} must reject mutable release tags"
  grep -Fq "org.opencontainers.image.revision" "${build_script}" || fail "${build_script} must stamp source revision labels"
  grep -Fq 'source_revision\trepository\timage\ttag\tdigest\tpackage\tbinary\tpushed' "${build_script}" || fail "digest manifest must include source_revision"

  grep -Fq "release-publishability-check" Makefile.ai-blaise || fail "Makefile must expose release-publishability-check"
  grep -Eq '^gate-close:.*release-publishability-check|^gate-close: release-publishability-check$' Makefile.ai-blaise || fail "gate-close must depend on release-publishability-check"
  grep -Fq "release-publishability-check" .github/workflows/ci-image.yml || fail "image CI must run release-publishability-check"

  grep -Fq "REQUIRE_PUBLISHED_DIGESTS=1" "${release_docs}" || fail "release docs must show the published manifest gate"
  grep -Fq "SOURCE_REVISION" "${release_docs}" || fail "release docs must tie images to a source revision"
  grep -Fq "source_revision" "${production_runbook}" || fail "production runbook must explain source_revision in the image handoff"
  grep -Fq "command-center image handoff" "${deploy_readme}" || fail "deploy README must document the command-center image handoff"
}

check_matrix() {
  local matrix line_count expected row found
  matrix="$(LIST_IMAGES=true "${build_script}")"
  line_count="$(printf '%s\n' "${matrix}" | wc -l | tr -d ' ')"
  # Relaxed from exact-21 to header + at-least-N (where N = required_rows).
  # The per-row presence check below is the real contract; this guard
  # only ensures the matrix is non-empty and at least covers the required
  # rows. Adding a new image row no longer reds CI on the count alone.
  local min_lines=$(( ${#required_rows[@]} + 1 ))
  [[ "${line_count}" -ge "${min_lines}" ]] || fail "image matrix must include header plus at least ${#required_rows[@]} rows, got ${line_count}"

  [[ "$(printf '%s\n' "${matrix}" | sed -n '1p')" == $'repository\tpackage\tbinary\tdefault_args' ]] || fail "unexpected image matrix header"

  for expected in "${required_rows[@]}"; do
    found="false"
    while IFS= read -r row; do
      if [[ "${row}" == "${expected}" ]]; then
        found="true"
        break
      fi
    done < <(printf '%s\n' "${matrix}" | sed '1d')
    [[ "${found}" == "true" ]] || fail "image matrix missing row: ${expected}"
  done
}

check_manifest() {
  local manifest_path="$1"
  local expected_header line_count data_count
  local source_revision repository image tag digest package binary pushed
  local expected_payload expected_package expected_binary expected_args
  local seen_repositories=()

  [[ -s "${manifest_path}" ]] || fail "manifest is empty: ${manifest_path}"
  expected_header=$'source_revision\trepository\timage\ttag\tdigest\tpackage\tbinary\tpushed'
  [[ "$(head -n 1 "${manifest_path}")" == "${expected_header}" ]] || fail "unexpected manifest header in ${manifest_path}"

  line_count="$(wc -l <"${manifest_path}" | tr -d ' ')"
  data_count="$((line_count - 1))"
  [[ "${data_count}" -eq "${#required_rows[@]}" ]] || fail "manifest must include ${#required_rows[@]} image rows, got ${data_count}"

  while IFS=$'\t' read -r source_revision repository image tag digest package binary pushed; do
    [[ -n "${source_revision}" && "${source_revision}" != "unknown" ]] || fail "${repository} missing source revision"
    [[ "${source_revision}" =~ ^[0-9a-f]{7,40}$ ]] || fail "${repository} source_revision must be a git SHA: ${source_revision}"
    [[ -n "${tag}" ]] || fail "${repository} missing tag"
    tag_is_mutable "${tag}" && fail "${repository} uses mutable tag ${tag}"
    [[ "${tag}" != *[[:space:]/@]* ]] || fail "${repository} tag contains invalid characters: ${tag}"
    [[ "${image}" == */"${repository}:${tag}" ]] || fail "${repository} image ${image} does not match repository/tag"
    [[ "${pushed}" == "true" || "${pushed}" == "false" ]] || fail "${repository} pushed column must be true or false"

    expected_payload="$(expected_payload_for "${repository}")" || fail "manifest contains unexpected repository: ${repository}"
    IFS=$'\t' read -r expected_package expected_binary expected_args <<< "${expected_payload}"
    [[ "${package}" == "${expected_package}" ]] || fail "${repository} package mismatch: ${package}"
    [[ "${binary}" == "${expected_binary}" ]] || fail "${repository} binary mismatch: ${binary}"
    [[ -n "${expected_args}" ]] || fail "internal expected args missing for ${repository}"

    for seen in "${seen_repositories[@]}"; do
      [[ "${seen}" != "${repository}" ]] || fail "duplicate manifest row for ${repository}"
    done
    seen_repositories+=("${repository}")

    if [[ "${pushed}" == "true" || "${require_published}" == "true" ]]; then
      [[ "${pushed}" == "true" ]] || fail "${repository} must be pushed when REQUIRE_PUBLISHED_DIGESTS=1"
      [[ "${digest}" =~ ^sha256:[0-9a-f]{64}$ ]] || fail "${repository} missing immutable sha256 digest"
    fi
  done < <(tail -n +2 "${manifest_path}")

  for expected in "${required_rows[@]}"; do
    IFS=$'\t' read -r repository _ <<< "${expected}"
    local found="false"
    for seen in "${seen_repositories[@]}"; do
      if [[ "${seen}" == "${repository}" ]]; then
        found="true"
        break
      fi
    done
    [[ "${found}" == "true" ]] || fail "manifest missing repository: ${repository}"
  done
}

check_static_contract
check_matrix

case "${require_published}" in
  true | 1) require_published="true" ;;
  false | 0) require_published="false" ;;
  *) fail "REQUIRE_PUBLISHED_DIGESTS must be true/1 or false/0" ;;
esac

if [[ -e "${manifest}" ]]; then
  check_manifest "${manifest}"
elif [[ "${require_published}" == "true" ]]; then
  fail "required release digest manifest is missing: ${manifest}"
fi

echo "release publishability contract ok"
