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

choose_distinct_port() {
  local candidate
  local used
  while true; do
    candidate="$(choose_port)"
    for used in "$@"; do
      if [[ "${candidate}" == "${used}" ]]; then
        continue 2
      fi
    done
    printf '%s
' "${candidate}"
    return 0
  done
}

postgres_port="$(choose_port)"
pool_port="$(choose_distinct_port "${postgres_port}")"
admin_port="$(choose_distinct_port "${postgres_port}" "${pool_port}")"
container="ai-blaise-pool-proxy-smoke-${RANDOM}-$$"
pool_log="$(mktemp -t ai-blaise-pool-proxy.XXXXXX.log)"
auth_log="$(mktemp -t ai-blaise-pool-auth.XXXXXX.log)"
pool_pid=""
auth_pid=""
holder_pid=""

cleanup() {
  if [[ -n "${holder_pid}" ]] && kill -0 "${holder_pid}" >/dev/null 2>&1; then
    kill "${holder_pid}" >/dev/null 2>&1 || true
    wait "${holder_pid}" >/dev/null 2>&1 || true
  fi
  if [[ -n "${pool_pid}" ]] && kill -0 "${pool_pid}" >/dev/null 2>&1; then
    kill "${pool_pid}" >/dev/null 2>&1 || true
    wait "${pool_pid}" >/dev/null 2>&1 || true
  fi
  if [[ -n "${auth_pid}" ]] && kill -0 "${auth_pid}" >/dev/null 2>&1; then
    kill "${auth_pid}" >/dev/null 2>&1 || true
    wait "${auth_pid}" >/dev/null 2>&1 || true
  fi
  docker rm -f "${container}" >/dev/null 2>&1 || true
  rm -f "${pool_log}" "${auth_log}"
}
trap cleanup EXIT

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
docker run \
  --name "${container}" \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_HOST_AUTH_METHOD=trust \
  -p "127.0.0.1:${postgres_port}:5432" \
  -d "${postgres_image}" >/dev/null

postgres_init_complete=0
for _ in $(seq 1 120); do
  if docker logs "${container}" 2>&1 | grep -q "PostgreSQL init process complete"; then
    postgres_init_complete=1
    break
  fi
  sleep 1
done

if [[ "${postgres_init_complete}" != "1" ]]; then
  docker logs "${container}" >&2 || true
  echo "postgres container did not finish init scripts" >&2
  exit 1
fi

postgres_ready=0
for _ in $(seq 1 60); do
  if docker exec "${container}" psql -U postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
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
  AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST="127.0.0.0/8" \
  AI_BLAISE_POOL_SETTINGS_BUCKET_GUCS="citus.enable_repartition_joins" \
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

python3 - "${pool_port}" <<'PY'
import socket
import struct
import sys

pool_port = int(sys.argv[1])


def pack_startup(parameters):
    body = struct.pack("!I", 196608)
    for key, value in parameters.items():
        body += key.encode("ascii") + b"\x00" + value.encode("ascii") + b"\x00"
    body += b"\x00"
    return struct.pack("!I", len(body) + 4) + body


def pack_simple_query(query):
    body = query.encode("utf-8") + b"\x00"
    return b"Q" + struct.pack("!I", len(body) + 4) + body


def read_exact(sock, byte_count):
    chunks = []
    remaining = byte_count
    while remaining:
        chunk = sock.recv(remaining)
        if not chunk:
            raise RuntimeError("unexpected EOF from PostgreSQL server")
        chunks.append(chunk)
        remaining -= len(chunk)
    return b"".join(chunks)


def read_message(sock):
    message_type = read_exact(sock, 1)
    length = struct.unpack("!I", read_exact(sock, 4))[0]
    payload = read_exact(sock, length - 4)
    return message_type, payload


def error_message(payload):
    parts = []
    for field in payload.split(b"\x00"):
        if len(field) > 1:
            parts.append(field[1:].decode("utf-8", errors="replace"))
    return "; ".join(parts) or payload.decode("utf-8", errors="replace")


def data_row(payload):
    column_count = struct.unpack("!H", payload[:2])[0]
    offset = 2
    values = []
    for _ in range(column_count):
        value_length = struct.unpack("!i", payload[offset : offset + 4])[0]
        offset += 4
        if value_length == -1:
            values.append(None)
            continue
        value = payload[offset : offset + value_length]
        offset += value_length
        values.append(value.decode("utf-8", errors="strict"))
    return values


