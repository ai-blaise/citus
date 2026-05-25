#!/usr/bin/env bash
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"

if ! command -v docker >/dev/null 2>&1; then
  echo "Docker is required for fdw-credential-rotation-live-smoke" >&2
  exit 1
fi

if ! docker info >/dev/null 2>&1; then
  echo "Docker daemon is required for fdw-credential-rotation-live-smoke" >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required for fdw-credential-rotation-live-smoke" >&2
  exit 1
fi

postgres_image="${POSTGRES_IMAGE:-postgres:17-bookworm}"
suffix="$(date +%s)-$$"
network="ai-blaise-fdw-rotation-${suffix}"
remote_container="ai-blaise-fdw-remote-${suffix}"
local_container="ai-blaise-fdw-local-${suffix}"
old_password="old_fdw_password_${suffix//[^0-9A-Za-z]/_}"
new_password="new_fdw_password_${suffix//[^0-9A-Za-z]/_}"

cleanup() {
  docker rm -f "${remote_container}" "${local_container}" >/dev/null 2>&1 || true
  docker network rm "${network}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

wait_for_pg() {
  local container="$1"
  local attempt
  for attempt in $(seq 1 90); do
    if docker exec "${container}" pg_isready -U postgres -d postgres >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  echo "Postgres did not become ready in ${container}" >&2
  docker logs "${container}" >&2 || true
  exit 1
}

psql_remote() {
  docker exec -i -e PGPASSWORD=postgres "${remote_container}" \
    psql -v ON_ERROR_STOP=1 -U postgres -d postgres "$@"
}

psql_local() {
  docker exec -i -e PGPASSWORD=postgres "${local_container}" \
    psql -v ON_ERROR_STOP=1 -U postgres -d postgres "$@"
}

docker network create "${network}" >/dev/null
docker run -d \
  --name "${remote_container}" \
  --network "${network}" \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=postgres \
  "${postgres_image}" >/dev/null
docker run -d \
  --name "${local_container}" \
  --network "${network}" \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=postgres \
  "${postgres_image}" >/dev/null

wait_for_pg "${remote_container}"
wait_for_pg "${local_container}"

psql_remote -v old_password="${old_password}" <<'SQL'
CREATE ROLE fdw_user LOGIN PASSWORD :'old_password';
CREATE TABLE public.fdw_items(id integer PRIMARY KEY, label text NOT NULL);
INSERT INTO public.fdw_items(id, label)
VALUES (1, 'before-rotation'), (2, 'after-rotation');
GRANT USAGE ON SCHEMA public TO fdw_user;
GRANT SELECT ON public.fdw_items TO fdw_user;
SQL

psql_local -v remote_host="${remote_container}" -v old_password="${old_password}" <<'SQL'
CREATE EXTENSION postgres_fdw;
CREATE SERVER ai_blaise_remote
  FOREIGN DATA WRAPPER postgres_fdw
  OPTIONS (host :'remote_host', dbname 'postgres', port '5432', keep_connections 'false');
CREATE USER MAPPING FOR CURRENT_USER
  SERVER ai_blaise_remote
  OPTIONS (user 'fdw_user', password :'old_password');
CREATE FOREIGN TABLE public.fdw_items_remote(id integer, label text)
  SERVER ai_blaise_remote
  OPTIONS (schema_name 'public', table_name 'fdw_items');
SQL

initial_count="$(psql_local -Atc "SELECT count(*) FROM public.fdw_items_remote")"
if [[ "${initial_count}" != "2" ]]; then
  echo "initial FDW query returned ${initial_count}, expected 2" >&2
  exit 1
fi

psql_remote -v new_password="${new_password}" <<'SQL'
ALTER ROLE fdw_user PASSWORD :'new_password';
SQL

psql_local -Atc "SELECT postgres_fdw_disconnect_all()" >/dev/null

set +e
old_query_output="$(psql_local -Atc "SELECT count(*) FROM public.fdw_items_remote" 2>&1)"
old_query_status=$?
set -e

if [[ "${old_query_status}" -eq 0 ]]; then
  echo "FDW query with stale user mapping password unexpectedly succeeded: ${old_query_output}" >&2
  exit 1
fi

if ! grep -Eiq 'password authentication failed|could not connect to server|connection failed' <<<"${old_query_output}"; then
  echo "FDW stale password failure had unexpected output: ${old_query_output}" >&2
  exit 1
fi

rotation_report="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-fdw-credential-rotation-canonical)"
expected_header=$'feature_id\tserver\tmapping_user\tvalidation_table\tstatements\tdisconnect_calls\tuses_secret_variable\tplan_secret_literals'
expected_row=$'F4\tai_blaise_remote\tCURRENT_USER\tpublic.fdw_items_remote\t6\t2\ttrue\tfalse'

if ! grep -Fqx "${expected_header}" <<<"${rotation_report}"; then
  echo "FDW rotation report header mismatch" >&2
  printf '%s\n' "${rotation_report}" >&2
  exit 1
fi

if ! grep -Fqx "${expected_row}" <<<"${rotation_report}"; then
  echo "FDW rotation report row mismatch" >&2
  printf '%s\n' "${rotation_report}" >&2
  exit 1
fi

rotation_sql="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-fdw-credential-rotation-sql-canonical)"
for literal in "${old_password}" "${new_password}" "k8s/fdw-remote/old-password" "k8s/fdw-remote/new-password"; do
  if grep -Fq "${literal}" <<<"${rotation_sql}"; then
    echo "FDW rotation SQL leaked a secret literal or secret reference: ${literal}" >&2
    printf '%s\n' "${rotation_sql}" >&2
    exit 1
  fi
done

if ! grep -Fq "ALTER USER MAPPING FOR CURRENT_USER SERVER \"ai_blaise_remote\" OPTIONS (SET password :'fdw_new_password')" <<<"${rotation_sql}"; then
  echo "FDW rotation SQL did not render the parameterized user mapping rotation" >&2
  printf '%s\n' "${rotation_sql}" >&2
  exit 1
fi

printf '%s\n' "${rotation_sql}" | psql_local -v fdw_new_password="${new_password}" >/dev/null

new_count="$(psql_local -Atc "SELECT count(*) FROM public.fdw_items_remote")"
if [[ "${new_count}" != "2" ]]; then
  echo "FDW query after credential rotation returned ${new_count}, expected 2" >&2
  exit 1
fi

echo $'fdw_credential_rotation_live_smoke\told_password_rejected=true\tnew_password_succeeded=true\tplan_secret_literals=false\tpostgres_fdw_disconnect_all=true'
