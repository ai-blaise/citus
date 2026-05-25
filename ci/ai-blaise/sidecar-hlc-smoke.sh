#!/usr/bin/env bash
# Live HLC sidecar smoke for FEATURE: S9.
#
# Starts the real HLC sidecar HTTP server, advances its local clock, merges a
# peer clock exchange, verifies closed timestamp advancement, and proves
# follower-read AS OF requests serve only at or before the closed timestamp.

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if [[ -f "${HOME}/.cargo/env" ]]; then
  source "${HOME}/.cargo/env"
fi

echo "==> sidecar-hlc-smoke: run-runtime-canonical"
runtime_output="$(cargo run -q -p ai_blaise_citus_sidecar_hlc -- run-runtime-canonical)"
echo "${runtime_output}"
expected_header=$'shard_group\tlocal_physical_ms\tlocal_logical\tclosed_physical_ms\tclosed_logical\tmax_offset_ms\tmax_staleness_ms\treplica_count\tpeers'
actual_header="$(printf '%s\n' "${runtime_output}" | head -n 1)"
if [[ "${actual_header}" != "${expected_header}" ]]; then
  echo "sidecar-hlc-smoke: runtime header mismatch" >&2
  exit 1
fi

echo "==> sidecar-hlc-smoke: cargo build"
cargo build -q -p ai_blaise_citus_sidecar_hlc
hlc_bin="${repo_root}/target/debug/ai_blaise_citus_sidecar_hlc"
tmpdir="$(mktemp -d /tmp/sidecar-hlc-smoke.XXXXXX)"
cleanup() {
  kill "${hlc_pid:-}" >/dev/null 2>&1 || true
  rm -rf "${tmpdir}"
}
trap cleanup EXIT

port="$(python3 - <<'PY'
import socket

sock = socket.socket()
sock.bind(("127.0.0.1", 0))
print(sock.getsockname()[1])
sock.close()
PY
)"

AI_BLAISE_HLC_NODE_ID="worker-a" \
  AI_BLAISE_HLC_SHARD_GROUP="orders-sg" \
  AI_BLAISE_HLC_PEERS="worker-b,worker-c" \
  AI_BLAISE_HLC_REPLICA_COUNT="3" \
  AI_BLAISE_HLC_MAX_OFFSET_MS="500" \
  AI_BLAISE_HLC_MAX_STALENESS_MS="5000" \
  AI_BLAISE_HLC_INITIAL_PHYSICAL_MS="1700000000" \
  AI_BLAISE_LISTEN_ADDR="127.0.0.1:${port}" \
  "${hlc_bin}" serve >"${tmpdir}/hlc.log" 2>&1 &
hlc_pid="$!"

python3 - "${port}" <<'PY'
import http.client
import json
import sys
import time

port = int(sys.argv[1])


def request(method, path, body=""):
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
    conn.request(method, path, body=body, headers={"content-type": "application/x-www-form-urlencoded"})
    response = conn.getresponse()
    payload = response.read().decode()
    conn.close()
    try:
        data = json.loads(payload)
    except json.JSONDecodeError as exc:
        raise AssertionError((response.status, payload)) from exc
    return response.status, data


deadline = time.time() + 20
while True:
    try:
        status, data = request("GET", "/readyz")
        if status == 200 and data["ready"] is True:
            break
    except Exception:
        pass
    if time.time() > deadline:
        raise AssertionError("HLC sidecar did not become ready")
    time.sleep(0.1)

status, closed = request("GET", "/closed_ts")
assert status == 200, (status, closed)
initial_closed = closed["closed_at"]["physical_ms"]
assert closed["shard_group"] == "orders-sg", closed
assert closed["replica_count"] == 3, closed
assert initial_closed == 1699999500, closed

status, tick = request("POST", "/clock/tick", "physical_ms=1700000100")
assert status == 200, (status, tick)
assert tick["event"] == "tick", tick
assert tick["local_clock"]["physical_ms"] == 1700000100, tick
assert tick["closed_at"]["physical_ms"] > initial_closed, tick

status, observed = request(
    "POST",
    "/clock/observe",
    "from=worker-c&physical_ms=1700000200&logical=2&local_physical_ms=1700000201",
)
assert status == 200, (status, observed)
assert observed["event"] == "observe", observed
assert observed["closed_at"]["physical_ms"] >= tick["closed_at"]["physical_ms"], observed
assert observed["closed_at"]["physical_ms"] <= observed["local_clock"]["physical_ms"], observed

status, closed = request("GET", "/closed_ts")
assert status == 200, (status, closed)
closed_physical = closed["closed_at"]["physical_ms"]
assert any(peer["node"] == "worker-c" and peer["physical_ms"] == 1700000200 for peer in closed["peers"]), closed

status, served = request(
    "GET",
    f"/follower_read?replica=worker-a-replica&as_of_physical_ms={closed_physical}&as_of_logical=0",
)
assert status == 200, (status, served)
assert served["decision"] == "serve_from_follower", served
assert served["serve_from_follower"] is True, served

status, rejected = request(
    "GET",
    f"/follower_read?replica=worker-a-replica&as_of_physical_ms={closed_physical + 1}&as_of_logical=0",
)
assert status == 409, (status, rejected)
assert rejected["decision"] == "reject_not_closed", rejected
assert rejected["serve_from_follower"] is False, rejected

status, unknown = request(
    "POST",
    "/clock/observe",
    "from=worker-z&physical_ms=1700000300&logical=0&local_physical_ms=1700000301",
)
assert status == 409, (status, unknown)
assert "unknown HLC peer" in unknown["error"], unknown

print("hlc_live_gate=passed")
PY

echo "==> sidecar-hlc-smoke: cargo test"
cargo test -p ai_blaise_citus_sidecar_hlc --all-targets -- --nocapture

echo "sidecar-hlc-smoke passed"
