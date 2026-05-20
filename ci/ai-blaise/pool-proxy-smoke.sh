#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

postgres_image="${POOL_PROXY_SMOKE_IMAGE:-postgres:17}"
require_docker="${REQUIRE_DOCKER:-0}"

if ! command -v docker >/dev/null 2>&1; then
  if [[ "${require_docker}" == "1" ]]; then
    echo "docker is required for pool proxy smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping pool proxy smoke"
  exit 0
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required for pool proxy smoke" >&2
  exit 1
fi

choose_port() {
  python3 - <<'PY'
import socket

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
}

postgres_port="$(choose_port)"
pool_port="$(choose_port)"
admin_port="$(choose_port)"
container="ai-blaise-pool-proxy-smoke-${RANDOM}-$$"
pool_log="$(mktemp -t ai-blaise-pool-proxy.XXXXXX.log)"
pool_pid=""

cleanup() {
  if [[ -n "${pool_pid}" ]] && kill -0 "${pool_pid}" >/dev/null 2>&1; then
    kill "${pool_pid}" >/dev/null 2>&1 || true
    wait "${pool_pid}" >/dev/null 2>&1 || true
  fi
  docker rm -f "${container}" >/dev/null 2>&1 || true
  rm -f "${pool_log}"
}
trap cleanup EXIT

docker run \
  --name "${container}" \
  -e POSTGRES_PASSWORD=postgres \
  -p "127.0.0.1:${postgres_port}:5432" \
  -d "${postgres_image}" >/dev/null

postgres_ready=0
for _ in $(seq 1 60); do
  if docker exec "${container}" pg_isready -U postgres >/dev/null 2>&1; then
    postgres_ready=1
    break
  fi
  sleep 1
done

if [[ "${postgres_ready}" != "1" ]]; then
  docker logs "${container}" >&2 || true
  echo "postgres container did not become ready" >&2
  exit 1
fi

AI_BLAISE_POOL_LISTEN_ADDR="127.0.0.1:${pool_port}" \
  AI_BLAISE_POOL_ADMIN_ADDR="127.0.0.1:${admin_port}" \
  AI_BLAISE_POOL_UPSTREAM_ADDR="127.0.0.1:${postgres_port}" \
  cargo run -q -p ai_blaise_citus_pool -- serve >"${pool_log}" 2>&1 &
pool_pid="$!"

pool_ready=0
for _ in $(seq 1 120); do
  if ! kill -0 "${pool_pid}" >/dev/null 2>&1; then
    cat "${pool_log}" >&2
    echo "pool proxy exited before readiness" >&2
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
  echo "pool proxy did not report upstream-ready readiness" >&2
  exit 1
fi

query_output="$(
  docker run --rm \
    -i \
    --network host \
    -e PGPASSWORD=postgres \
    "${postgres_image}" \
    psql -h 127.0.0.1 -p "${pool_port}" -U postgres -d postgres -Atqv ON_ERROR_STOP=1 <<'SQL'
SELECT 42::int;
CREATE TEMP TABLE pool_proxy_smoke(value integer);
INSERT INTO pool_proxy_smoke VALUES (7), (35);
SELECT sum(value)::int FROM pool_proxy_smoke;
SQL
)"

if [[ "${query_output}" != $'42\n42' ]]; then
  cat "${pool_log}" >&2
  echo "unexpected PostgreSQL query output through pool proxy:" >&2
  printf '%s\n' "${query_output}" >&2
  exit 1
fi

metrics="$(curl -fsS "http://127.0.0.1:${admin_port}/metrics")"
if ! printf '%s\n' "${metrics}" | awk '
  /^ai_blaise_citus_pool_upstream_ready/ && $2 == 1 { upstream_ready = 1 }
  /^ai_blaise_citus_pool_requests_total / && $2 >= 1 { requests = 1 }
  END { exit upstream_ready && requests ? 0 : 1 }
'; then
  cat "${pool_log}" >&2
  echo "pool metrics did not show upstream readiness and proxied PostgreSQL traffic" >&2
  printf '%s\n' "${metrics}" >&2
  exit 1
fi

echo "ai_blaise_citus pool proxy smoke passed with ${postgres_image}"
