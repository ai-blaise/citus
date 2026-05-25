#!/usr/bin/env bash
set -euo pipefail

# FEATURE: S1
# Live bounded Citus shard-split proof. This smoke starts a real Citus server,
# creates a distributed table with four initial shards, calls Citus'
# isolate_tenant_to_new_shard primitive through the companion-rendered SQL plan,
# and verifies shard-count growth, tenant row preservation, shard rerouting, and
# exact isolated hash range metadata. It does not claim an automated policy
# scheduler, threshold telemetry, rollback automation, multi-node movement, or
# Kubernetes traffic.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

if [[ "${REQUIRE_DOCKER:-0}" != "1" ]]; then
  echo "REQUIRE_DOCKER=1 is required for shard split live smoke" >&2
  exit 2
fi

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "$1 is required for shard split live smoke" >&2
    exit 1
  }
}

need_cmd cargo
need_cmd docker
need_cmd python3

image="${AI_BLAISE_CITUS_COHAB_IMAGE:-ai-blaise-citus-timescale-cohabitation:local}"
if ! docker image inspect "${image}" >/dev/null 2>&1; then
  echo "missing Citus/Timescale cohabitation image: ${image}" >&2
  exit 1
fi

container="ai-blaise-s1-shard-split-${$}-${RANDOM}"
cleanup() {
  docker rm -f "${container}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run -d --name "${container}" \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=postgres \
  "${image}" \
  -c shared_preload_libraries=timescaledb,citus \
  -c citus.cohabit_extensions=timescaledb \
  -c wal_level=logical >/dev/null

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
DROP TABLE IF EXISTS public.s1_orders CASCADE;
CREATE TABLE public.s1_orders(
  tenant_id integer NOT NULL,
  order_id integer NOT NULL,
  total numeric NOT NULL,
  PRIMARY KEY (tenant_id, order_id)
);
SELECT create_distributed_table('public.s1_orders', 'tenant_id', shard_count => 4);
INSERT INTO public.s1_orders
SELECT tenant_id, order_id, order_id::numeric
FROM generate_series(1, 8) AS tenant_id,
     generate_series(1, 10) AS order_id;
SQL

s1_sql="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-shard-split-sql-canonical)"
s1_output="$(docker exec -i "${container}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres -X -q -AtF $'\t' <<SQL
${s1_sql}
SQL
)"

s1_file="$(mktemp)"
printf '%s\n' "${s1_output}" > "${s1_file}"
python3 - "${s1_file}" <<'PY_CHECK'
from pathlib import Path
import sys

lines = [line for line in Path(sys.argv[1]).read_text(encoding="utf-8").splitlines() if line]
values = {}
details = {}
for line in lines:
    parts = line.split("\t")
    if len(parts) >= 2:
        values[parts[0]] = parts[1]
        details[parts[0]] = parts[2] if len(parts) > 2 else ""
required = [
    "split_wal_level",
    "split_shard_count_before",
    "split_tenant_rows_before",
    "split_tenant_shard_before",
    "split_new_shard_id",
    "split_shard_count_after",
    "split_tenant_rows_after",
    "split_tenant_shard_after",
    "split_tenant_shard_changed",
    "split_isolated_range_exact",
    "policy_scheduler_exercised",
    "threshold_telemetry_exercised",
    "rollback_automation_exercised",
    "multi_node_movement_exercised",
    "kubernetes_traffic_exercised",
]
missing = [marker for marker in required if marker not in values]
if missing:
    raise SystemExit(f"missing S1 shard split markers {missing}; output={lines!r}")
if values["split_wal_level"] != "logical":
    raise SystemExit(f"expected wal_level logical, got {values['split_wal_level']}")
if values["split_shard_count_before"] != "4":
    raise SystemExit(f"expected four initial shards, got {values['split_shard_count_before']}")
if values["split_tenant_rows_before"] != "10":
    raise SystemExit(f"expected 10 tenant rows before split, got {values['split_tenant_rows_before']}")
if values["split_tenant_rows_after"] != "10":
    raise SystemExit(f"expected 10 tenant rows after split, got {values['split_tenant_rows_after']}")
if int(values["split_shard_count_after"]) != 6:
    raise SystemExit(f"expected six shards after tenant isolation, got {values['split_shard_count_after']}")
if int(values["split_new_shard_id"]) <= 0:
    raise SystemExit(f"new shard id must be positive, got {values['split_new_shard_id']}")
if values["split_tenant_shard_after"] != values["split_new_shard_id"]:
    raise SystemExit(
        f"tenant did not route to new shard: after={values['split_tenant_shard_after']} new={values['split_new_shard_id']}"
    )
if values["split_tenant_shard_changed"] != "true":
    raise SystemExit("tenant shard id did not change after isolation")
if values["split_isolated_range_exact"] != "true":
    raise SystemExit(f"new shard range was not exact: {details.get('split_isolated_range_exact')}")
for marker in (
    "policy_scheduler_exercised",
    "threshold_telemetry_exercised",
    "rollback_automation_exercised",
    "multi_node_movement_exercised",
    "kubernetes_traffic_exercised",
):
    if values[marker] != "false":
        raise SystemExit(f"{marker} must remain false, got {values[marker]}")
PY_CHECK
rm -f "${s1_file}"

new_shard_id="$(printf '%s\n' "${s1_output}" | awk -F '\t' '$1 == "split_new_shard_id" {print $2; exit}')"
range_detail="$(printf '%s\n' "${s1_output}" | awk -F '\t' '$1 == "split_isolated_range_exact" {print $3; exit}')"

printf 'shard_split_live=passed\n'
printf 'isolate_tenant_to_new_shard_executed=true\n'
printf 'wal_level_logical_required=true\n'
printf 'split_tenant_id=4\n'
printf 'split_shard_count_before=4\n'
printf 'split_shard_count_after=6\n'
printf 'split_new_shard_id=%s\n' "${new_shard_id}"
printf 'split_new_shard_created=true\n'
printf 'split_tenant_rows_preserved=10\n'
printf 'split_tenant_shard_changed=true\n'
printf 'split_isolated_range_exact=true\n'
printf 'split_isolated_range_detail=%s\n' "${range_detail}"
printf 'policy_scheduler_exercised=false\n'
printf 'threshold_telemetry_exercised=false\n'
printf 'rollback_automation_exercised=false\n'
printf 'multi_node_movement_exercised=false\n'
printf 'kubernetes_traffic_exercised=false\n'
printf 'shard_split_live\tpassed\n'