def startup_ok(sock):
    while True:
        message_type, payload = read_message(sock)
        if message_type == b"R":
            auth_code = struct.unpack("!I", payload[:4])[0]
            if auth_code != 0:
                raise RuntimeError(f"expected trust auth code 0, got {auth_code}")
        elif message_type == b"E":
            raise RuntimeError(f"startup failed: {error_message(payload)}")
        elif message_type == b"Z":
            return


def connect_with_repartition(setting):
    sock = socket.create_connection(("127.0.0.1", pool_port), timeout=10)
    sock.sendall(
        pack_startup(
            {
                "user": "postgres",
                "database": "postgres",
                "application_name": f"ai_blaise_settings_{setting}",
                "options": f"-c citus.enable_repartition_joins={setting}",
            }
        )
    )
    startup_ok(sock)
    return sock


def simple_query(sock, query):
    sock.sendall(pack_simple_query(query))
    rows = []
    while True:
        message_type, payload = read_message(sock)
        if message_type == b"D":
            rows.append(data_row(payload))
        elif message_type == b"E":
            raise RuntimeError(f"query failed: {error_message(payload)}")
        elif message_type == b"Z":
            return rows



with socket.create_connection(("127.0.0.1", pool_port), timeout=10) as sock:
    sock.sendall(
        pack_startup(
            {
                "user": "postgres",
                "database": "postgres",
                "application_name": "ai_blaise_pipeline_smoke",
            }
        )
    )

    startup_ok(sock)

    # Send both query frames before reading either result. This proves the pool
    # preserves PostgreSQL simple-query pipelining instead of relying on psql's
    # request/response pacing.
    sock.sendall(
        pack_simple_query("SELECT 'pipeline_one'::text")
        + pack_simple_query("SELECT 'pipeline_two'::text")
    )

    rows = []
    ready_count = 0
    while ready_count < 2:
        message_type, payload = read_message(sock)
        if message_type == b"D":
            rows.append(data_row(payload))
        elif message_type == b"E":
            raise RuntimeError(f"pipelined query failed: {error_message(payload)}")
        elif message_type == b"Z":
            ready_count += 1

    expected = [["pipeline_one"], ["pipeline_two"]]
    if rows != expected:
        raise RuntimeError(f"unexpected pipelined rows: {rows!r}")

    sock.sendall(b"X" + struct.pack("!I", 4))

with connect_with_repartition("on") as on_sock, connect_with_repartition("off") as off_sock:
    setting_query = "SELECT current_setting('citus.enable_repartition_joins', true), pg_backend_pid()::text"
    on_rows = simple_query(on_sock, setting_query)
    off_rows = simple_query(off_sock, setting_query)
    if len(on_rows) != 1 or len(off_rows) != 1:
        raise RuntimeError(f"unexpected settings rows: {on_rows!r} {off_rows!r}")
    if on_rows[0][0] != "on" or off_rows[0][0] != "off":
        raise RuntimeError(f"tracked GUC state bled across settings buckets: {on_rows!r} {off_rows!r}")
    if on_rows[0][1] == off_rows[0][1]:
        raise RuntimeError(f"expected distinct backend pids for simultaneous tracked-GUC settings: {on_rows!r} {off_rows!r}")
    on_sock.sendall(b"X" + struct.pack("!I", 4))
    off_sock.sendall(b"X" + struct.pack("!I", 4))

print("raw PostgreSQL pipelined simple-query and settings-bucket smoke passed through pool proxy")
PY

metrics="$(curl -fsS "http://127.0.0.1:${admin_port}/metrics")"
if ! printf '%s
' "${metrics}" | awk '
  /^ai_blaise_citus_pool_upstream_ready/ && $2 == 1 { upstream_ready = 1 }
  /^ai_blaise_citus_pool_requests_total / && $2 >= 3 { requests = 1 }
  /^ai_blaise_citus_pool_settings_bucket_unique_fingerprints / && $2 >= 3 { bucket_unique = 1 }
  /^ai_blaise_citus_pool_settings_bucket_backend_borrows_total / && $2 >= 3 { bucket_borrows = 1 }
  /^ai_blaise_citus_pool_settings_bucket_assigned_connections / && $2 == 0 { bucket_released = 1 }
  /^ai_blaise_citus_pool_settings_bucket_release_errors_total / && $2 == 0 { bucket_release_errors = 1 }
  END { exit upstream_ready && requests && bucket_unique && bucket_borrows && bucket_released && bucket_release_errors ? 0 : 1 }
