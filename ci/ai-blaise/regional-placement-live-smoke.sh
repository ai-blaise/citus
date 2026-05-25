#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if [[ "${REQUIRE_DOCKER:-0}" != "1" ]]; then
  echo "REQUIRE_DOCKER=1 is required for regional placement live Citus smoke" >&2
  exit 2
fi

image="${AI_BLAISE_CITUS_COHAB_IMAGE:-ai-blaise-citus-timescale-cohabitation:local}"
if ! docker image inspect "${image}" >/dev/null 2>&1; then
  echo "missing Citus cohabitation image: ${image}" >&2
  exit 1
fi

container="ai-blaise-s8-s12-regional-${$}-${RANDOM}"
cleanup() {
  docker rm -f "${container}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run -d --name "${container}" \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=postgres \
  "${image}" \
  -c shared_preload_libraries=timescaledb,citus \
  -c citus.cohabit_extensions=timescaledb >/dev/null

ready=0
for _ in $(seq 1 90); do
  if docker exec "${container}" pg_isready -U postgres >/dev/null 2>&1 \
    && docker exec "${container}" psql -U postgres -d postgres -Atqc "SELECT 1" >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 2
done
if [[ "${ready}" != "1" ]]; then
  echo "postgres did not become ready in ${container}" >&2
  docker logs "${container}" >&2 || true
  exit 1
fi

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
printf 'regional_placement_live\tpassed\n'
