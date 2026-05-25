#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "${tmp_dir}"
}
trap cleanup EXIT

parquet_path="${tmp_dir}/orders.parquet"
output="$(AI_BLAISE_ANALYTICAL_PARQUET_ARTIFACT="${parquet_path}" cargo run -q -p ai_blaise_citus_sidecar_analytical -- run-local-parquet-read-canonical)"
header="$(printf '%s\n' "${output}" | sed -n '1p')"
row="$(printf '%s\n' "${output}" | sed -n '2p')"

expected_header=$'feature_id\ttable\tformat\tparquet_path\tparquet_bytes\tsource_rows\tsource_total\tdatafusion_output_rows\tdatafusion_output_total\tprojection_pushdown_executed\tfilter_pushdown_executed\tlimit_pushdown_executed\tlocal_parquet_file_created\tdatafusion_parquet_read_executed\texternal_io_attempted\tobject_store_io_attempted\ticeberg_runtime_exercised\tdelta_runtime_exercised\tpg_lake_runtime_exercised\tmotherduck_session_exercised\tkubernetes_traffic_exercised\tevidence_boundary'
if [[ "${header}" != "${expected_header}" ]]; then
  echo "unexpected local Parquet read header" >&2
  printf '%s\n' "${header}" >&2
  exit 1
fi

IFS=$'\t' read -r \
  feature_id table format reported_parquet_path parquet_bytes source_rows source_total \
  datafusion_output_rows datafusion_output_total projection_pushdown_executed \
  filter_pushdown_executed limit_pushdown_executed local_parquet_file_created \
  datafusion_parquet_read_executed external_io_attempted object_store_io_attempted \
  iceberg_runtime_exercised delta_runtime_exercised pg_lake_runtime_exercised \
  motherduck_session_exercised kubernetes_traffic_exercised evidence_boundary <<<"${row}"

[[ "${feature_id}" == "L3" ]]
[[ "${table}" == "public.orders" ]]
[[ "${format}" == "parquet" ]]
[[ "${reported_parquet_path}" == "${parquet_path}" ]]
[[ "${parquet_bytes}" =~ ^[0-9]+$ ]]
(( parquet_bytes > 0 ))
[[ -s "${parquet_path}" ]]
[[ "${source_rows}" == "4" ]]
[[ "${source_total}" == "5500" ]]
[[ "${datafusion_output_rows}" == "2" ]]
[[ "${datafusion_output_total}" == "3000" ]]
[[ "${projection_pushdown_executed}" == "true" ]]
[[ "${filter_pushdown_executed}" == "true" ]]
[[ "${limit_pushdown_executed}" == "true" ]]
[[ "${local_parquet_file_created}" == "true" ]]
[[ "${datafusion_parquet_read_executed}" == "true" ]]
[[ "${external_io_attempted}" == "false" ]]
[[ "${object_store_io_attempted}" == "false" ]]
[[ "${iceberg_runtime_exercised}" == "false" ]]
[[ "${delta_runtime_exercised}" == "false" ]]
[[ "${pg_lake_runtime_exercised}" == "false" ]]
[[ "${motherduck_session_exercised}" == "false" ]]
[[ "${kubernetes_traffic_exercised}" == "false" ]]
[[ "${evidence_boundary}" == "local-datafusion-parquet-file-only" ]]

printf 'parquet_lakehouse_read_live=passed\n'
printf 'l3_local_parquet_file_created=%s\n' "${local_parquet_file_created}"
printf 'l3_datafusion_parquet_read_executed=%s\n' "${datafusion_parquet_read_executed}"
printf 'l3_source_rows=%s\n' "${source_rows}"
printf 'l3_source_total=%s\n' "${source_total}"
printf 'l3_datafusion_output_rows=%s\n' "${datafusion_output_rows}"
printf 'l3_datafusion_output_total=%s\n' "${datafusion_output_total}"
printf 'object_store_io_attempted=%s\n' "${object_store_io_attempted}"
printf 'iceberg_runtime_exercised=%s\n' "${iceberg_runtime_exercised}"
printf 'delta_runtime_exercised=%s\n' "${delta_runtime_exercised}"
printf 'pg_lake_runtime_exercised=%s\n' "${pg_lake_runtime_exercised}"
printf 'motherduck_session_exercised=%s\n' "${motherduck_session_exercised}"
printf 'kubernetes_traffic_exercised=%s\n' "${kubernetes_traffic_exercised}"
printf 'sidecar_analytical_parquet_read_smoke\tfeature_id=%s\trows=%s\ttotal=%s\tevidence_boundary=%s\n' "${feature_id}" "${datafusion_output_rows}" "${datafusion_output_total}" "${evidence_boundary}"
