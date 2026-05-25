#!/usr/bin/env bash
set -euo pipefail

server_pid=""
tmp_dir="$(mktemp -d)"
cleanup() {
  if [[ -n "${server_pid}" ]]; then
    kill "${server_pid}" >/dev/null 2>&1 || true
    wait "${server_pid}" >/dev/null 2>&1 || true
  fi
  rm -rf "${tmp_dir}"
}
trap cleanup EXIT

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

output="$(cargo run -q -p ai_blaise_citus_sidecar_analytical -- run-runtime-canonical)"
header="$(printf '%s\n' "${output}" | sed -n '1p')"
row="$(printf '%s\n' "${output}" | sed -n '2p')"

expected_header=$'mirror\tengine\ttable\tformat\tobject_uri\tpushdown_plan\tprojected_columns\tpredicates\tpushed_down\tlimit\testimated_rows\tsnapshot_id\tfederated_catalogs\tfederation_targets\tduckdb_extensions\tmotherduck\tmirrored_cdc_events\tlakehouse_reads\tpushed_down_plans\tsnapshot_commits\tfederated_catalog_publications\tduckdb_extension_loads\tmotherduck_sessions\tquery_engine_executions\tquery_engine_output_rows\tdatafusion_output_rows\tdatafusion_output_total\tprojection_pushdown_executed\tfilter_pushdown_executed\tlimit_pushdown_executed\tallowed_engines\tallowed_object_uri_schemes\tmax_pushdown_limit\texternal_io_enabled\texternal_io_attempted\tquery_engine_executed\tevidence_boundary'
if [[ "${header}" != "${expected_header}" ]]; then
  echo "unexpected analytical runtime header" >&2
  printf '%s\n' "${header}" >&2
  exit 1
fi

IFS=$'\t' read -r mirror engine table format object_uri pushdown_plan projected_columns predicates pushed_down limit estimated_rows snapshot_id federated_catalogs federation_targets duckdb_extensions motherduck mirrored_cdc_events lakehouse_reads pushed_down_plans snapshot_commits federated_catalog_publications duckdb_extension_loads motherduck_sessions query_engine_executions query_engine_output_rows datafusion_output_rows datafusion_output_total projection_pushdown_executed filter_pushdown_executed limit_pushdown_executed allowed_engines allowed_object_uri_schemes max_pushdown_limit external_io_enabled external_io_attempted query_engine_executed evidence_boundary <<<"${row}"

[[ "${mirror}" == "orders_mirror" ]]
[[ "${engine}" == "datafusion" ]]
[[ "${table}" == "public.orders" ]]
[[ "${format}" == "iceberg" ]]
[[ "${object_uri}" == "s3://lake/warehouse/orders" ]]
[[ "${pushdown_plan}" == "orders-scan" ]]
[[ "${projected_columns}" == "tenant_id,total" ]]
[[ "${predicates}" == "total > 0" ]]
[[ "${pushed_down}" == "true" ]]
[[ "${limit}" == "2" ]]
[[ "${estimated_rows}" == "1" ]]
[[ "${snapshot_id}" == "snapshot-1" ]]
[[ "${federated_catalogs}" == "databricks" ]]
[[ "${federation_targets}" == "databricks" ]]
[[ "${duckdb_extensions}" == "httpfs,iceberg" ]]
[[ "${motherduck}" == "analytics" ]]
[[ "${mirrored_cdc_events}" == "3" ]]
[[ "${lakehouse_reads}" == "1" ]]
[[ "${pushed_down_plans}" == "1" ]]
[[ "${snapshot_commits}" == "1" ]]
[[ "${federated_catalog_publications}" == "1" ]]
[[ "${duckdb_extension_loads}" == "2" ]]
[[ "${motherduck_sessions}" == "1" ]]
[[ "${query_engine_executions}" == "1" ]]
[[ "${query_engine_output_rows}" == "2" ]]
[[ "${datafusion_output_rows}" == "2" ]]
[[ "${datafusion_output_total}" == "3000" ]]
[[ "${projection_pushdown_executed}" == "true" ]]
[[ "${filter_pushdown_executed}" == "true" ]]
[[ "${limit_pushdown_executed}" == "true" ]]
[[ "${allowed_engines}" == "datafusion" ]]
[[ "${allowed_object_uri_schemes}" == "s3" ]]
[[ "${max_pushdown_limit}" == "50000" ]]
[[ "${external_io_enabled}" == "false" ]]
[[ "${external_io_attempted}" == "false" ]]
[[ "${query_engine_executed}" == "true" ]]
[[ "${evidence_boundary}" == "local-datafusion-recordbatch-only" ]]

addr="127.0.0.1:$((19000 + ($$ % 1000)))"
AI_BLAISE_LISTEN_ADDR="${addr}" cargo run -q -p ai_blaise_citus_sidecar_analytical -- serve \
  >"${tmp_dir}/server.stdout" \
  2>"${tmp_dir}/server.stderr" &
server_pid="$!"

for _ in $(seq 1 60); do
  if curl -fsS --max-time 1 "http://${addr}/healthz" >"${tmp_dir}/healthz.json" 2>/dev/null; then
    break
  fi
  if ! kill -0 "${server_pid}" >/dev/null 2>&1; then
    echo "analytical probe server exited before healthz became ready" >&2
    cat "${tmp_dir}/server.stderr" >&2 || true
    exit 1
  fi
  sleep 1
done

if [[ ! -s "${tmp_dir}/healthz.json" ]]; then
  echo "analytical probe server did not answer healthz" >&2
  cat "${tmp_dir}/server.stderr" >&2 || true
  exit 1
fi

healthz="$(cat "${tmp_dir}/healthz.json")"
readyz="$(curl -fsS --max-time 2 "http://${addr}/readyz")"
metrics="$(curl -fsS --max-time 2 "http://${addr}/metrics")"
drain="$(curl -fsS --max-time 2 -X POST "http://${addr}/drain")"
readyz_after_drain_status="$(curl -sS --max-time 2 -o "${tmp_dir}/readyz-after-drain.json" -w '%{http_code}' "http://${addr}/readyz")"

[[ "${healthz}" == *'"component":"analytical"'* ]]
[[ "${healthz}" == *'"state":"ready"'* ]]
[[ "${readyz}" == *'"ready":true'* ]]
[[ "${metrics}" == *'ai_blaise_sidecar_ready{component="analytical"} 1'* ]]
[[ "${drain}" == *'"accepting_new_work":false'* ]]
[[ "${drain}" == *'"drained":true'* ]]
[[ "${readyz_after_drain_status}" == "503" ]]

printf 'sidecar_analytical_smoke\tengine=%s\texternal_io_attempted=%s\tquery_engine_executed=%s\tdatafusion_output_rows=%s\tprojection_pushdown_executed=%s\tfilter_pushdown_executed=%s\tlimit_pushdown_executed=%s\tevidence_boundary=%s\n' "${engine}" "${external_io_attempted}" "${query_engine_executed}" "${datafusion_output_rows}" "${projection_pushdown_executed}" "${filter_pushdown_executed}" "${limit_pushdown_executed}" "${evidence_boundary}"
printf 'sidecar_analytical_probe_smoke\taddr=%s\thealthz=200\treadyz_after_drain=%s\n' "${addr}" "${readyz_after_drain_status}"