'; then
  cat "${pool_log}" >&2
  echo "pool metrics did not show upstream readiness, settings-bucket borrow/release accounting, and proxied PostgreSQL traffic" >&2
  printf '%s\n' "${metrics}" >&2
  exit 1
fi

kill "${pool_pid}" >/dev/null 2>&1 || true
wait "${pool_pid}" >/dev/null 2>&1 || true
pool_pid=""
: >"${pool_log}"

AI_BLAISE_POOL_LISTEN_ADDR="127.0.0.1:${pool_port}" \
  AI_BLAISE_POOL_ADMIN_ADDR="127.0.0.1:${admin_port}" \
  AI_BLAISE_POOL_UPSTREAM_ADDR="127.0.0.1:${postgres_port}" \
  AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST="192.0.2.0/24" \
  cargo run -q -p ai_blaise_citus_pool -- serve >"${pool_log}" 2>&1 &
pool_pid="$!"

pool_ready=0
for _ in $(seq 1 120); do
  if ! kill -0 "${pool_pid}" >/dev/null 2>&1; then
    cat "${pool_log}" >&2
    echo "pool proxy exited before CIDR deny readiness" >&2
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
  echo "pool proxy did not report upstream-ready readiness for CIDR deny smoke" >&2
  exit 1
fi

denied_stdout="$(mktemp -t ai-blaise-pool-cidr-deny.XXXXXX.out)"
denied_stderr="$(mktemp -t ai-blaise-pool-cidr-deny.XXXXXX.err)"
if docker run --rm \
  --network host \
  -e PGPASSWORD=postgres \
  -e PGCONNECT_TIMEOUT=5 \
  "${postgres_image}" \
  psql -h 127.0.0.1 -p "${pool_port}" -U postgres -d postgres -Atqc 'SELECT 1' \
  >"${denied_stdout}" 2>"${denied_stderr}"; then
  cat "${pool_log}" >&2
  cat "${denied_stdout}" >&2
  cat "${denied_stderr}" >&2
  echo "pool CIDR deny smoke unexpectedly allowed PostgreSQL traffic" >&2
  rm -f "${denied_stdout}" "${denied_stderr}"
  exit 1
fi
rm -f "${denied_stdout}" "${denied_stderr}"

metrics="$(curl -fsS "http://127.0.0.1:${admin_port}/metrics")"
if ! printf '%s\n' "${metrics}" | awk '
  /^ai_blaise_citus_pool_rejected_connections_total / && $2 >= 1 { rejected = 1 }
  END { exit rejected ? 0 : 1 }
'; then
  cat "${pool_log}" >&2
  echo "pool metrics did not show CIDR-denied PostgreSQL traffic" >&2
  printf '%s\n' "${metrics}" >&2
  exit 1
fi


kill "${pool_pid}" >/dev/null 2>&1 || true
wait "${pool_pid}" >/dev/null 2>&1 || true
pool_pid=""
: >"${pool_log}"

AI_BLAISE_POOL_LISTEN_ADDR="127.0.0.1:${pool_port}" \
  AI_BLAISE_POOL_ADMIN_ADDR="127.0.0.1:${admin_port}" \
  AI_BLAISE_POOL_UPSTREAM_ADDR="127.0.0.1:${postgres_port}" \
  AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST="127.0.0.0/8" \
  AI_BLAISE_POOL_MAX_ACTIVE_CONNECTIONS="1" \
  AI_BLAISE_POOL_ADMISSION_TIMEOUT_MS="100" \
  AI_BLAISE_POOL_STARTUP_TIMEOUT_MS="1000" \
  cargo run -q -p ai_blaise_citus_pool -- serve >"${pool_log}" 2>&1 &
pool_pid="$!"

pool_ready=0
for _ in $(seq 1 120); do
  if ! kill -0 "${pool_pid}" >/dev/null 2>&1; then
    cat "${pool_log}" >&2
    echo "pool proxy exited before overload readiness" >&2
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
  echo "pool proxy did not report upstream-ready readiness for overload smoke" >&2
  exit 1
