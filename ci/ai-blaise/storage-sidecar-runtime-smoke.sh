#!/usr/bin/env bash
set -euo pipefail

# FEATURE: Sto1 Sto3 Sto4 Sto5

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

export PATH="${HOME}/.cargo/bin:${PATH}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/cargo-target-storage-smoke}"

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


def request(port, method, path, body=None):
    headers = {}
    if body is not None:
        headers["content-type"] = "application/json"
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
    conn.request(method, path, body=body, headers=headers)
    response = conn.getresponse()
    data = response.read().decode("utf-8")
    conn.close()
    return response.status, data


def require_json(status, data, expected):
    assert status == expected, data
    return json.loads(data)


port = free_port()
env = os.environ.copy()
env["AI_BLAISE_LISTEN_ADDR"] = f"127.0.0.1:{port}"
proc = subprocess.Popen(
    ["cargo", "run", "-q", "-p", "ai_blaise_citus_sidecar_storage", "--", "serve"],
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
    env=env,
)

try:
    for _ in range(80):
        try:
            status, data = request(port, "GET", "/readyz")
            if status == 200 and '"component":"storage"' in data:
                break
        except OSError:
            pass
        if proc.poll() is not None:
            stderr = proc.stderr.read() if proc.stderr is not None else ""
            raise AssertionError(f"storage serve exited before readiness: {stderr}")
        time.sleep(0.25)
    else:
        raise AssertionError("storage sidecar did not become ready")

    status, data = request(port, "GET", "/healthz")
    health = require_json(status, data, 200)
    assert health["component"] == "storage"
    assert health["ready"] is True

    status, data = request(port, "GET", "/metrics")
    assert status == 200, data
    assert 'ai_blaise_sidecar_ready{component="storage"} 1' in data

    status, data = request(port, "GET", "/storage/policy")
    policy = require_json(status, data, 200)
    assert policy["provider"] == "s3"
    assert policy["bucket"] == "tenant-files"
    assert policy["tenant_id"] == "tenant-a"
    assert policy["acl"] == "tenant_read_write"
    assert policy["metadata_table"] == "storage.objects"
    assert policy["presigned_url_ttl_seconds"] == 900
    assert policy["antivirus_fail_closed"] is True
    assert policy["quarantine_bucket"] == "quarantine"

    status, data = request(port, "POST", "/storage/presign", '{"method":"put","ttl_seconds":900}')
    presign = require_json(status, data, 200)
    assert presign["bucket"] == "tenant-files"
    assert presign["method"] == "put"
    assert presign["expires_in_seconds"] == 900
    assert "signature=ai-blaise-canonical" in presign["url"]

    status, data = request(port, "POST", "/storage/presign", '{"ttl_seconds":901}')
    failure = require_json(status, data, 400)
    assert "exceeds policy" in failure["error"]

    clean_upload = json.dumps(
        {
            "bucket": "tenant-files",
            "object_key": "orders/2.pdf",
            "tenant_id": "tenant-a",
            "content_type": "application/pdf",
            "size_bytes": 128,
            "content_digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "scan_signature": "clean:pdf",
        },
        separators=(",", ":"),
    )
    status, data = request(port, "POST", "/storage/upload", clean_upload)
    upload = require_json(status, data, 200)
    assert upload["stored"] is True
    assert upload["quarantined"] is False
    assert upload["antivirus_verdict"] == "clean"
    assert upload["state"]["stored_objects"] == 1
    assert upload["state"]["scanned_objects"] == 1

    infected_upload = json.dumps(
        {
            "bucket": "tenant-files",
            "object_key": "orders/eicar.txt",
            "tenant_id": "tenant-a",
            "content_type": "text/plain",
            "size_bytes": 64,
            "content_digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "scan_signature": "malware:eicar-test",
        },
        separators=(",", ":"),
    )
    status, data = request(port, "POST", "/storage/upload", infected_upload)
    infected = require_json(status, data, 200)
    assert infected["stored"] is False
    assert infected["quarantined"] is True
    assert infected["antivirus_verdict"] == "infected"
    assert infected["state"]["stored_objects"] == 1
    assert infected["state"]["quarantined_objects"] == 1
    assert infected["state"]["scanned_objects"] == 2

    status, data = request(port, "GET", "/storage/state")
    state = require_json(status, data, 200)
    assert state["stored_objects"] == 1
    assert state["quarantined_objects"] == 1
    assert state["issued_urls"] == 1
    assert state["scanned_objects"] == 2

    status, data = request(port, "POST", "/drain")
    drain = require_json(status, data, 202)
    assert drain["accepting_new_work"] is False
    status, data = request(port, "GET", "/readyz")
    ready = require_json(status, data, 503)
    assert ready["state"] == "draining"
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

print("ai_blaise_citus storage sidecar runtime smoke passed")
PY
