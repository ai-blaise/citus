#!/usr/bin/env bash
set -euo pipefail

if [[ "${REQUIRE_DOCKER:-0}" != "1" ]]; then
  echo "sidecar analytical DuckDB extension live smoke requires REQUIRE_DOCKER=1" >&2
  exit 1
fi
if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required for sidecar analytical DuckDB extension live smoke" >&2
  exit 1
fi

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

duckdb_image="${DUCKDB_EXTENSION_IMAGE:-duckdb/duckdb@sha256:ddc7ffc382dfd3f8213ac3d29435a7ce0ea4446fb3fc966a57a28d39b46174b1}"
output="$(cargo run -q -p ai_blaise_citus_sidecar_analytical -- run-duckdb-extension-catalog-canonical)"
header="$(printf '%s\n' "${output}" | sed -n '1p')"
row="$(printf '%s\n' "${output}" | sed -n '2p')"
expected_header=$'feature_id\tallowed_extensions\tallowed_extension_count\tinstall_sql\tload_sql\texternal_io_attempted\tpg_duckdb_runtime_exercised\tmotherduck_session_exercised\tevidence_boundary'
if [[ "${header}" != "${expected_header}" ]]; then
  echo "unexpected DuckDB extension catalog header" >&2
  printf '%s\n' "${header}" >&2
  exit 1
fi

IFS=$'\t' read -r feature_id allowed_extensions allowed_extension_count install_sql load_sql external_io_attempted pg_duckdb_runtime_exercised motherduck_session_exercised evidence_boundary <<<"${row}"
[[ "${feature_id}" == "L12" ]]
[[ "${allowed_extensions}" == "httpfs,iceberg" ]]
[[ "${allowed_extension_count}" == "2" ]]
[[ "${install_sql}" == "INSTALL httpfs;INSTALL iceberg" ]]
[[ "${load_sql}" == "LOAD httpfs;LOAD iceberg" ]]
[[ "${external_io_attempted}" == "false" ]]
[[ "${pg_duckdb_runtime_exercised}" == "false" ]]
[[ "${motherduck_session_exercised}" == "false" ]]
[[ "${evidence_boundary}" == "live-duckdb-container-extension-load-only" ]]

duckdb_sql="INSTALL httpfs; LOAD httpfs; INSTALL iceberg; LOAD iceberg; SELECT extension_name, loaded, installed FROM duckdb_extensions() WHERE extension_name IN ('httpfs', 'iceberg') ORDER BY extension_name;"
duckdb_output="$(docker run --rm "${duckdb_image}" duckdb -csv -c "${duckdb_sql}")"
expected_duckdb=$'extension_name,loaded,installed\nhttpfs,true,true\niceberg,true,true'
if [[ "${duckdb_output}" != "${expected_duckdb}" ]]; then
  echo "unexpected DuckDB extension state" >&2
  printf '%s\n' "${duckdb_output}" >&2
  exit 1
fi

printf 'duckdb_extension_catalog_live=passed\n'
printf 'l12_duckdb_image=%s\n' "${duckdb_image}"
printf 'l12_allowed_extensions=%s\n' "${allowed_extensions}"
printf 'l12_extensions_installed=%s\n' "${allowed_extension_count}"
printf 'l12_extensions_loaded=%s\n' "${allowed_extension_count}"
printf 'l12_duckdb_extensions_catalog_queried=true\n'
printf 'external_io_attempted=%s\n' "${external_io_attempted}"
printf 'pg_duckdb_runtime_exercised=%s\n' "${pg_duckdb_runtime_exercised}"
printf 'motherduck_session_exercised=%s\n' "${motherduck_session_exercised}"
printf 'object_store_io_attempted=false\n'
printf 'extension_repository_mirror_verified=false\n'
printf 'sidecar_analytical_duckdb_extension_live_smoke\tfeature_id=%s\tallowed_extensions=%s\tduckdb_loaded_extensions=%s\tevidence_boundary=%s\n' "${feature_id}" "${allowed_extensions}" "${allowed_extension_count}" "${evidence_boundary}"
