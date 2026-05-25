#!/usr/bin/env bash
# Live pg_graphql data-plane smoke for FEATURE: API3.
#
# Starts a PostgreSQL image that contains pg_graphql, creates an RLS-protected
# table, runs the real GraphQL sidecar in live execution mode, and verifies
# POST /graphql/v1 returns table data through graphql.resolve(...) while tenant
# claims are installed as request.jwt.claims.

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if [[ -f "${HOME}/.cargo/env" ]]; then
  source "${HOME}/.cargo/env"
fi

image="${AI_BLAISE_PGGRAPHQL_IMAGE:-ai-blaise-citus-overlay:bundle1-source-smoke-pg17}"
if ! docker image inspect "${image}" >/dev/null 2>&1; then
  echo "graphql-pggraphql-live-smoke: required image ${image} is not present" >&2
  echo "Build it with: REQUIRE_DOCKER=1 BUNDLE1_SOURCE_BUILD=1 bash ci/ai-blaise/sql-extension-smoke.sh" >&2
  exit 1
fi

echo "==> graphql-pggraphql-live-smoke: build GraphQL sidecar"
cargo build -q -p ai_blaise_citus_sidecar_graphql
graphql_bin="${repo_root}/target/debug/ai_blaise_citus_sidecar_graphql"

tmpdir="$(mktemp -d /tmp/graphql-pggraphql-live-smoke.XXXXXX)"
pids=()
container=""
cleanup() {
  for pid in "${pids[@]:-}"; do
    kill "${pid}" >/dev/null 2>&1 || true
  done
  if [[ -n "${container}" ]]; then
    docker rm -f "${container}" >/dev/null 2>&1 || true
  fi
  rm -rf "${tmpdir}"
}
trap cleanup EXIT

ports=$(python3 - <<'PY'
import socket

sockets = []
ports = []
for _ in range(2):
    sock = socket.socket()
    sock.bind(("127.0.0.1", 0))
    sockets.append(sock)
    ports.append(sock.getsockname()[1])
print(*ports)
for sock in sockets:
    sock.close()
PY
)
read -r pg_port graphql_port <<<"${ports}"

container="api3-pggraphql-live-${RANDOM}-$$"
docker run \
  --name "${container}" \
  -p "127.0.0.1:${pg_port}:5432" \
  -e POSTGRES_PASSWORD=postgres \
  -e PGSODIUM_KEY=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef \
  -d "${image}" \
  -c shared_preload_libraries=citus,pgsodium >/dev/null

for _ in $(seq 1 120); do
  if docker exec "${container}" psql -U postgres -Atqc "SELECT 1" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

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

database_url="postgresql://web_anon:web_anon@127.0.0.1:${pg_port}/postgres"
jwt_secret="01234567890123456789012345678901"
AI_BLAISE_GRAPHQL_LIVE_EXECUTION=1 \
  AI_BLAISE_GRAPHQL_DATABASE_URL="${database_url}" \
  AI_BLAISE_GRAPHQL_JWT_SECRET="${jwt_secret}" \
  AI_BLAISE_LISTEN_ADDR="127.0.0.1:${graphql_port}" \
  "${graphql_bin}" serve >"${tmpdir}/graphql.log" 2>&1 &
pids+=("$!")

python3 - "${graphql_port}" "${database_url}" "${jwt_secret}" <<'PY'
import http.client
import json
import sys
import time

port = int(sys.argv[1])
database_url = sys.argv[2]
jwt_secret = sys.argv[3]


def request(method, path, body=None):
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
    headers = {"content-type": "application/json"} if body is not None else {}
    conn.request(method, path, body=body, headers=headers)
    response = conn.getresponse()
    raw = response.read().decode()
    conn.close()
    try:
        parsed = json.loads(raw)
    except json.JSONDecodeError:
        parsed = raw
    return response.status, raw, parsed


deadline = time.time() + 30
while True:
    try:
        status, raw, parsed = request("GET", "/readyz")
        if status == 200 and parsed["ready"] is True:
            break
    except Exception:
        pass
    if time.time() > deadline:
        raise AssertionError("GraphQL sidecar did not become ready")
    time.sleep(0.1)

query = {
    "query": "query { accountCollection { edges { node { id name tenant_id } } } }",
    "jwt_claims": "{\"tenant_id\":\"tenant-a\",\"role\":\"web_anon\"}",
}
status, raw, parsed = request("POST", "/graphql/v1", json.dumps(query))
assert status == 200, (status, raw)
edges = parsed["data"]["accountCollection"]["edges"]
assert edges == [{"node": {"id": 1, "name": "alice", "tenant_id": "tenant-a"}}], parsed
assert "bob" not in raw and "tenant-b" not in raw, raw
assert database_url not in raw and jwt_secret not in raw, raw

query["jwt_claims"] = "{\"tenant_id\":\"tenant-b\",\"role\":\"web_anon\"}"
status, raw, parsed = request("POST", "/graphql/v1", json.dumps(query))
assert status == 200, (status, raw)
edges = parsed["data"]["accountCollection"]["edges"]
assert edges == [{"node": {"id": 2, "name": "bob", "tenant_id": "tenant-b"}}], parsed
assert "alice" not in raw and "tenant-a" not in raw, raw

missing_claim = {"query": "query { accountCollection { edges { node { id } } } }"}
status, raw, parsed = request("POST", "/graphql/v1", json.dumps(missing_claim))
assert status == 400, (status, raw)
assert "request.jwt.claims is missing" in raw, raw

introspection = {
    "query": "query { __schema { types { name } } }",
    "jwt_claims": "{\"tenant_id\":\"tenant-a\"}",
}
status, raw, parsed = request("POST", "/graphql/v1", json.dumps(introspection))
assert status == 400, (status, raw)
assert "introspection is disabled" in raw, raw

status, raw, parsed = request("GET", "/metrics")
assert status == 200, (status, raw)
assert 'ai_blaise_sidecar_ready{component="graphql"} 1' in raw, raw

print("graphql_pggraphql_live=passed")
print("tenant_a_rows=1")
print("tenant_b_rows=1")
print("rls_cross_tenant_hidden=true")
print("graphql_resolve_executed=true")
PY

echo "graphql-pggraphql-live-smoke passed"