fi

holder_stdout="$(mktemp -t ai-blaise-pool-overload-holder.XXXXXX.out)"
holder_stderr="$(mktemp -t ai-blaise-pool-overload-holder.XXXXXX.err)"
docker run --rm \
  --network host \
  -e PGPASSWORD=postgres \
  -e PGCONNECT_TIMEOUT=5 \
  "${postgres_image}" \
  psql -h 127.0.0.1 -p "${pool_port}" -U postgres -d postgres -Atqc 'SELECT pg_sleep(3)' \
  >"${holder_stdout}" 2>"${holder_stderr}" &
holder_pid="$!"

active_seen=0
for _ in $(seq 1 60); do
  metrics="$(curl -fsS "http://127.0.0.1:${admin_port}/metrics")"
  if printf '%s\n' "${metrics}" | awk '
    /^ai_blaise_citus_pool_active_connections / && $2 >= 1 { active = 1 }
    END { exit active ? 0 : 1 }
  '; then
    active_seen=1
    break
  fi
  sleep 0.1
done

if [[ "${active_seen}" != "1" ]]; then
  cat "${pool_log}" >&2
  cat "${holder_stdout}" >&2 || true
  cat "${holder_stderr}" >&2 || true
  echo "pool overload smoke did not observe the held active connection" >&2
  exit 1
fi

overload_stdout="$(mktemp -t ai-blaise-pool-overload.XXXXXX.out)"
overload_stderr="$(mktemp -t ai-blaise-pool-overload.XXXXXX.err)"
if docker run --rm \
  --network host \
  -e PGPASSWORD=postgres \
  -e PGSSLMODE=disable \
  -e PGCONNECT_TIMEOUT=2 \
  "${postgres_image}" \
  psql -h 127.0.0.1 -p "${pool_port}" -U postgres -d postgres -Atqc 'SELECT 1' \
  >"${overload_stdout}" 2>"${overload_stderr}"; then
  cat "${pool_log}" >&2
  cat "${overload_stdout}" >&2
  cat "${overload_stderr}" >&2
  echo "pool overload smoke unexpectedly admitted a second connection" >&2
  exit 1
fi
rm -f "${overload_stdout}" "${overload_stderr}"

if ! wait "${holder_pid}"; then
  cat "${pool_log}" >&2
  cat "${holder_stdout}" >&2 || true
  cat "${holder_stderr}" >&2 || true
  echo "pool overload holder query failed" >&2
  rm -f "${holder_stdout}" "${holder_stderr}"
  holder_pid=""
  exit 1
fi
holder_pid=""
rm -f "${holder_stdout}" "${holder_stderr}"

metrics="$(curl -fsS "http://127.0.0.1:${admin_port}/metrics")"
if ! printf '%s\n' "${metrics}" | awk '
  /^ai_blaise_citus_pool_overloaded_connections_total / && $2 >= 1 { overloaded = 1 }
  /^ai_blaise_citus_pool_rejected_connections_total / && $2 >= 1 { rejected = 1 }
  END { exit overloaded && rejected ? 0 : 1 }
'; then
  cat "${pool_log}" >&2
  echo "pool metrics did not show overload rejection" >&2
  printf '%s\n' "${metrics}" >&2
  exit 1
fi

kill "${pool_pid}" >/dev/null 2>&1 || true
wait "${pool_pid}" >/dev/null 2>&1 || true
pool_pid=""
: >"${pool_log}"

AI_BLAISE_POOL_LISTEN_ADDR="127.0.0.1:${pool_port}" \
  AI_BLAISE_POOL_ADMIN_ADDR="127.0.0.1:${admin_port}" \
  AI_BLAISE_POOL_UPSTREAM_ADDR="127.0.0.1:${postgres_port}" \
  AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST="127.0.0.0/8" \
  AI_BLAISE_POOL_QUOTA_TENANT_ID="tenant-a" \
  AI_BLAISE_POOL_QUOTA_BURST="1" \
  AI_BLAISE_POOL_QUOTA_REFILL_PER_SECOND="1" \
  AI_BLAISE_POOL_STARTUP_TIMEOUT_MS="1000" \
  cargo run -q -p ai_blaise_citus_pool -- serve >"${pool_log}" 2>&1 &
