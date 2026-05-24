#!/usr/bin/env bash
set -euo pipefail

# FEATURE: TS6 TS18
#
# TS-version cohabitation matrix smoke. Iterates over the TS minor lines
# pinned in `tests/cohab-matrix/<TS_VERSION>/expected-hook-claims.tsv` and
# runs the existing single-version cohabitation smoke against each image
# pinned in `tests/cohab-matrix/<TS_VERSION>/image-tag.txt`.
#
# Per-version semantics:
#   - If the Docker image tag is not yet published, log a PASS-WITH-NOTE skip
#     and continue for optional versions. Required versions fail closed.
#   - If the image exists while the expected table still contains `unknown`
#     hook rows, the comparator fails so live compatibility is measured before
#     the row can become production evidence.
#   - If the image exists, set `TIMESCALE_COHABITATION_BASE_IMAGE` to the
#     pinned tag and exec the load-bearing
#     `ci/ai-blaise/timescale-cohabitation-smoke.sh`. That script is the
#     production evidence path; the matrix is the forward-compat regression
#     net.
#
# Env knobs:
#   TS_VERSION_MATRIX            -- space-separated TS versions, defaults to
#                                   every subdir under tests/cohab-matrix/.
#   TS_VERSION_MATRIX_REQUIRED   -- space-separated TS versions whose absence
#                                   is a hard failure (defaults: 2.27).
#   REQUIRE_DOCKER               -- forwarded to the inner script.
#
# Exit codes:
#   0 -- matrix passed (every required version ran; optional versions either
#        ran or were skipped with note).
#   1 -- a required version failed or its image was unavailable.

repo_root="$(git rev-parse --show-toplevel)"
matrix_dir="${repo_root}/tests/cohab-matrix"
inner_smoke="${repo_root}/ci/ai-blaise/timescale-cohabitation-smoke.sh"
evidence_dir="${repo_root}/artifacts"
matrix_log="${evidence_dir}/ts-version-matrix-smoke.tsv"

if [[ ! -d "${matrix_dir}" ]]; then
  echo "missing matrix directory: ${matrix_dir}" >&2
  exit 1
fi

if [[ ! -x "${inner_smoke}" ]]; then
  echo "missing executable inner cohabitation smoke: ${inner_smoke}" >&2
  exit 1
fi

if [[ ! -x "${matrix_dir}/compare-hook-claims.sh" ]]; then
  echo "missing executable matrix comparator: ${matrix_dir}/compare-hook-claims.sh" >&2
  exit 1
fi

declare -a discovered=()
while IFS= read -r -d '' subdir; do
  version="$(basename "${subdir}")"
  discovered+=("${version}")
done < <(find "${matrix_dir}" -mindepth 1 -maxdepth 1 -type d -print0 | LC_ALL=C sort -z)

