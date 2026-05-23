#!/usr/bin/env bash
set -euo pipefail

# FEATURE: MCP4

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

postgres_image="${MCP_DB_SMOKE_IMAGE:-postgres:17}"
require_docker="${REQUIRE_DOCKER:-0}"

if ! command -v docker >/dev/null 2>&1; then
  if [[ "${require_docker}" == "1" ]]; then
    echo "docker is required for MCP database smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping MCP database smoke"
  exit 0
fi

container="ai-blaise-mcp-db-smoke-${RANDOM}-$$"
cleanup() {
  docker rm -f "${container}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run \
  --name "${container}" \
  -e POSTGRES_HOST_AUTH_METHOD=trust \
  -p 127.0.0.1::5432 \
  -d "${postgres_image}" >/dev/null

ready=0
init_complete=0
for _ in $(seq 1 120); do
  if docker logs "${container}" 2>&1 | grep -Fq 'PostgreSQL init process complete'; then
    init_complete=1
  fi
  if [[ "${init_complete}" == "1" ]] && docker exec "${container}" psql -U postgres -d postgres -Atqc 'SELECT 1' 2>/dev/null | grep -qx '1'; then
    ready=1
    break
  fi
  sleep 1
done

if [[ "${ready}" != "1" ]]; then
  docker logs "${container}" >&2 || true
  echo "MCP database smoke postgres container did not become ready" >&2
  exit 1
fi

port="$(
  docker port "${container}" 5432/tcp |
    awk -F: 'NR == 1 { print $NF }'
)"
if [[ -z "${port}" ]]; then
  echo "could not discover mapped PostgreSQL port for MCP database smoke" >&2
  exit 1
fi

docker exec -i "${container}" psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
CREATE SCHEMA tenant_a;
CREATE TABLE tenant_a.orders (
  id integer PRIMARY KEY,
  tenant_id text NOT NULL,
  total integer NOT NULL
);
INSERT INTO tenant_a.orders VALUES
  (1, 'tenant-a', 10),
  (2, 'tenant-a', 20);

CREATE TABLE public.pg_dist_shard (
  shardid bigint PRIMARY KEY,
  logicalrelid regclass NOT NULL
);
INSERT INTO public.pg_dist_shard VALUES (102008, 'tenant_a.orders'::regclass);
SQL

AI_BLAISE_MCP_DATABASE_URL="postgresql://postgres@127.0.0.1:${port}/postgres" \
AI_BLAISE_MCP_MAX_ROWS=10 \
python3 <<'PY'
import json
import os
import subprocess
import sys

env = os.environ.copy()
proc = subprocess.Popen(
    ["cargo", "run", "-q", "-p", "ai_blaise_citus_mcp", "--", "serve-stdio"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
    env=env,
)


def request(payload):
    assert proc.stdin is not None
    assert proc.stdout is not None
    proc.stdin.write(json.dumps(payload, separators=(",", ":")) + "\n")
    proc.stdin.flush()
    line = proc.stdout.readline()
    if not line:
        stderr = proc.stderr.read() if proc.stderr is not None else ""
        raise AssertionError(f"citus-mcp database serve-stdio closed stdout early: {stderr}")
    return json.loads(line)


try:
    initialize = request({"jsonrpc": "2.0", "id": 1, "method": "initialize"})
    assert initialize["result"]["serverInfo"]["name"] == "ai-blaise-citus-mcp"

    query = request(
        {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "query_with_timeout",
                "arguments": {
                    "sql": "SELECT id, total FROM tenant_a.orders ORDER BY id",
                    "timeout_ms": 1000,
                    "tenant_id": "tenant-a",
                    "allowed_schemas": ["tenant_a"],
                },
            },
        }
    )
    assert query["result"]["isError"] is False
    query_text = query["result"]["content"][0]["text"]
    assert "executed query_with_timeout" in query_text
    assert "rows=2" in query_text
    assert '"total":20' in query_text

    explain = request(
        {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "run_explain",
                "arguments": {
                    "sql": "SELECT * FROM tenant_a.orders WHERE id = 1",
                    "tenant_id": "tenant-a",
                    "allowed_schemas": ["tenant_a"],
                },
            },
        }
    )
    assert explain["result"]["isError"] is False
    assert "executed run_explain" in explain["result"]["content"][0]["text"]

    shards = request(
        {
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "list_shards",
                "arguments": {
                    "tenant_id": "tenant-a",
                    "allowed_schemas": ["tenant_a"],
                },
            },
        }
    )
    assert shards["result"]["isError"] is False
    shards_text = shards["result"]["content"][0]["text"]
    assert "executed list_shards" in shards_text
    assert "102008" in shards_text
    assert "tenant_a.orders" in shards_text

    cross_schema = request(
        {
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "query_with_timeout",
                "arguments": {
                    "sql": "SELECT count(*) FROM tenant_b.orders",
                    "timeout_ms": 1000,
                    "tenant_id": "tenant-a",
                    "allowed_schemas": ["tenant_a"],
                },
            },
        }
    )
    assert cross_schema["result"]["isError"] is True
    assert "schema tenant_b is outside allowed_schemas" in cross_schema["result"]["content"][0]["text"]

    destructive = request(
        {
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "tenant_archive",
                "arguments": {
                    "tenant_name": "tenant-a",
                    "tenant_id": "tenant-a",
                    "allowed_schemas": ["tenant_a"],
                },
            },
        }
    )
    assert destructive["result"]["isError"] is True
    assert "safe mode denied a destructive tool" in destructive["result"]["content"][0]["text"]
finally:
    if proc.stdin is not None:
        proc.stdin.close()
    try:
        proc.wait(timeout=20)
    except subprocess.TimeoutExpired:
        proc.terminate()
        proc.wait(timeout=20)

if proc.returncode != 0:
    stderr = proc.stderr.read() if proc.stderr is not None else ""
    print(stderr, file=sys.stderr)
    raise SystemExit(proc.returncode)

print("ai_blaise_citus_mcp database smoke passed")
PY
