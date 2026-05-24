#!/usr/bin/env bash
set -euo pipefail

# FEATURE: API1 API2 API3 API5 API6 EF1 EF2 EF4 EF5

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


def request(port, method, path, body=None, headers=None):
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
    headers = dict(headers or {})
    if body is not None and "content-type" not in {key.lower() for key in headers}:
        headers["content-type"] = "application/json"
    conn.request(method, path, body=body, headers=headers)
    response = conn.getresponse()
    data = response.read().decode("utf-8")
    conn.close()
    return response.status, data


def start_service(package):
    port = free_port()
    env = os.environ.copy()
    env["AI_BLAISE_LISTEN_ADDR"] = f"127.0.0.1:{port}"
    proc = subprocess.Popen(
        ["cargo", "run", "-q", "-p", package, "--", "serve"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
    )
    return proc, port


def wait_ready(proc, port, component):
    for _ in range(90):
        try:
            status, data = request(port, "GET", "/readyz")
            if status == 200 and f'"component":"{component}"' in data:
                return
        except OSError:
            pass
        if proc.poll() is not None:
            stderr = proc.stderr.read() if proc.stderr is not None else ""
            raise AssertionError(f"{component} exited before readiness: {stderr}")
        time.sleep(0.5)
    raise AssertionError(f"{component} did not become ready")


def stop(proc):
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


def smoke_postgrest():
    proc, port = start_service("ai_blaise_citus_sidecar_postgrest")
    try:
        wait_ready(proc, port, "postgrest")
        status, data = request(port, "GET", "/openapi.json")
        assert status == 200, (status, data)
        assert '"/orders"' in data
        assert '"tenant_claim":"tenant_id"' in data

        status, data = request(port, "GET", "/api/orders")
        assert status == 200, (status, data)
        assert '"table":"orders"' in data
        assert '"distribution_column":"tenant_id"' in data

        status, data = request(port, "GET", "/metrics")
        assert status == 200, (status, data)
        assert 'ai_blaise_sidecar_ready{component="postgrest"} 1' in data
    finally:
        stop(proc)


def smoke_graphql():
    proc, port = start_service("ai_blaise_citus_sidecar_graphql")
    try:
        wait_ready(proc, port, "graphql")
        status, data = request(port, "GET", "/graphql")
        assert status == 200, (status, data)
        assert "ai-blaise GraphQL" in data

        body = json.dumps(
            {
                "query": "query { orderCollection { edges { node { id total } } } }",
                "jwt_claims": '{"tenant_id":"tenant-a","role":"web_anon"}',
            },
            separators=(",", ":"),
        )
        status, data = request(port, "POST", "/graphql/v1", body)
        assert status == 200, (status, data)
        assert '"namespace":"public_api"' in data
        assert '"tenant_id":"tenant-a"' in data

        subscription = json.dumps(
            {
                "query": "subscription { orderInserted { id total } }",
                "jwt_claims": '{"tenant_id":"tenant-a"}',
            },
            separators=(",", ":"),
        )
        status, data = request(port, "POST", "/graphql/ws", subscription)
        assert status == 200, (status, data)
        assert '"transport":"websocket"' in data
        assert '"subscription_field":"orderInserted"' in data
        assert "public_api.public.orders" in data
    finally:
        stop(proc)


def smoke_edge_functions():
    proc, port = start_service("ai_blaise_citus_sidecar_edge_functions")
    try:
        wait_ready(proc, port, "edge-functions")
        status, data = request(port, "GET", "/functions")
        assert status == 200, (status, data)
        assert '"name":"order_created"' in data
        assert '"runtime":"deno"' in data

        body = json.dumps(
            {"tenant_id": "tenant-a", "payload_bytes": 512, "timeout_ms": 500},
            separators=(",", ":"),
        )
        status, data = request(port, "POST", "/functions/order_created", body)
        assert status == 200, (status, data)
        assert '"function":"order_created"' in data
        assert '"status":"succeeded"' in data
        assert '"db_callback_used":true' in data
    finally:
        stop(proc)


smoke_postgrest()
smoke_graphql()
smoke_edge_functions()
print("ai-blaise API trio runtime smoke passed")
PY
