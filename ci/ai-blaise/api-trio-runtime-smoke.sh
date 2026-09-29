#!/usr/bin/env bash
set -euo pipefail

# FEATURE: API1 API2 API3 API5 API6 EF1 EF2 EF4 EF5
# GraphQL is exercised here only as a fail-closed startup member of the trio;
# its authenticated Auth3/mTLS data plane is covered by the dedicated live
# pg_graphql smoke.

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
    env = os.environ.copy()
    for name in (
        "AI_BLAISE_GRAPHQL_DATABASE_URL",
        "AI_BLAISE_GRAPHQL_LIVE_EXECUTION",
        "AI_BLAISE_GRAPHQL_AUTH_INTROSPECTION_URL",
        "AI_BLAISE_GRAPHQL_AUTH_CA_CERT_PATH",
        "AI_BLAISE_GRAPHQL_AUTH_CLIENT_IDENTITY_PATH",
        "AI_BLAISE_GRAPHQL_AUTH_EXPECTED_ISSUER",
        "AI_BLAISE_GRAPHQL_AUTH_EXPECTED_AUDIENCE",
        "AI_BLAISE_GRAPHQL_AUTH_TIMEOUT_MS",
    ):
        env.pop(name, None)
    result = subprocess.run(
        ["cargo", "run", "-q", "-p", "ai_blaise_citus_sidecar_graphql", "--", "serve"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
        timeout=60,
    )
    assert result.returncode != 0, result.stdout
    assert (
        "missing runtime dependency: AI_BLAISE_GRAPHQL_AUTH_INTROSPECTION_URL"
        in result.stderr
    ), result.stderr


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
        assert '"status":"planned"' in data
        assert '"execution_mode":"plan_only"' in data
        assert '"db_callback_used":true' in data
    finally:
        stop(proc)


smoke_postgrest()
smoke_graphql()
smoke_edge_functions()
print("ai-blaise API trio runtime smoke passed")
PY
