#!/usr/bin/env bash
# FEATURE: O15
#
# Production-style runtime smoke for the per-sidecar structured-log contract.
# It builds the real companion and sidecar/shared binaries, emits canonical
# sidecar JSON log records, applies the companion-generated typed SQL views to a
# real PostgreSQL 17 container, ingests every sidecar record as jsonb, and then
# queries every typed view.

set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "${ROOT}"

POSTGRES_IMAGE=${POSTGRES_IMAGE:-postgres:17}
POSTGRES_PASSWORD=${POSTGRES_PASSWORD:-ai-blaise-structured-log-smoke}
TARGET_DIR=${CARGO_TARGET_DIR:-target}

fail() {
  echo "structured-log-ingestion-smoke: $*" >&2
  exit 1
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    fail "$1 is required"
  fi
}

require_command cargo
require_command docker
require_command psql
require_command python3

if ! docker info >/dev/null 2>&1; then
  fail "docker daemon is required"
fi

tmpdir=$(mktemp -d)
container="ai-blaise-structured-log-smoke-$RANDOM-$$"
cleanup() {
  docker rm -f "${container}" >/dev/null 2>&1 || true
  rm -rf "${tmpdir}"
}
trap cleanup EXIT

free_port() {
  python3 - <<'PY'
import socket

sock = socket.socket()
sock.bind(("127.0.0.1", 0))
print(sock.getsockname()[1])
sock.close()
PY
}

port=$(free_port)

cargo build -q \
  -p ai_blaise_citus_companion --bin companion_contracts \
  -p ai_blaise_citus_sidecar_shared --bin ai_blaise_citus_sidecar_shared

companion_contracts="${TARGET_DIR}/debug/companion_contracts"
sidecar_shared="${TARGET_DIR}/debug/ai_blaise_citus_sidecar_shared"

[[ -x "${companion_contracts}" ]] || fail "missing built binary: ${companion_contracts}"
[[ -x "${sidecar_shared}" ]] || fail "missing built binary: ${sidecar_shared}"

"${companion_contracts}" run-log-view-sql-canonical >"${tmpdir}/views.sql"
grep -Fq 'CREATE OR REPLACE VIEW "companion"."sidecar_vectorizer_log"' "${tmpdir}/views.sql" || \
  fail "generated SQL did not include vectorizer typed view"
grep -Fq 'FROM companion.sidecar_log_raw' "${tmpdir}/views.sql" || \
  fail "generated SQL did not target companion.sidecar_log_raw"

"${sidecar_shared}" log-schema-records-canonical >"${tmpdir}/records.tsv"
python3 - "${tmpdir}/records.tsv" "${tmpdir}/records.jsonl" "${tmpdir}/expected-sidecars.txt" <<'PY'
import json
import sys
from pathlib import Path

records_tsv = Path(sys.argv[1])
records_jsonl = Path(sys.argv[2])
expected_sidecars = Path(sys.argv[3])

lines = [line for line in records_tsv.read_text().splitlines() if line.strip()]
if not lines or lines[0] != "sidecar\tvalidated\tjson":
    raise SystemExit(f"unexpected records header: {lines[:1]!r}")

sidecars = []
with records_jsonl.open("w") as out:
    for line in lines[1:]:
        sidecar, validated, payload = line.split("\t", 2)
        if validated != "true":
            raise SystemExit(f"record for {sidecar} was not validated: {validated!r}")
        decoded = (
            payload.replace("\\n", "\n")
            .replace("\\t", "\t")
            .replace("\\\\", "\\")
        )
        record = json.loads(decoded)
        if record.get("sidecar") != sidecar:
            raise SystemExit(f"sidecar mismatch for {sidecar}: {record!r}")
        if not record.get("traceparent") or not isinstance(record.get("fields"), dict):
            raise SystemExit(f"record missing traceparent/fields for {sidecar}: {record!r}")
        sidecars.append(sidecar)
        out.write(json.dumps(record, sort_keys=True, separators=(",", ":")))
        out.write("\n")