pool_pid="$!"

pool_ready=0
for _ in $(seq 1 120); do
  if ! kill -0 "${pool_pid}" >/dev/null 2>&1; then
    cat "${pool_log}" >&2
    echo "pool proxy exited before quota readiness" >&2
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
  echo "pool proxy did not report upstream-ready readiness for quota smoke" >&2
  exit 1
fi

python3 - "${pool_port}" <<'PY_QUOTA'
import socket
import struct
import sys

pool_port = int(sys.argv[1])


def pack_startup(app_name):
    parameters = {
        "user": "postgres",
        "database": "postgres",
        "application_name": app_name,
    }
    body = struct.pack("!I", 196608)
    for key, value in parameters.items():
        body += key.encode("ascii") + b"\x00" + value.encode("ascii") + b"\x00"
    body += b"\x00"
    return struct.pack("!I", len(body) + 4) + body


def read_exact(sock, byte_count):
    chunks = []
    remaining = byte_count
    while remaining:
        chunk = sock.recv(remaining)
        if not chunk:
            raise RuntimeError("unexpected EOF from PostgreSQL server")
        chunks.append(chunk)
        remaining -= len(chunk)
    return b"".join(chunks)


def read_message(sock):
    message_type = read_exact(sock, 1)
    length = struct.unpack("!I", read_exact(sock, 4))[0]
    payload = read_exact(sock, length - 4)
    return message_type, payload


def connect_with_tenant():
    sock = socket.create_connection(("127.0.0.1", pool_port), timeout=10)
    sock.sendall(pack_startup("application=pool_quota_smoke;tenant_id=tenant-a"))
    return sock

with connect_with_tenant() as sock:
    ready = False
    while not ready:
        message_type, _payload = read_message(sock)
        if message_type == b"E":
            raise RuntimeError("first quota-admitted startup failed")
        if message_type == b"Z":
            ready = True
    sock.sendall(b"X" + struct.pack("!I", 4))

with connect_with_tenant() as sock:
    message_type, _payload = read_message(sock)
    if message_type != b"E":
        raise RuntimeError(f"expected quota denial ErrorResponse, got {message_type!r}")

print("tenant quota fail-closed smoke passed through pool proxy")
PY_QUOTA

metrics="$(curl -fsS "http://127.0.0.1:${admin_port}/metrics")"
if ! printf '%s\n' "${metrics}" | awk '
  /^ai_blaise_citus_pool_tenant_quota_rejections_total / && $2 >= 1 { quota = 1 }
  /^ai_blaise_citus_pool_fail_closed_routes_total / && $2 >= 1 { fail_closed = 1 }
  END { exit quota && fail_closed ? 0 : 1 }
'; then
  cat "${pool_log}" >&2
  echo "pool metrics did not show tenant quota fail-closed rejection" >&2
  printf '%s\n' "${metrics}" >&2
  exit 1
fi

kill "${pool_pid}" >/dev/null 2>&1 || true
wait "${pool_pid}" >/dev/null 2>&1 || true
pool_pid=""
: >"${pool_log}"

auth_port="$(choose_distinct_port "${postgres_port}" "${pool_port}" "${admin_port}")"
auth_base="http://127.0.0.1:${auth_port}"
auth_secret="pool-auth-smoke-secret-32-bytes-minimum-material"
AI_BLAISE_LISTEN_ADDR="127.0.0.1:${auth_port}" \
  AI_BLAISE_AUTH_ISSUER="https://auth.example.com" \
  AI_BLAISE_AUTH_AUDIENCE="postgres" \
  AI_BLAISE_AUTH_TTL_SECONDS="300" \
  AI_BLAISE_AUTH_HS256_SECRET="${auth_secret}" \
  cargo run -q -p ai_blaise_citus_sidecar_auth -- serve >"${auth_log}" 2>&1 &
auth_pid="$!"

for _ in $(seq 1 120); do
  if curl -fsS "${auth_base}/healthz" >/dev/null 2>&1; then
    break
  fi
  if ! kill -0 "${auth_pid}" >/dev/null 2>&1; then
    cat "${auth_log}" >&2
    echo "auth sidecar exited before pool auth smoke" >&2
    exit 1
  fi
  sleep 0.25
done

