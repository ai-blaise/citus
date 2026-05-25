#!/usr/bin/env bash
# Live txn-status -> Raft HTTP transport smoke for FEATURE: T5.
#
# Starts three real raft sidecar OS processes, elects worker-a, then starts the
# txn-status sidecar configured with AI_BLAISE_TXN_RAFT_LEADER_ADDR. The test
# proves stage/commit decisions are appended through the networked Raft log
# before the txn-status HTTP API acknowledges them, and that follower-backed
# replication failures fail closed without materialising a txn record.

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if [[ -f "${HOME}/.cargo/env" ]]; then
  source "${HOME}/.cargo/env"
fi

echo "==> txn-status-networked-raft-smoke: build binaries"
cargo build -q -p ai_blaise_citus_sidecar_raft -p ai_blaise_citus_sidecar_txn_status
raft_bin="${repo_root}/target/debug/ai_blaise_citus_sidecar_raft"
txn_status_bin="${repo_root}/target/debug/ai_blaise_citus_sidecar_txn_status"

tmpdir="$(mktemp -d /tmp/txn-status-networked-raft-smoke.XXXXXX)"
pids=()
cleanup() {
  for pid in "${pids[@]:-}"; do
    kill "${pid}" >/dev/null 2>&1 || true
  done
  rm -rf "${tmpdir}"
}
trap cleanup EXIT

ports=$(python3 - <<'PY'
import socket

sockets = []
ports = []
for _ in range(5):
    sock = socket.socket()
    sock.bind(("127.0.0.1", 0))
    sockets.append(sock)
    ports.append(sock.getsockname()[1])
print(*ports)
for sock in sockets:
    sock.close()
PY
)
read -r port_a port_b port_c txn_port follower_txn_port <<<"${ports}"
members="worker-a,worker-b,worker-c"
peers="worker-a=127.0.0.1:${port_a},worker-b=127.0.0.1:${port_b},worker-c=127.0.0.1:${port_c}"

start_raft_node() {
  local node_id="$1"
  local port="$2"
  AI_BLAISE_RAFT_NODE_ID="${node_id}" \
    AI_BLAISE_RAFT_MEMBERS="${members}" \
    AI_BLAISE_RAFT_PEERS="${peers}" \
    AI_BLAISE_LISTEN_ADDR="127.0.0.1:${port}" \
    "${raft_bin}" serve >"${tmpdir}/${node_id}.log" 2>&1 &
  pids+=("$!")
}

start_txn_status() {
  local name="$1"
  local port="$2"
  local raft_addr="$3"
  AI_BLAISE_TXN_RAFT_GROUP="txn-status-orders" \
    AI_BLAISE_TXN_RAFT_VOTERS="${members}" \
    AI_BLAISE_TXN_RAFT_LEADER_ADDR="${raft_addr}" \
    AI_BLAISE_LISTEN_ADDR="127.0.0.1:${port}" \
    "${txn_status_bin}" serve >"${tmpdir}/${name}.log" 2>&1 &
  pids+=("$!")
}

start_raft_node worker-a "${port_a}"
start_raft_node worker-b "${port_b}"
start_raft_node worker-c "${port_c}"
start_txn_status txn-status-leader "${txn_port}" "127.0.0.1:${port_a}"
start_txn_status txn-status-follower "${follower_txn_port}" "127.0.0.1:${port_b}"

python3 - "${port_a}" "${port_b}" "${port_c}" "${txn_port}" "${follower_txn_port}" <<'PY'
import http.client
import json
import sys
import time

port_a, port_b, port_c, txn_port, follower_txn_port = map(int, sys.argv[1:])


def request(port, method, path, body=None, content_type="application/json"):
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
    payload = None
    headers = {}
    if body is not None:
        if isinstance(body, str):
            payload = body
            headers["content-type"] = content_type
        else:
            payload = json.dumps(body)
            headers["content-type"] = "application/json"
    conn.request(method, path, payload, headers)
    response = conn.getresponse()
    raw = response.read().decode()
    conn.close()
    try:
        parsed = json.loads(raw)
    except json.JSONDecodeError:
        parsed = raw
    return response.status, parsed


