#!/usr/bin/env bash
set -euo pipefail

# FEATURE: MR5
# Live bounded GeoIP/CIDR pool routing proof. This smoke starts two real
# PostgreSQL regional replicas and proves the pool routes a localhost client to
# the selected regional upstream using AI_BLAISE_POOL_GEO_* configuration. It
# does not claim managed MaxMind DB loading, Region-CR synchronization,
# hot-swap reloads, cross-region/WAN behavior, edge-replica traffic, or
# Kubernetes traffic.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

if [[ "${REQUIRE_DOCKER:-0}" != "1" ]]; then
  echo "REQUIRE_DOCKER=1 is required for MR5 pool GeoIP live smoke" >&2
  exit 2
fi

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "$1 is required for MR5 pool GeoIP live smoke" >&2
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

postgres_image="${MR5_POSTGRES_IMAGE:-postgres:17-bookworm}"
east_port="$(choose_port)"
west_port="$(choose_port "${east_port}")"
pool_port="$(choose_port "${east_port}" "${west_port}")"
admin_port="$(choose_port "${east_port}" "${west_port}" "${pool_port}")"
invalid_pool_port="$(choose_port "${east_port}" "${west_port}" "${pool_port}" "${admin_port}")"
invalid_admin_port="$(choose_port "${east_port}" "${west_port}" "${pool_port}" "${admin_port}" "${invalid_pool_port}")"
east_container="ai-blaise-mr5-east-${$}-${RANDOM}"
west_container="ai-blaise-mr5-west-${$}-${RANDOM}"
pool_log="$(mktemp -t ai-blaise-mr5-pool.XXXXXX.log)"
invalid_log="$(mktemp -t ai-blaise-mr5-invalid.XXXXXX.log)"
pool_pid=""

cleanup() {
  if [[ -n "${pool_pid}" ]] && kill -0 "${pool_pid}" >/dev/null 2>&1; then
    kill "${pool_pid}" >/dev/null 2>&1 || true
    wait "${pool_pid}" >/dev/null 2>&1 || true
  fi
  docker rm -f "${east_container}" "${west_container}" >/dev/null 2>&1 || true
  rm -f "${pool_log}" "${invalid_log}"
}
trap cleanup EXIT

start_regional_postgres() {
  local container="$1"
  local port="$2"
  # Pre-pull with bounded retry to keep the 60s ready-wait
  # budget for actual init time, not for registry-1.docker.io
  # pulls. Matches the retry pattern used in t6-pg18-io-uring,
  # mr9-regional-failover, and sidecar-cdc smokes.
  for attempt in 1 2 3; do
    if docker pull "${postgres_image}" >/dev/null; then break; fi
    if [ "${attempt}" = "3" ]; then
      echo "docker pull ${postgres_image} failed after 3 attempts" >&2; exit 1
    fi
    sleep 5
  done
  docker run -d --name "${container}" \
    -p "127.0.0.1:${port}:5432" \
    -e POSTGRES_PASSWORD=postgres \
    -e POSTGRES_HOST_AUTH_METHOD=trust \
    -e POSTGRES_DB=postgres \
    "${postgres_image}" >/dev/null
  for _ in $(seq 1 90); do
    if docker exec "${container}" pg_isready -U postgres >/dev/null 2>&1 \
      && docker exec "${container}" psql -U postgres -d postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  docker logs "${container}" >&2 || true
  echo "${container} did not become ready" >&2
  exit 1
}

seed_region() {
  local port="$1"
  local region="$2"
  psql -h 127.0.0.1 -p "${port}" -U postgres -d postgres -v ON_ERROR_STOP=1 -q <<SQL
CREATE TABLE public.geo_route_marker(region text PRIMARY KEY);
INSERT INTO public.geo_route_marker(region) VALUES ('${region}');
SQL
}

start_pool() {
  local rules="$1"
  : >"${pool_log}"
  AI_BLAISE_POOL_LISTEN_ADDR="127.0.0.1:${pool_port}" \
    AI_BLAISE_POOL_ADMIN_ADDR="127.0.0.1:${admin_port}" \
    AI_BLAISE_POOL_UPSTREAM_ADDR="127.0.0.1:${east_port}" \
    AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST="127.0.0.0/8" \
    AI_BLAISE_POOL_GEO_DEFAULT_REGION="us-east-1" \
    AI_BLAISE_POOL_GEO_RULES="${rules}" \
    AI_BLAISE_POOL_GEO_REPLICAS="us-east-1,1,127.0.0.1,${east_port};eu-west-1,1,127.0.0.1,${west_port}" \
    cargo run -q -p ai_blaise_citus_pool -- serve >"${pool_log}" 2>&1 &
  pool_pid="$!"
  local ready=0
  for _ in $(seq 1 120); do
    if ! kill -0 "${pool_pid}" >/dev/null 2>&1; then
      cat "${pool_log}" >&2
      echo "pool exited before MR5 readiness" >&2
      exit 1
    fi
    if curl -fsS "http://127.0.0.1:${admin_port}/readyz" 2>/dev/null |
      grep -Fq '"upstream_ready":true'; then
      ready=1
      break
    fi
    sleep 1
  done
  if [[ "${ready}" != "1" ]]; then
    cat "${pool_log}" >&2
    echo "pool did not report ready for MR5" >&2
    exit 1
  fi
}

