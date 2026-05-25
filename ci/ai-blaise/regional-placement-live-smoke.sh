#!/usr/bin/env bash
set -euo pipefail

# FEATURE: S8
# FEATURE: S12
# FEATURE: MR3
# Live bounded regional placement proof. The first phase proves the S8/S12
# locality-key and tablespace catalog guard against a real Citus/PostgreSQL
# server. The second phase starts a real multi-worker Citus deployment,
# isolates explicit regional locality keys into dedicated shards, moves those
# placements to region-named workers, and verifies rows plus catalog placement.
# It does not claim WAN/multi-region network execution, Kubernetes operator
# reconciliation, automatic repartition scheduling, regional traffic routing,
# or regional failover.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

if [[ "${REQUIRE_DOCKER:-0}" != "1" ]]; then
  echo "REQUIRE_DOCKER=1 is required for regional placement live Citus smoke" >&2
  exit 2
fi

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "$1 is required for regional placement live smoke" >&2
    exit 1
  }
}

need_cmd cargo
need_cmd docker
need_cmd python3

image="${AI_BLAISE_CITUS_COHAB_IMAGE:-ai-blaise-citus-timescale-cohabitation:local}"
if ! docker image inspect "${image}" >/dev/null 2>&1; then
  echo "missing Citus cohabitation image: ${image}" >&2
  exit 1
fi

container="ai-blaise-s8-s12-regional-${$}-${RANDOM}"
network=""
coordinator=""
us_worker=""
eu_worker=""
cleanup() {
  docker rm -f "${container}" >/dev/null 2>&1 || true
  if [[ -n "${coordinator}" || -n "${us_worker}" || -n "${eu_worker}" ]]; then
    docker rm -f "${coordinator}" "${us_worker}" "${eu_worker}" >/dev/null 2>&1 || true
  fi
  if [[ -n "${network}" ]]; then
    docker network rm "${network}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

wait_for_postgres_ready() {
  local pg_container="$1"
  for _ in $(seq 1 90); do
    if docker exec "${pg_container}" pg_isready -U postgres >/dev/null 2>&1 \
      && docker exec "${pg_container}" psql -U postgres -d postgres -Atqc "SELECT 1" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  docker logs "${pg_container}" >&2 || true
  echo "postgres did not become ready in ${pg_container}" >&2
  exit 1
}

ensure_citus_extension() {
  local pg_container="$1"
  local output=""
  if ! output="$(docker exec "${pg_container}" psql -U postgres -d postgres -v ON_ERROR_STOP=1 -q -c 'CREATE EXTENSION IF NOT EXISTS citus;' 2>&1)"; then
    if ! grep -Eq 'citus_setup_ssl|server closed the connection|connection to server was lost|database system is shutting down' <<<"${output}"; then
      printf '%s\n' "${output}" >&2
      exit 1
    fi
    wait_for_postgres_ready "${pg_container}"
    docker exec "${pg_container}" psql -U postgres -d postgres -v ON_ERROR_STOP=1 -q -c 'CREATE EXTENSION IF NOT EXISTS citus;' >/dev/null
  fi
}

docker run -d --name "${container}" \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=postgres \
  "${image}" \
  -c shared_preload_libraries=timescaledb,citus \
  -c citus.cohabit_extensions=timescaledb >/dev/null

wait_for_postgres_ready "${container}"
ensure_citus_extension "${container}"

docker exec -u root "${container}" mkdir -p \
  /var/lib/postgresql/tablespaces/us_east_1 \
  /var/lib/postgresql/tablespaces/eu_west_1
docker exec -u root "${container}" chown -R postgres:postgres /var/lib/postgresql/tablespaces

docker exec -i "${container}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres <<'SQL'
CREATE EXTENSION IF NOT EXISTS citus;
CREATE TABLESPACE ai_blaise_us_east_1 LOCATION '/var/lib/postgresql/tablespaces/us_east_1';
CREATE TABLESPACE ai_blaise_eu_west_1 LOCATION '/var/lib/postgresql/tablespaces/eu_west_1';
CREATE TABLE public.locality_orders (
  locality_key text NOT NULL,
  tenant_id text NOT NULL,
  order_id bigint NOT NULL,
  total numeric NOT NULL,
  PRIMARY KEY (locality_key, tenant_id, order_id)
) TABLESPACE ai_blaise_us_east_1;
CREATE TABLE public.locality_orders_eu (
  locality_key text NOT NULL,
  tenant_id text NOT NULL,
  order_id bigint NOT NULL,
  total numeric NOT NULL,
  PRIMARY KEY (locality_key, tenant_id, order_id)
) TABLESPACE ai_blaise_eu_west_1;
SELECT create_distributed_table('public.locality_orders', 'locality_key');
SELECT create_distributed_table('public.locality_orders_eu', 'locality_key');
SQL

regional_sql="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-regional-placement-sql-canonical)"
regional_output="$(docker exec -i "${container}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres -X -q -AtF $'\t' <<SQL
${regional_sql}
SQL
)"

