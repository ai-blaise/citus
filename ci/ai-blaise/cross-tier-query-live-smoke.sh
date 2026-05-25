#!/usr/bin/env bash
set -euo pipefail

# FEATURE: L10
# Live bounded Citus cross-tier query proof. This smoke creates one hot row
# distributed table plus warm and cold distributed columnar tables, executes the
# companion-rendered cross-tier SQL over all three, and verifies a real Citus
# adaptive plan with ColumnarScan. It does not claim automatic workload routing,
# automatic query rewrites, cost-model tier selection, object-store cold reads,
# or Kubernetes traffic.
# The extension helper emits CREATE EXTENSION IF NOT EXISTS citus_columnar.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

if [[ "${REQUIRE_DOCKER:-0}" != "1" ]]; then
  echo "REQUIRE_DOCKER=1 is required for cross-tier query live smoke" >&2
  exit 2
fi

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "$1 is required for cross-tier query live smoke" >&2
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

network="ai-blaise-cross-tier-query-${$}-${RANDOM}"
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
  for _ in $(seq 1 8); do
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
    sleep 1
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
DROP TABLE IF EXISTS public.l10_hot_orders CASCADE;
DROP TABLE IF EXISTS public.l10_warm_orders CASCADE;
DROP TABLE IF EXISTS public.l10_cold_orders CASCADE;
CREATE TABLE public.l10_hot_orders (
  tenant_id integer NOT NULL,
  order_id integer NOT NULL,
  total numeric NOT NULL,
  PRIMARY KEY (tenant_id, order_id)
);
CREATE TABLE public.l10_warm_orders (
  tenant_id integer NOT NULL,
  order_id integer NOT NULL,
  total numeric NOT NULL
) USING columnar;
CREATE TABLE public.l10_cold_orders (
  tenant_id integer NOT NULL,
  order_id integer NOT NULL,
  total numeric NOT NULL
) USING columnar;
SELECT create_distributed_table('public.l10_hot_orders', 'tenant_id', shard_count => 4);
SELECT create_distributed_table('public.l10_warm_orders', 'tenant_id', shard_count => 4);
SELECT create_distributed_table('public.l10_cold_orders', 'tenant_id', shard_count => 4);
INSERT INTO public.l10_hot_orders
SELECT tenant_id, order_id, (tenant_id * 10 + order_id)::numeric
FROM generate_series(1, 2) AS tenant_id
CROSS JOIN generate_series(1, 2) AS order_id;
INSERT INTO public.l10_warm_orders
SELECT tenant_id, order_id, (tenant_id * 100 + order_id)::numeric
FROM generate_series(1, 2) AS tenant_id
CROSS JOIN generate_series(1, 2) AS order_id;
INSERT INTO public.l10_cold_orders
SELECT tenant_id, order_id, (tenant_id * 1000 + order_id)::numeric
FROM generate_series(1, 2) AS tenant_id
CROSS JOIN generate_series(1, 2) AS order_id;
SQL

cross_tier_sql="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-cross-tier-query-sql-canonical)"
cross_tier_output="$(docker exec -i "${coordinator}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres -X -q -AtF $'\t' <<SQL
${cross_tier_sql}
SQL
)"

explain_output="$(docker exec -i "${coordinator}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres -X -q -At <<'SQL'
EXPLAIN (COSTS OFF)
WITH cross_tier_orders AS (
  SELECT 'hot'::text AS tier, tenant_id, order_id, total FROM public.l10_hot_orders
  UNION ALL
  SELECT 'warm'::text AS tier, tenant_id, order_id, total FROM public.l10_warm_orders
  UNION ALL
  SELECT 'cold'::text AS tier, tenant_id, order_id, total FROM public.l10_cold_orders
)
SELECT tier, count(*)::int, sum(total)::int FROM cross_tier_orders GROUP BY tier ORDER BY tier;
SQL
)"

cross_tier_file="$(mktemp)"
explain_file="$(mktemp)"
printf '%s\n' "${cross_tier_output}" > "${cross_tier_file}"
printf '%s\n' "${explain_output}" > "${explain_file}"
python3 - "${cross_tier_file}" "${explain_file}" <<'PYCHECK'
from pathlib import Path
import sys

lines = [line for line in Path(sys.argv[1]).read_text(encoding="utf-8").splitlines() if line]
explain = Path(sys.argv[2]).read_text(encoding="utf-8")
values = {}
details = {}
for line in lines:
    parts = line.split("\t")
    if len(parts) < 2:
        raise SystemExit(f"malformed cross-tier marker: {line!r}")
    values[parts[0]] = parts[1]
    details[parts[0]] = parts[2] if len(parts) > 2 else ""
required_true = [
    "l10_hot_tier_row_table",
    "l10_warm_tier_columnar_table",
    "l10_cold_tier_columnar_table",
    "l10_tiers_distributed",
    "l10_cross_tier_union_executed",
    "l10_tier_rollups_preserved",
    "l10_companion_rendered_query_executed",
    "l10_explain_plan_required",
]
missing = [key for key in required_true if key not in values]
if missing:
    raise SystemExit(f"missing L10 markers {missing}: {values}")
for key in required_true:
    if values[key] != "true":
        raise SystemExit(f"{key} expected true, got {values[key]!r} detail={details.get(key, '')!r}")
if values.get("l10_cross_tier_query_feature_id") != "L10":
    raise SystemExit(f"missing L10 feature marker: {values}")
for key in (
    "automatic_workload_routing_exercised",
    "automatic_query_rewrite_exercised",
    "cost_model_selection_exercised",
    "object_store_cold_read_exercised",
    "kubernetes_traffic_exercised",
):
    if values.get(key) != "false":
        raise SystemExit(f"{key} expected false, got {values.get(key)!r}")
if "rows=12 total=6678" not in details.get("l10_cross_tier_union_executed", ""):
    raise SystemExit(f"combined rows/total mismatch: {details}")
rollups = details.get("l10_tier_rollups_preserved", "")
for expected in ("cold:4:6006", "hot:4:66", "warm:4:606"):
    if expected not in rollups:
        raise SystemExit(f"missing tier rollup {expected}: {rollups}")
if "Custom Scan (Citus" not in explain:
    raise SystemExit(f"Citus custom scan missing from EXPLAIN: {explain}")
if explain.count("ColumnarScan") < 2:
    raise SystemExit(f"expected warm and cold ColumnarScan entries in EXPLAIN: {explain}")
PYCHECK
rm -f "${cross_tier_file}" "${explain_file}"

printf 'cross_tier_query_live=passed\n'
printf 'l10_hot_tier_rows=4\n'
printf 'l10_warm_tier_rows=4\n'
printf 'l10_cold_tier_rows=4\n'
printf 'l10_cross_tier_rows=12\n'
printf 'l10_cross_tier_total=6678\n'
printf 'l10_citus_custom_scan_executed=true\n'
printf 'l10_columnar_scan_executed=true\n'
printf 'automatic_workload_routing_exercised=false\n'
printf 'automatic_query_rewrite_exercised=false\n'
printf 'cost_model_selection_exercised=false\n'
printf 'object_store_cold_read_exercised=false\n'
printf 'kubernetes_traffic_exercised=false\n'
printf 'cross_tier_query\tfeature_id=L10\trows=12\ttotal=6678\tcolumnar_scans=2\n'
