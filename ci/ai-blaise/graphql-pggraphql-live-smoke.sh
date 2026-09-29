#!/usr/bin/env bash
# Live Auth3 -> mTLS -> GraphQL -> pg_graphql tenant-isolation smoke for API3.

set -euo pipefail
umask 077

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if [[ -f "${HOME}/.cargo/env" ]]; then
  source "${HOME}/.cargo/env"
fi

for required_tool in cargo curl docker openssl python3; do
  command -v "${required_tool}" >/dev/null || {
    echo "graphql-pggraphql-live-smoke: ${required_tool} is required" >&2
    exit 1
  }
done

image="${AI_BLAISE_PGGRAPHQL_IMAGE:-ai-blaise-citus-overlay:bundle1-source-smoke-pg17}"
if ! docker image inspect "${image}" >/dev/null 2>&1; then
  echo "graphql-pggraphql-live-smoke: required image ${image} is not present" >&2
  echo "Build it with: REQUIRE_DOCKER=1 BUNDLE1_SOURCE_BUILD=1 bash ci/ai-blaise/sql-extension-smoke.sh" >&2
  exit 1
fi

echo "==> graphql-pggraphql-live-smoke: build Auth3 and GraphQL sidecars"
cargo build -q \
  -p ai_blaise_citus_sidecar_auth \
  -p ai_blaise_citus_sidecar_graphql
