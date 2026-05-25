#!/usr/bin/env bash
set -euo pipefail

# FEATURE: L7
# FEATURE: R3
# FEATURE: R8
# Live bounded Citus columnar proof. This smoke creates a real Citus coordinator
# and worker, installs citus_columnar, creates a distributed USING columnar
# table, verifies a Citus adaptive plan with ColumnarScan, and verifies the
# worker sees the same columnar table and rows. It does not claim cost-model
# tier selection, automatic tier movement, workload-routing rewrites, or
# Kubernetes traffic.
# The extension helper emits CREATE EXTENSION IF NOT EXISTS citus_columnar.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

if [[ "${REQUIRE_DOCKER:-0}" != "1" ]]; then
  echo "REQUIRE_DOCKER=1 is required for columnar tiering live smoke" >&2
  exit 2
fi

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "$1 is required for columnar tiering live smoke" >&2
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

network="ai-blaise-columnar-tiering-${$}-${RANDOM}"
coordinator="${network}-coord"
worker="${network}-worker"
cleanup() {
  docker rm -f "${coordinator}" "${worker}" >/dev/null 2>&1 || true
  docker network rm "${network}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

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
    -c citus.cohabit_extensions=timescaledb >/dev/null
}

wait_for_postgres_ready() {
  local pg_container="$1"
  for _ in $(seq 1 120); do
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

ensure_extension() {
  local pg_container="$1"
  local extension="$2"
  local output=""
  local code=0
  for _ in $(seq 1 6); do
    set +e
    output="$(docker exec "${pg_container}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres -q -c "CREATE EXTENSION IF NOT EXISTS ${extension};" 2>&1)"
    code=$?
    set -e
    wait_for_postgres_ready "${pg_container}"
    if docker exec "${pg_container}" psql -U postgres -d postgres -Atqc "SELECT 1 FROM pg_extension WHERE extname='${extension}'" | grep -qx '1'; then
      return 0
    fi
    if [[ "${code}" -ne 0 ]] && ! grep -Eq 'server closed the connection|connection to server was lost|database system is shutting down|citus_setup_ssl' <<<"${output}"; then
      printf '%s\n' "${output}" >&2
      exit 1
    fi
  done
  printf 'extension %s was not installed in %s\n' "${extension}" "${pg_container}" >&2
  printf '%s\n' "${output}" >&2
  exit 1
}

run_citus_node "${coordinator}"
run_citus_node "${worker}"
wait_for_postgres_ready "${coordinator}"
wait_for_postgres_ready "${worker}"
for pg_container in "${coordinator}" "${worker}"; do
  ensure_extension "${pg_container}" citus
  ensure_extension "${pg_container}" citus_columnar
done

docker exec -i "${coordinator}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres -X -q <<SQL
SELECT citus_add_node('${worker}', 5432);
DROP TABLE IF EXISTS public.hot_orders CASCADE;
DROP TABLE IF EXISTS public.columnar_orders CASCADE;
CREATE TABLE public.hot_orders (
  tenant_id integer NOT NULL,
  order_id integer NOT NULL,
  total numeric NOT NULL,
  PRIMARY KEY (tenant_id, order_id)
);
INSERT INTO public.hot_orders
SELECT tenant_id, order_id, (tenant_id * 100 + order_id)::numeric
FROM generate_series(1, 4) AS tenant_id
CROSS JOIN generate_series(1, 3) AS order_id;
CREATE TABLE public.columnar_orders (
  tenant_id integer NOT NULL,
  order_id integer NOT NULL,
  total numeric NOT NULL
) USING columnar;
SELECT create_distributed_table('public.columnar_orders', 'tenant_id', shard_count => 4);
INSERT INTO public.columnar_orders
SELECT tenant_id, order_id, (tenant_id * 100 + order_id)::numeric
FROM generate_series(1, 4) AS tenant_id
CROSS JOIN generate_series(1, 3) AS order_id;
SQL

columnar_sql="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-columnar-tiering-sql-canonical)"
columnar_output="$(docker exec -i "${coordinator}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres -X -q -AtF $'\t' <<SQL
${columnar_sql}
SQL
)"

explain_output="$(docker exec -i "${coordinator}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres -X -q -At <<'SQL'
EXPLAIN (COSTS OFF)
SELECT tenant_id, sum(total)
FROM public.columnar_orders
GROUP BY tenant_id
ORDER BY tenant_id;
SQL
)"

