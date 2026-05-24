#!/usr/bin/env bash
set -euo pipefail

# FEATURE: EF1 EF2 EF4 EF5
# Bounded runtime-boundary smoke for the edge-functions sidecar. This proves the
# Rust sidecar/server contract and fail-closed request boundary, not external
# Deno/Bun user-code execution.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

python3 <<'PY'
import http.client
import os
import socket
import subprocess
import sys
import time

PACKAGE = "ai_blaise_citus_sidecar_edge_functions"
TARGET_DIR = os.environ.get("CARGO_TARGET_DIR", os.path.join(os.getcwd(), "target"))
if not os.path.isabs(TARGET_DIR):
    TARGET_DIR = os.path.abspath(TARGET_DIR)
BINARY = os.path.join(TARGET_DIR, "debug", PACKAGE)


def fail(message):
    raise AssertionError(message)


def run(args, *, env=None, timeout=180):
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


def parse_tsv(stdout):
    lines = [line for line in stdout.splitlines() if line.strip()]
    if len(lines) != 2:
        fail(f"expected two-line TSV report, got {len(lines)} lines: {stdout!r}")
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


def wait_ready(proc, port):
    for _ in range(100):
        try:
            status, body = request(port, "GET", "/readyz")
            if status == 200 and '"component":"edge-functions"' in body:
                return
        except OSError:
            pass
        if proc.poll() is not None:
            stderr = proc.stderr.read() if proc.stderr is not None else ""
            fail(f"edge-functions serve exited before readiness: {stderr}")
        time.sleep(0.25)
    fail("edge-functions HTTP server did not become ready")


def terminate(proc):
    proc.terminate()
    try:
        proc.wait(timeout=20)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=20)
    if proc.returncode not in (0, -15):
        stderr = proc.stderr.read() if proc.stderr is not None else ""
        fail(f"edge-functions process exited with {proc.returncode}: {stderr}")


run(["cargo", "build", "-q", "-p", PACKAGE], timeout=300)
if not os.path.isfile(BINARY) or not os.access(BINARY, os.X_OK):
    fail(f"built edge-functions binary missing or not executable: {BINARY}")

launch = parse_tsv(run([BINARY, "run-canonical"]))
assert launch["function"] == "order_created", launch
assert launch["executable"] == "deno", launch
assert launch["args"] == "run --no-prompt --allow-env --allow-net=unix inline.ts", launch
assert launch["secret_refs"] == "orders-api-key", launch
assert launch["db_callback_socket"] == "/var/run/postgresql/.s.PGSQL.5432", launch

runtime = parse_tsv(run([BINARY, "run-runtime-canonical"]))
assert runtime["runtime"] == "deno", runtime
assert runtime["command"] == "deno run --no-prompt --allow-env --allow-net=unix inline.ts", runtime
assert runtime["db_callback_used"] == "true", runtime
assert runtime["status"] == "planned", runtime
assert runtime["execution_mode"] == "plan_only", runtime

bun = parse_tsv(run([BINARY, "run-bun-runtime-canonical"]))
assert bun["runtime"] == "bun", bun
assert bun["command"] == "bun run index.ts", bun
assert bun["status"] == "planned", bun
assert bun["execution_mode"] == "plan_only", bun

registry = parse_tsv(run([BINARY, "run-registry-canonical"]))
assert registry["function"] == "order_created", registry
assert registry["triggers"] == "http,cdc_event", registry
assert registry["uds_statements"] == "1", registry

port = free_port()
env = os.environ.copy()
env["AI_BLAISE_LISTEN_ADDR"] = f"127.0.0.1:{port}"
proc = subprocess.Popen(
    [BINARY, "serve"],
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
    env=env,
)
try:
    wait_ready(proc, port)

    status, body = request(port, "GET", "/functions")
    assert status == 200, body
    assert '"name":"order_created"' in body, body
    assert '"cdc_event"' in body, body

    register = '{"name":"hello","runtime":"deno","code":"export default async () => Response.json({ok:true})","http_path":"/hello","env_secret_refs":["tenant-api-key"]}'
    status, body = request(port, "POST", "/functions", body=register)
    assert status == 201, body
    assert '"registered":"hello"' in body, body

    invoke = '{"tenant_id":"tenant-a","payload_bytes":64,"timeout_ms":250}'
    status, body = request(port, "POST", "/functions/hello", body=invoke)
    assert status == 200, body
    assert '"function":"hello"' in body, body
    assert '"status":"planned"' in body, body
    assert '"execution_mode":"plan_only"' in body, body

    status, body = request(port, "POST", "/functions/order_created", body=invoke)
    assert status == 200, body
    assert '"function":"order_created"' in body, body
    assert '"db_callback_used":true' in body, body
    assert '"execution_mode":"plan_only"' in body, body

    live = '{"tenant_id":"tenant-a","payload_bytes":64,"timeout_ms":250,"execution_mode":"live"}'
    status, body = request(port, "POST", "/functions/order_created", body=live)
    assert status == 501, body
    assert "external Deno/Bun user-code execution" in body, body

    oversized_payload = '{"tenant_id":"tenant-a","payload_bytes":1048577,"timeout_ms":250}'
    status, body = request(port, "POST", "/functions/order_created", body=oversized_payload)
    assert status == 400, body
    assert "payload_bytes" in body, body

    timeout = '{"tenant_id":"tenant-a","payload_bytes":64,"timeout_ms":30001}'
    status, body = request(port, "POST", "/functions/order_created", body=timeout)
    assert status == 400, body
    assert "timeout" in body, body

    bad_register = '{"name":"bad","runtime":"deno","code":"ok","http_path":"admin"}'
    status, body = request(port, "POST", "/functions", body=bad_register)
    assert status == 400, body
    assert "HTTP trigger path" in body, body

    bad_runtime = '{"name":"node_fn","runtime":"node","code":"ok","http_path":"/node"}'
    status, body = request(port, "POST", "/functions", body=bad_runtime)
    assert status == 400, body
    assert "runtime" in body, body

    status, body = request(port, "POST", "/functions/not_registered", body=invoke)
    assert status == 404, body
    assert "no registered function" in body, body

    status, body = request(port, "POST", "/functions", body="not-json")
    assert status == 400, body
    assert "malformed" in body, body

    status, body = request(port, "POST", "/drain")
    assert status == 202, body
    assert '"accepting_new_work":false' in body, body
    status, body = request(port, "GET", "/readyz")
    assert status == 503, body
    assert '"state":"draining"' in body, body
finally:
    terminate(proc)

print("ai_blaise_citus edge-functions runtime boundary smoke passed")
PY
