#!/usr/bin/env bash
set -euo pipefail

# FEATURE: MCP1 MCP2 MCP3 D11

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

python3 <<'PY'
import json
import subprocess
import sys

proc = subprocess.Popen(
    ["cargo", "run", "-q", "-p", "ai_blaise_citus_mcp", "--", "serve-stdio"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
)


def request(payload):
    assert proc.stdin is not None
    assert proc.stdout is not None
    proc.stdin.write(json.dumps(payload, separators=(",", ":")) + "\n")
    proc.stdin.flush()
    line = proc.stdout.readline()
    if not line:
        stderr = proc.stderr.read() if proc.stderr is not None else ""
        raise AssertionError(f"citus-mcp serve-stdio closed stdout early: {stderr}")
    return json.loads(line)


try:
    initialize = request(
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "clientInfo": {"name": "ai-blaise-smoke", "version": "0"},
            },
        }
    )
    assert initialize["result"]["serverInfo"]["name"] == "ai-blaise-citus-mcp"
    assert initialize["result"]["capabilities"]["tools"]["listChanged"] is False

    tools = request({"jsonrpc": "2.0", "id": 2, "method": "tools/list"})
    tool_names = {tool["name"] for tool in tools["result"]["tools"]}
    for expected in {
        "list_shards",
        "query_with_timeout",
        "rebalance_dry_run",
        "tenant_archive",
    }:
        assert expected in tool_names, f"tools/list missing {expected}: {sorted(tool_names)}"

    validated = request(
        {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "query_with_timeout",
                "arguments": {
                    "sql": "SELECT count(*) FROM tenant_a.orders",
                    "timeout_ms": 1000,
                    "tenant_id": "tenant-a",
                    "allowed_schemas": ["tenant_a"],
                },
            },
        }
    )
    assert validated["result"]["isError"] is False
    assert "validated query_with_timeout" in validated["result"]["content"][0]["text"]
    assert "tenant-a" in validated["result"]["content"][0]["text"]

    denied = request(
        {
            "jsonrpc": "2.0",
            "id": 4,
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
    assert denied["result"]["isError"] is True
    assert "safe mode denied a destructive tool" in denied["result"]["content"][0]["text"]

    missing_scope = request(
        {
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "query_with_timeout",
                "arguments": {
                    "sql": "SELECT 1",
                    "timeout_ms": 1000,
                },
            },
        }
    )
    assert missing_scope["result"]["isError"] is True
    assert "tenant_scope is required" in missing_scope["result"]["content"][0]["text"]

    cross_schema = request(
        {
            "jsonrpc": "2.0",
            "id": 6,
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

print("ai_blaise_citus_mcp stdio smoke passed")
PY
