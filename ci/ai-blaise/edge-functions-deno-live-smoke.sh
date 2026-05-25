#!/usr/bin/env bash
set -euo pipefail

# FEATURE: EF1 EF5
# Live inline-Deno execution smoke for the edge-functions sidecar. This proves
# the explicit opt-in Deno process path, bounded timeout behavior, fail-closed
# disabled mode, default no-env-permission isolate boundary, and scheduled/CDC
# trigger dispatch through the same live executor. It does not
# prove Bun execution, package installation, URI/bundle fetching, user-code
# initiated DB callbacks, queue/broker delivery, live CDC slot tailing, or
# Kubernetes deployment.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

python3 <<'PY'
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
DENO_VERSION = os.environ.get("DENO_VERSION", "v2.1.4")


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


def deno_asset_name():
    system = platform.system().lower()
    machine = platform.machine().lower()
    if system == "linux" and machine in ("x86_64", "amd64"):
        return "deno-x86_64-unknown-linux-gnu.zip"
    if system == "linux" and machine in ("aarch64", "arm64"):
        return "deno-aarch64-unknown-linux-gnu.zip"
    if system == "darwin" and machine in ("aarch64", "arm64"):
        return "deno-aarch64-apple-darwin.zip"
    if system == "darwin" and machine in ("x86_64", "amd64"):
        return "deno-x86_64-apple-darwin.zip"
    fail(f"unsupported platform for Deno smoke bootstrap: {system}/{machine}")


