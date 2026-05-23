#!/usr/bin/env bash
# FEATURE: C1 C2 C3 C9 C14 C15 WH3
#
# Smoke test for the CDC sidecar real runtime:
# 1. Spin the sidecar's serve-runtime TCP control plane on an ephemeral port.
# 2. POST a canonical wal2json frame to /ingest.
# 3. Assert the dispatch report enumerates all seven sinks
#    (webhook, realtime, nats, pubsub, kafka, kinesis, http2),
#    that the email column was anonymized, and that bytes_total > 0.
# 4. Hit /streams, /state, /dlq and validate their JSON.
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

# Source cargo if needed.
if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

bin="ai_blaise_citus_sidecar_cdc"
port=$(python3 -c 'import socket, sys
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()')
addr="127.0.0.1:${port}"
export CDC_LISTEN_ADDR="${addr}"
dlq_path="$(mktemp)"
export CDC_DLQ_PATH="${dlq_path}"

cleanup() {
  if [[ -n "${pid:-}" ]] && kill -0 "${pid}" 2>/dev/null; then
    kill "${pid}" 2>/dev/null || true
    wait "${pid}" 2>/dev/null || true
  fi
  rm -f "${dlq_path}"
}
trap cleanup EXIT

cargo run -q -p "${bin}" -- serve-runtime &
pid=$!

# Wait until the port is accepting connections.
for _ in $(seq 1 50); do
  if (echo > "/dev/tcp/127.0.0.1/${port}") >/dev/null 2>&1; then
    break
  fi
  sleep 0.1
done

python3 - "${addr}" <<'PY'
import http.client
import json
import sys

addr = sys.argv[1]
host, port = addr.split(":")

def request(method, path, body=None, status=200):
    conn = http.client.HTTPConnection(host, int(port), timeout=10)
    headers = {"Connection": "close"}
    if body is not None:
        headers["Content-Type"] = "application/json"
    conn.request(method, path, body=body, headers=headers)
    response = conn.getresponse()
    raw = response.read()
    if response.status != status:
        raise SystemExit(f"{method} {path} -> {response.status}: {raw!r}")
    if response.getheader("Content-Type", "").startswith("application/json"):
        return json.loads(raw)
    return raw

# health/ready/metrics
health = request("GET", "/healthz")
assert health["status"] == "ok", health
ready = request("GET", "/readyz")
assert ready["ready"] is True, ready
metrics = request("GET", "/metrics")
assert b"ai_blaise_cdc_delivered_events" in metrics, metrics

# /streams
streams = request("GET", "/streams")
sink_kinds = sorted({entry["kind"] for entry in streams})
expected_kinds = sorted([
    "webhook", "realtime", "nats", "pubsub", "kafka", "kinesis", "http2"
])
assert sink_kinds == expected_kinds, f"sink kinds: {sink_kinds}"

# /state initial
state = request("GET", "/state")
assert state["dlq_pending"] == 0, state
assert state["delivered_events"] == 0, state

# Ingest canonical wal2json frame.
frame = {
    "start_lsn": "16/B374D848",
    "end_lsn": "16/B374D900",
    "payload": json.dumps({
        "change": [{
            "kind": "insert",
            "schema": "public",
            "table": "orders",
            "columnnames": ["id", "tenant_id", "status", "email"],
            "columnvalues": [1, "tenant-a", "paid", "person@example.com"],
        }]
    }),
}
report = request("POST", "/ingest", body=json.dumps(frame))
assert report["start_lsn"] == "16/B374D848", report
assert report["end_lsn"] == "16/B374D900", report
assert report["bytes_total"] > 0, report
assert len(report["events"]) == 1, report
event = report["events"][0]
assert event["tenant_id"] == "tenant-a"
assert event["table"] == "public.orders"
assert event["operation"] == "insert"
assert "email" in event["anonymized_columns"], event
frame_sinks = sorted({frame["sink"] for frame in event["frames"]})
assert frame_sinks == expected_kinds, frame_sinks
for entry in event["frames"]:
    assert entry["bytes"] > 0, entry
    # Live dispatch is disabled by default, so every frame is "encoded".
    assert entry["outcome"] == "encoded", entry

# State should now show advanced LSNs.
state = request("GET", "/state")
assert state["delivered_events"] == 1, state
assert state["delivered_sink_writes"] == 7, state

# /dlq should still be empty in encoded-only mode.
dlq = request("GET", "/dlq")
assert dlq == [], dlq

# Restart a stream.
restart = request("POST", "/streams/cdc/restart", body=b"", status=202)
assert restart["restarted"] is True, restart

print("OK cdc-sidecar serve-runtime: 7 sinks, email anonymized, LSN advanced")
PY

# Also run the deterministic stdout TSV path so the smoke covers both paths.
tsv=$(cargo run -q -p "${bin}" -- run-live-canonical)
echo "${tsv}" | head -5
echo "${tsv}" | grep -q "kafka	cdc.orders" || { echo "missing kafka row" >&2; exit 1; }
echo "${tsv}" | grep -q "kinesis	cdc-orders" || { echo "missing kinesis row" >&2; exit 1; }
echo "${tsv}" | grep -q "http2	https://h2.example.com/cdc/orders" || { echo "missing http2 row" >&2; exit 1; }
echo "${tsv}" | grep -q "email" || { echo "expected anonymized=email column" >&2; exit 1; }

echo "OK cdc-sidecar smoke complete"