if len(sidecars) != 17:
    raise SystemExit(f"expected 17 sidecar records, got {len(sidecars)}")
expected_sidecars.write_text("\n".join(sidecars) + "\n")
PY

docker run -d \
  --name "${container}" \
  -e POSTGRES_PASSWORD="${POSTGRES_PASSWORD}" \
  -e POSTGRES_DB=observability \
  -p "127.0.0.1:${port}:5432" \
  "${POSTGRES_IMAGE}" >/dev/null

export PGPASSWORD="${POSTGRES_PASSWORD}"
psql_cmd=(psql -v ON_ERROR_STOP=1 -h 127.0.0.1 -p "${port}" -U postgres -d observability)

for _ in $(seq 1 60); do
  if "${psql_cmd[@]}" -Atqc "SELECT 1" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
"${psql_cmd[@]}" -Atqc "SELECT 1" >/dev/null 2>&1 || fail "PostgreSQL did not become ready"

"${psql_cmd[@]}" >/dev/null <<'SQL'
CREATE SCHEMA companion;
CREATE TABLE companion.sidecar_log_raw(
  line jsonb NOT NULL,
  captured_at timestamptz NOT NULL DEFAULT now()
);
SQL

"${psql_cmd[@]}" -f "${tmpdir}/views.sql" >/dev/null
"${psql_cmd[@]}" -c "\\copy companion.sidecar_log_raw(line) FROM '${tmpdir}/records.jsonl' WITH (FORMAT text)" >/dev/null

raw_count=$("${psql_cmd[@]}" -Atqc "SELECT count(*) FROM companion.sidecar_log_raw")
[[ "${raw_count}" == "17" ]] || fail "expected 17 ingested raw records, got ${raw_count}"

view_count=$("${psql_cmd[@]}" -Atqc "SELECT count(*) FROM information_schema.views WHERE table_schema = 'companion' AND table_name LIKE 'sidecar_%_log'")
[[ "${view_count}" == "17" ]] || fail "expected 17 typed views, got ${view_count}"

python3 - "${tmpdir}/expected-sidecars.txt" "${tmpdir}/verify.sql" <<'PY'
import sys
from pathlib import Path

sidecars = [line.strip() for line in Path(sys.argv[1]).read_text().splitlines() if line.strip()]
parts = []
for sidecar in sidecars:
    view = f'companion."sidecar_{sidecar}_log"'
    parts.append(
        "SELECT "
        f"'{sidecar}' AS sidecar, "
        "count(*)::int AS rows, "
        "bool_and(traceparent = '00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01') AS trace_ok, "
        "bool_and(tenant_id = 'tenant-a') AS tenant_ok, "
        f"bool_and(request_id = 'req-{sidecar}') AS request_ok "
        f"FROM {view}"
    )

Path(sys.argv[2]).write_text(
    "WITH view_counts AS (\n"
    + "\nUNION ALL\n".join(parts)
    + "\n)\n"
    + "SELECT count(*) FROM view_counts WHERE rows = 1 AND trace_ok AND tenant_ok AND request_ok;\n"
)
PY

typed_view_rows=$("${psql_cmd[@]}" -Atqf "${tmpdir}/verify.sql")
[[ "${typed_view_rows}" == "17" ]] || fail "expected all 17 typed views to project one valid row, got ${typed_view_rows}"

vectorizer_types=$("${psql_cmd[@]}" -Atqc "SELECT concat_ws(chr(44), pg_typeof(\"timestamp\")::text, pg_typeof(tokens)::text, pg_typeof(cost_usd)::text) FROM companion.\"sidecar_vectorizer_log\"")
[[ "${vectorizer_types}" == "timestamp with time zone,bigint,double precision" ]] || \
  fail "unexpected vectorizer typed-view column types: ${vectorizer_types}"

printf "structured_log_ingestion_smoke\tpostgres_image=%s\trecords=%s\ttyped_views=%s\tverified_views=%s\tvectorizer_types=%s\n" "${POSTGRES_IMAGE}" "${raw_count}" "${view_count}" "${typed_view_rows}" "${vectorizer_types}"
