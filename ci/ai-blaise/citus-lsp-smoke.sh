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

echo "citus-lsp file-backed smoke passed"
