#!/usr/bin/env bash
# FEATURE: C10 M2 T5
# Runtime boundary smoke for schema-job manifest/controller contracts and
# txn-status HTTP stage/finalize/ack behavior.
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

tmpdir="$(mktemp -d /tmp/schema-txn-runtime.XXXXXX)"
schema_pid=""
txn_pid=""
cleanup() {
  for pid in ${schema_pid:-} ${txn_pid:-}; do
    if [[ -n "${pid}" ]] && kill -0 "${pid}" 2>/dev/null; then
      kill "${pid}" 2>/dev/null || true
      wait "${pid}" 2>/dev/null || true
    fi
  done
  rm -rf "${tmpdir}"
}
trap cleanup EXIT

alloc_port() {
  python3 - <<'PYPORT'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PYPORT
}

wait_port() {
  local port="$1"
  python3 - "$port" <<'PYWAIT'
import socket, sys, time
port = int(sys.argv[1])
for _ in range(100):
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=0.1):
            sys.exit(0)
    except OSError:
        time.sleep(0.05)
print(f"port {port} did not open", file=sys.stderr)
sys.exit(1)
PYWAIT
}

echo "==> schema-txn-runtime-smoke: cargo build"
cargo build -q -p ai_blaise_citus_sidecar_schema_job -p ai_blaise_citus_sidecar_txn_status
target_dir="${CARGO_TARGET_DIR:-target}"
schema_job_bin="${target_dir}/debug/ai_blaise_citus_sidecar_schema_job"
txn_status_bin="${target_dir}/debug/ai_blaise_citus_sidecar_txn_status"

echo "==> schema-txn-runtime-smoke: schema-job canonical runner"
schema_canonical=$(cargo run -q -p ai_blaise_citus_sidecar_schema_job -- run-canonical)
echo "${schema_canonical}"
if [[ "${schema_canonical}" != *$'apply_delete_only'* ]]; then
  echo "schema canonical runner did not emit apply_delete_only" >&2
  exit 1
fi

echo "==> schema-txn-runtime-smoke: schema-job controller canonical runner"
controller_canonical=$(cargo run -q -p ai_blaise_citus_sidecar_schema_job -- run-controller-canonical)
echo "${controller_canonical}"
for token in $'advance' $'wait' $'rollback' $'true'; do
  if [[ "${controller_canonical}" != *"${token}"* ]]; then
    echo "controller canonical runner missed ${token}" >&2
    exit 1
  fi
done

valid_manifest="${tmpdir}/valid-schema-job.json"
invalid_cutover_manifest="${tmpdir}/invalid-cutover.json"
unsafe_manifest="${tmpdir}/unsafe-sql.json"
malformed_manifest="${tmpdir}/malformed.json"

python3 - "$valid_manifest" "$invalid_cutover_manifest" "$unsafe_manifest" "$malformed_manifest" <<'PYMANIFEST'
import json, pathlib, sys
valid, invalid, unsafe, malformed = map(pathlib.Path, sys.argv[1:])
base = {
    "job": {
        "name": "users-display-name",
        "table": "public.users",
        "state": "backfill",
        "lease_seconds": 30,
        "operations": [
            {"kind": "add_column", "column": "display_name", "sql_type": "text"},
            {"kind": "backfill", "statement": "UPDATE public.users SET display_name = name WHERE display_name IS NULL"},
        ],
    },
    "worker_id": "schema-worker-a",
    "lease": {"holder": "schema-worker-a", "epoch": 1, "expires_at": "2026-05-19T12:00:00Z"},
    "backfill": {"batch_size": 1000, "max_parallel_shards": 4, "throttle_ms": 50},
    "safety": {
        "max_replication_lag_bytes": 16777216,
        "max_lock_ms": 500,
        "allow_blocking_cutover": False,
        "require_data_invariants": True,
        "data_invariants_verified": True,
    },
    "shadow": {
        "source_table": "public.users",
        "shadow_table": "public._users_new",
        "changelog_table": "public._users_changelog",
        "cutover_lock_timeout_ms": 500,
    },
}
valid.write_text(json.dumps(base), encoding="utf-8")
bad = json.loads(json.dumps(base))
bad["job"]["state"] = "public"
bad["safety"]["data_invariants_verified"] = False
invalid.write_text(json.dumps(bad), encoding="utf-8")
unsafe_doc = json.loads(json.dumps(base))
unsafe_doc["job"]["operations"][0]["sql_type"] = "text; drop table public.users"
unsafe.write_text(json.dumps(unsafe_doc), encoding="utf-8")
malformed.write_text("{", encoding="utf-8")
PYMANIFEST

valid_out=$(cargo run -q -p ai_blaise_citus_sidecar_schema_job -- validate-manifest "${valid_manifest}")
echo "${valid_out}"
if [[ "${valid_out}" != *$'run_backfill'* ]]; then
  echo "schema manifest validator did not emit run_backfill" >&2
  exit 1
fi
if cargo run -q -p ai_blaise_citus_sidecar_schema_job -- validate-manifest "${invalid_cutover_manifest}" >"${tmpdir}/invalid.out" 2>"${tmpdir}/invalid.err"; then
  echo "schema manifest accepted unverified public cutover" >&2
  exit 1
