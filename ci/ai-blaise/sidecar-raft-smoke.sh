#!/usr/bin/env bash
# 3-node Raft round-trip smoke for FEATURE: S5.
#
# Drives the in-process sidecar runtime through one election + one proposal,
# verifies durable log/snapshot replay, then starts three real raft sidecar
# processes and exercises the HTTP network transport across loopback ports.

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if [[ -f "${HOME}/.cargo/env" ]]; then
  source "${HOME}/.cargo/env"
fi

echo "==> sidecar-raft-smoke: run-runtime-canonical"
canonical_output=$(cargo run -q -p ai_blaise_citus_sidecar_raft -- run-runtime-canonical)
echo "${canonical_output}"

# Validate the TSV header carries the runtime fields the production
# audit reads.
expected_header=$'elected_leader\tterm\tcommitted_index\tcommitted_payload\tcommit_indices\tlast_log_indices'
actual_header=$(printf '%s\n' "${canonical_output}" | head -n 1)
if [[ "${actual_header}" != "${expected_header}" ]]; then
  echo "sidecar-raft-smoke: header mismatch" >&2
  echo "  expected: ${expected_header}" >&2
  echo "  actual:   ${actual_header}" >&2
  exit 1
fi

# Verify the canonical leader, payload, and majority commit.
data_row=$(printf '%s\n' "${canonical_output}" | sed -n '2p')
if [[ "${data_row}" != *"worker-a"* ]]; then
  echo "sidecar-raft-smoke: expected worker-a leader" >&2
  exit 1
fi
if [[ "${data_row}" != *"shard-placement-canonical"* ]]; then
  echo "sidecar-raft-smoke: expected canonical placement payload" >&2
  exit 1
fi
if [[ "${data_row}" != *"worker-a=1"*"worker-b=1"*"worker-c=1"* ]]; then
  echo "sidecar-raft-smoke: every voter must commit at index 1" >&2
  exit 1
fi

echo "==> sidecar-raft-smoke: run-durable-canonical"
durable_output=$(cargo run -q -p ai_blaise_citus_sidecar_raft -- run-durable-canonical)
echo "${durable_output}"
durable_header=$(printf '%s\n' "${durable_output}" | head -n 1)
expected_durable_header=$'appended_entries\treplayed_entries\tsnapshot_index\tsnapshot_term\tlog_path\tsnapshot_path'
if [[ "${durable_header}" != "${expected_durable_header}" ]]; then
  echo "sidecar-raft-smoke: durable header mismatch" >&2
  exit 1
fi
durable_row=$(printf '%s\n' "${durable_output}" | sed -n '2p')
if [[ "${durable_row}" != $'2\t2\t2\t1'* ]]; then
  echo "sidecar-raft-smoke: durable log/snapshot round trip did not replay expected watermark" >&2
  exit 1
fi

echo "==> sidecar-raft-smoke: live multi-process HTTP transport"
cargo build -q -p ai_blaise_citus_sidecar_raft
raft_bin="${repo_root}/target/debug/ai_blaise_citus_sidecar_raft"
tmpdir="$(mktemp -d /tmp/sidecar-raft-smoke.XXXXXX)"
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
for _ in range(3):
    sock = socket.socket()
    sock.bind(("127.0.0.1", 0))
    sockets.append(sock)
    ports.append(sock.getsockname()[1])
print(*ports)
for sock in sockets:
    sock.close()
PY
)
read -r port_a port_b port_c <<<"${ports}"
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

start_raft_node worker-a "${port_a}"
start_raft_node worker-b "${port_b}"
start_raft_node worker-c "${port_c}"

python3 - "${port_a}" "${port_b}" "${port_c}" <<'PY'
import http.client
import json
import sys
import time

port_a, port_b, port_c = map(int, sys.argv[1:])


def request(port, method, path, body=""):
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
    conn.request(method, path, body=body, headers={"content-type": "text/plain"})
    response = conn.getresponse()
    payload = response.read().decode()
    conn.close()
    return response.status, payload


def request_json(port, method, path, body=""):
    status, payload = request(port, method, path, body)
    try:
        data = json.loads(payload)
    except json.JSONDecodeError as exc:
        raise AssertionError((status, payload)) from exc
    return status, data


deadline = time.time() + 20
for port in (port_a, port_b, port_c):
    while True:
        try:
            status, data = request_json(port, "GET", "/readyz")
            if status == 200 and data["ready"] is True:
                break
        except Exception:
            pass
        if time.time() > deadline:
            raise AssertionError(f"raft node on {port} did not become ready")
        time.sleep(0.1)

status, data = request_json(port_a, "POST", "/raft/campaign")
assert status == 200, (status, data)
assert data["node_id"] == "worker-a", data
assert data["role"] == "leader", data
assert data["term"] == 1, data

status, data = request_json(port_a, "POST", "/raft/propose", "networked-placement-intent")
assert status == 201, (status, data)
assert data["role"] == "leader", data
assert data["commit_index"] == 1, data
assert data["committed_payload"] == "networked-placement-intent", data

for node, port in (("worker-a", port_a), ("worker-b", port_b), ("worker-c", port_c)):
    status, data = request_json(port, "GET", "/raft/status")
    assert status == 200, (node, status, data)
    assert data["node_id"] == node, data
    assert data["term"] == 1, data
    assert data["leader_id"] == "worker-a", data
    assert data["commit_index"] == 1, data
    assert data["last_log_index"] == 1, data
    assert data["committed_payload"] == "networked-placement-intent", data

status, data = request_json(port_b, "POST", "/raft/propose", "follower-should-not-commit")
assert status == 409, (status, data)
assert "not leader" in data["error"], data

status, data = request_json(port_b, "POST", "/raft/message", "not-a-valid-wire-message\n")
assert status == 400, (status, data)
assert "from" in data["error"] or "message" in data["error"], data

print("networked_raft_transport=passed")
PY

echo "==> sidecar-raft-smoke: cargo test integration round-trip"
cargo test -p ai_blaise_citus_sidecar_raft --test raft_round_trip -- --nocapture

echo "sidecar-raft-smoke passed"
