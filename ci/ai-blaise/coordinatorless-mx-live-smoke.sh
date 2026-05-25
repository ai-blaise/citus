#!/usr/bin/env bash
set -euo pipefail

# FEATURE: S4
# Live bounded coordinator-less topology proof. This smoke proves a real Citus
# MX worker entry point can serve a distributed-table query after metadata sync,
# and that the ai-blaise pool can use that worker as its PostgreSQL upstream.
# It does not claim coordinator bootstrap removal, dynamic shard-aware pool
# routing, multi-shard plan-leader execution, Kubernetes reconciliation, or
# WAN/cross-region behavior.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

if [[ "${REQUIRE_DOCKER:-0}" != "1" ]]; then
  echo "REQUIRE_DOCKER=1 is required for coordinator-less MX live smoke" >&2
  exit 2
fi

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "$1 is required for coordinator-less MX live smoke" >&2
    exit 1
  }
}

choose_port() {
  python3 - "$@" <<'PY_PORT'
import socket
import sys

reserved = {int(port) for port in sys.argv[1:] if port}
for _ in range(128):
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.bind(("127.0.0.1", 0))
    port = sock.getsockname()[1]
    sock.close()
    if port not in reserved:
        print(port)
        raise SystemExit(0)
raise SystemExit("could not allocate a distinct loopback port")
PY_PORT
}

need_cmd cargo
need_cmd curl
need_cmd docker
need_cmd psql
need_cmd python3

image="${AI_BLAISE_CITUS_COHAB_IMAGE:-ai-blaise-citus-timescale-cohabitation:local}"
if ! docker image inspect "${image}" >/dev/null 2>&1; then
  echo "missing Citus cohabitation image: ${image}" >&2
  exit 1
fi

worker_port="$(choose_port)"
pool_port="$(choose_port "${worker_port}")"
admin_port="$(choose_port "${worker_port}" "${pool_port}")"
network="ai-blaise-s4-mx-${$}-${RANDOM}"
coordinator="${network}-coord"
worker1="${network}-worker1"
worker2="${network}-worker2"
pool_log="$(mktemp -t ai-blaise-s4-pool.XXXXXX.log)"
pool_pid=""

cleanup() {
  if [[ -n "${pool_pid}" ]] && kill -0 "${pool_pid}" >/dev/null 2>&1; then
    kill "${pool_pid}" >/dev/null 2>&1 || true
    wait "${pool_pid}" >/dev/null 2>&1 || true
  fi
  docker rm -f "${coordinator}" "${worker1}" "${worker2}" >/dev/null 2>&1 || true
  docker network rm "${network}" >/dev/null 2>&1 || true
  rm -f "${pool_log}"
}
trap cleanup EXIT

docker network create "${network}" >/dev/null

run_postgres_container() {
  local container="$1"
  shift
  docker run -d \
    --name "${container}" \
    --network "${network}" \
    "$@" \
    -e POSTGRES_PASSWORD=postgres \
    -e POSTGRES_HOST_AUTH_METHOD=trust \
    -e POSTGRES_DB=postgres \
    "${image}" \
    -c shared_preload_libraries=timescaledb,citus \
    -c citus.cohabit_extensions=timescaledb >/dev/null
}

run_postgres_container "${coordinator}"
run_postgres_container "${worker1}" -p "127.0.0.1:${worker_port}:5432"
run_postgres_container "${worker2}"

wait_for_postgres_ready() {
  local container="$1"
  local ready=0
  for _ in $(seq 1 90); do
    if docker exec "${container}" pg_isready -U postgres >/dev/null 2>&1 \
      && docker exec "${container}" psql -U postgres -d postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
      ready=1
      break
    fi
    sleep 2
  done
  if [[ "${ready}" != "1" ]]; then
    docker logs "${container}" >&2 || true
    echo "${container} did not become ready" >&2
    exit 1
  fi
}

ensure_citus_extension() {
  local container="$1"
  local output=""
  if ! output="$(docker exec "${container}" psql -U postgres -d postgres -v ON_ERROR_STOP=1 -q \
    -c 'CREATE EXTENSION IF NOT EXISTS citus;' 2>&1)"; then
    if ! grep -Eq 'citus_setup_ssl|server closed the connection|connection to server was lost' <<<"${output}"; then
      printf '%s\n' "${output}" >&2
      exit 1
    fi
    wait_for_postgres_ready "${container}"
    docker exec "${container}" psql -U postgres -d postgres -v ON_ERROR_STOP=1 -q \
      -c 'CREATE EXTENSION IF NOT EXISTS citus;' >/dev/null
  fi
}

for container in "${coordinator}" "${worker1}" "${worker2}"; do
  wait_for_postgres_ready "${container}"
  ensure_citus_extension "${container}"
done

cargo test -q -p ai_blaise_citus_operator coordinator_less_plan_omits_dedicated_coordinator_instances
grep -Fq 'coordinator-less topology requires {component}' operator/src/crds/citus_cluster.rs
grep -Fq 'self.pool.is_none()' operator/src/crds/citus_cluster.rs

docker exec -i "${coordinator}" psql -U postgres -d postgres -v ON_ERROR_STOP=1 -q <<SQL
SELECT citus_set_coordinator_host('${coordinator}', 5432);
SELECT citus_add_node('${worker1}', 5432);
SELECT citus_add_node('${worker2}', 5432);
CREATE TABLE public.s4_orders (
  tenant_id integer NOT NULL,
  order_id integer NOT NULL,
  total integer NOT NULL,
  PRIMARY KEY (tenant_id, order_id)
);
SELECT create_distributed_table('public.s4_orders', 'tenant_id');
INSERT INTO public.s4_orders
SELECT 1, generated_order, generated_order * 10
FROM generate_series(1, 10) AS generated_order;
INSERT INTO public.s4_orders
SELECT 2, generated_order, generated_order * 20
FROM generate_series(1, 10) AS generated_order;
SQL