token_file="$(mktemp -t ai-blaise-pool-auth-token.XXXXXX)"
python3 - "${auth_base}" >"${token_file}" <<'PY_AUTH'
import http.client
import json
import sys

base = sys.argv[1]
host_port = base.removeprefix("http://")
host, port = host_port.split(":")

def request(method, path, body=None, status=200):
    conn = http.client.HTTPConnection(host, int(port), timeout=10)
    headers = {"Connection": "close"}
    if body is not None:
        headers["Content-Type"] = "application/json"
        body = json.dumps(body)
    conn.request(method, path, body=body, headers=headers)
    response = conn.getresponse()
    raw = response.read()
    if response.status != status:
        raise SystemExit(f"{method} {path} -> {response.status}: {raw!r}")
    return json.loads(raw or b"{}")

request("POST", "/auth/users", {
    "username": "poolalice",
    "password": "hunter2-correct-horse",
    "role": "authenticated",
    "tenant_id": "tenant-a",
}, status=201)
login = request("POST", "/auth/login", {
    "username": "poolalice",
    "password": "hunter2-correct-horse",
})
print(login["access_token"])
PY_AUTH

access_token="$(sed -n '1p' "${token_file}")"
rm -f "${token_file}"

AI_BLAISE_POOL_LISTEN_ADDR="127.0.0.1:${pool_port}" \
  AI_BLAISE_POOL_ADMIN_ADDR="127.0.0.1:${admin_port}" \
  AI_BLAISE_POOL_UPSTREAM_ADDR="127.0.0.1:${postgres_port}" \
  AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST="127.0.0.0/8" \
  AI_BLAISE_POOL_AUTH_INTROSPECTION_URL="${auth_base}/auth/introspect" \
  AI_BLAISE_POOL_AUTH_CACHE_TTL_MS="0" \
  cargo run -q -p ai_blaise_citus_pool -- serve >"${pool_log}" 2>&1 &
pool_pid="$!"

pool_ready=0
for _ in $(seq 1 120); do
  if ! kill -0 "${pool_pid}" >/dev/null 2>&1; then
    cat "${pool_log}" >&2
    echo "pool proxy exited before auth readiness" >&2
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
  echo "pool proxy did not report upstream-ready readiness for auth smoke" >&2
  exit 1
fi

python3 - "${pool_port}" "${access_token}" <<'PY_AUTH_PG'
import socket
import struct
import sys

pool_port = int(sys.argv[1])
access_token = sys.argv[2]


def pack_startup(parameters):
    body = struct.pack("!I", 196608)
    for key, value in parameters.items():
        body += key.encode("utf-8") + b"\x00" + value.encode("utf-8") + b"\x00"
    body += b"\x00"
    return struct.pack("!I", len(body) + 4) + body


def pack_simple_query(query):
    body = query.encode("utf-8") + b"\x00"
    return b"Q" + struct.pack("!I", len(body) + 4) + body


def read_exact(sock, byte_count):
    chunks = []
    remaining = byte_count
    while remaining:
        chunk = sock.recv(remaining)
        if not chunk:
            raise RuntimeError("unexpected EOF from PostgreSQL server")
        chunks.append(chunk)
        remaining -= len(chunk)
    return b"".join(chunks)


def read_message(sock):
    message_type = read_exact(sock, 1)
    length = struct.unpack("!I", read_exact(sock, 4))[0]
    payload = read_exact(sock, length - 4)
    return message_type, payload


def error_message(payload):
    parts = []
    for field in payload.split(b"\x00"):
        if len(field) > 1:
            parts.append(field[1:].decode("utf-8", errors="replace"))
    return "; ".join(parts) or payload.decode("utf-8", errors="replace")


def data_row(payload):
    column_count = struct.unpack("!H", payload[:2])[0]
    offset = 2
    values = []
    for _ in range(column_count):
        value_length = struct.unpack("!i", payload[offset : offset + 4])[0]
        offset += 4
        if value_length == -1:
            values.append(None)
            continue
        value = payload[offset : offset + value_length]
        offset += value_length
        values.append(value.decode("utf-8", errors="strict"))
    return values


def connect_with_startup(parameters):
    sock = socket.create_connection(("127.0.0.1", pool_port), timeout=10)
    sock.sendall(pack_startup(parameters))
    return sock


