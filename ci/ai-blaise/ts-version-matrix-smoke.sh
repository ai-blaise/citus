#!/usr/bin/env bash
set -euo pipefail

# FEATURE: TS6 TS18
#
# TS-version cohabitation matrix smoke. Iterates over the TS minor lines
# pinned in `tests/cohab-matrix/<TS_VERSION>/expected-hook-claims.tsv` and
# runs the existing single-version cohabitation smoke against each exact image
# pinned in the fixture lock and `tests/cohab-matrix/<TS_VERSION>/image-tag.txt`.
#
# Per-version semantics:
#   - The shared builder selects the requested minor from its exact digest lock
#     and rejects missing, mismatched, or mutable images.
#   - If the expected table contains `unknown` hook rows, the comparator fails
#     so live compatibility is measured before the row can become evidence.
#   - The matrix passes the verified immutable image ID to the load-bearing
#     `ci/ai-blaise/timescale-cohabitation-smoke.sh`. That script is the
#     cohabitation evidence path; the matrix is the forward-compat regression
#     net. Neither path is release qualification.
#
# Env knobs:
#   TS_VERSION_MATRIX            -- space-separated TS versions, defaults to
#                                   every subdir under tests/cohab-matrix/.
#   TS_VERSION_MATRIX_REQUIRED   -- space-separated TS versions whose absence
#                                   is a hard failure (defaults: 2.27 and 2.28).
#   REQUIRE_DOCKER               -- forwarded to the inner script.
#
# Exit codes:
#   0 -- matrix passed (every required version ran; optional versions either
#        ran or were skipped with note).
#   1 -- a required version failed or its image was unavailable.

repo_root="$(git rev-parse --show-toplevel)"
matrix_dir="${repo_root}/tests/cohab-matrix"
inner_smoke="${repo_root}/ci/ai-blaise/timescale-cohabitation-smoke.sh"
fixture_builder="${repo_root}/ci/ai-blaise/build-real-citus-timescale-test-fixture.sh"
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
if [[ ! -x "${fixture_builder}" ]]; then
  echo "missing executable real-Citus Timescale fixture builder: ${fixture_builder}" >&2
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

required_versions_value="${TS_VERSION_MATRIX_REQUIRED-2.27 2.28}"
declare -a required_versions=()
if [[ -n "${required_versions_value}" ]]; then
  read -r -a required_versions <<<"${required_versions_value}"
fi

is_required_version() {
  local candidate="$1"
  local required_version
  for required_version in "${required_versions[@]}"; do
    if [[ "${candidate}" == "${required_version}" ]]; then
      return 0
    fi
  done
  return 1
}

for required_version in "${required_versions[@]}"; do
  selected=0
  for version in "${versions[@]}"; do
    if [[ "${version}" == "${required_version}" ]]; then
      selected=1
      break
    fi
  done
  if [[ "${selected}" != "1" ]]; then
    echo "required TS version was not selected: ${required_version}" >&2
    exit 1
  fi
done

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

  if [[ ! -s "${image_tag_file}" ]]; then
    echo "matrix entry missing immutable image reference: ${ts_version}" >&2
    log_matrix_row "${ts_version}" "-" "fail" "missing immutable image reference" "-"
    overall=1
    continue
  fi
  base_image="$(head -n 1 "${image_tag_file}" | tr -d '[:space:]')"
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

  if ! fixture_image="$("${fixture_builder}" --timescaledb-minor "${ts_version}")"; then
    echo "TS ${ts_version} exact source fixture could not be built or verified" >&2
    write_matrix_note_evidence "${cohab_evidence}" "${ts_version}" "${base_image}" "fail" "exact fixture unavailable" "${expected_tsv}"
    log_matrix_row "${ts_version}" "${base_image}" "fail" "exact fixture unavailable" "${cohab_evidence}"
    overall=1
    continue
  fi
  if [[ ! "${fixture_image}" =~ ^sha256:[0-9a-f]{64}$ ]]; then
    echo "TS ${ts_version} fixture builder did not return an immutable image ID" >&2
    overall=1
    continue
  fi

  echo "TS ${ts_version}: running ${inner_smoke} against its verified fixture"
  if TIMESCALE_COHABITATION_IMAGE="${fixture_image}" \
     TIMESCALE_COHABITATION_EVIDENCE="${cohab_evidence}" \
     TIMESCALE_COHABITATION_EXPECTED_TS_MINOR="${ts_version}" \
     "${inner_smoke}"; then
    container="ai-blaise-cohab-matrix-${ts_version//./-}-$$"
    cleanup_probe() {
      docker rm --force --volumes "${container}" >/dev/null 2>&1 || true
    }
    trap cleanup_probe EXIT

    docker run --name "${container}" \
      --network none \
      -e POSTGRES_PASSWORD=postgres \
      -d "${fixture_image}" \
      postgres \
      -c shared_preload_libraries=timescaledb,citus \
      -c citus.cohabit_extensions=timescaledb >/dev/null

    init_complete=0
    for _ in $(seq 1 180); do
      probe_logs="$(docker logs --tail 200 "${container}" 2>&1 || true)"
      if [[ "${probe_logs}" == *"PostgreSQL init process complete"* ]]; then
        init_complete=1
        break
      fi
      sleep 1
    done
    if [[ "${init_complete}" != "1" ]]; then
      echo "TS ${ts_version} matrix probe container did not finish init scripts" >&2
      log_matrix_row "${ts_version}" "${base_image}" "fail" "probe init not complete" "${cohab_evidence}"
      overall=1
      cleanup_probe
      trap - EXIT
      continue
    fi

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
      log_matrix_row "${ts_version}" "${base_image}" "pass" "exact-image cohabitation passed; static hook inventory structurally closed" "${cohab_evidence}"
      ran_any=1
    else
      log_matrix_row "${ts_version}" "${base_image}" "fail" "runtime admission or static hook-inventory structure failed" "${cohab_evidence}"
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
  if [[ ${#required_versions[@]} -eq 0 ]]; then
    echo "TS-version matrix smoke: nothing ran and no required versions were configured; matrix log at ${matrix_log}"
    exit 0
  fi
  echo "TS-version matrix smoke: no required version actually executed; see ${matrix_log}" >&2
  exit 1
fi

echo "TS-version matrix smoke passed; matrix log at ${matrix_log}"
