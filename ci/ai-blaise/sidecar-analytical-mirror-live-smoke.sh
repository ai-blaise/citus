#!/usr/bin/env bash
set -euo pipefail

if [[ "${REQUIRE_DOCKER:-0}" != "1" ]]; then
  echo "sidecar analytical mirror live smoke requires REQUIRE_DOCKER=1" >&2
  exit 1
fi
if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required for sidecar analytical mirror live smoke" >&2
  exit 1
fi

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

tmp_dir="$(mktemp -d)"
container="ai-blaise-l8-mirror-$RANDOM-$$"
cleanup() {
  docker rm -f "${container}" >/dev/null 2>&1 || true
  rm -rf "${tmp_dir}"
}
trap cleanup EXIT

docker run -d \
  --name "${container}" \
  -e POSTGRES_PASSWORD=postgres \
  postgres:17-bookworm \
  -c wal_level=logical \
  -c max_replication_slots=4 \
  -c max_wal_senders=4 \
  >"${tmp_dir}/container-id"

for _ in $(seq 1 90); do
  if docker exec "${container}" pg_isready -U postgres -d postgres >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
if ! docker exec "${container}" pg_isready -U postgres -d postgres >/dev/null 2>&1; then
  echo "postgres logical-decoding container did not become ready" >&2
  docker logs "${container}" >&2 || true
  exit 1
fi

docker exec -i "${container}" psql -X -q -v ON_ERROR_STOP=1 -U postgres -d postgres >/dev/null <<'SQL'
CREATE TABLE public.l8_orders (
  tenant_id integer NOT NULL,
  order_id integer NOT NULL,
  total bigint NOT NULL
);
SELECT * FROM pg_create_logical_replication_slot('l8_slot', 'test_decoding');
INSERT INTO public.l8_orders (tenant_id, order_id, total) VALUES
  (1, 1, 1000),
  (2, 2, 2000),
  (3, 3, 3000);
SQL

decoded_file="${tmp_dir}/decoded.txt"
docker exec -i "${container}" psql -X -qAt -v ON_ERROR_STOP=1 -U postgres -d postgres \
  -c "SELECT data FROM pg_logical_slot_get_changes('l8_slot', NULL, NULL);" \
  >"${decoded_file}"

insert_lines="$(grep -c 'table public.l8_orders: INSERT:' "${decoded_file}" || true)"
if [[ "${insert_lines}" != "3" ]]; then
  echo "expected 3 logical-decoding insert lines, saw ${insert_lines}" >&2
  cat "${decoded_file}" >&2
  exit 1
fi

artifact_path="${tmp_dir}/l8-mirror.tsv"
mirror_output="$(
  AI_BLAISE_ANALYTICAL_MIRROR_ARTIFACT="${artifact_path}" \
    cargo run -q -p ai_blaise_citus_sidecar_analytical -- run-logical-mirror-materialization-from-stdin \
    <"${decoded_file}"
)"
header="$(printf '%s\n' "${mirror_output}" | sed -n '1p')"
row="$(printf '%s\n' "${mirror_output}" | sed -n '2p')"
expected_header=$'feature_id\tmirror\tsource_table\tsource_plugin\tdecoded_change_lines\tmaterialized_rows\tmaterialized_total\tartifact_path\tartifact_bytes\tdatafusion_query_executed\tdatafusion_output_rows\tdatafusion_output_total\tlocal_mirror_artifact_created\tobject_store_io_attempted\tlong_running_slot_tailing\tcheckpoint_persistence_exercised\tkubernetes_traffic_exercised'
if [[ "${header}" != "${expected_header}" ]]; then
  echo "unexpected logical mirror materialization header" >&2
  printf '%s\n' "${header}" >&2
  exit 1
fi

IFS=$'\t' read -r feature_id mirror source_table source_plugin decoded_change_lines materialized_rows materialized_total reported_artifact_path artifact_bytes datafusion_query_executed datafusion_output_rows datafusion_output_total local_mirror_artifact_created object_store_io_attempted long_running_slot_tailing checkpoint_persistence_exercised kubernetes_traffic_exercised <<<"${row}"

[[ "${feature_id}" == "L8" ]]
[[ "${mirror}" == "orders_mirror" ]]
[[ "${source_table}" == "public.l8_orders" ]]
[[ "${source_plugin}" == "test_decoding" ]]
[[ "${decoded_change_lines}" == "3" ]]
[[ "${materialized_rows}" == "3" ]]
[[ "${materialized_total}" == "6000" ]]
[[ "${reported_artifact_path}" == "${artifact_path}" ]]
[[ "${artifact_bytes}" =~ ^[0-9]+$ ]]
(( artifact_bytes > 0 ))
[[ "${datafusion_query_executed}" == "true" ]]
[[ "${datafusion_output_rows}" == "3" ]]
[[ "${datafusion_output_total}" == "6000" ]]
[[ "${local_mirror_artifact_created}" == "true" ]]
[[ "${object_store_io_attempted}" == "false" ]]
[[ "${long_running_slot_tailing}" == "false" ]]
[[ "${checkpoint_persistence_exercised}" == "false" ]]
[[ "${kubernetes_traffic_exercised}" == "false" ]]
[[ -s "${artifact_path}" ]]
[[ "$(wc -l <"${artifact_path}" | tr -d ' ')" == "4" ]]

printf 'sidecar_analytical_mirror_live_smoke\tfeature_id=%s\tsource_plugin=%s\tdecoded_change_lines=%s\tmaterialized_rows=%s\tmaterialized_total=%s\tdatafusion_query_executed=%s\tartifact_bytes=%s\n' "${feature_id}" "${source_plugin}" "${decoded_change_lines}" "${materialized_rows}" "${materialized_total}" "${datafusion_query_executed}" "${artifact_bytes}"
printf 'logical_mirror_live=passed\n'
printf 'l8_test_decoding_slot_consumed=true\n'
printf 'l8_local_mirror_artifact_created=true\n'
printf 'l8_materialized_rows=%s\n' "${materialized_rows}"
printf 'l8_materialized_total=%s\n' "${materialized_total}"
printf 'l8_datafusion_mirror_query_executed=%s\n' "${datafusion_query_executed}"
printf 'object_store_io_attempted=%s\n' "${object_store_io_attempted}"
printf 'long_running_slot_tailing=%s\n' "${long_running_slot_tailing}"
printf 'checkpoint_persistence_exercised=%s\n' "${checkpoint_persistence_exercised}"
printf 'kubernetes_traffic_exercised=%s\n' "${kubernetes_traffic_exercised}"