fi
grep -Fq "data invariants are not verified" "${tmpdir}/invalid.err"
if cargo run -q -p ai_blaise_citus_sidecar_schema_job -- validate-manifest "${unsafe_manifest}" >"${tmpdir}/unsafe.out" 2>"${tmpdir}/unsafe.err"; then
  echo "schema manifest accepted unsafe SQL fragment" >&2
  exit 1
fi
grep -Fq "outside the schema-job apply boundary" "${tmpdir}/unsafe.err"
if cargo run -q -p ai_blaise_citus_sidecar_schema_job -- validate-manifest "${malformed_manifest}" >"${tmpdir}/malformed.out" 2>"${tmpdir}/malformed.err"; then
  echo "schema manifest accepted malformed JSON" >&2
  exit 1
fi
grep -Fq "manifest JSON invalid" "${tmpdir}/malformed.err"

echo "==> schema-txn-runtime-smoke: schema-job probe server"
schema_port="$(alloc_port)"
AI_BLAISE_LISTEN_ADDR="127.0.0.1:${schema_port}" "${schema_job_bin}" serve >"${tmpdir}/schema.log" 2>&1 &
schema_pid=$!
wait_port "${schema_port}"
python3 - "${schema_port}" <<'PYSCHEMAHTTP'
import http.client, socket, sys
port = int(sys.argv[1])
for path in ["/healthz", "/readyz", "/metrics"]:
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=2)
    conn.request("GET", path)
    resp = conn.getresponse()
    body = resp.read().decode()
    assert resp.status == 200, (path, resp.status, body)
    conn.close()
with socket.create_connection(("127.0.0.1", port), timeout=2) as sock:
    sock.sendall(b"not-http\r\n\r\n")
    data = sock.recv(4096).decode("utf-8", "replace")
assert "400 Bad Request" in data, data
PYSCHEMAHTTP

echo "==> schema-txn-runtime-smoke: txn-status canonical runners"
txn_runtime=$(cargo run -q -p ai_blaise_citus_sidecar_txn_status -- run-runtime-canonical)
echo "${txn_runtime}"
if [[ "${txn_runtime}" != *$'commit'* || "${txn_runtime}" != *$'committed'* ]]; then
  echo "txn runtime canonical runner did not commit" >&2
  exit 1
fi
microbench=$(cargo run -q -p ai_blaise_citus_sidecar_txn_status -- run-parallel-commit-microbench 5)
echo "${microbench}"
if [[ "${microbench}" != *$'5	10	2'* ]]; then
  echo "txn microbench did not prove expected 5-shard step count" >&2
  exit 1
fi

echo "==> schema-txn-runtime-smoke: txn-status HTTP runtime"
txn_port="$(alloc_port)"
AI_BLAISE_LISTEN_ADDR="127.0.0.1:${txn_port}" "${txn_status_bin}" serve >"${tmpdir}/txn.log" 2>&1 &
txn_pid=$!
wait_port "${txn_port}"
python3 - "${txn_port}" <<'PYTXNHTTP'
import http.client, json, sys
port = int(sys.argv[1])

def call(method, path, body=None):
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=3)
    payload = None if body is None else json.dumps(body)
    headers = {"content-type": "application/json"} if payload is not None else {}
    conn.request(method, path, payload, headers)
    resp = conn.getresponse()
    raw = resp.read().decode()
    conn.close()
    try:
        parsed = json.loads(raw)
    except json.JSONDecodeError:
        parsed = raw
    return resp.status, parsed

status, body = call("GET", "/healthz")
assert status == 200, (status, body)
staging = {
    "txn_id": "txn-live-1",
    "coordinator": "worker-a",
    "staging_physical_ms": 1700000000,
    "intents": [{"shard_id": 10, "key_range": "[a,m)", "required_acks": 2, "replica_acks": 0}],
}
status, body = call("POST", "/txn/staging", staging)
assert status == 201 and body["status"] == "staging", (status, body)
status, body = call("POST", "/txn/staging", staging)
assert status == 409 and "already staged" in body["error"], (status, body)
finalize = {"txn_id": "txn-live-1", "observed_physical_ms": 1700000010}
status, body = call("POST", "/txn/finalize", finalize)
assert status == 200 and body["decision"] == "wait_for_replication_evidence", (status, body)
status, body = call("POST", "/txn/ack", {"txn_id": "txn-live-1", "shard_id": 10, "replica_acks": 2})
assert status == 200 and body["intents"][0]["replica_acks"] == 2, (status, body)
status, body = call("POST", "/txn/finalize", finalize)
assert status == 200 and body["decision"] == "commit" and body["status"] == "committed", (status, body)
status, body = call("GET", "/txn/txn-live-1/status")
assert status == 200 and body["status"] == "committed", (status, body)
status, body = call("POST", "/txn/staging", {**staging, "unexpected": True})
assert status == 400 and "unknown field" in body["error"], (status, body)
conn = http.client.HTTPConnection("127.0.0.1", port, timeout=3)
conn.request("POST", "/txn/staging", "{", {"content-type": "application/json"})
resp = conn.getresponse()
raw = resp.read().decode()
conn.close()
assert resp.status == 400 and "invalid staging JSON" in raw, (resp.status, raw)
status, body = call("GET", "/txn/staging")
assert status == 405, (status, body)
PYTXNHTTP

echo "schema-txn-runtime-smoke passed"
