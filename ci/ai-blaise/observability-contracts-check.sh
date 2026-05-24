#!/usr/bin/env bash
set -euo pipefail

# FEATURE: O4
# FEATURE: O15

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

python3 <<'PY'
import http.client
import json
import os
import socket
import subprocess
import sys
import tempfile
import threading
import time

TIMEOUT_SECONDS = float(os.environ.get("OBSERVABILITY_CONTRACT_TIMEOUT", "90"))

SERVICES = [
    {
        "label": "operator",
        "package": "ai_blaise_citus_operator",
        "component": "operator",
        "listen_env": "AI_BLAISE_LISTEN_ADDR",
    },
    {
        "label": "sidecar-shared",
        "package": "ai_blaise_citus_sidecar_shared",
        "component": "sidecar-shared",
        "listen_env": "AI_BLAISE_LISTEN_ADDR",
    },
    {
        "label": "analytical",
        "package": "ai_blaise_citus_sidecar_analytical",
        "component": "analytical",
        "listen_env": "AI_BLAISE_LISTEN_ADDR",
        "schema": "analytical",
    },
    {
        "label": "auth",
        "package": "ai_blaise_citus_sidecar_auth",
        "component": "auth-sidecar",
        "listen_env": "AI_BLAISE_LISTEN_ADDR",
        "schema": "auth",
        "env": {
            "AI_BLAISE_AUTH_HS256_SECRET": "observability-contracts-auth-secret-32-bytes-minimum",
        },
    },
    {
        "label": "backup",
        "package": "ai_blaise_citus_sidecar_backup",
        "component": "backup",
        "listen_env": "AI_BLAISE_LISTEN_ADDR",
        "schema": "backup",
        "metrics_fragments": [
            "# TYPE ai_blaise_backup_completed_base_backups counter",
            "ai_blaise_backup_completed_base_backups 0",
            "# TYPE ai_blaise_backup_archived_wal_segments counter",
            "# TYPE ai_blaise_backup_queryable_branches gauge",
        ],
    },
    {
        "label": "cdc",
        "package": "ai_blaise_citus_sidecar_cdc",
        "component": "cdc",
        "listen_env": "AI_BLAISE_SIDECAR_LISTEN_ADDR",
        "schema": "cdc",
    },
    {
        "label": "coldtier",
        "package": "ai_blaise_citus_sidecar_coldtier",
        "component": "coldtier",
        "listen_env": "AI_BLAISE_LISTEN_ADDR",
        "schema": "coldtier",
    },
    {
        "label": "edge-functions",
        "package": "ai_blaise_citus_sidecar_edge_functions",
        "component": "edge-functions",
        "listen_env": "AI_BLAISE_LISTEN_ADDR",
        "schema": "edge_functions",
    },
    {
        "label": "graphql",
        "package": "ai_blaise_citus_sidecar_graphql",
        "component": "graphql",
        "listen_env": "AI_BLAISE_LISTEN_ADDR",
        "schema": "graphql",
    },
    {
        "label": "hlc",
        "package": "ai_blaise_citus_sidecar_hlc",
        "component": "hlc",
        "listen_env": "AI_BLAISE_LISTEN_ADDR",
        "schema": "hlc",
    },
    {
        "label": "mcp-sidecar",
        "package": "ai_blaise_citus_sidecar_mcp",
        "component": "mcp-sidecar",
        "listen_env": "AI_BLAISE_LISTEN_ADDR",
        "schema": "mcp",
    },
    {
        "label": "postgrest",
        "package": "ai_blaise_citus_sidecar_postgrest",
        "component": "postgrest",
        "listen_env": "AI_BLAISE_LISTEN_ADDR",
        "schema": "postgrest",
    },
    {
        "label": "raft",
        "package": "ai_blaise_citus_sidecar_raft",
        "component": "raft",
        "listen_env": "AI_BLAISE_LISTEN_ADDR",
        "schema": "raft",
    },
    {
        "label": "realtime",
        "package": "ai_blaise_citus_sidecar_realtime",
        "component": "realtime",
        "listen_env": "AI_BLAISE_LISTEN_ADDR",
        "schema": "realtime",
    },
    {
        "label": "repack",
        "package": "ai_blaise_citus_sidecar_repack",
        "component": "repack",
        "listen_env": "AI_BLAISE_LISTEN_ADDR",
        "schema": "repack",
    },
    {
        "label": "schema-job",
        "package": "ai_blaise_citus_sidecar_schema_job",
        "component": "schema-job",
        "listen_env": "AI_BLAISE_LISTEN_ADDR",
        "schema": "schema_job",
    },
    {
        "label": "storage",
        "package": "ai_blaise_citus_sidecar_storage",
        "component": "storage",
        "listen_env": "AI_BLAISE_LISTEN_ADDR",
        "schema": "storage",
    },
    {
        "label": "txn-status",
        "package": "ai_blaise_citus_sidecar_txn_status",
        "component": "txn-status",
        "listen_env": "AI_BLAISE_LISTEN_ADDR",
        "schema": "txn_status",
    },
    {
        "label": "vectorizer",
        "package": "ai_blaise_citus_sidecar_vectorizer",
        "component": "vectorizer",
        "listen_env": "AI_BLAISE_LISTEN_ADDR",
        "schema": "vectorizer",
    },
]


