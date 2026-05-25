#!/usr/bin/env bash
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"

if ! command -v docker >/dev/null 2>&1; then
  echo "Docker is required for schema-drift-live-smoke" >&2
  exit 1
fi

if ! docker info >/dev/null 2>&1; then
  echo "Docker daemon is required for schema-drift-live-smoke" >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required for schema-drift-live-smoke" >&2
  exit 1
fi

postgres_image="${POSTGRES_IMAGE:-postgres:17-bookworm}"
suffix="$(date +%s)-$$"
container="ai-blaise-schema-drift-${suffix}"

cleanup() {
  docker rm -f "${container}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run -d \
  --name "${container}" \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=postgres \
  "${postgres_image}" >/dev/null

for _ in $(seq 1 90); do
  if docker exec "${container}" pg_isready -U postgres -d postgres >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

if ! docker exec "${container}" pg_isready -U postgres -d postgres >/dev/null 2>&1; then
  echo "Postgres did not become ready in ${container}" >&2
  docker logs "${container}" >&2 || true
  exit 1
fi

psql_live() {
  docker exec -i -e PGPASSWORD=postgres "${container}" \
    psql -v ON_ERROR_STOP=1 -U postgres -d postgres "$@"
}

schema_report="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-schema-drift-canonical)"
expected_header=$'feature_id\texpected_columns\tstatements\tdrift_kinds\tinformation_schema_queries\ttemporary_tables'
expected_row=$'M4\t4\t3\tmissing_column,type_mismatch,nullability_mismatch,unexpected_column\t1\t1'

if ! grep -Fqx "${expected_header}" <<<"${schema_report}"; then
  echo "schema drift report header mismatch" >&2
  printf '%s\n' "${schema_report}" >&2
  exit 1
fi

if ! grep -Fqx "${expected_row}" <<<"${schema_report}"; then
  echo "schema drift report row mismatch" >&2
  printf '%s\n' "${schema_report}" >&2
  exit 1
fi

schema_sql="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-schema-drift-sql-canonical)"
for phrase in \
  "CREATE TEMP TABLE ai_blaise_expected_schema_columns" \
  "information_schema.columns" \
  "'missing_column' AS drift_kind" \
  "'type_mismatch' AS drift_kind" \
  "'nullability_mismatch' AS drift_kind" \
  "'unexpected_column' AS drift_kind"; do
  if ! grep -Fq "${phrase}" <<<"${schema_sql}"; then
    echo "schema drift SQL missing phrase: ${phrase}" >&2
    printf '%s\n' "${schema_sql}" >&2
    exit 1
  fi
done

psql_live <<'SQL'
CREATE TABLE public.accounts(
  id integer PRIMARY KEY,
  tenant_id bigint NOT NULL,
  email text,
  obsolete text
);
SQL

drift_output="$(printf '%s\n' "${schema_sql}" | psql_live -qAt -F $'\t')"

for expected in \
  $'public\taccounts\temail\ttext\ttext\tNO\tYES\tnullability_mismatch' \
  $'public\taccounts\tobsolete\t\ttext\t\tYES\tunexpected_column' \
  $'public\taccounts\ttenant_id\ttext\tbigint\tNO\tNO\ttype_mismatch' \
  $'public\taccounts\tupdated_at\ttimestamp with time zone\t\tNO\t\tmissing_column'; do
  if ! grep -Fqx "${expected}" <<<"${drift_output}"; then
    echo "schema drift output missing expected row: ${expected}" >&2
    printf '%s\n' "${drift_output}" >&2
    exit 1
  fi
done

drift_count="$(grep -c '^public' <<<"${drift_output}")"
if [[ "${drift_count}" != "4" ]]; then
  echo "schema drift output returned ${drift_count} drift rows, expected 4" >&2
  printf '%s\n' "${drift_output}" >&2
  exit 1
fi

psql_live <<'SQL'
ALTER TABLE public.accounts ALTER COLUMN tenant_id TYPE text USING tenant_id::text;
ALTER TABLE public.accounts ALTER COLUMN email SET NOT NULL;
ALTER TABLE public.accounts ADD COLUMN updated_at timestamp with time zone NOT NULL DEFAULT now();
ALTER TABLE public.accounts DROP COLUMN obsolete;
SQL

clean_output="$(printf '%s\n' "${schema_sql}" | psql_live -qAt -F $'\t')"
if [[ -n "${clean_output}" ]]; then
  echo "schema drift clean pass returned unexpected rows" >&2
  printf '%s\n' "${clean_output}" >&2
  exit 1
fi

echo $'schema_drift_live_smoke\tmissing_column=true\ttype_mismatch=true\tnullability_mismatch=true\tunexpected_column=true\tclean_schema_zero_drift=true'
