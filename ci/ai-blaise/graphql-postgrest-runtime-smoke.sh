#!/usr/bin/env bash
set -euo pipefail

# FEATURE: API1 API2 API3 API5 API6
# Focused runtime smoke for the GraphQL and PostgREST sidecar front doors.
# This proves live process/socket behavior and dependency fail-closed contracts;
# it does not prove table-backed upstream PostgREST or pg_graphql execution.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

python3 <<'PY_SMOKE'
import http.client
import json
import os
import shutil
import socket
import subprocess
import sys
import time

TARGET_DIR = os.environ.get("CARGO_TARGET_DIR", os.path.join(os.getcwd(), "target"))
if not os.path.isabs(TARGET_DIR):
    TARGET_DIR = os.path.abspath(TARGET_DIR)

POSTGREST_PKG = "ai_blaise_citus_sidecar_postgrest"
GRAPHQL_PKG = "ai_blaise_citus_sidecar_graphql"
JWT_SECRET = "01234567890123456789012345678901"
POSTGRES_URL = "postgresql://postgres@127.0.0.1/postgres"


def fail(message):
    raise AssertionError(message)


def require_tool(name):
    if shutil.which(name) is None:
        fail(f"{name} is required for GraphQL/PostgREST runtime smoke")


def binary_path(package):
    return os.path.join(TARGET_DIR, "debug", package)


def require_binary(package):
    path = binary_path(package)
    if not os.path.isfile(path) or not os.access(path, os.X_OK):
        fail(f"built sidecar binary is missing or not executable: {path}")
    return path


def run(args, *, env=None, timeout=120):
    result = subprocess.run(
        args,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
        timeout=timeout,
    )
    if result.returncode != 0:
        sys.stderr.write(result.stdout)
        sys.stderr.write(result.stderr)
        fail(f"command failed with exit {result.returncode}: {' '.join(args)}")
    return result.stdout


def run_expect_failure(args, expected, *, env=None, timeout=60):
    result = subprocess.run(
        args,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
        timeout=timeout,
    )
    if result.returncode == 0:
        sys.stderr.write(result.stdout)
        fail(f"command unexpectedly succeeded: {' '.join(args)}")
    combined = result.stdout + result.stderr
    if expected not in combined:
        sys.stderr.write(result.stdout)
        sys.stderr.write(result.stderr)
        fail(f"command output did not contain {expected!r}: {' '.join(args)}")
    return combined


def parse_tsv(stdout):
    lines = [line for line in stdout.splitlines() if line.strip()]
    if len(lines) != 2:
        fail(f"expected a two-line TSV report, got {len(lines)} lines: {stdout!r}")
    header = lines[0].split("\t")
    row = lines[1].split("\t")
    if len(header) != len(row):
        fail(f"TSV header/row length mismatch: {stdout!r}")
    return dict(zip(header, row))


def free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def request(port, method, path, body=None, headers=None):
    headers = dict(headers or {})
    if body is not None and "content-type" not in {key.lower() for key in headers}:
        headers["content-type"] = "application/json"
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
    conn.request(method, path, body=body, headers=headers)
    response = conn.getresponse()
    response_body = response.read().decode("utf-8")
    conn.close()
    return response.status, response_body


def raw_request(port, payload):
    with socket.create_connection(("127.0.0.1", port), timeout=5) as sock:
        sock.sendall(payload)
        sock.shutdown(socket.SHUT_WR)
        return sock.recv(8192).decode("utf-8", errors="replace")


def wait_ready(proc, port, component):
    for _ in range(80):
        try:
            status, body = request(port, "GET", "/readyz")
            if status == 200 and f'"component":"{component}"' in body:
                return
        except OSError:
            pass
        if proc.poll() is not None:
            stderr = proc.stderr.read() if proc.stderr is not None else ""
            fail(f"{component} serve exited before readiness: {stderr}")
        time.sleep(0.25)
    fail(f"{component} HTTP server did not become ready")


def terminate(proc):
    proc.terminate()
    try:
        proc.wait(timeout=20)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=20)
    if proc.returncode not in (0, -15):
        stderr = proc.stderr.read() if proc.stderr is not None else ""
        fail(f"sidecar process exited with {proc.returncode}: {stderr}")