def fail(message):
    print(message, file=sys.stderr)
    sys.exit(1)


def free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def read_log(log_file):
    log_file.flush()
    with open(log_file.name, encoding="utf-8", errors="replace") as handle:
        return handle.read()


def binary_path(package):
    return os.path.join("target", "debug", package)


def start_process(label, args, env):
    log_file = tempfile.NamedTemporaryFile(
        prefix=f"ai-blaise-{label}-", suffix=".log", mode="w+", delete=False
    )
    proc = subprocess.Popen(
        args,
        stdout=log_file,
        stderr=log_file,
        text=True,
        env=env,
    )
    return proc, log_file


def run_checked(args, **kwargs):
    return subprocess.run(args, check=True, text=True, **kwargs)


def start_vectorizer_postgres():
    image = os.environ.get("OBSERVABILITY_CONTRACTS_POSTGRES_IMAGE", "postgres:17")
    container = f"ai-blaise-observability-vectorizer-{os.getpid()}-{free_port()}"
    run_checked(
        [
            "docker",
            "run",
            "--name",
            container,
            "-e",
            "POSTGRES_HOST_AUTH_METHOD=trust",
            "-p",
            "127.0.0.1::5432",
            "-d",
            image,
        ],
        stdout=subprocess.DEVNULL,
    )
    deadline = time.monotonic() + TIMEOUT_SECONDS
    while time.monotonic() < deadline:
        probe = subprocess.run(
            ["docker", "exec", container, "psql", "-U", "postgres", "-Atqc", "SELECT 1"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        if probe.returncode == 0:
            port_output = subprocess.check_output(
                ["docker", "port", container, "5432/tcp"], text=True
            ).strip()
            if not port_output:
                fail("vectorizer postgres did not expose a host port")
            host_port = port_output.split(":")[-1]
            return container, f"postgres://postgres@127.0.0.1:{host_port}/postgres"
        time.sleep(1)
    logs = subprocess.run(
        ["docker", "logs", container], text=True, capture_output=True
    ).stderr
    fail(f"vectorizer postgres did not become ready within {TIMEOUT_SECONDS}s:\n{logs}")


def stop_docker_container(container):
    subprocess.run(
        ["docker", "rm", "-f", container],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def stop_process(proc, log_file):
    if proc.poll() is None:
        proc.terminate()
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=10)
    log_file.close()
    try:
        os.unlink(log_file.name)
    except FileNotFoundError:
        pass


def http_get(port, path):
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
    try:
        conn.request("GET", path)
        response = conn.getresponse()
        body = response.read().decode("utf-8")
        return response.status, dict(response.getheaders()), body
    finally:
        conn.close()


def wait_for_probe(label, proc, log_file, port, path, predicate):
    deadline = time.monotonic() + TIMEOUT_SECONDS
    last_error = "probe was not attempted"
    while time.monotonic() < deadline:
        if proc.poll() is not None:
            fail(f"{label} exited before {path} became ready:\n{read_log(log_file)}")
        try:
            status, headers, body = http_get(port, path)
            if predicate(status, headers, body):
                return status, headers, body
            last_error = f"status={status} body={body[:240]!r}"
        except OSError as error:
            last_error = str(error)
        time.sleep(0.25)
    fail(f"{label} did not satisfy {path} within {TIMEOUT_SECONDS}s: {last_error}\n{read_log(log_file)}")


def assert_json_probe(label, body, component, ready):
    try:
        payload = json.loads(body)
    except json.JSONDecodeError as error:
        fail(f"{label} probe did not return JSON: {error}: {body!r}")
    if payload.get("component") != component:
        fail(f"{label} probe component mismatch: {payload!r}")
    if payload.get("ready") is not ready:
        fail(f"{label} probe ready mismatch: {payload!r}")
    if ready and payload.get("accepting_new_work") is not True:
        fail(f"{label} probe must accept new work while ready: {payload!r}")


def expected_metrics_fragments(service):
    if "metrics_fragments" in service:
        return service["metrics_fragments"]
    component = service["component"]
    return [
        "# TYPE ai_blaise_sidecar_ready gauge",
        f'ai_blaise_sidecar_ready{{component="{component}"}} 1',
        f'ai_blaise_sidecar_accepting_new_work{{component="{component}"}} 1',
        f'ai_blaise_sidecar_in_flight_work{{component="{component}"}} 0',
    ]


def assert_service_metrics(service, metrics):
    label = service["label"]
    required_fragments = expected_metrics_fragments(service)
    for fragment in required_fragments:
        if fragment not in metrics:
            fail(f"{label} metrics missing {fragment!r}:\n{metrics}")


def smoke_service(service):
    port = free_port()
    env = os.environ.copy()
    env[service["listen_env"]] = f"127.0.0.1:{port}"
    env.update(service.get("env", {}))
    cleanup_callbacks = []
    proc = None
    log_file = None
    try:
        if service["label"] == "vectorizer":
            if subprocess.run(
                ["docker", "version"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
            ).returncode != 0:
                fail("docker is required for the vectorizer observability contract")
            container, postgres_url = start_vectorizer_postgres()
            cleanup_callbacks.append(lambda: stop_docker_container(container))
            env.update(
                {
                    "AI_BLAISE_VECTORIZER_DATABASE_URL": postgres_url,
                    "AI_BLAISE_VECTORIZER_PROVIDER_MODE": "mock",
                    "AI_BLAISE_VECTORIZER_BATCH_SIZE": "4",
                    "AI_BLAISE_VECTORIZER_POLL_INTERVAL_MS": "200",
                    "AI_BLAISE_VECTORIZER_MOCK_DIMENSIONS": "4",
                }
            )
        proc, log_file = start_process(
            service["label"],
            [binary_path(service["package"]), "serve"],
            env,
        )
        _, _, ready_body = wait_for_probe(
            service["label"],
            proc,
            log_file,
            port,
            "/readyz",
            lambda status, _headers, body: status == 200 and service["component"] in body,
        )
        assert_json_probe(service["label"], ready_body, service["component"], True)

        status, _, health_body = http_get(port, "/healthz")
        if status != 200:
            fail(f"{service['label']} /healthz returned {status}: {health_body!r}")
        assert_json_probe(service["label"], health_body, service["component"], True)

        status, _, metrics = http_get(port, "/metrics")
        if status != 200:
            fail(f"{service['label']} /metrics returned {status}: {metrics!r}")
        assert_service_metrics(service, metrics)
    finally:
        if proc is not None and log_file is not None:
            stop_process(proc, log_file)
        for cleanup in reversed(cleanup_callbacks):
            cleanup()


def run_tcp_acceptor(stop_event):
    listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    listener.bind(("127.0.0.1", 0))
    listener.listen()
    listener.settimeout(0.25)
    port = listener.getsockname()[1]

    def loop():
        try:
            while not stop_event.is_set():
                try:
                    conn, _addr = listener.accept()
                except socket.timeout:
                    continue
                with conn:
                    pass
        finally:
            listener.close()

    thread = threading.Thread(target=loop, name="dummy-postgres-upstream", daemon=True)
    thread.start()
    return port, thread


def smoke_pool():
    data_port = free_port()
    admin_port = free_port()
    stop_event = threading.Event()
    upstream_port, upstream_thread = run_tcp_acceptor(stop_event)
    env = os.environ.copy()
    env.update(
        {
            "AI_BLAISE_POOL_LISTEN_ADDR": f"127.0.0.1:{data_port}",
            "AI_BLAISE_POOL_ADMIN_ADDR": f"127.0.0.1:{admin_port}",
            "AI_BLAISE_POOL_UPSTREAM_ADDR": f"127.0.0.1:{upstream_port}",
            "AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST": "127.0.0.0/8",
        }
    )
    proc, log_file = start_process(
        "pool",
        [binary_path("ai_blaise_citus_pool"), "serve"],
        env,
    )
    try:
        _, _, ready_body = wait_for_probe(
            "pool",
            proc,
            log_file,
            admin_port,
            "/readyz",
            lambda status, _headers, body: status == 200 and '"upstream_ready":true' in body,
        )
        pool_ready = json.loads(ready_body)
        if pool_ready.get("component") != "pool" or pool_ready.get("ready") is not True:
            fail(f"pool readiness JSON mismatch: {pool_ready!r}")

        status, _, health_body = http_get(admin_port, "/healthz")
        if status != 200 or '"component":"pool"' not in health_body:
            fail(f"pool /healthz failed: status={status} body={health_body!r}")

        status, _, metrics = http_get(admin_port, "/metrics")
        if status != 200:
            fail(f"pool /metrics returned {status}: {metrics!r}")
        for fragment in (
            "# TYPE ai_blaise_citus_pool_upstream_ready gauge",
            f'ai_blaise_citus_pool_upstream_ready{{upstream="127.0.0.1:{upstream_port}"}} 1',
            "ai_blaise_citus_pool_requests_total 0",
            "ai_blaise_citus_pool_rejected_connections_total 0",
        ):
            if fragment not in metrics:
                fail(f"pool metrics missing {fragment!r}:\\n{metrics}")
    finally:
        stop_process(proc, log_file)
        stop_event.set()
        upstream_thread.join(timeout=5)


def verify_log_schema_catalog():
    output = subprocess.check_output(
        [binary_path("ai_blaise_citus_sidecar_shared"), "log-schema-canonical"],
        text=True,
    )
    lines = [line for line in output.splitlines() if line.strip()]
    expected_header = "sidecar	common_fields	extension_fields	required_fields	total_fields"
    if not lines or lines[0] != expected_header:
        fail(f"unexpected log-schema-canonical header:\\n{output}")
    rows = {}
    for line in lines[1:]:
        sidecar, common, extensions, required, total = line.split("	")
        rows[sidecar] = tuple(map(int, (common, extensions, required, total)))
    expected = {service["schema"] for service in SERVICES if "schema" in service}
    if set(rows) != expected:
        fail(f"structured-log schema coverage mismatch: got={sorted(rows)} expected={sorted(expected)}")
    for sidecar, (common, extensions, required, total) in rows.items():
        if common != 10 or required != 4 or total != common + extensions:
            fail(f"structured-log schema counts invalid for {sidecar}: {rows[sidecar]!r}")


def main():
    packages = sorted({service["package"] for service in SERVICES} | {"ai_blaise_citus_pool"})
    build_cmd = ["cargo", "build", "-q", "--bins"]
    for package in packages:
        build_cmd.extend(["-p", package])
    subprocess.run(build_cmd, check=True)

    verify_log_schema_catalog()
    for service in SERVICES:
        smoke_service(service)
    smoke_pool()
    print(
        "observability_contracts_check	"
        f"serve_surfaces={len(SERVICES)}	"
        f"sidecar_log_schemas={len([service for service in SERVICES if 'schema' in service])}	"
        "pool_admin_metrics=true"
    )


if __name__ == "__main__":
    main()
PY