docker exec -i "${coordinator}" psql -U postgres -d postgres -v ON_ERROR_STOP=1 -q <<SQL
SELECT start_metadata_sync_to_node('${worker1}', 5432);
SELECT start_metadata_sync_to_node('${worker2}', 5432);
SQL

metadata_synced_workers="$(docker exec "${coordinator}" psql -U postgres -d postgres -Atq -v ON_ERROR_STOP=1 \
  -c "SELECT count(*)::int FROM pg_dist_node WHERE nodename IN ('${worker1}', '${worker2}') AND hasmetadata;")"
if [[ "${metadata_synced_workers}" != "2" ]]; then
  docker exec "${coordinator}" psql -U postgres -d postgres -c 'TABLE pg_dist_node;' >&2 || true
  echo "expected both workers to have synced Citus MX metadata" >&2
  exit 1
fi

worker_sum="$(docker exec "${worker1}" psql -U postgres -d postgres -Atq -v ON_ERROR_STOP=1 \
  -c 'SELECT sum(total)::int FROM public.s4_orders WHERE tenant_id = 1;')"
if [[ "${worker_sum}" != "550" ]]; then
  echo "worker MX entry query expected 550, got ${worker_sum}" >&2
  exit 1
fi

worker_explain="$(docker exec "${worker1}" psql -U postgres -d postgres -Atq -v ON_ERROR_STOP=1 \
  -c 'EXPLAIN (COSTS OFF) SELECT sum(total)::int FROM public.s4_orders WHERE tenant_id = 1;')"
if ! grep -Fq 'Custom Scan (Citus Adaptive)' <<<"${worker_explain}"; then
  printf '%s\n' "${worker_explain}" >&2
  echo "worker MX entry query did not use the Citus adaptive executor" >&2
  exit 1
fi
if ! grep -Fq 'Task Count: 1' <<<"${worker_explain}"; then
  printf '%s\n' "${worker_explain}" >&2
  echo "worker MX entry query did not remain single-task routed" >&2
  exit 1
fi
if grep -Fq "host=${coordinator}" <<<"${worker_explain}"; then
  printf '%s\n' "${worker_explain}" >&2
  echo "worker MX entry query unexpectedly routed back through the coordinator" >&2
  exit 1
fi

AI_BLAISE_POOL_LISTEN_ADDR="127.0.0.1:${pool_port}" \
  AI_BLAISE_POOL_ADMIN_ADDR="127.0.0.1:${admin_port}" \
  AI_BLAISE_POOL_UPSTREAM_ADDR="127.0.0.1:${worker_port}" \
  AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST="127.0.0.0/8" \
  cargo run -q -p ai_blaise_citus_pool -- serve >"${pool_log}" 2>&1 &
pool_pid="$!"

pool_ready=0
for _ in $(seq 1 120); do
  if ! kill -0 "${pool_pid}" >/dev/null 2>&1; then
    cat "${pool_log}" >&2
    echo "pool exited before readiness" >&2
    exit 1
  fi
  if curl -fsS "http://127.0.0.1:${admin_port}/readyz" 2>/dev/null |
    grep -Fq '"upstream_ready":true'; then
    pool_ready=1
    break
  fi
  sleep 1
done
if [[ "${pool_ready}" != "1" ]]; then
  cat "${pool_log}" >&2
  echo "pool did not report upstream-ready worker entry point" >&2
  exit 1
fi

pool_sum="$(PGCONNECT_TIMEOUT=10 psql -h 127.0.0.1 -p "${pool_port}" -U postgres -d postgres -Atq \
  -v ON_ERROR_STOP=1 -c 'SELECT sum(total)::int FROM public.s4_orders WHERE tenant_id = 1;')"
if [[ "${pool_sum}" != "550" ]]; then
  cat "${pool_log}" >&2
  echo "pool-to-worker MX entry query expected 550, got ${pool_sum}" >&2
  exit 1
fi

metrics="$(curl -fsS "http://127.0.0.1:${admin_port}/metrics")"
if ! awk '
  /^ai_blaise_citus_pool_upstream_ready/ && $2 == 1 { upstream_ready = 1 }
  /^ai_blaise_citus_pool_requests_total / && $2 >= 1 { requests = 1 }
  END { exit upstream_ready && requests ? 0 : 1 }
' <<<"${metrics}"; then
  cat "${pool_log}" >&2
  printf '%s\n' "${metrics}" >&2
  echo "pool metrics did not record the worker-entry request" >&2
  exit 1
fi

printf 'coordinatorless_mx_live=passed\n'
printf 'operator_coordinatorless_admission_checked=true\n'
printf 'dedicated_coordinators=0\n'
printf 'citus_mx_metadata_synced=true\n'
printf 'metadata_synced_workers=2\n'
printf 'worker_entry_query_served=true\n'
printf 'worker_entry_sum=550\n'
printf 'pool_worker_entry_query_served=true\n'
printf 'pool_worker_entry_sum=550\n'
printf 'citus_adaptive_plan_observed=true\n'
printf 'citus_task_count_observed=1\n'
printf 'coordinator_reroute_observed=false\n'
printf 'coordinator_bootstrap_removed=false\n'
printf 'dynamic_shard_aware_pool_routing=false\n'
printf 'multi_shard_plan_leader_executed=false\n'
printf 'kubernetes_reconciliation_exercised=false\n'
printf 'wan_or_cross_region_exercised=false\n'
printf 'coordinatorless_mx_live\tpassed\n'
