#!/usr/bin/env bash
set -euo pipefail

# FEATURE: T10
# FEATURE: T11
# Live bounded bulk-fetch and DistSQL physical-pushdown proof. This smoke starts
# a real Citus PostgreSQL container, executes the companion-rendered SQL over a
# distributed table, and proves the 4096-row fetch budget plus Citus adaptive
# task-plan evidence. It does not claim a custom PostgreSQL wire-protocol
# implementation, backpressure scheduling, optimizer rewrite engine,
# multi-worker fanout, or Kubernetes traffic.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

if [[ "${REQUIRE_DOCKER:-0}" != "1" ]]; then
  echo "REQUIRE_DOCKER=1 is required for bulk/DistSQL live Citus smoke" >&2
  exit 2
fi

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "$1 is required for bulk/DistSQL live Citus smoke" >&2
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

container="ai-blaise-t10-t11-bulk-distsql-${$}-${RANDOM}"
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
DROP TABLE IF EXISTS public.bulk_distsql_orders CASCADE;
CREATE TABLE public.bulk_distsql_orders (
  tenant_id integer NOT NULL,
  order_id integer NOT NULL,
  total numeric NOT NULL,
  PRIMARY KEY (tenant_id, order_id)
);
SELECT create_distributed_table('public.bulk_distsql_orders', 'tenant_id');
INSERT INTO public.bulk_distsql_orders
SELECT 1, generated_order, generated_order::numeric
FROM generate_series(1, 4096) AS generated_order;
INSERT INTO public.bulk_distsql_orders
SELECT 2, generated_order, generated_order::numeric
FROM generate_series(1, 16) AS generated_order;
SQL

bulk_sql="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-bulk-distsql-sql-canonical)"
bulk_output="$(docker exec -i "${container}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres -X -q -AtF $'\t' <<SQL
${bulk_sql}
SQL
)"

bulk_file="$(mktemp)"
printf '%s\n' "${bulk_output}" > "${bulk_file}"
python3 - "${bulk_file}" <<'PY_CHECK'
from pathlib import Path
import re
import sys

text = Path(sys.argv[1]).read_text(encoding="utf-8")
lines = [line for line in text.splitlines() if line]
rows = [line.split("\t") for line in lines if line.startswith("bulk_fetch_row\t")]
if len(rows) != 4096:
    raise SystemExit(f"expected 4096 bulk fetch rows, got {len(rows)}")
if rows[0][1] != "1" or rows[-1][1] != "4096":
    raise SystemExit(f"bulk fetch rows not ordered from 1..4096: first={rows[0]!r} last={rows[-1]!r}")
values = {}
for line in lines:
    parts = line.split("\t")
    if len(parts) >= 2:
        values.setdefault(parts[0], parts[1])
if values.get("bulk_fetch_rows_returned") != "4096":
    raise SystemExit(f"bulk_fetch_rows_returned expected 4096, got {values.get('bulk_fetch_rows_returned')}")
if "Custom Scan (Citus Adaptive)" not in text:
    raise SystemExit("Citus adaptive custom scan not present in DistSQL EXPLAIN output")
match = re.search(r"Task Count: (\d+)", text)
if not match:
    raise SystemExit("Citus task count not present in DistSQL EXPLAIN output")
task_count = int(match.group(1))
if task_count < 1 or task_count > 16:
    raise SystemExit(f"Citus task count {task_count} outside worker task budget 1..16")
PY_CHECK
rm -f "${bulk_file}"

observed_task_count="$(printf '%s\n' "${bulk_output}" | sed -n 's/.*Task Count: \([0-9][0-9]*\).*/\1/p' | head -n 1)"

printf 'bulk_distsql_live=passed\n'
printf 'bulk_fetch_rows_requested=4096\n'
printf 'bulk_fetch_rows_returned=4096\n'
printf 'distsql_physical_pushdown_explain=true\n'
printf 'citus_adaptive_plan_observed=true\n'
printf 'citus_task_count_observed=%s\n' "${observed_task_count}"
printf 'worker_task_budget=16\n'
printf 'worker_task_budget_exceeded=false\n'
printf 'wire_protocol_implementation=false\n'
printf 'backpressure_scheduler_exercised=false\n'
printf 'physical_plan_rewrite_exercised=false\n'
printf 'multi_worker_fanout_exercised=false\n'
printf 'kubernetes_traffic_exercised=false\n'
printf 'bulk_distsql_live\tpassed\n'
