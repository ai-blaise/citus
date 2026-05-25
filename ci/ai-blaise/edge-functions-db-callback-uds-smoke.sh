#!/usr/bin/env bash
set -euo pipefail

# FEATURE: EF4
# Live database-callback smoke for the edge-functions sidecar. This proves the
# bounded PostgreSQL Unix-domain-socket callback path, not external Deno/Bun
# user-code execution or triggered queue/CDC dispatch.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

python3 <<'PY'
import http.client
import json
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import time

PACKAGE = "ai_blaise_citus_sidecar_edge_functions"
TARGET_DIR = os.environ.get("CARGO_TARGET_DIR", os.path.join(os.getcwd(), "target"))
if not os.path.isabs(TARGET_DIR):
    TARGET_DIR = os.path.abspath(TARGET_DIR)
BINARY = os.path.join(TARGET_DIR, "debug", PACKAGE)
POSTGRES_IMAGE = os.environ.get("POSTGRES_IMAGE", "postgres:17")


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


def free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def request(port, method, path, body=None, headers=None):
    headers = dict(headers or {})
    if body is not None and "content-type" not in {key.lower() for key in headers}:
        headers["content-type"] = "application/json"
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=10)
    conn.request(method, path, body=body, headers=headers)
    response = conn.getresponse()
    response_body = response.read().decode("utf-8")
    conn.close()
    return response.status, response_body


def wait_ready(proc, port):
    for _ in range(160):
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


