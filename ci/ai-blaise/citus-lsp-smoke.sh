#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

tmpdir="$(mktemp -d)"
trap 'rm -rf "${tmpdir}"' EXIT

metadata="${tmpdir}/metadata.tsv"
sql="${tmpdir}/migration.sql"
bridged_sql="${tmpdir}/bridged.sql"
out="${tmpdir}/diagnostics.tsv"
bridged_out="${tmpdir}/bridged-diagnostics.tsv"
bad_metadata="${tmpdir}/bad-metadata.tsv"
bad_err="${tmpdir}/bad-metadata.err"
missing_err="${tmpdir}/missing.err"

require_output() {
  local pattern="$1"
  local file="$2"
  if ! grep -q "${pattern}" "${file}"; then
    echo "expected citus-lsp output pattern not found: ${pattern}" >&2
    cat "${file}" >&2
    exit 1
  fi
}

cat > "${metadata}" <<'EOF_METADATA'
distributed_table	public.orders	tenant_id	tenant	tenant_id
distributed_table	public.line_items	tenant_id	tenant	tenant_id
distributed_table	public.events	device_id	device	-
hypertable	public.events	created_at	public.events
search_index	orders_search	public.orders	-
tenant	tenant-a	tenant_a
EOF_METADATA

cat > "${sql}" <<'EOF_SQL'
CREATE TABLE tenant_a.invoices (
  tenant_id uuid NOT NULL,
  invoice_id uuid NOT NULL,
  total_cents bigint NOT NULL
);

CREATE TABLE public.shipments (
  tenant_id uuid NOT NULL,
  shipment_id uuid NOT NULL
);
SELECT create_distributed_table('public.shipments', 'tenant_id');

SELECT *
FROM public.orders
JOIN public.events ON orders.device_id = events.device_id;

ALTER TABLE public.orders DROP COLUMN tenant_id;

SELECT create_hypertable('public.events', 'created_at');

SELECT *
FROM public.orders
WHERE status = 'open';

CREATE INDEX orders_search ON public.orders USING bm25 (status);
EOF_SQL

cargo run -q -p ai_blaise_citus_lsp -- analyze --metadata "${metadata}" --sql "${sql}" > "${out}"

require_output $'^uri\tcode\tseverity\tmessage\tquick_fix$' "${out}"
require_output $'\tmissing_distribution_column\twarning\t' "${out}"
require_output $'\tnon_colocated_join\twarning\t' "${out}"
require_output $'\tdistribution_column_alter\terror\t' "${out}"
require_output $'\thypertable_invariant\twarning\t' "${out}"
require_output $'\tmissing_tenant_filter\twarning\t' "${out}"
require_output $'\tmissing_search_analyzer\twarning\t' "${out}"
require_output 'add_distribution_column table=tenant_a.invoices column=tenant_id' "${out}"
require_output 'align_colocation left_table=public.orders right_table=public.events distribution_column=tenant_id' "${out}"
require_output 'add_tenant_filter table=public.orders tenant_column=tenant_id' "${out}"
require_output 'use_distributed_hypertable_bridge table=public.events time_column=created_at' "${out}"
require_output 'set_search_analyzer index_name=orders_search analyzer=english' "${out}"

diagnostic_count="$(tail -n +2 "${out}" | wc -l | tr -d ' ')"
if [[ "${diagnostic_count}" != "7" ]]; then
  echo "expected seven LSP diagnostics from real SQL file, got ${diagnostic_count}" >&2
  cat "${out}" >&2
  exit 1
fi

cat > "${bridged_sql}" <<'EOF_SQL'
SELECT apply_distribute_hypertable('public.events', 'device_id', 'created_at', '1 day');
SELECT create_hypertable('public.events', 'created_at');
EOF_SQL

cargo run -q -p ai_blaise_citus_lsp -- analyze --metadata "${metadata}" --sql "${bridged_sql}" > "${bridged_out}"
if grep -q 'hypertable_invariant' "${bridged_out}"; then
  echo "distributed hypertable bridge should suppress the hypertable invariant diagnostic" >&2
  cat "${bridged_out}" >&2
  exit 1
fi

printf 'unknown\tshape\n' > "${bad_metadata}"
if cargo run -q -p ai_blaise_citus_lsp -- analyze --metadata "${bad_metadata}" --sql "${sql}" > /dev/null 2> "${bad_err}"; then
  echo "bad metadata unexpectedly succeeded" >&2
  exit 1