regional_file="$(mktemp)"
printf '%s\n' "${regional_output}" > "${regional_file}"
python3 - "${regional_file}" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
lines = [line for line in path.read_text(encoding="utf-8").splitlines() if line]
if len(lines) != 1:
    raise SystemExit(f"expected one regional placement row, got {len(lines)}: {lines!r}")
row = lines[0].split("\t")
if len(row) != 9:
    raise SystemExit(f"expected 9 columns, got {len(row)}: {row!r}")
expected = {
    0: "S8,S12",
    1: "public.locality_orders",
    2: "t",
    3: "2",
    4: "t",
    5: "2",
    6: "t",
    7: "f",
    8: "f",
}
for index, value in expected.items():
    if row[index] != value:
        raise SystemExit(f"regional placement column {index} expected {value!r}, got {row[index]!r}: {row!r}")
PY
rm -f "${regional_file}"

printf 'regional_placement_live=passed\n'
printf 'locality_prefixed_pk_valid=true\n'
printf 'citus_distribution_present=true\n'
printf 'region_tablespace_mappings_valid=true\n'
printf 'region_tablespace_count=2\n'
printf 'automatic_rebalance_executed=false\n'
printf 'shard_movement_executed=false\n'
printf 'worker_placement_enforced=false\n'
printf 'multi_region_failover_exercised=false\n'

network="ai-blaise-mr3-row-placement-${$}-${RANDOM}"
coordinator="${network}-coord"
us_worker="${network}-us"
eu_worker="${network}-eu"
docker network create "${network}" >/dev/null

run_citus_node() {
  docker run -d \
    --name "$1" \
    --network "${network}" \
    -e POSTGRES_PASSWORD=postgres \
    -e POSTGRES_HOST_AUTH_METHOD=trust \
    -e POSTGRES_DB=postgres \
    "${image}" \
    -c shared_preload_libraries=timescaledb,citus \
    -c citus.cohabit_extensions=timescaledb \
    -c wal_level=logical >/dev/null
}

run_citus_node "${coordinator}"
run_citus_node "${us_worker}"
run_citus_node "${eu_worker}"
for pg_container in "${coordinator}" "${us_worker}" "${eu_worker}"; do
  wait_for_postgres_ready "${pg_container}"
  ensure_citus_extension "${pg_container}"
done

docker exec -i "${coordinator}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres -X -q <<SQL
SELECT citus_set_coordinator_host('${coordinator}', 5432);
SELECT citus_add_node('${us_worker}', 5432);
DROP TABLE IF EXISTS public.mr3_orders CASCADE;
CREATE TABLE public.mr3_orders(
  locality_key text NOT NULL,
  tenant_id text NOT NULL,
  order_id integer NOT NULL,
  total numeric NOT NULL,
  PRIMARY KEY (locality_key, tenant_id, order_id)
);
SELECT create_distributed_table('public.mr3_orders', 'locality_key', shard_count => 4);
INSERT INTO public.mr3_orders
SELECT 'us-east-1:tenant-a', 'tenant-a', order_id, order_id::numeric
FROM generate_series(1, 8) AS order_id;
INSERT INTO public.mr3_orders
SELECT 'eu-west-1:tenant-b', 'tenant-b', order_id, (order_id * 10)::numeric
FROM generate_series(1, 8) AS order_id;
SELECT citus_add_node('${eu_worker}', 5432);
SQL

