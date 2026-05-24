#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

output="$(cargo run -q -p ai_blaise_citus_sidecar_analytical -- run-runtime-canonical)"
header="$(printf '%s\n' "${output}" | sed -n '1p')"
row="$(printf '%s\n' "${output}" | sed -n '2p')"

expected_header=$'mirror\tengine\ttable\tformat\tobject_uri\tpushdown_plan\tprojected_columns\tpredicates\tpushed_down\tlimit\testimated_rows\tsnapshot_id\tfederated_catalogs\tfederation_targets\tduckdb_extensions\tmotherduck\tmirrored_cdc_events\tlakehouse_reads\tpushed_down_plans\tsnapshot_commits\tfederated_catalog_publications\tduckdb_extension_loads\tmotherduck_sessions\tallowed_engines\tallowed_object_uri_schemes\tmax_pushdown_limit\texternal_io_enabled\texternal_io_attempted\tquery_engine_executed\tevidence_boundary'
if [[ "${header}" != "${expected_header}" ]]; then
  echo "unexpected analytical runtime header" >&2
  printf '%s\n' "${header}" >&2
  exit 1
fi

IFS=$'\t' read -r mirror engine table format object_uri pushdown_plan projected_columns predicates pushed_down limit estimated_rows snapshot_id federated_catalogs federation_targets duckdb_extensions motherduck mirrored_cdc_events lakehouse_reads pushed_down_plans snapshot_commits federated_catalog_publications duckdb_extension_loads motherduck_sessions allowed_engines allowed_object_uri_schemes max_pushdown_limit external_io_enabled external_io_attempted query_engine_executed evidence_boundary <<<"${row}"

[[ "${mirror}" == "orders_mirror" ]]
[[ "${engine}" == "datafusion" ]]
[[ "${table}" == "public.orders" ]]
[[ "${format}" == "iceberg" ]]
[[ "${object_uri}" == "s3://lake/warehouse/orders" ]]
[[ "${pushdown_plan}" == "orders-scan" ]]
[[ "${projected_columns}" == "tenant_id,total" ]]
[[ "${predicates}" == "total > 0" ]]
[[ "${pushed_down}" == "true" ]]
[[ "${limit}" == "10000" ]]
[[ "${estimated_rows}" == "9750" ]]
[[ "${snapshot_id}" == "snapshot-1" ]]
[[ "${federated_catalogs}" == "databricks" ]]
[[ "${federation_targets}" == "databricks" ]]
[[ "${duckdb_extensions}" == "httpfs,iceberg" ]]
[[ "${motherduck}" == "analytics" ]]
[[ "${mirrored_cdc_events}" == "30" ]]
[[ "${lakehouse_reads}" == "1" ]]
[[ "${pushed_down_plans}" == "1" ]]
[[ "${snapshot_commits}" == "1" ]]
[[ "${federated_catalog_publications}" == "1" ]]
[[ "${duckdb_extension_loads}" == "2" ]]
[[ "${motherduck_sessions}" == "1" ]]
[[ "${allowed_engines}" == "datafusion" ]]
[[ "${allowed_object_uri_schemes}" == "s3" ]]
[[ "${max_pushdown_limit}" == "50000" ]]
[[ "${external_io_enabled}" == "false" ]]
[[ "${external_io_attempted}" == "false" ]]
[[ "${query_engine_executed}" == "false" ]]
[[ "${evidence_boundary}" == "deterministic-runtime-report-only" ]]

printf 'sidecar_analytical_smoke\tengine=%s\texternal_io_attempted=%s\tquery_engine_executed=%s\tevidence_boundary=%s\n' "${engine}" "${external_io_attempted}" "${query_engine_executed}" "${evidence_boundary}"