stop_pool() {
  if [[ -n "${pool_pid}" ]] && kill -0 "${pool_pid}" >/dev/null 2>&1; then
    kill "${pool_pid}" >/dev/null 2>&1 || true
    wait "${pool_pid}" >/dev/null 2>&1 || true
  fi
  pool_pid=""
}

query_region_through_pool() {
  PGCONNECT_TIMEOUT=10 psql -h 127.0.0.1 -p "${pool_port}" -U postgres -d postgres -Atq \
    -v ON_ERROR_STOP=1 -c 'SELECT region FROM public.geo_route_marker;'
}

assert_geo_metrics() {
  local expected_fallback="$1"
  local metrics
  metrics="$(curl -fsS "http://127.0.0.1:${admin_port}/metrics")"
  if ! awk -v expected_fallback="${expected_fallback}" '
    /^ai_blaise_citus_pool_geo_routes_total / && $2 >= 1 { routes = 1 }
    /^ai_blaise_citus_pool_geo_fallback_routes_total / && $2 == expected_fallback { fallback = 1 }
    END { exit routes && fallback ? 0 : 1 }
  ' <<<"${metrics}"; then
    cat "${pool_log}" >&2
    printf '%s\n' "${metrics}" >&2
    echo "MR5 GeoIP metrics did not match expected fallback count ${expected_fallback}" >&2
    exit 1
  fi
}

start_regional_postgres "${east_container}" "${east_port}"
start_regional_postgres "${west_container}" "${west_port}"
seed_region "${east_port}" "us-east-1"
seed_region "${west_port}" "eu-west-1"

cargo test -q -p ai_blaise_citus_pool geo_routing_config_routes_client_ip_to_replica_and_fallback

start_pool "127.0.0.0/8=us-east-1"
selected_region="$(query_region_through_pool)"
if [[ "${selected_region}" != "us-east-1" ]]; then
  cat "${pool_log}" >&2
  echo "expected GeoIP-routed pool query to reach us-east-1, got ${selected_region}" >&2
  exit 1
fi
assert_geo_metrics 0
stop_pool

start_pool "127.0.0.0/8=moon"
fallback_region="$(query_region_through_pool)"
if [[ "${fallback_region}" != "us-east-1" ]]; then
  cat "${pool_log}" >&2
  echo "expected GeoIP fallback query to reach us-east-1, got ${fallback_region}" >&2
  exit 1
fi
assert_geo_metrics 1
stop_pool

if AI_BLAISE_POOL_LISTEN_ADDR="127.0.0.1:${invalid_pool_port}" \
  AI_BLAISE_POOL_ADMIN_ADDR="127.0.0.1:${invalid_admin_port}" \
  AI_BLAISE_POOL_UPSTREAM_ADDR="127.0.0.1:${east_port}" \
  AI_BLAISE_POOL_GEO_DEFAULT_REGION="us-east-1" \
  AI_BLAISE_POOL_GEO_RULES="127.0.0.0/99=us-east-1" \
  AI_BLAISE_POOL_GEO_REPLICAS="us-east-1,1,127.0.0.1,${east_port}" \
  cargo run -q -p ai_blaise_citus_pool -- serve >"${invalid_log}" 2>&1; then
  cat "${invalid_log}" >&2
  echo "invalid MR5 CIDR unexpectedly allowed pool startup" >&2
  exit 1
fi
if ! grep -Eq 'geo routing error|InvalidCidr|invalid CIDR' "${invalid_log}"; then
  cat "${invalid_log}" >&2
  echo "invalid MR5 CIDR did not fail closed with a geo routing error" >&2
  exit 1
fi

printf 'pool_geoip_live=passed\n'
printf 'AI_BLAISE_POOL_GEO_DEFAULT_REGION=us-east-1\n'
printf 'AI_BLAISE_POOL_GEO_RULES=127.0.0.0/8=us-east-1\n'
printf 'AI_BLAISE_POOL_GEO_REPLICAS=us-east-1,1,127.0.0.1,east;eu-west-1,1,127.0.0.1,west\n'
printf 'geoip_pool_route_selected_region=us-east-1\n'
printf 'geoip_pool_fallback_region=us-east-1\n'
printf 'geoip_live_routes_total=1\n'
printf 'geoip_live_fallback_routes_total=1\n'
printf 'geoip_invalid_cidr_fail_closed=true\n'
printf 'managed_maxmind_db_loaded=false\n'
printf 'region_cr_synchronization=false\n'
printf 'hot_swap_reload_exercised=false\n'
printf 'cross_region_wan_exercised=false\n'
printf 'edge_replica_traffic_exercised=false\n'
printf 'kubernetes_traffic_exercised=false\n'
printf 'pool_geoip_live\tpassed\n'