def wait_ready(port):
    deadline = time.time() + 20
    while True:
        try:
            status, body = request(port, "GET", "/readyz")
            if status == 200 and body["ready"] is True:
                return
        except Exception:
            pass
        if time.time() > deadline:
            raise AssertionError(f"service on port {port} did not become ready")
        time.sleep(0.1)


for port in (port_a, port_b, port_c, txn_port, follower_txn_port):
    wait_ready(port)

status, body = request(port_a, "POST", "/raft/campaign", "", "text/plain")
assert status == 200, (status, body)
assert body["node_id"] == "worker-a" and body["role"] == "leader", body

staging = {
    "txn_id": "txn-live-raft-1",
    "coordinator": "worker-a",
    "staging_physical_ms": 1700000000,
    "intents": [
        {"shard_id": 10, "key_range": "[a,m)", "required_acks": 2, "replica_acks": 0},
        {"shard_id": 11, "key_range": "[m,z)", "required_acks": 2, "replica_acks": 0},
    ],
}
status, body = request(txn_port, "POST", "/txn/staging", staging)
assert status == 201, (status, body)
assert body["status"] == "staging", body
assert body["raft_transport"] == "http_leader", body
assert body["raft_index"] == 1, body

status, raft_status = request(port_a, "GET", "/raft/status")
assert status == 200, (status, raft_status)
assert raft_status["commit_index"] == 1, raft_status
assert raft_status["committed_payload"] == "stage:txn-live-raft-1:worker-a", raft_status

finalize = {"txn_id": "txn-live-raft-1", "observed_physical_ms": 1700000010}
status, body = request(txn_port, "POST", "/txn/finalize", finalize)
assert status == 200, (status, body)
assert body["decision"] == "wait_for_replication_evidence", body
assert body["raft_index"] == 1, body

status, raft_status = request(port_a, "GET", "/raft/status")
assert status == 200, (status, raft_status)
assert raft_status["commit_index"] == 1, raft_status
assert raft_status["committed_payload"] == "stage:txn-live-raft-1:worker-a", raft_status

for shard_id in (10, 11):
    status, body = request(
        txn_port,
        "POST",
        "/txn/ack",
        {"txn_id": "txn-live-raft-1", "shard_id": shard_id, "replica_acks": 2},
    )
    assert status == 200, (status, body)

status, body = request(txn_port, "POST", "/txn/finalize", finalize)
assert status == 200, (status, body)
assert body["decision"] == "commit" and body["status"] == "committed", body
assert body["raft_index"] == 2, body

status, raft_status = request(port_a, "GET", "/raft/status")
assert status == 200, (status, raft_status)
assert raft_status["commit_index"] == 2, raft_status
assert raft_status["committed_payload"] == "commit:txn-live-raft-1", raft_status

for node, port in (("worker-a", port_a), ("worker-b", port_b), ("worker-c", port_c)):
    status, body = request(port, "GET", "/raft/status")
    assert status == 200, (node, status, body)
    assert body["node_id"] == node, body
    assert body["leader_id"] == "worker-a", body
    assert body["commit_index"] == 2, body
    assert body["committed_payload"] == "commit:txn-live-raft-1", body

follower_staging = {
    **staging,
    "txn_id": "txn-follower-fails-closed",
}
status, body = request(follower_txn_port, "POST", "/txn/staging", follower_staging)
assert status == 409, (status, body)
assert "not leader" in body["error"], body

status, body = request(follower_txn_port, "GET", "/txn/txn-follower-fails-closed/status")
assert status == 404, (status, body)

print("txn_status_networked_raft=passed")
print("stage_payload=stage:txn-live-raft-1:worker-a")
print("commit_payload=commit:txn-live-raft-1")
print("follower_replication_failure=fail_closed")
PY

echo "txn-status-networked-raft-smoke passed"
