#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if [[ "${REQUIRE_DOCKER:-0}" != "1" ]]; then
  echo "REQUIRE_DOCKER=1 is required for transaction state live Citus smoke" >&2
  exit 2
fi

image="${AI_BLAISE_CITUS_COHAB_IMAGE:-ai-blaise-citus-timescale-cohabitation:local}"
if ! docker image inspect "${image}" >/dev/null 2>&1; then
  echo "missing Citus cohabitation image: ${image}" >&2
  exit 1
fi

container="ai-blaise-t13-t14-txn-${$}-${RANDOM}"
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

docker exec -i "${container}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres <<'SQL'
CREATE EXTENSION IF NOT EXISTS citus;
DROP TABLE IF EXISTS public.txn_state_orders CASCADE;
CREATE TABLE public.txn_state_orders (
  tenant_id integer NOT NULL,
  order_id integer NOT NULL,
  total numeric NOT NULL,
  PRIMARY KEY (tenant_id, order_id)
);
SELECT create_distributed_table('public.txn_state_orders', 'tenant_id');
INSERT INTO public.txn_state_orders
SELECT 1, generated_order, generated_order::numeric
FROM generate_series(1, 5) AS generated_order;
SQL

txn_sql="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-transaction-state-sql-canonical)"
txn_output="$(docker exec -i "${container}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres -X -q -AtF $'\t' <<SQL
${txn_sql}
SQL
)"

txn_file="$(mktemp)"
printf '%s\n' "${txn_output}" > "${txn_file}"
python3 - "${txn_file}" <<'PY'
from pathlib import Path
import sys

text = Path(sys.argv[1]).read_text(encoding="utf-8")
lines = [line for line in text.splitlines() if line]
cursor_rows = [line.split("\t") for line in lines if line.startswith("cursor_row\t")]
if [row[1] for row in cursor_rows] != ["1", "2", "3", "4", "5"]:
    raise SystemExit(f"cursor rows did not fetch complete ordered result: {cursor_rows!r}")
values = {}
for line in lines:
    parts = line.split("\t")
    if len(parts) >= 2:
        values.setdefault(parts[0], parts[1])
expected = {
    "count_after_insert": "6",
    "count_after_rollback": "5",
    "final_count": "5",
}
for key, value in expected.items():
    if values.get(key) != value:
        raise SystemExit(f"{key} expected {value}, got {values.get(key)}; output={lines!r}")
if "Custom Scan (Citus Adaptive)" not in text:
    raise SystemExit("Citus adaptive custom scan not present in EXPLAIN output")
if "Task Count: 1" not in text:
    raise SystemExit("Citus task count not present in EXPLAIN output")
PY
rm -f "${txn_file}"

printf 'transaction_state_live=passed\n'
printf 'distributed_cursor_declared=true\n'
printf 'cursor_fetch_batches=2\n'
printf 'cursor_rows_fetched=5\n'
printf 'savepoint_rollback_verified=true\n'
printf 'count_after_insert=6\n'
printf 'count_after_rollback=5\n'
printf 'final_count=5\n'
printf 'citus_adaptive_plan_observed=true\n'
printf 'citus_task_count_observed=1\n'
printf 'coordinator_failover_exercised=false\n'
printf 'multi_worker_cleanup_exercised=false\n'
printf 'wire_protocol_portal_exercised=false\n'
printf 'transaction_state_live\tpassed\n'
