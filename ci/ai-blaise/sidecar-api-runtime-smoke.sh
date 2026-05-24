#!/usr/bin/env bash
set -euo pipefail

# FEATURE: API1 API2 API3 API4 API5 API6 EF1 EF2 EF4 EF5
# Bounded process/socket smoke for sidecar API runtimes whose feature tests are
# otherwise canonical/model-only.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

python3 <<'PY'
import http.client
import os
import shutil
import socket
import subprocess
import sys
import time

TARGET_DIR = os.environ.get("CARGO_TARGET_DIR", os.path.join(os.getcwd(), "target"))
if not os.path.isabs(TARGET_DIR):
    TARGET_DIR = os.path.abspath(TARGET_DIR)

COMPONENTS = [
    {
        "label": "postgrest",
        "package": "ai_blaise_citus_sidecar_postgrest",
        "component": "postgrest",
    },
    {
        "label": "graphql",
        "package": "ai_blaise_citus_sidecar_graphql",
        "component": "graphql",
    },
    {
        "label": "edge-functions",
        "package": "ai_blaise_citus_sidecar_edge_functions",
        "component": "edge-functions",
    },
]


def binary_path(package):
    return os.path.join(TARGET_DIR, "debug", package)


def require_binary(package):
    path = binary_path(package)
    if not os.path.isfile(path) or not os.access(path, os.X_OK):
        fail(f"built sidecar binary is missing or not executable: {path}")
    return path


def fail(message):
    raise AssertionError(message)


def require_tool(name):
    if shutil.which(name) is None:
        fail(f"{name} is required for sidecar API runtime smoke")


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


def run_expect_failure(args, expected_stderr, *, env=None, timeout=60):
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
    if expected_stderr not in result.stderr:
        sys.stderr.write(result.stdout)
        sys.stderr.write(result.stderr)
        fail(
            f"command stderr did not contain {expected_stderr!r}: {' '.join(args)}"
        )
    return result


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