fi
grep -q 'metadata line 1 is invalid' "${bad_err}"

if cargo run -q -p ai_blaise_citus_lsp -- analyze --sql "${sql}" > /dev/null 2> "${missing_err}"; then
  echo "missing metadata unexpectedly succeeded" >&2
  exit 1
fi
grep -q 'analyze requires --metadata <path>' "${missing_err}"


METADATA_PATH="${metadata}" SQL_PATH="${sql}" python3 <<'PY'
import json
import os
import subprocess
import sys

metadata = os.environ["METADATA_PATH"]
sql_text = open(os.environ["SQL_PATH"], encoding="utf-8").read()
uri = "file:///workspace/migration.sql"

proc = subprocess.Popen(
    ["cargo", "run", "-q", "-p", "ai_blaise_citus_lsp", "--", "serve-stdio", "--metadata", metadata],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
)


def send(payload):
    raw = payload if isinstance(payload, bytes) else json.dumps(payload, separators=(",", ":")).encode()
    assert proc.stdin is not None
    proc.stdin.write(b"Content-Length: " + str(len(raw)).encode() + b"\r\n\r\n" + raw)
    proc.stdin.flush()


def recv():
    assert proc.stdout is not None
    headers = {}
    while True:
        line = proc.stdout.readline()
        if not line:
            stderr = proc.stderr.read().decode(errors="replace") if proc.stderr else ""
            raise AssertionError(f"citus-lsp serve-stdio closed stdout early: {stderr}")
        if line in (b"\r\n", b"\n"):
            break
        name, value = line.decode().split(":", 1)
        headers[name.lower()] = value.strip()
    length = int(headers["content-length"])
    return json.loads(proc.stdout.read(length))

try:
    send(b'{"jsonrpc":"2.0","id":900,"method":')
    malformed = recv()
    assert malformed["error"]["code"] == -32700

    send({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"capabilities": {}}})
    initialize = recv()
    assert initialize["result"]["serverInfo"]["name"] == "ai-blaise-citus-lsp"
    assert initialize["result"]["capabilities"]["diagnosticProvider"]["workspaceDiagnostics"] is False

    send({"jsonrpc": "2.0", "method": "initialized", "params": {}})
    send(
        {
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "sql",
                    "version": 1,
                    "text": sql_text,
                }
            },
        }
    )
    published = recv()
    assert published["method"] == "textDocument/publishDiagnostics"
    diagnostics = published["params"]["diagnostics"]
    codes = {diagnostic["code"] for diagnostic in diagnostics}
    assert len(diagnostics) == 7, diagnostics
    for expected in {
        "missing_distribution_column",
        "non_colocated_join",
        "distribution_column_alter",
        "hypertable_invariant",
        "missing_tenant_filter",
        "missing_search_analyzer",
    }:
        assert expected in codes, codes
    assert any(
        diagnostic.get("data", {}).get("quickFix", "").startswith("add_distribution_column")
        for diagnostic in diagnostics
    )

    send({"jsonrpc": "2.0", "id": 2, "method": "textDocument/diagnostic", "params": {"textDocument": {"uri": uri}}})
    pulled = recv()
    assert pulled["result"]["kind"] == "full"
    assert len(pulled["result"]["items"]) == 7

    send({"jsonrpc": "2.0", "id": 3, "method": "workspace/symbol", "params": {}})
    unknown = recv()
    assert unknown["error"]["code"] == -32601

    send({"jsonrpc": "2.0", "id": 4, "method": "textDocument/diagnostic", "params": {"textDocument": {"uri": "file:///missing.sql"}}})
    missing = recv()
    assert missing["error"]["code"] == -32602
    assert "document is not open" in missing["error"]["message"]

    send({"jsonrpc": "2.0", "id": 5, "method": "shutdown", "params": None})
    shutdown = recv()
    assert shutdown["result"] is None
    send({"jsonrpc": "2.0", "method": "exit"})
finally:
    if proc.stdin is not None:
        proc.stdin.close()
    try:
        proc.wait(timeout=20)
    except subprocess.TimeoutExpired:
        proc.terminate()
        proc.wait(timeout=20)

if proc.returncode != 0:
    stderr = proc.stderr.read().decode(errors="replace") if proc.stderr else ""
    print(stderr, file=sys.stderr)
    raise SystemExit(proc.returncode)

print("citus-lsp LSP stdio smoke passed")
PY

echo "citus-lsp file-backed smoke passed"
