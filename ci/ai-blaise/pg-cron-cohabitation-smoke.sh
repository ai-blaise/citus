#!/usr/bin/env bash
set -euo pipefail

# FEATURE: Bundle1 TS19 TS20
# Live PG17 smoke for the narrow pg_cron cohabitation boundary. This proves
# startup parsing, pg_cron package availability, real Citus + pg_cron extension
# load, SQL-visible cohabit detection, and fail-closed mismatch handling. It
# does not prove the unexposed TS19 in-shmem clock-reservation flag.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

require_docker="${REQUIRE_DOCKER:-0}"
image_tag="${PG_CRON_COHABITATION_IMAGE:-ai-blaise-citus-pg-cron-cohabitation:local}"
base_image="${PG_CRON_COHABITATION_BASE_IMAGE:-postgres:17-bookworm}"
evidence_file="${PG_CRON_COHABITATION_EVIDENCE_FILE:-artifacts/pg-cron-cohabitation-evidence.tsv}"
make_jobs="${MAKE_JOBS:-2}"

if ! command -v docker >/dev/null 2>&1; then
  if [[ "${require_docker}" == "1" ]]; then
    echo "docker is required for pg_cron cohabitation smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping pg_cron cohabitation smoke"
  exit 0
fi

positive_container=""
negative_container=""
cleanup() {
  if [[ -n "${positive_container}" ]]; then
    docker rm -f "${positive_container}" >/dev/null 2>&1 || true
  fi
  if [[ -n "${negative_container}" ]]; then
    docker rm -f "${negative_container}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

wait_for_postgres() {
  local container="$1"
  local init_complete=0
  local _
  for _ in $(seq 1 180); do
    if docker logs "${container}" 2>&1 | grep -q "PostgreSQL init process complete"; then
      init_complete=1
      break
    fi
    sleep 1
  done
  if [[ "${init_complete}" != "1" ]]; then
    docker logs "${container}" >&2 || true
    echo "postgres container did not finish init scripts: ${container}" >&2
    exit 1
  fi

  local ready=0
  for _ in $(seq 1 90); do
    if docker exec "${container}" psql -U postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
      ready=1
      break
    fi
    sleep 1
  done
  if [[ "${ready}" != "1" ]]; then
    docker logs "${container}" >&2 || true
    echo "postgres container did not become ready: ${container}" >&2
    exit 1
  fi
}

run_sql() {
  local container="$1"
  docker exec -i "${container}" psql -U postgres -v ON_ERROR_STOP=1
}

mkdir -p "$(dirname "${evidence_file}")"

docker build \
  -f images/citus-pg-cron-cohabitation/Dockerfile \
  --build-arg BASE_IMAGE="${base_image}" \
  --build-arg MAKE_JOBS="${make_jobs}" \
  -t "${image_tag}" \
  .

positive_container="ai-blaise-pg-cron-cohabit-${RANDOM}-$$"
docker run \
  --name "${positive_container}" \
  -e POSTGRES_PASSWORD=postgres \
  -d "${image_tag}" \
  postgres \
    -c shared_preload_libraries=pg_cron,citus \
    -c citus.cohabit_extensions=pg_cron \
    -c cron.database_name=postgres >/dev/null
wait_for_postgres "${positive_container}"

run_sql "${positive_container}" <<'SQL'
CREATE EXTENSION IF NOT EXISTS citus;
CREATE EXTENSION IF NOT EXISTS pg_cron;
CREATE EXTENSION IF NOT EXISTS ai_blaise_citus;
SELECT companion_internal.assert_shared_preload_libraries(
  string_to_array(current_setting('shared_preload_libraries', true), ','),
  ARRAY['pg_cron']
);
SELECT companion_internal.assert_cohabit_extension_ready('pg_cron');
SELECT cron.schedule('ai_blaise_pg_cron_cohabit_smoke', '* * * * *', 'SELECT 1');
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1
    FROM cron.job
    WHERE jobname = 'ai_blaise_pg_cron_cohabit_smoke'
  ) THEN
    RAISE EXCEPTION 'pg_cron smoke job was not registered';
  END IF;
END;
$$;
SQL

{
  printf 'key\tvalue\n'
  printf 'git_sha\t%s\n' "$(git rev-parse HEAD)"
  printf 'image\t%s\n' "${image_tag}"
  printf 'base_image\t%s\n' "${base_image}"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'server_version_num' || E'\t' || current_setting('server_version_num')"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'shared_preload_libraries' || E'\t' || current_setting('shared_preload_libraries')"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'citus_cohabit_extensions' || E'\t' || current_setting('citus.cohabit_extensions')"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'pg_cron_extversion' || E'\t' || extversion FROM pg_extension WHERE extname = 'pg_cron'"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'citus_extversion' || E'\t' || extversion FROM pg_extension WHERE extname = 'citus'"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'pg_cron_detection' || E'\t' || role || ':' || ready || ':' || coalesce(reason, 'ok') FROM companion_internal.cohabit_extension_detection_report() WHERE extension_name = 'pg_cron'"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'cron_job_registered' || E'\t' || count(*) FROM cron.job WHERE jobname = 'ai_blaise_pg_cron_cohabit_smoke'"
} >"${evidence_file}"

grep -Fq $'pg_cron_detection\tclock-worker:true:ok' "${evidence_file}"
grep -Fq $'cron_job_registered\t1' "${evidence_file}"

negative_container="ai-blaise-pg-cron-cohabit-negative-${RANDOM}-$$"
docker run \
  --name "${negative_container}" \
  -e POSTGRES_PASSWORD=postgres \
  -d "${image_tag}" \
  postgres \
    -c shared_preload_libraries=pg_cron,citus \
    -c cron.database_name=postgres >/dev/null
wait_for_postgres "${negative_container}"

run_sql "${negative_container}" <<'SQL'
CREATE EXTENSION IF NOT EXISTS citus;
CREATE EXTENSION IF NOT EXISTS pg_cron;
CREATE EXTENSION IF NOT EXISTS ai_blaise_citus;
SQL

if docker exec "${negative_container}" psql -U postgres -v ON_ERROR_STOP=1 -c \
  "SELECT companion_internal.assert_cohabit_extension_ready('pg_cron');" >/tmp/pg-cron-negative-$$.out 2>&1; then
  cat /tmp/pg-cron-negative-$$.out >&2 || true
  rm -f /tmp/pg-cron-negative-$$.out
  echo "pg_cron cohabit detector did not fail closed without citus.cohabit_extensions" >&2
  exit 1
fi
if ! grep -Fq "missing-citus-cohabit-extensions" /tmp/pg-cron-negative-$$.out; then
  cat /tmp/pg-cron-negative-$$.out >&2 || true
  rm -f /tmp/pg-cron-negative-$$.out
  echo "pg_cron negative smoke failed for the wrong reason" >&2
  exit 1
fi
rm -f /tmp/pg-cron-negative-$$.out
printf 'negative_missing_cohabit_guc\tpass\n' >>"${evidence_file}"

cat "${evidence_file}"
echo "pg_cron cohabitation smoke passed"