def start_sidecar(enabled):
    port = free_port()
    env = os.environ.copy()
    env["AI_BLAISE_LISTEN_ADDR"] = f"127.0.0.1:{port}"
    if enabled:
        env["AI_BLAISE_EDGE_DB_CALLBACK_EXECUTION"] = "1"
    else:
        env.pop("AI_BLAISE_EDGE_DB_CALLBACK_EXECUTION", None)
    proc = subprocess.Popen(
        [BINARY, "serve"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
    )
    wait_ready(proc, port)
    return proc, port


def register_writer(port, socket_path):
    body = json.dumps(
        {
            "name": "writer",
            "runtime": "deno",
            "code": "export default async () => Response.json({ok:true})",
            "http_path": "/writer",
            "db_callback_socket": socket_path,
            "db_callback_database": "app",
            "db_callback_role": "edge_runtime",
            "db_callback_statement_timeout_ms": 1000,
        }
    )
    status, response = request(port, "POST", "/functions", body=body)
    if status != 201:
        fail(f"writer registration failed: {status} {response}")


def invoke_writer(port, statement):
    body = json.dumps(
        {
            "tenant_id": "tenant-a",
            "payload_bytes": 128,
            "timeout_ms": 1000,
            "db_statement": statement,
        }
    )
    return request(port, "POST", "/functions/writer", body=body)


def docker_exec(container, *args, timeout=120):
    return run(["docker", "exec", container, *args], timeout=timeout)


def docker_exec_retry(container, *args, attempts=90, delay=1.0, timeout=120):
    last_stdout = ""
    last_stderr = ""
    last_code = None
    for attempt in range(attempts):
        result = subprocess.run(
            ["docker", "exec", container, *args],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=timeout,
        )
        if result.returncode == 0:
            return result.stdout
        last_stdout = result.stdout
        last_stderr = result.stderr
        last_code = result.returncode
        if attempt + 1 < attempts:
            time.sleep(delay)
    sys.stderr.write(last_stdout)
    sys.stderr.write(last_stderr)
    fail(
        f"command failed with exit {last_code} after {attempts} attempts: "
        f"docker exec {container} {' '.join(args)}"
    )


run(["cargo", "build", "-q", "-p", PACKAGE], timeout=300)
if not os.path.isfile(BINARY) or not os.access(BINARY, os.X_OK):
    fail(f"built edge-functions binary missing or not executable: {BINARY}")

socket_dir = tempfile.mkdtemp(prefix="ai-blaise-ef4-uds-", dir="/tmp")
os.chmod(socket_dir, 0o777)
container = f"ai-blaise-ef4-uds-{os.getpid()}"
proc = None
proc_disabled = None

try:
    run(
        [
            "docker",
            "run",
            "--rm",
            "-d",
            "--name",
            container,
            "-e",
            "POSTGRES_PASSWORD=postgres",
            "-e",
            "POSTGRES_HOST_AUTH_METHOD=trust",
            "-v",
            f"{socket_dir}:/var/run/postgresql",
            POSTGRES_IMAGE,
            "-c",
            "unix_socket_directories=/var/run/postgresql",
            "-c",
            "listen_addresses=",
        ],
        timeout=120,
    )

    for _ in range(120):
        result = subprocess.run(
            ["docker", "exec", container, "pg_isready", "-h", "/var/run/postgresql", "-U", "postgres"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        if result.returncode == 0:
            break
        time.sleep(0.5)
    else:
        fail("postgres:17 did not become ready on Unix socket")

    docker_exec_retry(
        container,
        "psql",
        "-v",
        "ON_ERROR_STOP=1",
        "-U",
        "postgres",
        "-c",
        "CREATE ROLE edge_runtime LOGIN",
    )
    docker_exec_retry(
        container,
        "psql",
        "-v",
        "ON_ERROR_STOP=1",
        "-U",
        "postgres",
        "-c",
        "CREATE DATABASE app OWNER edge_runtime",
    )
    docker_exec_retry(
        container,
        "psql",
        "-v",
        "ON_ERROR_STOP=1",
        "-U",
        "postgres",
        "-d",
        "app",
        "-c",
        "CREATE TABLE edge_callback_events(id bigserial primary key, tenant_id text not null, payload jsonb not null)",
        "-c",
        "ALTER TABLE edge_callback_events OWNER TO edge_runtime",
    )

    socket_path = os.path.join(socket_dir, ".s.PGSQL.5432")
    if not os.path.exists(socket_path):
        fail(f"PostgreSQL UDS socket was not created at {socket_path}")

    proc_disabled, disabled_port = start_sidecar(enabled=False)
    register_writer(disabled_port, socket_path)
    status, body = invoke_writer(
        disabled_port,
        "insert into edge_callback_events(tenant_id, payload) values ('tenant-a', jsonb_build_object('source', 'disabled'))",
    )
    if status != 501 or "AI_BLAISE_EDGE_DB_CALLBACK_EXECUTION" not in body:
        fail(f"disabled executor did not fail closed: {status} {body}")
    terminate(proc_disabled)
    proc_disabled = None

    proc, port = start_sidecar(enabled=True)
    register_writer(port, socket_path)

    status, body = invoke_writer(port, "select 1; drop table edge_callback_events")
    if status != 400 or "single safe" not in body:
        fail(f"unsafe multi-statement callback was not rejected: {status} {body}")

    before = docker_exec(
        container,
        "psql",
        "-At",
        "-U",
        "postgres",
        "-d",
        "app",
        "-c",
        "select count(*) from edge_callback_events",
    ).strip()
    if before != "0":
        fail(f"unsafe callback mutated the database, row count={before}")

    status, body = invoke_writer(
        port,
        "insert into edge_callback_events(tenant_id, payload) values ('tenant-a', jsonb_build_object('source', 'edge-functions'))",
    )
    if status != 200:
        fail(f"live UDS callback insert failed: {status} {body}")
    if '"db_callback_statement_executed":true' not in body:
        fail(f"live UDS callback did not report statement execution: {body}")
    if '"db_callback_rows":1' not in body:
        fail(f"live UDS callback did not report one affected row: {body}")
    if '"status":"db_callback_executed"' not in body:
        fail(f"live UDS callback did not expose executed status: {body}")

    rows = docker_exec(
        container,
        "psql",
        "-At",
        "-U",
        "postgres",
        "-d",
        "app",
        "-c",
        "select count(*) from edge_callback_events where tenant_id = 'tenant-a' and payload->>'source' = 'edge-functions'",
    ).strip()
    if rows != "1":
        fail(f"expected exactly one edge-functions callback row, got {rows}")

    print("edge_db_callback_uds=passed")
    print("AI_BLAISE_EDGE_DB_CALLBACK_EXECUTION=1")
    print(f"POSTGRES_IMAGE={POSTGRES_IMAGE}")
    print(".s.PGSQL.5432")
    print("disabled_executor_rejected=true")
    print("unsafe_statement_rejected=true")
    print("db_callback_statement_executed=true")
    print("db_callback_rows=1")
    print("inserted_rows=1")
finally:
    if proc is not None:
        terminate(proc)
    if proc_disabled is not None:
        terminate(proc_disabled)
    subprocess.run(["docker", "rm", "-f", container], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    shutil.rmtree(socket_dir, ignore_errors=True)
PY