def smoke_http_runtime(entry):
    port = free_port()
    env = os.environ.copy()
    env["AI_BLAISE_LISTEN_ADDR"] = f"127.0.0.1:{port}"
    proc = subprocess.Popen(
        [require_binary(entry["package"]), "serve"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
    )
    try:
        component = entry["component"]
        wait_ready(proc, port, component)

        status, body = request(port, "GET", "/healthz")
        assert status == 200, body
        assert f'"component":"{component}"' in body, body
        assert '"ready":true' in body, body
        assert '"accepting_new_work":true' in body, body

        status, body = request(port, "GET", "/metrics")
        assert status == 200, body
        assert f'ai_blaise_sidecar_ready{{component="{component}"}} 1' in body, body
        assert (
            f'ai_blaise_sidecar_accepting_new_work{{component="{component}"}} 1'
            in body
        ), body

        status, body = request(port, "PUT", "/readyz")
        assert status == 405, body
        assert '"method not allowed"' in body, body

        status, body = request(port, "GET", "/does-not-exist")
        assert status == 404, body
        assert '"not found"' in body, body

        status, body = request(port, "POST", "/drain")
        assert status == 202, body
        assert '"accepting_new_work":false' in body, body

        status, body = request(port, "GET", "/readyz")
        assert status == 503, body
        assert '"state":"draining"' in body, body
        assert '"ready":false' in body, body
        assert '"accepting_new_work":false' in body, body

        status, body = request(port, "GET", "/metrics")
        assert status == 200, body
        assert f'ai_blaise_sidecar_ready{{component="{component}"}} 0' in body, body
        assert (
            f'ai_blaise_sidecar_accepting_new_work{{component="{component}"}} 0'
            in body
        ), body
        smoke_component_front_door(port, entry["label"])
    finally:
        terminate(proc)


def smoke_component_front_door(port, label):
    if label == "postgrest":
        status, body = request(port, "GET", "/openapi.json")
        assert status == 200, body
        assert '"openapi":"3.0.0"' in body, body
        assert '"/orders"' in body, body
        assert '"tenant_claim":"tenant_id"' in body, body

        status, body = request(port, "GET", "/api/orders")
        assert status == 200, body
        assert '"table":"orders"' in body, body
        assert '"method":"GET"' in body, body
        assert '"distribution_column":"tenant_id"' in body, body
        assert '"view":"api.orders"' in body, body

        status, body = request(port, "POST", "/api/public/orders", body="{}")
        assert status == 200, body
        assert '"schema":"public"' in body, body
        assert '"method":"POST"' in body, body

        status, body = request(port, "GET", "/api/missing")
        assert status == 404, body
        assert '"error"' in body, body
        return

    if label == "graphql":
        status, body = request(port, "GET", "/graphql")
        assert status == 200, body
        assert "ai-blaise GraphQL" in body, body
        assert "/graphql/v1" in body, body

        query = '{"query":"query { orderCollection { edges { node { id } } } }","jwt_claims":"{\\"tenant_id\\":\\"tenant-a\\"}"}'
        status, body = request(port, "POST", "/graphql/v1", body=query)
        assert status == 200, body
        assert '"namespace":"public_api"' in body, body
        assert '"tenant_id":"tenant-a"' in body, body

        subscription = '{"query":"subscription { orderInserted { id total } }","jwt_claims":"{\\"tenant_id\\":\\"tenant-a\\"}"}'
        status, body = request(port, "POST", "/graphql/ws", body=subscription)
        assert status == 200, body
        assert '"transport":"websocket"' in body, body
        assert '"subscription_field":"orderInserted"' in body, body
        assert "public_api.public.orders" in body, body

        status, body = request(port, "GET", "/graphql/ws")
        assert status == 426, body
        assert "upgrade required" in body, body
        return

    if label == "edge-functions":
        status, body = request(port, "GET", "/functions")
        assert status == 200, body
        assert '"order_created"' in body, body
        assert '"cdc_event"' in body, body

        register = '{"name":"hello","runtime":"deno","code":"export default async () => Response.json({ok:true})","http_path":"/hello"}'
        status, body = request(port, "POST", "/functions", body=register)
        assert status == 201, body
        assert '"registered":"hello"' in body, body

        status, body = request(port, "GET", "/functions")
        assert status == 200, body
        assert '"hello"' in body, body

        invoke = '{"tenant_id":"tenant-a","payload_bytes":64,"timeout_ms":250}'
        status, body = request(port, "POST", "/functions/hello", body=invoke)
        assert status == 200, body
        assert '"function":"hello"' in body, body
        assert '"status":"succeeded"' in body, body

        status, body = request(port, "POST", "/functions/order_created", body=invoke)
        assert status == 200, body
        assert '"function":"order_created"' in body, body
        assert '"db_callback_used":true' in body, body
        return

    fail(f"unknown sidecar API smoke label: {label}")


def smoke_fail_closed(entry):
    package = entry["package"]
    run_expect_failure(
        [require_binary(package), "definitely-not-a-command"],
        "unknown command",
    )

    env = os.environ.copy()
    env["AI_BLAISE_LISTEN_ADDR"] = ""
    run_expect_failure(
        [require_binary(package), "serve"],
        "invalid listen address",
        env=env,
    )


def smoke_canonical_reports():
    postgrest = parse_tsv(
        run(
            [require_binary("ai_blaise_citus_sidecar_postgrest"), "run-canonical"]
        )
    )
    assert postgrest["route"] == "public.orders", postgrest
    assert postgrest["methods"] == "get,post", postgrest
    assert postgrest["view"] == "api.orders", postgrest
    assert postgrest["openapi_path"] == "/openapi.json", postgrest
    assert postgrest["tenant_claim"] == "tenant_id", postgrest

    graphql = parse_tsv(
        run(
            [require_binary("ai_blaise_citus_sidecar_graphql"), "run-canonical"]
        )
    )
    assert graphql["endpoint"] == "/graphql/v1", graphql
    assert graphql["namespace"] == "public_api", graphql
    assert graphql["type_name"] == "Order", graphql
    assert graphql["route_function"] == "companion.route_distributed_graphql", graphql
    assert graphql["tenant_claim"] == "tenant_id", graphql
    assert graphql["introspection"] == "false", graphql

    edge_launch = parse_tsv(
        run(
            [require_binary("ai_blaise_citus_sidecar_edge_functions"), "run-canonical"]
        )
    )
    assert edge_launch["function"] == "order_created", edge_launch
    assert edge_launch["executable"] == "deno", edge_launch
    assert edge_launch["db_callback_socket"] == "/var/run/postgresql/.s.PGSQL.5432", edge_launch
    assert edge_launch["trigger"] == "cdc:public.orders:insert", edge_launch

    edge_deno = parse_tsv(
        run(
            [require_binary("ai_blaise_citus_sidecar_edge_functions"), "run-runtime-canonical"]
        )
    )
    assert edge_deno["runtime"] == "deno", edge_deno
    assert edge_deno["command"] == "deno run --allow-env --allow-net=unix inline.ts", edge_deno
    assert edge_deno["db_callback_used"] == "true", edge_deno
    assert edge_deno["db_callbacks"] == "1", edge_deno
    assert edge_deno["status"] == "succeeded", edge_deno

    edge_bun = parse_tsv(
        run(
            [require_binary("ai_blaise_citus_sidecar_edge_functions"), "run-bun-runtime-canonical"]
        )
    )
    assert edge_bun["runtime"] == "bun", edge_bun
    assert edge_bun["command"] == "bun run index.ts", edge_bun
    assert edge_bun["trigger"] == "scheduled:*/5 * * * *", edge_bun
    assert edge_bun["db_callback_used"] == "false", edge_bun
    assert edge_bun["db_callbacks"] == "0", edge_bun
    assert edge_bun["status"] == "succeeded", edge_bun


require_tool("cargo")
packages = []
for entry in COMPONENTS:
    if entry["package"] not in packages:
        packages.append(entry["package"])
run(["cargo", "build", "-q"] + sum((["-p", package] for package in packages), []), timeout=300)

smoke_canonical_reports()
for entry in COMPONENTS:
    smoke_fail_closed(entry)
    smoke_http_runtime(entry)

print("ai_blaise_citus sidecar API runtime smoke passed")
PY