def wait_ready(sock):
    while True:
        message_type, payload = read_message(sock)
        if message_type == b"R":
            auth_code = struct.unpack("!I", payload[:4])[0]
            if auth_code != 0:
                raise RuntimeError(f"expected trust auth code 0, got {auth_code}")
        elif message_type == b"E":
            raise RuntimeError(f"startup failed: {error_message(payload)}")
        elif message_type == b"Z":
            return


def simple_query(sock, query):
    sock.sendall(pack_simple_query(query))
    rows = []
    while True:
        message_type, payload = read_message(sock)
        if message_type == b"D":
            rows.append(data_row(payload))
        elif message_type == b"E":
            raise RuntimeError(f"query failed: {error_message(payload)}")
        elif message_type == b"Z":
            return rows

base_params = {
    "user": "postgres",
    "database": "postgres",
    "application_name": "pool_auth_smoke",
}

valid_params = dict(base_params)
valid_params["ai_blaise.tenant_id"] = "tenant-a"
valid_params["ai_blaise.jwt"] = access_token
with connect_with_startup(valid_params) as sock:
    wait_ready(sock)
    rows = simple_query(sock, "SELECT current_setting('application_name')")
    if rows != [["pool_auth_smoke"]]:
        raise RuntimeError(f"unexpected valid-token query rows: {rows!r}")
    sock.sendall(b"X" + struct.pack("!I", 4))

with connect_with_startup(base_params) as sock:
    message_type, payload = read_message(sock)
    if message_type != b"E":
        raise RuntimeError(f"expected missing-token startup ErrorResponse, got {message_type!r}")
    if "auth token is required" not in error_message(payload):
        raise RuntimeError(f"unexpected missing-token error: {error_message(payload)}")

print("pool auth valid-token admission and missing-token fail-closed smoke passed")
PY_AUTH_PG

curl -fsS -X POST -H 'content-type: application/json' \
  --data "{\"token\":\"${access_token}\"}" \
  "${auth_base}/auth/logout" >/dev/null

python3 - "${pool_port}" "${access_token}" <<'PY_AUTH_REVOKED'
import socket
import struct
import sys

pool_port = int(sys.argv[1])
access_token = sys.argv[2]


def pack_startup(parameters):
    body = struct.pack("!I", 196608)
    for key, value in parameters.items():
        body += key.encode("utf-8") + b"\x00" + value.encode("utf-8") + b"\x00"
    body += b"\x00"
    return struct.pack("!I", len(body) + 4) + body


def read_exact(sock, byte_count):
    chunks = []
    remaining = byte_count
    while remaining:
        chunk = sock.recv(remaining)
        if not chunk:
            raise RuntimeError("unexpected EOF from PostgreSQL server")
        chunks.append(chunk)
        remaining -= len(chunk)
    return b"".join(chunks)


def read_message(sock):
    message_type = read_exact(sock, 1)
    length = struct.unpack("!I", read_exact(sock, 4))[0]
    payload = read_exact(sock, length - 4)
    return message_type, payload


def error_message(payload):
    parts = []
    for field in payload.split(b"\x00"):
        if len(field) > 1:
            parts.append(field[1:].decode("utf-8", errors="replace"))
    return "; ".join(parts) or payload.decode("utf-8", errors="replace")

params = {
    "user": "postgres",
    "database": "postgres",
    "application_name": "pool_auth_smoke",
    "ai_blaise.tenant_id": "tenant-a",
    "ai_blaise.jwt": access_token,
}
with socket.create_connection(("127.0.0.1", pool_port), timeout=10) as sock:
    sock.sendall(pack_startup(params))
    message_type, payload = read_message(sock)
    if message_type != b"E":
        raise RuntimeError(f"expected revoked-token startup ErrorResponse, got {message_type!r}")
    message = error_message(payload)
    if "inactive" not in message and "revoked" not in message:
        raise RuntimeError(f"unexpected revoked-token error: {message}")

print("pool auth revoked-token fail-closed smoke passed")
PY_AUTH_REVOKED

metrics="$(curl -fsS "http://127.0.0.1:${admin_port}/metrics")"
if ! printf '%s\n' "${metrics}" | awk '
  /^ai_blaise_citus_pool_auth_verified_connections_total / && $2 >= 1 { verified = 1 }
  /^ai_blaise_citus_pool_auth_rejections_total / && $2 >= 2 { rejected = 1 }
  /^ai_blaise_citus_pool_fail_closed_routes_total / && $2 >= 2 { fail_closed = 1 }
  END { exit verified && rejected && fail_closed ? 0 : 1 }