worker_output="$(docker exec -i "${worker}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres -X -q -AtF $'\t' <<'SQL'
SELECT 'r3_worker_columnar_access_method', (am.amname = 'columnar')::text, am.amname
FROM pg_class c
JOIN pg_am am ON am.oid = c.relam
WHERE c.oid = 'public.columnar_orders'::regclass;
SELECT 'r3_worker_columnar_rows_preserved', (count(*) = 12 AND COALESCE(sum(total), 0)::bigint = 3024)::text,
       format('rows=%s total=%s', count(*), COALESCE(sum(total), 0)::bigint)
FROM public.columnar_orders;
SQL
)"

columnar_file="$(mktemp)"
explain_file="$(mktemp)"
worker_file="$(mktemp)"
printf '%s\n' "${columnar_output}" > "${columnar_file}"
printf '%s\n' "${explain_output}" > "${explain_file}"
printf '%s\n' "${worker_output}" > "${worker_file}"
python3 - "${columnar_file}" "${explain_file}" "${worker_file}" <<'PYCHECK'
from pathlib import Path
import sys

columnar_lines = [line for line in Path(sys.argv[1]).read_text(encoding="utf-8").splitlines() if line]
explain = Path(sys.argv[2]).read_text(encoding="utf-8")
worker_lines = [line for line in Path(sys.argv[3]).read_text(encoding="utf-8").splitlines() if line]
values = {}
details = {}
for line in columnar_lines + worker_lines:
    parts = line.split("\t")
    if len(parts) < 2:
        raise SystemExit(f"malformed columnar marker: {line!r}")
    values[parts[0]] = parts[1]
    details[parts[0]] = parts[2] if len(parts) > 2 else ""
required_true = [
    "l7_columnar_access_method",
    "l7_distributed_columnar_table",
    "l7_columnar_query_rows_preserved",
    "r3_worker_columnstore_policy_declared",
    "r8_non_hypertable_cold_columnar_path",
    "l10_cross_tier_tables_declared",
    "columnar_conversion_executed",
    "r3_worker_columnar_access_method",
    "r3_worker_columnar_rows_preserved",
]
missing = [key for key in required_true if key not in values]
if missing:
    raise SystemExit(f"missing columnar markers {missing}: {values}")
for key in required_true:
    if values[key] != "true":
        raise SystemExit(f"{key} expected true, got {values[key]!r} detail={details.get(key, '')!r}")
for key in (
    "cost_model_selection_exercised",
    "automatic_tier_movement_executed",
    "workload_routing_exercised",
    "kubernetes_traffic_exercised",
):
    if values.get(key) != "false":
        raise SystemExit(f"{key} expected false, got {values.get(key)!r}")
if "Custom Scan (Citus" not in explain:
    raise SystemExit(f"Citus custom scan missing from EXPLAIN: {explain}")
if "ColumnarScan" not in explain:
    raise SystemExit(f"ColumnarScan missing from EXPLAIN: {explain}")
if "rows=12 total=3024" not in details.get("l7_columnar_query_rows_preserved", ""):
    raise SystemExit(f"coordinator row preservation detail mismatch: {details}")
if "rows=12 total=3024" not in details.get("r3_worker_columnar_rows_preserved", ""):
    raise SystemExit(f"worker row preservation detail mismatch: {details}")
PYCHECK
rm -f "${columnar_file}" "${explain_file}" "${worker_file}"

printf 'columnar_tiering_live=passed\n'
printf 'l7_distributed_columnar_table=true\n'
printf 'l7_columnar_access_method=true\n'
printf 'l7_columnar_query_rows=12\n'
printf 'l7_columnar_query_total=3024\n'
printf 'l7_citus_custom_scan_executed=true\n'
printf 'l7_columnar_scan_executed=true\n'
printf 'r3_worker_columnstore_policy_live=true\n'
printf 'r3_worker_access_method=columnar\n'
printf 'r8_non_hypertable_cold_columnar_path=true\n'
printf 'cost_model_selection_exercised=false\n'
printf 'automatic_tier_movement_executed=false\n'
printf 'workload_routing_exercised=false\n'
printf 'kubernetes_traffic_exercised=false\n'
printf 'columnar_tiering\tfeature_ids=L7,R3,R8\trows=12\ttotal=3024\tworker_columnar=true\n'