if [[ ${#discovered[@]} -eq 0 ]]; then
  echo "no TS versions discovered under ${matrix_dir}" >&2
  exit 1
fi

if [[ -n "${TS_VERSION_MATRIX:-}" ]]; then
  read -r -a versions <<<"${TS_VERSION_MATRIX}"
else
  versions=("${discovered[@]}")
fi

required_versions="${TS_VERSION_MATRIX_REQUIRED-2.27}"

is_required_version() {
  local candidate="$1"
  [[ " ${required_versions} " == *" ${candidate} "* ]]
}

mkdir -p "${evidence_dir}"
printf 'ts_version\tbase_image\tstatus\tnote\tevidence_file\n' >"${matrix_log}"

log_matrix_row() {
  printf '%s\t%s\t%s\t%s\t%s\n' "$1" "$2" "$3" "$4" "$5" >>"${matrix_log}"
}

write_matrix_note_evidence() {
  local evidence_file="$1"
  local ts_version="$2"
  local base_image="$3"
  local status="$4"
  local note="$5"
  local expected_tsv="$6"
  local expected_sha="-"

  if [[ -s "${expected_tsv}" ]]; then
    expected_sha="$(sha256sum "${expected_tsv}" | awk '{ print $1 }')"
  fi

  {
    local required="false"
    if is_required_version "${ts_version}"; then
      required="true"
    fi

    printf 'ts_version\tbase_image\tstatus\tnote\tdocker_manifest_available\trequired_version\texpected_hook_claims_sha256\n'
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "${ts_version}" "${base_image}" "${status}" "${note}" "false" "${required}" "${expected_sha}"
  } >"${evidence_file}"
}

overall=0
ran_any=0
for ts_version in "${versions[@]}"; do
  expected_tsv="${matrix_dir}/${ts_version}/expected-hook-claims.tsv"
  image_tag_file="${matrix_dir}/${ts_version}/image-tag.txt"
  if [[ ! -s "${expected_tsv}" ]]; then
    echo "matrix entry missing expected-hook-claims.tsv: ${ts_version}" >&2
    log_matrix_row "${ts_version}" "-" "missing-expected-tsv" "no expected-hook-claims.tsv" "-"
    overall=1
    continue
  fi

  if [[ -s "${image_tag_file}" ]]; then
    base_image="$(head -n 1 "${image_tag_file}" | tr -d '[:space:]')"
  else
    base_image="timescale/timescaledb:${ts_version}-pg17"
  fi
  cohab_evidence="${evidence_dir}/timescale-cohabitation-${ts_version}.tsv"
  echo "=== TS ${ts_version}: ${base_image} ==="

  if ! command -v docker >/dev/null 2>&1; then
    if [[ "${REQUIRE_DOCKER:-0}" == "1" ]]; then
      echo "docker is required for the TS-version matrix smoke" >&2
      write_matrix_note_evidence "${cohab_evidence}" "${ts_version}" "${base_image}" "fail" "docker unavailable but REQUIRE_DOCKER=1" "${expected_tsv}"
      log_matrix_row "${ts_version}" "${base_image}" "fail" "docker unavailable but REQUIRE_DOCKER=1" "${cohab_evidence}"
      exit 1
    fi
    echo "(docker unavailable; matrix smoke records skip-with-note for TS ${ts_version})"
    write_matrix_note_evidence "${cohab_evidence}" "${ts_version}" "${base_image}" "skip-with-note" "docker unavailable" "${expected_tsv}"
    log_matrix_row "${ts_version}" "${base_image}" "skip-with-note" "docker unavailable" "${cohab_evidence}"
    continue
  fi

  if ! docker manifest inspect "${base_image}" >/dev/null 2>&1; then
    if is_required_version "${ts_version}"; then
      echo "TS ${ts_version} image ${base_image} not published; this version is in TS_VERSION_MATRIX_REQUIRED, failing matrix gate" >&2
      write_matrix_note_evidence "${cohab_evidence}" "${ts_version}" "${base_image}" "fail" "required version missing image" "${expected_tsv}"
      log_matrix_row "${ts_version}" "${base_image}" "fail" "required version missing image" "${cohab_evidence}"
      overall=1
      continue
    fi
    echo "(TS ${ts_version} image ${base_image} not yet available; skipping with PASS-WITH-NOTE)"
    write_matrix_note_evidence "${cohab_evidence}" "${ts_version}" "${base_image}" "skip-with-note" "image tag not yet published" "${expected_tsv}"
    log_matrix_row "${ts_version}" "${base_image}" "skip-with-note" "image tag not yet published" "${cohab_evidence}"
    continue
  fi

  echo "TS ${ts_version}: running ${inner_smoke} against ${base_image}"
  image_tag="ai-blaise-citus-timescale-cohabitation-${ts_version//./-}:local"
  if TIMESCALE_COHABITATION_BASE_IMAGE="${base_image}" \
     TIMESCALE_COHABITATION_TAG="${image_tag}" \
     TIMESCALE_COHABITATION_EVIDENCE="${cohab_evidence}" \
     TIMESCALE_COHABITATION_EXPECTED_TS_MINOR="${ts_version}" \
     "${inner_smoke}"; then
    container="ai-blaise-cohab-matrix-${ts_version//./-}-$$"
    cleanup_probe() {
      docker rm -f "${container}" >/dev/null 2>&1 || true
    }
    trap cleanup_probe EXIT

    docker run --name "${container}" \
      -e POSTGRES_PASSWORD=postgres \
      -d "${image_tag}" \
      postgres \
      -c shared_preload_libraries=timescaledb,citus \
      -c citus.cohabit_extensions=timescaledb >/dev/null

    ready=0
    for _ in $(seq 1 180); do
      if docker exec "${container}" psql -U postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
        ready=1
        break
      fi
      sleep 1
    done
    if [[ "${ready}" != "1" ]]; then
      echo "TS ${ts_version} matrix probe container did not become ready" >&2
      log_matrix_row "${ts_version}" "${base_image}" "fail" "probe container not ready" "${cohab_evidence}"
      overall=1
      cleanup_probe
      trap - EXIT
      continue
    fi

    extensions_ready=0
    for _ in $(seq 1 30); do
      if docker exec "${container}" psql -U postgres -v ON_ERROR_STOP=1 \
          -c 'CREATE EXTENSION IF NOT EXISTS citus; CREATE EXTENSION IF NOT EXISTS timescaledb;' >/dev/null 2>&1; then
        extensions_ready=1
        break
      fi
      sleep 1
    done
    if [[ "${extensions_ready}" != "1" ]]; then
      echo "TS ${ts_version} matrix probe extensions did not become ready" >&2
      log_matrix_row "${ts_version}" "${base_image}" "fail" "probe extensions not ready" "${cohab_evidence}"
      overall=1
      cleanup_probe
      trap - EXIT
      continue
    fi

    if bash "${matrix_dir}/compare-hook-claims.sh" "${ts_version}" "${container}"; then
      log_matrix_row "${ts_version}" "${base_image}" "pass" "cohabitation smoke and hook-claim compare matched expected" "${cohab_evidence}"
      ran_any=1
    else
      log_matrix_row "${ts_version}" "${base_image}" "fail" "hook-claim compare diverged" "${cohab_evidence}"
      overall=1
    fi
    cleanup_probe
    trap - EXIT
  else
    echo "TS ${ts_version} inner cohabitation smoke failed" >&2
    log_matrix_row "${ts_version}" "${base_image}" "fail" "inner cohabitation smoke failed" "${cohab_evidence}"
    overall=1
  fi
done

if [[ "${overall}" -ne 0 ]]; then
  echo "TS-version matrix smoke: at least one version failed; see ${matrix_log}" >&2
  exit 1
fi

if [[ "${ran_any}" -ne 1 ]]; then
  if [[ -z "${required_versions}" ]]; then
    echo "TS-version matrix smoke: nothing ran and no required versions were configured; matrix log at ${matrix_log}"
    exit 0
  fi
  echo "TS-version matrix smoke: no required version actually executed; see ${matrix_log}" >&2
  exit 1
fi

echo "TS-version matrix smoke passed; matrix log at ${matrix_log}"