'; then
  cat "${pool_log}" >&2
  cat "${auth_log}" >&2
  echo "pool metrics did not show auth admission and fail-closed rejections" >&2
  printf '%s\n' "${metrics}" >&2
  exit 1
fi

kill "${pool_pid}" >/dev/null 2>&1 || true
wait "${pool_pid}" >/dev/null 2>&1 || true
pool_pid=""
kill "${auth_pid}" >/dev/null 2>&1 || true
wait "${auth_pid}" >/dev/null 2>&1 || true
auth_pid=""
: >"${pool_log}"
closed_port="$(choose_distinct_port "${postgres_port}" "${pool_port}" "${admin_port}" "${auth_port}")"

AI_BLAISE_POOL_LISTEN_ADDR="127.0.0.1:${pool_port}" \
  AI_BLAISE_POOL_ADMIN_ADDR="127.0.0.1:${admin_port}" \
  AI_BLAISE_POOL_UPSTREAM_ADDR="127.0.0.1:${closed_port}" \
  AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST="127.0.0.0/8" \
  AI_BLAISE_POOL_STARTUP_TIMEOUT_MS="1000" \
  cargo run -q -p ai_blaise_citus_pool -- serve >"${pool_log}" 2>&1 &
pool_pid="$!"

pool_fail_closed_ready=0
readyz_body="$(mktemp -t ai-blaise-pool-fail-closed-readyz.XXXXXX.json)"
for _ in $(seq 1 120); do
  if ! kill -0 "${pool_pid}" >/dev/null 2>&1; then
    cat "${pool_log}" >&2
    echo "pool proxy exited before fail-closed readiness" >&2
    rm -f "${readyz_body}"
    exit 1
  fi
  status_code="$(curl -s -o "${readyz_body}" -w '%{http_code}' "http://127.0.0.1:${admin_port}/readyz" || true)"
  if [[ "${status_code}" == "503" ]] && grep -Fq '"upstream_ready":false' "${readyz_body}"; then
    pool_fail_closed_ready=1
    break
  fi
  sleep 1
done
rm -f "${readyz_body}"

if [[ "${pool_fail_closed_ready}" != "1" ]]; then
  cat "${pool_log}" >&2
  echo "pool proxy did not report fail-closed upstream readiness" >&2
  exit 1
fi

fail_closed_stdout="$(mktemp -t ai-blaise-pool-fail-closed.XXXXXX.out)"
fail_closed_stderr="$(mktemp -t ai-blaise-pool-fail-closed.XXXXXX.err)"
if docker run --rm \
  --network host \
  -e PGPASSWORD=postgres \
  -e PGCONNECT_TIMEOUT=2 \
  "${postgres_image}" \
  psql -h 127.0.0.1 -p "${pool_port}" -U postgres -d postgres -Atqc 'SELECT 1' \
  >"${fail_closed_stdout}" 2>"${fail_closed_stderr}"; then
  cat "${pool_log}" >&2
  cat "${fail_closed_stdout}" >&2
  cat "${fail_closed_stderr}" >&2
  echo "pool fail-closed upstream smoke unexpectedly routed SQL" >&2
  exit 1
fi
rm -f "${fail_closed_stdout}" "${fail_closed_stderr}"

metrics="$(curl -fsS "http://127.0.0.1:${admin_port}/metrics")"
if ! printf '%s\n' "${metrics}" | awk '
  /^ai_blaise_citus_pool_upstream_ready/ && $2 == 0 { upstream_closed = 1 }
  /^ai_blaise_citus_pool_fail_closed_routes_total / && $2 >= 1 { fail_closed = 1 }
  /^ai_blaise_citus_pool_errors_total / && $2 >= 1 { errors = 1 }
  END { exit upstream_closed && fail_closed && errors ? 0 : 1 }
'; then
  cat "${pool_log}" >&2
  echo "pool metrics did not show fail-closed upstream denial" >&2
  printf '%s\n' "${metrics}" >&2
  exit 1
fi

echo "ai_blaise_citus pool proxy smoke passed with ${postgres_image}"
