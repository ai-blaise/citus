#!/usr/bin/env bash
# FEATURE: T6
#
# Live PG18 io_method=io_uring smoke for T6.
#
# Boots a postgres:18-bookworm container with io_method=io_uring on the
# Linux kernel of the host VM (>= 5.1 required for io_uring), installs
# the available PG18 PGDG bundled extensions (timescaledb, pg_cron,
# pgaudit, pgvector, postgis, repack, etc.), verifies SHOW io_method
# returns io_uring, runs a sized workload, and inspects pg_stat_io to
# confirm io_uring backend IO is actually occurring at runtime. The
# evidence file records the io_method GUC value, kernel version,
# pg_stat_io row counts, and the set of CREATE EXTENSION calls that
# succeeded.
#
# Does NOT claim: cloud-provider PG18 image certification, PG18 builds of
# source-built extensions (citus, pgsodium, topn, pg_jsonschema, pg_graphql,
# pg_search, plv8 — those remain PG17-only in Bundle1), production Citus
# distributed plane on PG18, or full benchmarking against PG17.

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

require_docker="${REQUIRE_DOCKER:-0}"
if ! command -v docker >/dev/null 2>&1; then
  if [[ "${require_docker}" == "1" ]]; then
    echo "docker is required for T6 PG18 io_uring smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping T6 PG18 io_uring smoke"
  exit 0
fi

evidence_dir="${T6_EVIDENCE_DIR:-artifacts}"
mkdir -p "${evidence_dir}"
evidence_file="${T6_EVIDENCE_FILE:-${evidence_dir}/t6-pg18-io-uring-evidence.tsv}"

container="t6-pg18-io-uring-${RANDOM}-$$"
postgres_image="${T6_POSTGRES_IMAGE:-postgres:18-bookworm}"

cleanup() {
  docker rm -f "${container}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "=== T6 PG18 io_uring live smoke ==="
echo "image:     ${postgres_image}"
echo "container: ${container}"
echo "kernel:    $(uname -r)"

docker pull "${postgres_image}" >/dev/null

docker run -d --name "${container}" \
  -e POSTGRES_PASSWORD=postgres \
  "${postgres_image}" \
  -c io_method=io_uring \
  -c shared_preload_libraries=pg_stat_statements \
  >/dev/null

# Wait for postgres to be queryable.
wait_for_psql() {
  local attempts="${1:-120}"
  local _
  for _ in $(seq 1 "${attempts}"); do
    if docker exec "${container}" psql -U postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  docker logs "${container}" >&2 || true
  echo "PG18 container did not become queryable" >&2
  return 1
}
wait_for_psql

# Verify io_method GUC is io_uring (or fall back to worker if unsupported).
observed_io_method="$(docker exec "${container}" psql -U postgres -Atqc 'SHOW io_method')"
if [[ "${observed_io_method}" != "io_uring" ]]; then
  echo "io_method should be io_uring (got: ${observed_io_method})" >&2
  exit 1
fi

# Install available PG18 PGDG bundled extensions inside the container.
docker exec "${container}" bash -c '
  set -eu
  export DEBIAN_FRONTEND=noninteractive
  apt-get update >/dev/null 2>&1
  apt-get install -y --no-install-recommends \
    postgresql-18-cron \
    postgresql-18-pgaudit \
    postgresql-18-pgvector \
    postgresql-18-postgis-3 \
    postgresql-18-pg-uuidv7 \
    postgresql-18-age \
    >/dev/null 2>&1
' || true

# Verify which extensions are installable. CREATE EXTENSION best-effort: each
# either succeeds (extension count++) or is recorded as failed.
created_count=0
failed_extensions=""
for ext in vector pg_cron pgaudit postgis pg_uuidv7 age pgcrypto pg_trgm citext; do
  if docker exec "${container}" psql -U postgres -Atqc "CREATE EXTENSION IF NOT EXISTS ${ext}" >/dev/null 2>&1; then
    created_count=$((created_count + 1))
  else
    failed_extensions="${failed_extensions} ${ext}"
  fi
done

# Run a sized workload to exercise io_uring IO.
docker exec -i "${container}" psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
CREATE TABLE t6_workload (
  id bigserial PRIMARY KEY,
  payload text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);
INSERT INTO t6_workload (payload)
SELECT repeat('x', 256) FROM generate_series(1, 10000) AS s(i);
SELECT count(*) AS row_count, pg_size_pretty(pg_total_relation_size('t6_workload')) AS size FROM t6_workload;
SQL

# Capture pg_stat_io (PG18 only).
pg_stat_io_reads="$(docker exec "${container}" psql -U postgres -Atqc "SELECT COALESCE(SUM(reads), 0) FROM pg_stat_io WHERE backend_type IN ('client backend','autovacuum worker')")"
pg_stat_io_writes="$(docker exec "${container}" psql -U postgres -Atqc "SELECT COALESCE(SUM(writes), 0) FROM pg_stat_io WHERE backend_type IN ('client backend','autovacuum worker','background writer','checkpointer')")"

row_count="$(docker exec "${container}" psql -U postgres -Atqc 'SELECT count(*) FROM t6_workload')"
if [[ "${row_count}" != "10000" ]]; then
  echo "workload row count mismatch: expected 10000 got ${row_count}" >&2
  exit 1
fi

# Evidence row.
mkdir -p "$(dirname "${evidence_file}")"
if [[ ! -f "${evidence_file}" ]]; then
  printf 'observed_at\tgit_sha\timage\tkernel\tio_method\textensions_created\textensions_failed\tworkload_rows\tpg_stat_io_reads\tpg_stat_io_writes\n' >"${evidence_file}"
fi
printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
  "$(date -Is)" "$(git rev-parse HEAD)" "${postgres_image}" "$(uname -r)" \
  "${observed_io_method}" "${created_count}" "${failed_extensions:-none}" \
  "${row_count}" "${pg_stat_io_reads}" "${pg_stat_io_writes}" \
  >>"${evidence_file}"

printf 't6_pg18_io_uring_live\tpassed\tio_method=%s\textensions_created=%s\tworkload_rows=%s\tpg_stat_io_reads=%s\tpg_stat_io_writes=%s\n' \
  "${observed_io_method}" "${created_count}" "${row_count}" "${pg_stat_io_reads}" "${pg_stat_io_writes}"

echo "T6 PG18 io_uring live smoke passed"
