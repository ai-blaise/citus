#!/usr/bin/env bash
set -euo pipefail

# FEATURE: EF2
# Live inline-Bun execution smoke for the edge-functions sidecar. This proves
# the explicit opt-in Bun process path, child-environment clearing, bounded
# timeout/stdout behavior, fail-closed disabled mode, and scheduled/CDC trigger
# dispatch through the same live executor. It does not prove package
# installation, URI/Git bundle fetching, user-code initiated DB callbacks,
# queue/broker delivery, live CDC slot tailing, or Kubernetes deployment.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

python3 <<'PYSMOKE'
import http.client
import json
import os
import platform
import shutil
import socket
import stat
import subprocess
import sys
import tempfile
import time
import urllib.request
import zipfile

PACKAGE = "ai_blaise_citus_sidecar_edge_functions"
TARGET_DIR = os.environ.get("CARGO_TARGET_DIR", os.path.join(os.getcwd(), "target"))
if not os.path.isabs(TARGET_DIR):
    TARGET_DIR = os.path.abspath(TARGET_DIR)
BINARY = os.path.join(TARGET_DIR, "debug", PACKAGE)
BUN_VERSION = os.environ.get("BUN_VERSION", "bun-v1.1.38")


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


def bun_asset_name():
    system = platform.system().lower()
    machine = platform.machine().lower()
    if system == "linux" and machine in ("x86_64", "amd64"):
        return "bun-linux-x64.zip"
    if system == "linux" and machine in ("aarch64", "arm64"):
        return "bun-linux-aarch64.zip"
    if system == "darwin" and machine in ("aarch64", "arm64"):
        return "bun-darwin-aarch64.zip"
    if system == "darwin" and machine in ("x86_64", "amd64"):
        return "bun-darwin-x64.zip"
    fail(f"unsupported platform for Bun smoke bootstrap: {system}/{machine}")