mr3_sql="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-regional-row-placement-sql-canonical)"
mr3_output="$(docker exec -i "${coordinator}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres -X -q -AtF $'\t' \
  -v mr3_us_worker="${us_worker}" \
  -v mr3_eu_worker="${eu_worker}" <<SQL
${mr3_sql}
SQL
)"

mr3_file="$(mktemp)"
printf '%s\n' "${mr3_output}" > "${mr3_file}"
python3 - "${mr3_file}" <<'PY'
from pathlib import Path
import sys

values = {}
details = {}
for line in Path(sys.argv[1]).read_text(encoding="utf-8").splitlines():
    if not line:
        continue
    parts = line.split("\t")
    if len(parts) < 2:
        raise SystemExit(f"malformed MR3 row: {line!r}")
    values[parts[0]] = parts[1]
    details[parts[0]] = parts[2] if len(parts) > 2 else ""
required = {
    "mr3_feature_id": "MR3",
    "mr3_region_keys": "2",
    "mr3_shards_isolated": "true",
    "mr3_citus_move_shard_placement_executed": "true",
    "mr3_rows_preserved": "true",
    "mr3_worker_placement_enforced": "true",
    "mr3_matched_region_count": "2",
    "mr3_automatic_repartition_scheduler_exercised": "false",
    "mr3_kubernetes_operator_reconciliation_exercised": "false",
    "mr3_regional_traffic_router_exercised": "false",
    "mr3_multi_region_network_exercised": "false",
    "mr3_regional_failover_exercised": "false",
}
missing = [key for key in required if key not in values]
if missing:
    raise SystemExit(f"missing MR3 markers {missing}: {values}")
for key, expected in required.items():
    if values[key] != expected:
        raise SystemExit(f"{key} expected {expected}, got {values[key]} detail={details.get(key, '')}")
placement_detail = details["mr3_worker_placement_enforced"]
if "us-east-1:" not in placement_detail or "eu-west-1:" not in placement_detail:
    raise SystemExit(f"placement detail did not include both regions: {placement_detail}")
rows_detail = details["mr3_rows_preserved"]
if "us-east-1:8:36" not in rows_detail or "eu-west-1:8:360" not in rows_detail:
    raise SystemExit(f"row preservation detail did not include expected counts/sums: {rows_detail}")
move_detail = details["mr3_citus_move_shard_placement_executed"]
if "eu-west-1:" not in move_detail or "->" not in move_detail:
    raise SystemExit(f"move detail did not prove shard move: {move_detail}")
PY
rm -f "${mr3_file}"

matched_regions="$(printf '%s\n' "${mr3_output}" | awk -F '\t' '$1 == "mr3_matched_region_count" {print $2; exit}')"
placement_detail="$(printf '%s\n' "${mr3_output}" | awk -F '\t' '$1 == "mr3_worker_placement_enforced" {print $3; exit}')"
move_detail="$(printf '%s\n' "${mr3_output}" | awk -F '\t' '$1 == "mr3_citus_move_shard_placement_executed" {print $3; exit}')"

printf 'regional_row_placement_live=passed\n'
printf 'mr3_live_multi_worker_citus=true\n'
printf 'mr3_region_keys=2\n'
printf 'mr3_shards_isolated=true\n'
printf 'mr3_citus_move_shard_placement_executed=true\n'
printf 'mr3_worker_placement_enforced=true\n'
printf 'mr3_matched_region_count=%s\n' "${matched_regions}"
printf 'mr3_rows_preserved=true\n'
printf 'mr3_placement_detail=%s\n' "${placement_detail}"
printf 'mr3_move_detail=%s\n' "${move_detail}"
printf 'mr3_automatic_repartition_scheduler_exercised=false\n'
printf 'mr3_kubernetes_operator_reconciliation_exercised=false\n'
printf 'mr3_regional_traffic_router_exercised=false\n'
printf 'mr3_multi_region_network_exercised=false\n'
printf 'mr3_regional_failover_exercised=false\n'
printf 'regional_placement_live\tpassed\n'
