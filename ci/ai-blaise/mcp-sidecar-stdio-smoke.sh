#!/usr/bin/env bash
set -euo pipefail

# FEATURE: MCP1 MCP2 MCP3 D11

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

python3 <<'PY'
import json
import os
import subprocess
import sys

proc = subprocess.Popen(
    ["cargo", "run", "-q", "-p", "ai_blaise_citus_sidecar_mcp", "--", "serve-stdio"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
)


def raw_request(line_text):
    assert proc.stdin is not None
    assert proc.stdout is not None
    proc.stdin.write(line_text + "\n")
    proc.stdin.flush()
    line = proc.stdout.readline()
    if not line:
        stderr = proc.stderr.read() if proc.stderr is not None else ""
        raise AssertionError(f"mcp-sidecar serve-stdio closed stdout early: {stderr}")
    return json.loads(line)


def request(payload):
    return raw_request(json.dumps(payload, separators=(",", ":")))


try:
    initialize = request(
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "clientInfo": {"name": "ai-blaise-sidecar-smoke", "version": "0"},
            },
        }
    )
    assert initialize["result"]["serverInfo"]["name"] == "ai-blaise-citus-mcp-sidecar"
    assert initialize["result"]["capabilities"]["tools"]["listChanged"] is False

    parse_error = raw_request("{")
    assert parse_error["error"]["code"] == -32700
    assert "parse error" in parse_error["error"]["message"]

    unknown_method = request(
        {"jsonrpc": "2.0", "id": 7, "method": "resources/list"}
    )
    assert unknown_method["error"]["code"] == -32601
    assert "unknown method: resources/list" in unknown_method["error"]["message"]

    invalid_params = request(
        {"jsonrpc": "2.0", "id": 8, "method": "tools/call", "params": []}
    )
    assert invalid_params["error"]["code"] == -32602

    tools = request({"jsonrpc": "2.0", "id": 2, "method": "tools/list"})
    expected_tool_names = {
        "list_shards",
        "list_hypertables",
        "run_explain",
        "rebalance_dry_run",
        "suggest_index",
        "query_with_timeout",
        "current_lag",
        "current_replication_status",
        "tenant_archive",
    }
    tool_names = {tool["name"] for tool in tools["result"]["tools"]}
    assert tool_names == expected_tool_names, sorted(tool_names)
    for tool in tools["result"]["tools"]:
        assert tool["inputSchema"]["type"] == "object"

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


def assert_invalid_database_dependency_fails_closed():
    env = os.environ.copy()
    env["AI_BLAISE_MCP_DATABASE_URL"] = "postgresql://127.0.0.1:1/ai_blaise_missing"
    env["AI_BLAISE_MCP_MAX_ROWS"] = "25"
    db_proc = subprocess.Popen(
        ["cargo", "run", "-q", "-p", "ai_blaise_citus_sidecar_mcp", "--", "serve-stdio"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
    )

    def db_request(payload):
        assert db_proc.stdin is not None
        assert db_proc.stdout is not None
        db_proc.stdin.write(json.dumps(payload, separators=(",", ":")) + "\n")
        db_proc.stdin.flush()
        line = db_proc.stdout.readline()
        if not line:
            stderr = db_proc.stderr.read() if db_proc.stderr is not None else ""
            raise AssertionError(
                f"mcp-sidecar invalid-db process closed stdout early: {stderr}"
            )
        return json.loads(line)

    try:
        failed = db_request(
            {
                "jsonrpc": "2.0",
                "id": 21,
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
        assert failed["result"]["isError"] is True
        fail_text = failed["result"]["content"][0]["text"]
        assert "AI_BLAISE_MCP_DATABASE_URL connection failed" in fail_text, fail_text

        initialize_after_failure = db_request(
            {"jsonrpc": "2.0", "id": 22, "method": "initialize"}
        )
        assert (
            initialize_after_failure["result"]["serverInfo"]["name"]
            == "ai-blaise-citus-mcp-sidecar"
        )
    finally:
        if db_proc.stdin is not None:
            db_proc.stdin.close()
        try:
            db_proc.wait(timeout=20)
        except subprocess.TimeoutExpired:
            db_proc.terminate()
            db_proc.wait(timeout=20)

    if db_proc.returncode != 0:
        stderr = db_proc.stderr.read() if db_proc.stderr is not None else ""
        print(stderr, file=sys.stderr)
        raise SystemExit(db_proc.returncode)


assert_invalid_database_dependency_fails_closed()

print("ai_blaise_citus_sidecar_mcp stdio smoke passed")
PY