def ensure_deno():
    configured = os.environ.get("AI_BLAISE_DENO_BIN") or os.environ.get("DENO_BIN")
    if configured:
        if os.path.isfile(configured) and os.access(configured, os.X_OK):
            return configured
        fail(f"configured Deno binary is not executable: {configured}")
    discovered = shutil.which("deno")
    if discovered:
        return discovered

    cache_dir = os.path.join(tempfile.gettempdir(), "ai-blaise-deno-live-smoke", DENO_VERSION)
    deno_path = os.path.join(cache_dir, "deno")
    if os.path.isfile(deno_path) and os.access(deno_path, os.X_OK):
        return deno_path

    os.makedirs(cache_dir, exist_ok=True)
    asset = deno_asset_name()
    archive = os.path.join(cache_dir, asset)
    url = f"https://github.com/denoland/deno/releases/download/{DENO_VERSION}/{asset}"
    urllib.request.urlretrieve(url, archive)
    with zipfile.ZipFile(archive) as zip_file:
        zip_file.extractall(cache_dir)
    mode = os.stat(deno_path).st_mode
    os.chmod(deno_path, mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    return deno_path


def start_sidecar(*, runtime_enabled, deno_bin):
    port = free_port()
    env = os.environ.copy()
    env["AI_BLAISE_LISTEN_ADDR"] = f"127.0.0.1:{port}"
    env.pop("AI_BLAISE_EDGE_DB_CALLBACK_EXECUTION", None)
    if runtime_enabled:
        env["AI_BLAISE_EDGE_RUNTIME_EXECUTION"] = "1"
        env["AI_BLAISE_DENO_BIN"] = deno_bin
    else:
        env.pop("AI_BLAISE_EDGE_RUNTIME_EXECUTION", None)
        env.pop("AI_BLAISE_DENO_BIN", None)
    proc = subprocess.Popen(
        [BINARY, "serve"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
    )
    wait_ready(proc, port)
    return proc, port


def register(port, name, code):
    body = json.dumps(
        {
            "name": name,
            "runtime": "deno",
            "code": code,
            "http_path": f"/{name}",
        }
    )
    status, response = request(port, "POST", "/functions", body=body)
    if status != 201:
        fail(f"{name} registration failed: {status} {response}")


def register_triggered(port, name, code, *, schedule=None, cdc_table=None, cdc_operation=None):
    body = {
        "name": name,
        "runtime": "deno",
        "code": code,
    }
    if schedule is not None:
        body["schedule"] = schedule
    if cdc_table is not None:
        body["cdc_table"] = cdc_table
    if cdc_operation is not None:
        body["cdc_operation"] = cdc_operation
    status, response = request(port, "POST", "/functions", body=json.dumps(body))
    if status != 201:
        fail(f"{name} triggered registration failed: {status} {response}")


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

deno_bin = ensure_deno()
deno_version = run([deno_bin, "--version"], timeout=30).splitlines()[0]

disabled = None
enabled = None
try:
    disabled, disabled_port = start_sidecar(runtime_enabled=False, deno_bin=deno_bin)
    register(
        disabled_port,
        "hello_disabled",
        "export default async function handler(input) { return {should_not_execute: input}; }",
    )
    status, body = invoke_live(disabled_port, "hello_disabled", {"custom": "blocked"})
    assert status == 501, body
    assert "AI_BLAISE_EDGE_RUNTIME_EXECUTION" in body, body
    terminate(disabled)
    disabled = None

    enabled, enabled_port = start_sidecar(runtime_enabled=True, deno_bin=deno_bin)
    register(
        enabled_port,
        "hello_live",
        """
export default async function handler(input) {
  let envStatus = "not_checked";
  try {
    Deno.env.get("HOME");
    envStatus = "leaked";
  } catch (_) {
    envStatus = "permission_denied";
  }
  return {
    ok: true,
    tenant: input.tenant_id,
    payload_bytes: input.payload_bytes,
    custom: input.payload.custom,
    env_status: envStatus
  };
}
""",
    )
    status, body = invoke_live(enabled_port, "hello_live", {"custom": "deno-live"})
    assert status == 200, body
    execution = json.loads(body)
    assert execution["status"] == "executed", execution
    assert execution["execution_mode"] == "live", execution
    assert execution["user_code_executed"] is True, execution
    assert execution["db_callback_used"] is False, execution
    runtime_response = json.loads(execution["runtime_response_json"])
    assert runtime_response["ok"] is True, runtime_response
    assert runtime_response["tenant"] == "tenant-a", runtime_response
    assert runtime_response["custom"] == "deno-live", runtime_response
    assert runtime_response["env_status"] == "permission_denied", runtime_response
    assert execution["response_bytes"] == len(execution["runtime_response_json"]), execution

    register(
        enabled_port,
        "slow_live",
        """
export default async function handler(_) {
  await new Promise((resolve) => setTimeout(resolve, 1000));
  return {ok: true};
}
""",
    )
    status, body = invoke_live(enabled_port, "slow_live", {"custom": "slow"}, timeout_ms=50)
    assert status == 504, body
    assert "timeout_ms 50" in body, body

    register(
        enabled_port,
        "noisy_live",
        """
export default async function handler(_) {
  console.log("x".repeat(70000));
  return {ok: true};
}
""",
    )
    status, body = invoke_live(enabled_port, "noisy_live", {"custom": "noisy"})
    assert status == 400, body
    assert "runtime stdout bytes" in body, body

    register_triggered(
        enabled_port,
        "scheduled_live",
        """
export default async function handler(input) {
  return {
    ok: true,
    trigger: "scheduled",
    schedule_payload: input.payload.custom,
    tenant: input.tenant_id
  };
}
""",
        schedule="*/5 * * * *",
    )
    status, body = dispatch_scheduled(enabled_port, {"custom": "scheduled-live"})
    assert status == 200, body
    dispatch = json.loads(body)
    assert dispatch["trigger"] == "scheduled", dispatch
    assert dispatch["matched"] == 1, dispatch
    assert dispatch["dispatched"] == 1, dispatch
    scheduled_execution = dispatch["executions"][0]
    assert scheduled_execution["function"] == "scheduled_live", scheduled_execution
    assert scheduled_execution["execution_mode"] == "live", scheduled_execution
    assert scheduled_execution["user_code_executed"] is True, scheduled_execution
    scheduled_response = json.loads(scheduled_execution["runtime_response_json"])
    assert scheduled_response["schedule_payload"] == "scheduled-live", scheduled_response

    register_triggered(
        enabled_port,
        "cdc_live",
        """
export default async function handler(input) {
  return {
    ok: true,
    trigger: "cdc",
    cdc_payload: input.payload.custom,
    function_name: input.function_name
  };
}
""",
        cdc_table="public.edge_orders",
        cdc_operation="insert",
    )
    status, body = dispatch_cdc(enabled_port, {"custom": "cdc-live"})
    assert status == 200, body
    dispatch = json.loads(body)
    assert dispatch["trigger"] == "cdc", dispatch
    assert dispatch["matched"] == 1, dispatch
    assert dispatch["dispatched"] == 1, dispatch
    cdc_execution = dispatch["executions"][0]
    assert cdc_execution["function"] == "cdc_live", cdc_execution
    assert cdc_execution["execution_mode"] == "live", cdc_execution
    assert cdc_execution["user_code_executed"] is True, cdc_execution
    cdc_response = json.loads(cdc_execution["runtime_response_json"])
    assert cdc_response["cdc_payload"] == "cdc-live", cdc_response
    assert cdc_response["function_name"] == "cdc_live", cdc_response
finally:
    if disabled is not None:
        terminate(disabled)
    if enabled is not None:
        terminate(enabled)

print("edge_deno_live=passed")
print(f"deno_version={deno_version}")
print("AI_BLAISE_EDGE_RUNTIME_EXECUTION=1")
print(f"AI_BLAISE_DENO_BIN={deno_bin}")
print("user_code_executed=true")
print("runtime_response_contains=deno-live")
print("runtime_default_env_permission=permission_denied")
print("live_mode_without_executor_rejected=true")
print("live_timeout_rejected=true")
print("live_stdout_cap_rejected=true")
print("trigger_dispatch_scheduled_live=true")
print("trigger_dispatch_cdc_live=true")
PY