def ensure_bun():
    configured = os.environ.get("AI_BLAISE_BUN_BIN") or os.environ.get("BUN_BIN")
    if configured:
        if os.path.isfile(configured) and os.access(configured, os.X_OK):
            return configured
        fail(f"configured Bun binary is not executable: {configured}")
    discovered = shutil.which("bun")
    if discovered:
        return discovered

    asset = bun_asset_name()
    cache_dir = os.path.join(tempfile.gettempdir(), "ai-blaise-bun-live-smoke", BUN_VERSION)
    bun_dir = os.path.join(cache_dir, asset[:-4])
    bun_path = os.path.join(bun_dir, "bun")
    if os.path.isfile(bun_path) and os.access(bun_path, os.X_OK):
        return bun_path

    os.makedirs(cache_dir, exist_ok=True)
    archive = os.path.join(cache_dir, asset)
    url = f"https://github.com/oven-sh/bun/releases/download/{BUN_VERSION}/{asset}"
    urllib.request.urlretrieve(url, archive)
    with zipfile.ZipFile(archive) as zip_file:
        zip_file.extractall(cache_dir)
    mode = os.stat(bun_path).st_mode
    os.chmod(bun_path, mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    return bun_path


def start_sidecar(*, runtime_enabled, bun_bin):
    port = free_port()
    env = os.environ.copy()
    env["AI_BLAISE_LISTEN_ADDR"] = f"127.0.0.1:{port}"
    env.pop("AI_BLAISE_EDGE_DB_CALLBACK_EXECUTION", None)
    env.pop("AI_BLAISE_DENO_BIN", None)
    if runtime_enabled:
        env["AI_BLAISE_EDGE_RUNTIME_EXECUTION"] = "1"
        env["AI_BLAISE_BUN_BIN"] = bun_bin
    else:
        env.pop("AI_BLAISE_EDGE_RUNTIME_EXECUTION", None)
        env.pop("AI_BLAISE_BUN_BIN", None)
    proc = subprocess.Popen(
        [BINARY, "serve"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
    )
    wait_ready(proc, port)
    return proc, port


def register(port, name, code, *, schedule=None, cdc_table=None, cdc_operation=None):
    body = {
        "name": name,
        "runtime": "bun",
        "code": code,
    }
    if schedule is None and cdc_table is None:
        body["http_path"] = f"/{name}"
    if schedule is not None:
        body["schedule"] = schedule
    if cdc_table is not None:
        body["cdc_table"] = cdc_table
    if cdc_operation is not None:
        body["cdc_operation"] = cdc_operation
    status, response = request(port, "POST", "/functions", body=json.dumps(body))
    if status != 201:
        fail(f"{name} registration failed: {status} {response}")


def invoke_live(port, name, payload, timeout_ms=5000):
    body = json.dumps(
        {
            "tenant_id": "tenant-a",
            "payload_bytes": 128,
            "timeout_ms": timeout_ms,
            "execution_mode": "live",
            "payload": payload,
        }
    )
    return request(port, "POST", f"/functions/{name}", body=body)


def dispatch_scheduled(port, payload):
    body = json.dumps(
        {
            "epoch_seconds": 0,
            "tenant_id": "tenant-a",
            "payload_bytes": 128,
            "timeout_ms": 5000,
            "execution_mode": "live",
            "payload": payload,
        }
    )
    return request(port, "POST", "/triggers/scheduled", body=body)


def dispatch_cdc(port, payload):
    body = json.dumps(
        {
            "table": "public.edge_orders",
            "operation": "insert",
            "tenant_id": "tenant-a",
            "payload_bytes": 128,
            "timeout_ms": 5000,
            "execution_mode": "live",
            "payload": payload,
        }
    )
    return request(port, "POST", "/triggers/cdc", body=body)


run(["cargo", "build", "-q", "-p", PACKAGE], timeout=300)
if not os.path.isfile(BINARY) or not os.access(BINARY, os.X_OK):
    fail(f"built edge-functions binary missing or not executable: {BINARY}")

bun_bin = ensure_bun()
bun_version = run([bun_bin, "--version"], timeout=30).strip()

disabled = None
enabled = None
try:
    disabled, disabled_port = start_sidecar(runtime_enabled=False, bun_bin=bun_bin)
    register(
        disabled_port,
        "bun_disabled",
        "export default async function handler(input) { return {should_not_execute: input}; }",
    )
    status, body = invoke_live(disabled_port, "bun_disabled", {"custom": "blocked"})
    assert status == 501, body
    assert "AI_BLAISE_EDGE_RUNTIME_EXECUTION" in body, body
    terminate(disabled)
    disabled = None

    enabled, enabled_port = start_sidecar(runtime_enabled=True, bun_bin=bun_bin)
    register(
        enabled_port,
        "bun_live",
        """
export default async function handler(input) {
  const envCleared = process.env.HOME === undefined && process.env.AI_BLAISE_EDGE_RUNTIME_EXECUTION === undefined;
  return {
    ok: true,
    runtime: "bun",
    tenant: input.tenant_id,
    payload_bytes: input.payload_bytes,
    custom: input.payload.custom,
    env_status: envCleared ? "cleared" : "leaked"
  };
}
""",
    )
    status, body = invoke_live(enabled_port, "bun_live", {"custom": "bun-live"})
    assert status == 200, body
    execution = json.loads(body)
    assert execution["runtime"] == "bun", execution
    assert execution["status"] == "executed", execution
    assert execution["execution_mode"] == "live", execution
    assert execution["user_code_executed"] is True, execution
    assert execution["db_callback_used"] is False, execution
    runtime_response = json.loads(execution["runtime_response_json"])
    assert runtime_response["ok"] is True, runtime_response
    assert runtime_response["runtime"] == "bun", runtime_response
    assert runtime_response["tenant"] == "tenant-a", runtime_response
    assert runtime_response["custom"] == "bun-live", runtime_response
    assert runtime_response["env_status"] == "cleared", runtime_response
    assert execution["response_bytes"] == len(execution["runtime_response_json"]), execution

    register(
        enabled_port,
        "bun_slow",
        """
export default async function handler(_) {
  await new Promise((resolve) => setTimeout(resolve, 1000));
  return {ok: true};
}
""",
    )
    status, body = invoke_live(enabled_port, "bun_slow", {"custom": "slow"}, timeout_ms=50)
    assert status == 504, body
    assert "timeout_ms 50" in body, body

    register(
        enabled_port,
        "bun_noisy",
        """
export default async function handler(_) {
  console.log("x".repeat(70000));
  return {ok: true};
}
""",
    )
    status, body = invoke_live(enabled_port, "bun_noisy", {"custom": "noisy"})
    assert status == 400, body
    assert "runtime stdout bytes" in body, body

    register(
        enabled_port,
        "bun_scheduled",
        """
export default async function handler(input) {
  return {
    ok: true,
    runtime: "bun",
    trigger: "scheduled",
    schedule_payload: input.payload.custom,
    tenant: input.tenant_id
  };
}
""",
        schedule="*/5 * * * *",
    )
    status, body = dispatch_scheduled(enabled_port, {"custom": "scheduled-bun-live"})
    assert status == 200, body
    dispatch = json.loads(body)
    assert dispatch["trigger"] == "scheduled", dispatch
    assert dispatch["matched"] == 1, dispatch
    assert dispatch["dispatched"] == 1, dispatch
    scheduled_execution = dispatch["executions"][0]
    assert scheduled_execution["function"] == "bun_scheduled", scheduled_execution
    assert scheduled_execution["runtime"] == "bun", scheduled_execution
    assert scheduled_execution["execution_mode"] == "live", scheduled_execution
    assert scheduled_execution["user_code_executed"] is True, scheduled_execution
    scheduled_response = json.loads(scheduled_execution["runtime_response_json"])
    assert scheduled_response["schedule_payload"] == "scheduled-bun-live", scheduled_response

    register(
        enabled_port,
        "bun_cdc",
        """
export default async function handler(input) {
  return {
    ok: true,
    runtime: "bun",
    trigger: "cdc",
    cdc_payload: input.payload.custom,
    function_name: input.function_name
  };
}
""",
        cdc_table="public.edge_orders",
        cdc_operation="insert",
    )
    status, body = dispatch_cdc(enabled_port, {"custom": "cdc-bun-live"})
    assert status == 200, body
    dispatch = json.loads(body)
    assert dispatch["trigger"] == "cdc", dispatch
    assert dispatch["matched"] == 1, dispatch
    assert dispatch["dispatched"] == 1, dispatch
    cdc_execution = dispatch["executions"][0]
    assert cdc_execution["function"] == "bun_cdc", cdc_execution
    assert cdc_execution["runtime"] == "bun", cdc_execution
    assert cdc_execution["execution_mode"] == "live", cdc_execution
    assert cdc_execution["user_code_executed"] is True, cdc_execution
    cdc_response = json.loads(cdc_execution["runtime_response_json"])
    assert cdc_response["cdc_payload"] == "cdc-bun-live", cdc_response
    assert cdc_response["function_name"] == "bun_cdc", cdc_response
finally:
    if disabled is not None:
        terminate(disabled)
    if enabled is not None:
        terminate(enabled)

print("edge_bun_live=passed")
print(f"bun_version={bun_version}")
print("AI_BLAISE_EDGE_RUNTIME_EXECUTION=1")
print(f"AI_BLAISE_BUN_BIN={bun_bin}")
print("user_code_executed=true")
print("runtime_response_contains=bun-live")
print("runtime_env_cleared=true")
print("live_mode_without_executor_rejected=true")
print("live_timeout_rejected=true")
print("live_stdout_cap_rejected=true")
print("trigger_dispatch_scheduled_live=true")
print("trigger_dispatch_cdc_live=true")
PYSMOKE