def start_server(binary, component):
    port = free_port()
    env = os.environ.copy()
    env["AI_BLAISE_LISTEN_ADDR"] = f"127.0.0.1:{port}"
    proc = subprocess.Popen(
        [binary, "serve"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
    )
    wait_ready(proc, port, component)
    return proc, port


def smoke_common_runtime(binary, component):
    proc, port = start_server(binary, component)
    try:
        status, body = request(port, "GET", "/healthz")
        assert status == 200, body
        assert f'"component":"{component}"' in body, body
        assert '"ready":true' in body, body

        status, body = request(port, "GET", "/metrics")
        assert status == 200, body
        assert f'ai_blaise_sidecar_ready{{component="{component}"}} 1' in body, body

        response = raw_request(port, b"not-http")
        assert response.startswith("HTTP/1.1 400"), response
        assert "malformed" in response, response

        status, body = request(port, "PUT", "/readyz")
        assert status == 405, body
        assert "method not allowed" in body, body

        status, body = request(port, "GET", "/does-not-exist")
        assert status == 404, body
        assert "not found" in body, body

        status, body = request(port, "POST", "/drain")
        assert status == 202, body
        assert '"accepting_new_work":false' in body, body

        status, body = request(port, "GET", "/readyz")
        assert status == 503, body
        assert '"state":"draining"' in body, body

        status, body = request(port, "GET", "/metrics")
        assert status == 200, body
        assert f'ai_blaise_sidecar_ready{{component="{component}"}} 0' in body, body
    finally:
        terminate(proc)


def smoke_postgrest(binary):
    missing = os.environ.copy()
    for key in ("POSTGREST_DB_URI", "POSTGREST_JWT_SECRET", "AI_BLAISE_POSTGREST_BINARY"):
        missing.pop(key, None)
    run_expect_failure([binary, "check-runtime-dependencies"], "missing runtime dependency: POSTGREST_DB_URI", env=missing)

    invalid = missing.copy()
    invalid.update({
        "POSTGREST_DB_URI": "http://postgres",
        "POSTGREST_JWT_SECRET": JWT_SECRET,
        "AI_BLAISE_POSTGREST_BINARY": binary,
    })
    run_expect_failure([binary, "check-runtime-dependencies"], "must be a PostgreSQL URL", env=invalid)

    short_secret = missing.copy()
    short_secret.update({
        "POSTGREST_DB_URI": POSTGRES_URL,
        "POSTGREST_JWT_SECRET": "too-short",
        "AI_BLAISE_POSTGREST_BINARY": binary,
    })
    run_expect_failure([binary, "check-runtime-dependencies"], "must be at least 32 bytes", env=short_secret)

    valid = missing.copy()
    valid.update({
        "POSTGREST_DB_URI": POSTGRES_URL,
        "POSTGREST_JWT_SECRET": JWT_SECRET,
        "AI_BLAISE_POSTGREST_BINARY": binary,
    })
    report = parse_tsv(run([binary, "check-runtime-dependencies"], env=valid))
    assert report["db_uri_env"] == "POSTGREST_DB_URI", report
    assert report["jwt_secret_env"] == "POSTGREST_JWT_SECRET", report
    assert report["schemas"] == "public,api", report
    assert report["route_count"] == "1", report

    proc, port = start_server(binary, "postgrest")
    try:
        status, body = request(port, "GET", "/openapi.json")
        assert status == 200, body
        openapi = json.loads(body)
        assert openapi["openapi"] == "3.0.0", openapi
        assert openapi["info"] == {"title": "ai-blaise Citus API", "version": "v1alpha1"}, openapi
        assert openapi["x-ai-blaise"]["schemas"] == ["public", "api"], openapi
        assert openapi["x-ai-blaise"]["rls_required"] is True, openapi
        assert openapi["x-ai-blaise"]["tenant_claim"] == "tenant_id", openapi
        orders = openapi["paths"]["/orders"]
        assert sorted(orders) == ["get", "post"], orders
        assert orders["get"]["tags"] == ["public.orders"], orders
        assert orders["post"]["summary"] == "POST public.orders", orders
        assert POSTGRES_URL not in body, body
        assert JWT_SECRET not in body, body

        status, body = request(port, "GET", "/postgrest.conf")
        assert status == 200, body
        assert 'db-uri = "env:POSTGREST_DB_URI"' in body, body
        assert 'jwt-secret = "env:POSTGREST_JWT_SECRET"' in body, body
        assert POSTGRES_URL not in body, body
        assert JWT_SECRET not in body, body

        status, body = request(port, "GET", "/api/orders")
        assert status == 200, body
        assert '"schema":"public"' in body, body
        assert '"method":"GET"' in body, body
        assert '"distribution_column":"tenant_id"' in body, body
        assert '"allowed_methods":["GET","POST"]' in body, body

        status, body = request(port, "POST", "/api/public/orders", body="{}")
        assert status == 200, body
        assert '"schema":"public"' in body, body
        assert '"method":"POST"' in body, body

        status, body = request(port, "PUT", "/api/orders")
        assert status == 405, body
        assert "method not allowed for route" in body, body

        status, body = request(port, "GET", "/api/missing")
        assert status == 404, body
        assert "no route configured" in body, body
    finally:
        terminate(proc)

    smoke_common_runtime(binary, "postgrest")


def smoke_graphql(binary):
    missing = os.environ.copy()
    for key in ("AI_BLAISE_GRAPHQL_DATABASE_URL", "AI_BLAISE_GRAPHQL_JWT_SECRET"):
        missing.pop(key, None)
    run_expect_failure([binary, "check-runtime-dependencies"], "missing runtime dependency: AI_BLAISE_GRAPHQL_DATABASE_URL", env=missing)

    invalid = missing.copy()
    invalid.update({
        "AI_BLAISE_GRAPHQL_DATABASE_URL": "http://postgres",
        "AI_BLAISE_GRAPHQL_JWT_SECRET": JWT_SECRET,
    })
    run_expect_failure([binary, "check-runtime-dependencies"], "must be a PostgreSQL URL", env=invalid)

    short_secret = missing.copy()
    short_secret.update({
        "AI_BLAISE_GRAPHQL_DATABASE_URL": POSTGRES_URL,
        "AI_BLAISE_GRAPHQL_JWT_SECRET": "too-short",
    })
    run_expect_failure([binary, "check-runtime-dependencies"], "must be at least 32 bytes", env=short_secret)

    valid = missing.copy()
    valid.update({
        "AI_BLAISE_GRAPHQL_DATABASE_URL": POSTGRES_URL,
        "AI_BLAISE_GRAPHQL_JWT_SECRET": JWT_SECRET,
    })
    report = parse_tsv(run([binary, "check-runtime-dependencies"], env=valid))
    assert report["database_url_env"] == "AI_BLAISE_GRAPHQL_DATABASE_URL", report
    assert report["jwt_secret_env"] == "AI_BLAISE_GRAPHQL_JWT_SECRET", report
    assert report["endpoint"] == "/graphql/v1", report
    assert report["pg_graphql_required"] == "true", report

    proc, port = start_server(binary, "graphql")
    try:
        status, body = request(port, "GET", "/graphql")
        assert status == 200, body
        assert "ai-blaise GraphQL" in body, body
        assert "/graphql/v1" in body, body

        query = '{"query":"query { orderCollection { edges { node { id } } } }","jwt_claims":"{\\"tenant_id\\":\\"tenant-a\\"}"}'
        status, body = request(port, "POST", "/graphql/v1", body=query)
        assert status == 200, body
        assert '"namespace":"public_api"' in body, body
        assert '"tenant_id":"tenant-a"' in body, body

        missing_claim = '{"query":"query { orderCollection { edges { node { id } } } }"}'
        status, body = request(port, "POST", "/graphql/v1", body=missing_claim)
        assert status == 400, body
        assert "request.jwt.claims is missing" in body, body
        assert '"errors"' in body, body

        introspection = '{"query":"query { __schema { types { name } } }","jwt_claims":"{\\"tenant_id\\":\\"tenant-a\\"}"}'
        status, body = request(port, "POST", "/graphql/v1", body=introspection)
        assert status == 400, body
        assert "introspection is disabled" in body, body

        malformed = '{"variables":{}}'
        status, body = request(port, "POST", "/graphql/v1", body=malformed)
        assert status == 400, body
        assert "missing query field" in body, body

        subscription = '{"query":"subscription { orderInserted { id total } }","jwt_claims":"{\\"tenant_id\\":\\"tenant-a\\"}"}'
        status, body = request(port, "POST", "/graphql/ws", body=subscription)
        assert status == 200, body
        assert '"transport":"websocket"' in body, body
        assert '"subscription_field":"orderInserted"' in body, body

        status, body = request(port, "GET", "/graphql/ws")
        assert status == 426, body
        assert "upgrade required" in body, body
    finally:
        terminate(proc)

    smoke_common_runtime(binary, "graphql")


require_tool("cargo")
run(["cargo", "build", "-q", "-p", POSTGREST_PKG, "-p", GRAPHQL_PKG], timeout=300)
postgrest_binary = require_binary(POSTGREST_PKG)
graphql_binary = require_binary(GRAPHQL_PKG)
smoke_postgrest(postgrest_binary)
smoke_graphql(graphql_binary)
print("ai_blaise_citus GraphQL/PostgREST runtime smoke passed")
PY_SMOKE
