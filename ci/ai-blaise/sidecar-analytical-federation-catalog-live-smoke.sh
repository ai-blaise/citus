#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

tmp_dir="$(mktemp -d)"
server_pid=""
cleanup() {
  if [[ -n "${server_pid}" ]]; then
    kill "${server_pid}" >/dev/null 2>&1 || true
    wait "${server_pid}" >/dev/null 2>&1 || true
  fi
  rm -rf "${tmp_dir}"
}
trap cleanup EXIT

artifact_path="${tmp_dir}/federation-catalog.json"
output="$(AI_BLAISE_FEDERATION_CATALOG_ARTIFACT="${artifact_path}" cargo run -q -p ai_blaise_citus_sidecar_analytical -- run-federation-catalog-publication-canonical)"
header="$(printf '%s\n' "${output}" | sed -n '1p')"
row="$(printf '%s\n' "${output}" | sed -n '2p')"
expected_header=$'feature_id\tversion\tcatalog_names\tfederation_targets\tcatalog_count\tartifact_path\tartifact_bytes\tlocal_catalog_artifact_created\texternal_warehouse_connections_attempted\tobject_store_io_attempted\tcatalog_auth_exercised\tevidence_boundary'
if [[ "${header}" != "${expected_header}" ]]; then
  echo "unexpected federation catalog publication header" >&2
  printf '%s\n' "${header}" >&2
  exit 1
fi

IFS=$'\t' read -r feature_id version catalog_names federation_targets catalog_count reported_artifact_path artifact_bytes local_catalog_artifact_created external_warehouse_connections_attempted object_store_io_attempted catalog_auth_exercised evidence_boundary <<<"${row}"
[[ "${feature_id}" == "L6" ]]
[[ "${version}" == "v1" ]]
[[ "${catalog_names}" == "databricks,snowflake,trino,spark" ]]
[[ "${federation_targets}" == "databricks,snowflake,trino,spark" ]]
[[ "${catalog_count}" == "4" ]]
[[ "${reported_artifact_path}" == "${artifact_path}" ]]
[[ "${artifact_bytes}" =~ ^[0-9]+$ ]]
(( artifact_bytes > 0 ))
[[ "${local_catalog_artifact_created}" == "true" ]]
[[ "${external_warehouse_connections_attempted}" == "false" ]]
[[ "${object_store_io_attempted}" == "false" ]]
[[ "${catalog_auth_exercised}" == "false" ]]
[[ "${evidence_boundary}" == "local-federation-catalog-artifact-http-only" ]]

python3 - "${artifact_path}" <<'JSONPY'
import json
import sys
path = sys.argv[1]
with open(path, encoding="utf-8") as handle:
    payload = json.load(handle)
assert payload["feature_id"] == "L6"
assert payload["version"] == "v1"
assert [catalog["target"] for catalog in payload["catalogs"]] == [
    "databricks",
    "snowflake",
    "trino",
    "spark",
]
assert payload["external_warehouse_connections_attempted"] is False
assert payload["object_store_io_attempted"] is False
assert payload["catalog_auth_exercised"] is False
JSONPY

port="$((21000 + ($$ % 1000)))"
(
  cd "${tmp_dir}"
  python3 -m http.server "${port}" --bind 127.0.0.1 >/dev/null 2>&1
) &
server_pid="$!"
for _ in $(seq 1 50); do
  if curl -fsS --max-time 1 "http://127.0.0.1:${port}/federation-catalog.json" >"${tmp_dir}/fetched.json" 2>/dev/null; then
    break
  fi
  sleep 0.2
done
if [[ ! -s "${tmp_dir}/fetched.json" ]]; then
  echo "local federation catalog HTTP publication did not become readable" >&2
  exit 1
fi
cmp "${artifact_path}" "${tmp_dir}/fetched.json"

printf 'federation_catalog_publication_live=passed\n'
printf 'l6_catalog_version=%s\n' "${version}"
printf 'l6_catalog_count=%s\n' "${catalog_count}"
printf 'l6_federation_targets=%s\n' "${federation_targets}"
printf 'l6_local_catalog_artifact_created=%s\n' "${local_catalog_artifact_created}"
printf 'l6_local_http_catalog_served=true\n'
printf 'external_warehouse_connections_attempted=%s\n' "${external_warehouse_connections_attempted}"
printf 'object_store_io_attempted=%s\n' "${object_store_io_attempted}"
printf 'catalog_auth_exercised=%s\n' "${catalog_auth_exercised}"
printf 'sidecar_analytical_federation_catalog_live_smoke\tfeature_id=%s\ttargets=%s\tcatalog_count=%s\tevidence_boundary=%s\n' "${feature_id}" "${federation_targets}" "${catalog_count}" "${evidence_boundary}"
