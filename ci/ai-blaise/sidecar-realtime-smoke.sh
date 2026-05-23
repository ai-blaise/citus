#!/usr/bin/env bash
# FEATURE: RT1 RT2 RT3 RT4 RT5
#
# End-to-end smoke test for the realtime sidecar:
# 1. Start sidecar/cdc serve-runtime on an ephemeral port.
# 2. Start sidecar/realtime serve-runtime on ephemeral WS + CDC-ingest ports,
#    pointing the cdc -> realtime bridge at the realtime CDC ingest port.
# 3. Open a raw WS client, perform the phoenix-channel join, then send a
#    canonical wal2json INSERT to the CDC sidecar's /ingest endpoint.
# 4. Assert the WS client receives a `postgres_changes` frame on the
#    `realtime:public:orders` channel.
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

# Allocate two TCP ports and one Unix-domain socket path up-front.
read -r cdc_port rt_ws_port < <(python3 -c '
import socket
ports = []
sockets = []
for _ in range(2):
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    ports.append(s.getsockname()[1])
    sockets.append(s)
for s in sockets:
    s.close()
print(" ".join(str(p) for p in ports))
')

cdc_addr="127.0.0.1:${cdc_port}"
rt_ws_addr="127.0.0.1:${rt_ws_port}"
rt_cdc_sock="$(mktemp -u /tmp/ai-blaise-realtime-cdc.XXXXXX.sock)"
rt_cdc_addr="unix:${rt_cdc_sock}"

export CDC_LISTEN_ADDR="${cdc_addr}"
export CDC_REALTIME_BRIDGE_ADDR="${rt_cdc_addr}"
export REALTIME_WS_LISTEN_ADDR="${rt_ws_addr}"
export REALTIME_CDC_INGEST_ADDR="${rt_cdc_addr}"

# Build both binaries up-front so the runtime starts respond quickly.
cargo build -q -p ai_blaise_citus_sidecar_cdc -p ai_blaise_citus_sidecar_realtime

cleanup() {
  rm -f "${rt_cdc_sock:-}"
  for pid in ${cdc_pid:-} ${rt_pid:-}; do
    if [[ -n "${pid}" ]] && kill -0 "${pid}" 2>/dev/null; then
      kill "${pid}" 2>/dev/null || true
      wait "${pid}" 2>/dev/null || true
    fi
  done
}
trap cleanup EXIT

# Start realtime first so the CDC bridge has a target when events come in.
target/debug/ai_blaise_citus_sidecar_realtime serve-runtime &
rt_pid=$!

# Wait for WS TCP port and CDC UDS path.
for _ in $(seq 1 50); do
  if (echo > "/dev/tcp/127.0.0.1/${rt_ws_port}") >/dev/null 2>&1; then
    break
  fi
  sleep 0.1
done
for _ in $(seq 1 50); do
  if [[ -S "${rt_cdc_sock}" ]]; then
    break
  fi
  sleep 0.1
done
[[ -S "${rt_cdc_sock}" ]] || { echo "realtime CDC UDS did not appear" >&2; exit 1; }

target/debug/ai_blaise_citus_sidecar_cdc serve-runtime &
cdc_pid=$!

for _ in $(seq 1 50); do
  if (echo > "/dev/tcp/127.0.0.1/${cdc_port}") >/dev/null 2>&1; then
    break
  fi
  sleep 0.1
done

python3 - "${cdc_addr}" "${rt_ws_addr}" <<'PY'
import base64
import http.client
import json
import re
import socket
import sys
import time

cdc_addr, rt_ws_addr = sys.argv[1:3]
cdc_host, cdc_port = cdc_addr.split(":")
rt_host, rt_ws_port = rt_ws_addr.split(":")

def send_ws_frame(sock, text, opcode=0x1):
    payload = text.encode("utf-8")
    header = bytearray([0x80 | opcode])
    if len(payload) < 126:
        header.append(0x80 | len(payload))
    else:
        header.append(0x80 | 126)
        header += len(payload).to_bytes(2, "big")
    header += b"\x00\x00\x00\x00"  # mask
    sock.sendall(bytes(header) + payload)


def decode_ws_frame(buffer):
    if len(buffer) < 2:
        return None, 0
    opcode = buffer[0] & 0x0F
    masked = buffer[1] & 0x80
    length = buffer[1] & 0x7F
    pos = 2
    if length == 126:
        if len(buffer) < 4:
            return None, 0
        length = int.from_bytes(buffer[2:4], "big")
        pos = 4
    elif length == 127:
        if len(buffer) < 10:
            return None, 0
        length = int.from_bytes(buffer[2:10], "big")
        pos = 10
    mask = b""
    if masked:
        if len(buffer) < pos + 4:
            return None, 0
        mask = buffer[pos:pos + 4]
        pos += 4
    if len(buffer) < pos + length:
        return None, 0
    payload = bytearray(buffer[pos:pos + length])
    if masked:
        for i in range(length):
            payload[i] ^= mask[i % 4]
    return (opcode, bytes(payload)), pos + length


# 0. Health/ready/metrics on the WS listener.
def probe(path):
    conn = http.client.HTTPConnection(rt_host, int(rt_ws_port), timeout=10)
    conn.request("GET", path, headers={"Connection": "close"})
    response = conn.getresponse()
    raw = response.read()
    assert response.status == 200, (path, response.status, raw)
    return raw

assert b"ok" in probe("/healthz")
assert b"ready" in probe("/readyz")
assert b"ai_blaise_realtime_broadcasts" in probe("/metrics")

# 1. WS upgrade + phx_join.
sock = socket.create_connection((rt_host, int(rt_ws_port)), timeout=10)
sock.settimeout(10)
sock.sendall(
    "GET /realtime/v1/websocket?vsn=2.0.0 HTTP/1.1\r\n"
    "Host: localhost\r\n"
    "Upgrade: websocket\r\n"
    "Connection: Upgrade\r\n"
    "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n"
    "Sec-WebSocket-Version: 13\r\n\r\n".encode("utf-8")
)
buffer = b""
while b"\r\n\r\n" not in buffer:
    chunk = sock.recv(1024)
    if not chunk:
        sys.exit("ws handshake closed prematurely")
    buffer += chunk
header_text = buffer.split(b"\r\n\r\n", 1)[0].decode("utf-8")
assert header_text.startswith("HTTP/1.1 101 "), header_text
assert "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=" in header_text, header_text
buffer = buffer.split(b"\r\n\r\n", 1)[1]

join = [
    "1",
    "1",
    "realtime:public:orders",
    "phx_join",
    {
        "tenant_id": "tenant-a",
        "user_id": "user-a",
        "schema": "public",
        "table": "orders",
        "operation": "INSERT",
        "presence": True,
        "online_at": "2026-05-19T12:00:00Z",
    },
]
send_ws_frame(sock, json.dumps(join))

# Drain server frames for phx_reply + presence_diff.
received = []
deadline = time.time() + 5
while time.time() < deadline and len(received) < 2:
    chunk = sock.recv(2048)
    if not chunk:
        break
    buffer += chunk
    while True:
        frame, consumed = decode_ws_frame(buffer)
        if frame is None:
            break
        opcode, payload = frame
        buffer = buffer[consumed:]
        if opcode != 0x1:
            continue
        received.append(json.loads(payload.decode("utf-8")))

assert any(f[3] == "phx_reply" and f[4]["status"] == "ok" for f in received), received
assert any(f[3] == "presence_diff" for f in received), received

# 2. Push CDC event through cdc /ingest. The cdc->realtime bridge relays
#    it to the realtime ingest port, which broadcasts via the hub.
conn = http.client.HTTPConnection(cdc_host, int(cdc_port), timeout=10)
frame_payload = {
    "start_lsn": "16/B374D848",
    "end_lsn": "16/B374D900",
    "payload": json.dumps({
        "change": [{
            "kind": "insert",
            "schema": "public",
            "table": "orders",
            "columnnames": ["id", "tenant_id", "status", "email"],
            "columnvalues": [42, "tenant-a", "paid", "person@example.com"],
        }]
    }),
}
conn.request(
    "POST",
    "/ingest",
    body=json.dumps(frame_payload),
    headers={"Content-Type": "application/json", "Connection": "close"},
)
response = conn.getresponse()
raw = response.read()
assert response.status == 200, (response.status, raw)
report = json.loads(raw)
assert report["events"][0]["table"] == "public.orders"

# 3. Wait for postgres_changes on the WS client.
postgres_changes = None
deadline = time.time() + 5
while time.time() < deadline and postgres_changes is None:
    chunk = sock.recv(2048)
    if not chunk:
        time.sleep(0.05)
        continue
    buffer += chunk
    while True:
        frame, consumed = decode_ws_frame(buffer)
        if frame is None:
            break
        opcode, payload = frame
        buffer = buffer[consumed:]
        if opcode != 0x1:
            continue
        envelope = json.loads(payload.decode("utf-8"))
        if envelope[3] == "postgres_changes":
            postgres_changes = envelope
            break

assert postgres_changes is not None, "no postgres_changes frame"
assert postgres_changes[2] == "realtime:public:orders", postgres_changes
payload = postgres_changes[4]
assert payload["schema"] == "public", payload
assert payload["table"] == "orders", payload
assert payload["type"] == "INSERT", payload
assert payload["tenant_id"] == "tenant-a", payload
print(
    "OK realtime sidecar e2e: phx_join, presence_diff, "
    f"postgres_changes (tenant={payload['tenant_id']}, lsn={payload['lsn']})"
)
PY

echo "OK realtime-sidecar smoke complete"