target_dir="${CARGO_TARGET_DIR:-${repo_root}/target}"
if [[ "${target_dir}" != /* ]]; then
  target_dir="${repo_root}/${target_dir}"
fi
auth_bin="${target_dir}/debug/ai_blaise_citus_sidecar_auth"
graphql_bin="${target_dir}/debug/ai_blaise_citus_sidecar_graphql"

tmpdir="$(mktemp -d /tmp/graphql-pggraphql-live-smoke.XXXXXX)"
pids=()
container=""
cleanup() {
  exit_status=$?
  if [[ "${exit_status}" != "0" && -f "${tmpdir}/graphql.log" ]]; then
    grep -E '^ai-blaise graphql request rejected: stage=[a-z-]+ category=[a-z-]+$' \
      "${tmpdir}/graphql.log" >&2 || true
  fi
  for pid in "${pids[@]:-}"; do
    kill "${pid}" >/dev/null 2>&1 || true
  done
  for pid in "${pids[@]:-}"; do
    wait "${pid}" >/dev/null 2>&1 || true
  done
  if [[ -n "${container}" ]]; then
    docker rm -fv "${container}" >/dev/null 2>&1 || true
  fi
  rm -rf "${tmpdir}"
}
trap cleanup EXIT

ports="$(python3 - <<'PY'
import socket

sockets = []
ports = []
for _ in range(4):
    sock = socket.socket()
    sock.bind(("127.0.0.1", 0))
    sockets.append(sock)
    ports.append(sock.getsockname()[1])
print(*ports)
for sock in sockets:
    sock.close()
PY
)"
read -r pg_port auth_port auth_tls_port graphql_port <<<"${ports}"

container="api3-pggraphql-live-${RANDOM}-$$"
docker run \
  --name "${container}" \
  -p "127.0.0.1:${pg_port}:5432" \
  -e POSTGRES_PASSWORD=postgres \
  -e PGSODIUM_KEY=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef \
  -d "${image}" >/dev/null

pg_ready=0
for _ in $(seq 1 120); do
  if docker exec "${container}" psql -U postgres -Atqc "SELECT 1" 2>/dev/null | grep -qx '1'; then
    pg_ready=1
    break
  fi
  sleep 1
done
if [[ "${pg_ready}" != "1" ]]; then
  docker logs "${container}" >&2 || true
  echo "graphql-pggraphql-live-smoke: PostgreSQL did not become ready" >&2
  exit 1
fi

docker exec -i "${container}" psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
CREATE EXTENSION IF NOT EXISTS pg_graphql;
CREATE ROLE web_anon LOGIN PASSWORD 'web_anon';
CREATE TABLE public.account(
  id integer PRIMARY KEY,
  name text NOT NULL,
  tenant_id text NOT NULL
);
INSERT INTO public.account VALUES
  (1, 'alice', 'tenant-a'),
  (2, 'bob', 'tenant-b');
ALTER TABLE public.account ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.account FORCE ROW LEVEL SECURITY;
CREATE POLICY account_tenant ON public.account
  FOR SELECT TO web_anon
  USING (
    tenant_id = (
      nullif(current_setting('request.jwt.claims', true), '')::jsonb ->> 'tenant_id'
    )
  );
GRANT USAGE ON SCHEMA public TO web_anon;
GRANT USAGE ON SCHEMA graphql TO web_anon;
GRANT SELECT ON public.account TO web_anon;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA graphql TO web_anon;
SQL

# Exercise the exact prepared-statement signature used by the Rust executor as
# the unprivileged database role before adding Auth3 and HTTP to the path.
docker exec -i "${container}" psql -U postgres -v ON_ERROR_STOP=1 -Atq <<'SQL' |
BEGIN;
SET LOCAL ROLE web_anon;
SET LOCAL request.jwt.claims = '{"tenant_id":"tenant-a","role":"web_anon"}';
PREPARE ai_blaise_graphql_smoke(text, text, text) AS
  SELECT graphql.resolve($1, $2::text::jsonb, $3)::text;
EXECUTE ai_blaise_graphql_smoke(
  'query { accountCollection { edges { node { id name tenant_id } } } }',
  '{}',
  NULL
);
ROLLBACK;
SQL
python3 -c '
import json
import sys

lines = [line for line in sys.stdin.read().splitlines() if line]
assert len(lines) == 1, "database preflight returned an unexpected result shape"
response = json.loads(lines[0])
assert "errors" not in response, "database preflight returned a GraphQL error"
edges = response["data"]["accountCollection"]["edges"]
assert edges == [{"node": {"id": 1, "name": "alice", "tenant_id": "tenant-a"}}]
'

openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout "${tmpdir}/ca.key" -out "${tmpdir}/ca.pem" \
  -days 1 -subj '/CN=ai-blaise-test-ca' >/dev/null 2>&1
openssl req -newkey rsa:2048 -nodes \
  -keyout "${tmpdir}/server.key" -out "${tmpdir}/server.csr" \
  -subj '/CN=127.0.0.1' >/dev/null 2>&1
printf '%s\n' \
  'subjectAltName=IP:127.0.0.1' \
  'extendedKeyUsage=serverAuth' \
  'keyUsage=digitalSignature,keyEncipherment' >"${tmpdir}/server.ext"
openssl x509 -req -in "${tmpdir}/server.csr" \
  -CA "${tmpdir}/ca.pem" -CAkey "${tmpdir}/ca.key" -CAcreateserial \
  -out "${tmpdir}/server.pem" -days 1 -sha256 \
  -extfile "${tmpdir}/server.ext" >/dev/null 2>&1
openssl req -newkey rsa:2048 -nodes \
  -keyout "${tmpdir}/client.key" -out "${tmpdir}/client.csr" \
  -subj '/CN=graphql-sidecar' >/dev/null 2>&1
printf '%s\n' \
  'extendedKeyUsage=clientAuth' \
  'keyUsage=digitalSignature' >"${tmpdir}/client.ext"
openssl x509 -req -in "${tmpdir}/client.csr" \
  -CA "${tmpdir}/ca.pem" -CAkey "${tmpdir}/ca.key" -CAcreateserial \
  -out "${tmpdir}/client.crt" -days 1 -sha256 \
  -extfile "${tmpdir}/client.ext" >/dev/null 2>&1
cp "${tmpdir}/client.crt" "${tmpdir}/client.pem"
printf '\n' >>"${tmpdir}/client.pem"
sed -n '1,$p' "${tmpdir}/client.key" >>"${tmpdir}/client.pem"

auth_secret='auth-graphql-live-secret-32-byte-minimum-material'
AI_BLAISE_LISTEN_ADDR="127.0.0.1:${auth_port}" \
AI_BLAISE_AUTH_ISSUER='https://auth.example.com' \
AI_BLAISE_AUTH_AUDIENCE='postgres' \
AI_BLAISE_AUTH_TTL_SECONDS='300' \
AI_BLAISE_AUTH_HS256_SECRET="${auth_secret}" \
"${auth_bin}" serve >"${tmpdir}/auth.log" 2>&1 &
pids+=("$!")

auth_ready=0
for _ in $(seq 1 80); do
  if curl -fsS --max-time 2 "http://127.0.0.1:${auth_port}/readyz" >/dev/null 2>&1; then
    auth_ready=1
    break
  fi
  if ! kill -0 "${pids[0]}" >/dev/null 2>&1; then
    echo "graphql-pggraphql-live-smoke: Auth3 exited before readiness" >&2
    exit 1
  fi
  sleep 0.25
done
if [[ "${auth_ready}" != "1" ]]; then
  echo "graphql-pggraphql-live-smoke: Auth3 did not become ready" >&2
  exit 1
fi

python3 - "${auth_port}" "${tmpdir}/tenant-a.token" "${tmpdir}/tenant-b.token" <<'PY'
import http.client
import json
import sys

port = int(sys.argv[1])

def post(path, payload, expected):
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
    body = json.dumps(payload, separators=(",", ":"))
    connection.request("POST", path, body=body, headers={"content-type": "application/json"})
    response = connection.getresponse()
    raw = response.read().decode("utf-8")
    connection.close()
    assert response.status == expected, (response.status, path)
    return json.loads(raw)

for username, tenant, token_path in [
    ("alice", "tenant-a", sys.argv[2]),
    ("bob", "tenant-b", sys.argv[3]),
]:
    post(
        "/auth/users",
        {
            "username": username,
            "password": "correct-horse-battery-staple",
            "role": "web_anon",
            "tenant_id": tenant,
        },
        201,
    )
    login = post(
        "/auth/login",
        {"username": username, "password": "correct-horse-battery-staple"},
        200,
    )
    with open(token_path, "w", encoding="ascii") as handle:
        handle.write(login["access_token"])
PY

python3 - \
  "${auth_tls_port}" "${auth_port}" \
  "${tmpdir}/server.pem" "${tmpdir}/server.key" "${tmpdir}/ca.pem" \
  "${tmpdir}/proxy.ready" <<'PY' >"${tmpdir}/proxy.log" 2>&1 &
import signal
import socket
import ssl
import sys

listen_port = int(sys.argv[1])
upstream_port = int(sys.argv[2])
server_cert, server_key, ca_cert, ready_path = sys.argv[3:]
running = True

def stop(_signum, _frame):
    global running
    running = False

signal.signal(signal.SIGTERM, stop)
context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
context.minimum_version = ssl.TLSVersion.TLSv1_2
context.load_cert_chain(server_cert, server_key)
context.load_verify_locations(cafile=ca_cert)
context.verify_mode = ssl.CERT_REQUIRED

listener = socket.socket()
listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
listener.bind(("127.0.0.1", listen_port))
listener.listen(8)
listener.settimeout(0.5)
with open(ready_path, "w", encoding="ascii") as handle:
    handle.write("ready\n")

def receive_http(connection):
    data = bytearray()
    while len(data) <= 65536:
        chunk = connection.recv(8192)
        if not chunk:
            break
        data.extend(chunk)
        marker = data.find(b"\r\n\r\n")
        if marker < 0:
            continue
        header = data[:marker].decode("ascii")
        content_length = 0
        for line in header.split("\r\n")[1:]:
            name, _, value = line.partition(":")
            if name.lower() == "content-length":
                content_length = int(value.strip())
        if len(data) >= marker + 4 + content_length:
            return bytes(data)
    raise ValueError("invalid or oversized HTTP request")

while running:
    try:
        raw_connection, _ = listener.accept()
    except socket.timeout:
        continue
    try:
        with context.wrap_socket(raw_connection, server_side=True) as incoming:
            request = receive_http(incoming)
            with socket.create_connection(("127.0.0.1", upstream_port), timeout=3) as upstream:
                upstream.sendall(request)
                upstream.shutdown(socket.SHUT_WR)
                while True:
                    chunk = upstream.recv(8192)
                    if not chunk:
                        break
                    incoming.sendall(chunk)
    except (OSError, ssl.SSLError, ValueError):
        raw_connection.close()
listener.close()
PY
pids+=("$!")

for _ in $(seq 1 50); do
  [[ -f "${tmpdir}/proxy.ready" ]] && break
  if ! kill -0 "${pids[1]}" >/dev/null 2>&1; then
    echo "graphql-pggraphql-live-smoke: mTLS proxy exited before readiness" >&2
    exit 1
  fi
  sleep 0.1
done
[[ -f "${tmpdir}/proxy.ready" ]] || {
  echo "graphql-pggraphql-live-smoke: mTLS proxy did not become ready" >&2
  exit 1
}

# The TLS hop must reject clients that present no certificate.
if curl -fsS --max-time 3 --cacert "${tmpdir}/ca.pem" \
  -H 'content-type: application/json' \
  --data '{"token":"must-not-reach-auth3"}' \
  "https://127.0.0.1:${auth_tls_port}/auth/introspect" \
  >"${tmpdir}/unexpected-no-client-cert.out" 2>/dev/null; then
  echo "graphql-pggraphql-live-smoke: mTLS endpoint admitted a client without a certificate" >&2
  exit 1
fi

# Prove the mutually authenticated hop reaches the real Auth3 introspection
# endpoint before attributing any later closed failure to GraphQL or PostgreSQL.
# The bearer stays in mode-0600 files and never appears in argv or command logs.
python3 - "${tmpdir}/tenant-a.token" "${tmpdir}/introspection-request.json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="ascii") as handle:
    token = handle.read()
with open(sys.argv[2], "w", encoding="ascii") as handle:
    json.dump({"token": token}, handle, separators=(",", ":"))
PY
curl -fsS --max-time 3 \
  --cacert "${tmpdir}/ca.pem" \
  --cert "${tmpdir}/client.crt" \
  --key "${tmpdir}/client.key" \
  -H 'content-type: application/json' \
  --data-binary "@${tmpdir}/introspection-request.json" \
  "https://127.0.0.1:${auth_tls_port}/auth/introspect" \
  >"${tmpdir}/introspection-response.json"
python3 - "${tmpdir}/introspection-response.json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    response = json.load(handle)
assert response["active"] is True, "mTLS introspection did not return an active identity"
assert response["tenant_id"] == "tenant-a", "mTLS introspection returned the wrong tenant"
assert response["role"] == "web_anon", "mTLS introspection returned the wrong role"
assert response["iss"] == "https://auth.example.com", "mTLS introspection returned the wrong issuer"
assert response["aud"] == "postgres", "mTLS introspection returned the wrong audience"
PY

database_url="postgresql://web_anon:web_anon@127.0.0.1:${pg_port}/postgres"
AI_BLAISE_GRAPHQL_LIVE_EXECUTION=1 \
AI_BLAISE_GRAPHQL_DATABASE_URL="${database_url}" \
AI_BLAISE_GRAPHQL_AUTH_INTROSPECTION_URL="https://127.0.0.1:${auth_tls_port}/auth/introspect" \
AI_BLAISE_GRAPHQL_AUTH_CA_CERT_PATH="${tmpdir}/ca.pem" \
AI_BLAISE_GRAPHQL_AUTH_CLIENT_IDENTITY_PATH="${tmpdir}/client.pem" \
AI_BLAISE_GRAPHQL_AUTH_EXPECTED_ISSUER='https://auth.example.com' \
AI_BLAISE_GRAPHQL_AUTH_EXPECTED_AUDIENCE='postgres' \
AI_BLAISE_GRAPHQL_AUTH_TIMEOUT_MS='2000' \
AI_BLAISE_LISTEN_ADDR="127.0.0.1:${graphql_port}" \
"${graphql_bin}" serve >"${tmpdir}/graphql.log" 2>&1 &
pids+=("$!")

python3 - \
  "${graphql_port}" "${auth_port}" \
  "${tmpdir}/tenant-a.token" "${tmpdir}/tenant-b.token" <<'PY'
import http.client
import json
import sys
import time

graphql_port = int(sys.argv[1])
auth_port = int(sys.argv[2])
with open(sys.argv[3], encoding="ascii") as handle:
    tenant_a_token = handle.read()
with open(sys.argv[4], encoding="ascii") as handle:
    tenant_b_token = handle.read()

def request(port, method, path, body=None, token=None):
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
    headers = {}
    if body is not None:
        headers["content-type"] = "application/json"
    if token is not None:
        headers["authorization"] = f"Bearer {token}"
    connection.request(method, path, body=body, headers=headers)
    response = connection.getresponse()
    raw = response.read().decode("utf-8")
    connection.close()
    return response.status, raw

deadline = time.time() + 30
while True:
    try:
        status, raw = request(graphql_port, "GET", "/readyz")
        if status == 200 and json.loads(raw)["ready"] is True:
            break
    except (OSError, ValueError, KeyError):
        pass
    if time.time() > deadline:
        raise AssertionError("GraphQL sidecar did not become ready")
    time.sleep(0.1)

query = {
    "query": "query { accountCollection { edges { node { id name tenant_id } } } }"
}
query_body = json.dumps(query, separators=(",", ":"))
status, raw = request(graphql_port, "POST", "/graphql/v1", query_body, tenant_a_token)
assert status == 200, (status, raw)
edges = json.loads(raw)["data"]["accountCollection"]["edges"]
assert edges == [{"node": {"id": 1, "name": "alice", "tenant_id": "tenant-a"}}], edges
assert "bob" not in raw and "tenant-b" not in raw, raw

status, raw = request(graphql_port, "POST", "/graphql/v1", query_body, tenant_b_token)
assert status == 200, (status, raw)
edges = json.loads(raw)["data"]["accountCollection"]["edges"]
assert edges == [{"node": {"id": 2, "name": "bob", "tenant_id": "tenant-b"}}], edges
assert "alice" not in raw and "tenant-a" not in raw, raw

status, raw = request(graphql_port, "POST", "/graphql/v1", query_body)
assert status == 401 and "authentication rejected" in raw, (status, raw)

forged = dict(query)
forged["jwt_claims"] = '{"tenant_id":"tenant-b"}'
status, raw = request(
    graphql_port,
    "POST",
    "/graphql/v1",
    json.dumps(forged, separators=(",", ":")),
    tenant_a_token,
)
assert status == 400 and "identity claims are forbidden" in raw, (status, raw)
assert "tenant-b" not in raw and tenant_a_token not in raw, raw

introspection = json.dumps(
    {"query": "query { __schema { types { name } } }"}, separators=(",", ":")
)
status, raw = request(
    graphql_port, "POST", "/graphql/v1", introspection, tenant_a_token
)
assert status == 400 and "introspection disabled" in raw, (status, raw)

subscription = json.dumps(
    {"query": "subscription { orderInserted { id total } }"},
    separators=(",", ":"),
)
status, raw = request(
    graphql_port, "POST", "/graphql/ws", subscription, tenant_a_token
)
assert status == 501 and "subscription transport unavailable" in raw, (status, raw)

logout_body = json.dumps({"token": tenant_a_token}, separators=(",", ":"))
status, raw = request(auth_port, "POST", "/auth/logout", logout_body)
assert status == 200, (status, raw)
status, raw = request(graphql_port, "POST", "/graphql/v1", query_body, tenant_a_token)
assert status == 401 and "authentication rejected" in raw, (status, raw)
assert tenant_a_token not in raw, raw

status, raw = request(graphql_port, "GET", "/metrics")
assert status == 200, (status, raw)
assert 'ai_blaise_sidecar_ready{component="graphql"} 1' in raw, raw

print("graphql_pggraphql_live=passed")
print("auth3_introspection=true")
print("mtls_client_certificate_required=true")
print("tenant_a_rows=1")
print("tenant_b_rows=1")
print("rls_cross_tenant_hidden=true")
print("body_identity_rejected=true")
print("revoked_token_rejected=true")
print("graphql_resolve_executed=true")
PY

echo "graphql-pggraphql-live-smoke passed"
