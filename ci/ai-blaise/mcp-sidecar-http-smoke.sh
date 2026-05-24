#!/usr/bin/env bash
set -euo pipefail

# FEATURE: MCP1 MCP2 MCP3 D11

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

python3 <<'PY'
import http.client
import json
import os
import socket
import subprocess
import sys
import time


def free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


port = free_port()
env = os.environ.copy()
env["AI_BLAISE_LISTEN_ADDR"] = f"127.0.0.1:{port}"
proc = subprocess.Popen(
    ["cargo", "run", "-q", "-p", "ai_blaise_citus_sidecar_mcp", "--", "serve"],
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
    env=env,
)


def request(method, path, body=None):
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
    headers = {}
    if body is not None:
        headers["content-type"] = "application/json"
    conn.request(method, path, body=body, headers=headers)
    response = conn.getresponse()
    data = response.read().decode("utf-8")
    conn.close()
    return response.status, data


def raw_http(payload):
    with socket.create_connection(("127.0.0.1", port), timeout=5) as sock:
        sock.sendall(payload)
        sock.shutdown(socket.SHUT_WR)
        chunks = []
        while True:
            chunk = sock.recv(4096)
            if not chunk:
                break
            chunks.append(chunk)
    return b"".join(chunks).decode("utf-8", errors="replace")


try:
    for _ in range(60):
        try:
            status, data = request("GET", "/readyz")
            if status == 200 and '"component":"mcp-sidecar"' in data:
                break
        except OSError:
            pass
        if proc.poll() is not None:
            stderr = proc.stderr.read() if proc.stderr is not None else ""
            raise AssertionError(f"mcp-sidecar serve exited early: {stderr}")
        time.sleep(0.5)
    else:
        raise AssertionError("mcp-sidecar HTTP server did not become ready")

    status, data = request("GET", "/healthz")
    assert status == 200
    assert '"component":"mcp-sidecar"' in data

    status, data = request("GET", "/metrics")
    assert status == 200
    assert 'ai_blaise_sidecar_ready{component="mcp-sidecar"} 1' in data

    payload = json.dumps(
        {"jsonrpc": "2.0", "id": 1, "method": "initialize"},
        separators=(",", ":"),
    )
    status, data = request("POST", "/mcp", payload)
    assert status == 200
    initialize = json.loads(data)
    assert initialize["result"]["serverInfo"]["name"] == "ai-blaise-citus-mcp-sidecar"
    assert initialize["result"]["capabilities"]["tools"]["listChanged"] is False

    status, data = request("POST", "/mcp", "{")
    assert status == 200
    parse_error = json.loads(data)
    assert parse_error["error"]["code"] == -32700
    assert "parse error" in parse_error["error"]["message"]

    unknown_payload = json.dumps(
        {"jsonrpc": "2.0", "id": 7, "method": "resources/list"},
        separators=(",", ":"),
    )
    status, data = request("POST", "/mcp", unknown_payload)
    assert status == 200
    unknown_method = json.loads(data)
    assert unknown_method["error"]["code"] == -32601
    assert "unknown method: resources/list" in unknown_method["error"]["message"]

    tools_payload = json.dumps(
        {"jsonrpc": "2.0", "id": 8, "method": "tools/list"},
        separators=(",", ":"),
    )
    status, data = request("POST", "/mcp", tools_payload)
    assert status == 200
    tools = json.loads(data)
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

    validated_payload = json.dumps(
        {
            "jsonrpc": "2.0",
            "id": 2,
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
        },
        separators=(",", ":"),
    )
    status, data = request("POST", "/mcp", validated_payload)
    assert status == 200
    validated = json.loads(data)
    assert validated["result"]["isError"] is False
    assert "validated query_with_timeout" in validated["result"]["content"][0]["text"]

    denied_payload = json.dumps(
        {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "tenant_archive",
                "arguments": {
                    "tenant_name": "tenant-a",
                    "tenant_id": "tenant-a",
                    "allowed_schemas": ["tenant_a"],
                },
            },
        },
        separators=(",", ":"),
    )
    status, data = request("POST", "/mcp", denied_payload)
    assert status == 200
    denied = json.loads(data)
    assert denied["result"]["isError"] is True
    assert "safe mode denied a destructive tool" in denied["result"]["content"][0]["text"]

    cross_schema_payload = json.dumps(
        {
            "jsonrpc": "2.0",
            "id": 4,
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
        },
        separators=(",", ":"),
    )
    status, data = request("POST", "/mcp", cross_schema_payload)
    assert status == 200
    cross_schema = json.loads(data)
    assert cross_schema["result"]["isError"] is True
    assert "schema tenant_b is outside allowed_schemas" in cross_schema["result"]["content"][0]["text"]

    malformed = raw_http(b"not-http")
    assert malformed.startswith("HTTP/1.1 400 Bad Request"), malformed
    assert "malformed MCP sidecar HTTP request" in malformed

    status, data = request("POST", "/drain")
    assert status == 202
    assert '"accepting_new_work":false' in data

    status, data = request("GET", "/readyz")
    assert status == 503
    assert '"state":"draining"' in data

    status, data = request("GET", "/metrics")
    assert status == 200
    assert 'ai_blaise_sidecar_ready{component="mcp-sidecar"} 0' in data
finally:
    proc.terminate()
    try:
        proc.wait(timeout=20)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=20)

if proc.returncode not in (0, -15):
    stderr = proc.stderr.read() if proc.stderr is not None else ""
    print(stderr, file=sys.stderr)
    raise SystemExit(proc.returncode)

print("ai_blaise_citus_sidecar_mcp HTTP smoke passed")
PY
