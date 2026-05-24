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
# 5. When Docker is available, boot a live PostgreSQL event-trigger DDL capture
#    harness and feed the captured DDL stream row through the same /ingest path.
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
ddl_payload_file="$(mktemp)"
export CDC_DLQ_PATH="${dlq_path}"
pg_container=""

cleanup() {
  if [[ -n "${pid:-}" ]] && kill -0 "${pid}" 2>/dev/null; then
    kill "${pid}" 2>/dev/null || true
    wait "${pid}" 2>/dev/null || true
  fi
  if [[ -n "${pg_container}" ]]; then
    docker rm -f "${pg_container}" >/dev/null 2>&1 || true
  fi
  rm -f "${dlq_path}" "${ddl_payload_file}"
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
    if entry["sink"] == "nats":
        assert entry["target"] == "tenant.orders", entry
    if entry["sink"] == "pubsub":
        assert entry["target"] == "orders", entry
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


if command -v docker >/dev/null 2>&1; then
  pg_container="ai-blaise-cdc-ddl-${RANDOM}-$$"
  docker run \
    --name "${pg_container}" \
    -e POSTGRES_HOST_AUTH_METHOD=trust \
    -d postgres:17-bookworm >/dev/null

  for _ in $(seq 1 90); do
    if docker logs "${pg_container}" 2>&1 | grep -q "PostgreSQL init process complete" && \
       docker exec "${pg_container}" psql -U postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
      break
    fi
    sleep 1
  done
  docker exec -i "${pg_container}" psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
CREATE SCHEMA cdc;
CREATE TABLE cdc.ddl_events(
  id bigserial PRIMARY KEY,
  tenant_id text NOT NULL DEFAULT 'schema-capture',
  command_tag text NOT NULL,
  object_schema text NOT NULL,
  object_identity text NOT NULL,
  ddl text NOT NULL,
  occurred_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE OR REPLACE FUNCTION cdc.capture_ddl_event()
RETURNS event_trigger
LANGUAGE plpgsql
AS $$
DECLARE
  ddl_command record;
BEGIN
  FOR ddl_command IN SELECT * FROM pg_event_trigger_ddl_commands() LOOP
    IF ddl_command.schema_name = 'public' THEN
      INSERT INTO cdc.ddl_events(
        tenant_id, command_tag, object_schema, object_identity, ddl
      )
      VALUES (
        'schema-capture',
        ddl_command.command_tag,
        ddl_command.schema_name,
        ddl_command.object_identity,
        current_query()
      );
    END IF;
  END LOOP;
END;
$$;
CREATE EVENT TRIGGER ai_blaise_capture_ddl
ON ddl_command_end
EXECUTE FUNCTION cdc.capture_ddl_event();
SQL
  docker exec "${pg_container}" psql -U postgres -v ON_ERROR_STOP=1 -c \
    'CREATE TABLE public.cdc_schema_smoke(id bigint PRIMARY KEY, tenant_id text NOT NULL);'
  docker exec "${pg_container}" psql -U postgres -Atqc "
    SELECT json_build_object(
      'change', json_build_array(json_build_object(
        'kind', 'insert',
        'schema', 'cdc',
        'table', 'ddl_events',
        'columnnames', json_build_array(
          'tenant_id', 'command_tag', 'object_schema', 'object_identity', 'ddl', 'occurred_at'
        ),
        'columnvalues', json_build_array(
          tenant_id,
          command_tag,
          object_schema,
          object_identity,
          ddl,
          to_char(occurred_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
        )
      ))
    )::text
    FROM cdc.ddl_events
    WHERE object_identity LIKE '%cdc_schema_smoke%'
      AND command_tag = 'CREATE TABLE'
    ORDER BY id DESC
    LIMIT 1;
  " >"${ddl_payload_file}"
  grep -Fq 'cdc_schema_smoke' "${ddl_payload_file}"
  python3 - "${addr}" "${ddl_payload_file}" <<'PY'
import http.client
import json
import pathlib
import sys

addr, ddl_payload_path = sys.argv[1:3]
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

ddl_payload = pathlib.Path(ddl_payload_path).read_text().strip()
frame = {
    "start_lsn": "16/B374D900",
    "end_lsn": "16/B374DA00",
    "payload": ddl_payload,
}
report = request("POST", "/ingest", body=json.dumps(frame))
assert report["ddl_events_total"] == 1, report
ddl = report["ddl_events"][0]
assert ddl["ddl_stream_table"] == "cdc.ddl_events", ddl
assert ddl["command_tag"] == "CREATE TABLE", ddl
assert ddl["object_schema"] == "public", ddl
assert "cdc_schema_smoke" in ddl["object_identity"], ddl
assert "CREATE TABLE public.cdc_schema_smoke" in ddl["ddl"], ddl
assert report["events"][0]["ddl_event"]["object_identity"] == ddl["object_identity"], report
state = request("GET", "/state")
assert state["delivered_events"] == 2, state
print("OK cdc-sidecar live Postgres DDL capture parsed through /ingest")
PY
else
  if [[ "${REQUIRE_DOCKER:-0}" == "1" ]]; then
    echo "docker is required for CDC DDL capture smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping CDC DDL capture smoke"
fi

# Also run the deterministic stdout TSV path so the smoke covers both paths.
tsv=$(cargo run -q -p "${bin}" -- run-live-canonical)
echo "${tsv}" | head -5
echo "${tsv}" | grep -q "nats	tenant.orders" || { echo "missing nats row" >&2; exit 1; }
echo "${tsv}" | grep -q "pubsub	orders" || { echo "missing pubsub row" >&2; exit 1; }
echo "${tsv}" | grep -q "kafka	cdc.orders" || { echo "missing kafka row" >&2; exit 1; }
echo "${tsv}" | grep -q "kinesis	cdc-orders" || { echo "missing kinesis row" >&2; exit 1; }
echo "${tsv}" | grep -q "http2	https://h2.example.com/cdc/orders" || { echo "missing http2 row" >&2; exit 1; }
echo "${tsv}" | grep -q "email" || { echo "expected anonymized=email column" >&2; exit 1; }

echo "OK cdc-sidecar smoke complete"
