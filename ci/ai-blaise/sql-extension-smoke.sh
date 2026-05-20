#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
extension_dir="${repo_root}/images/citus-pg-overlay/extensions"
control_file="${extension_dir}/ai_blaise_citus.control"
sql_file="${extension_dir}/ai_blaise_citus--0.1.0.sql"
postgres_image="${SQL_EXTENSION_SMOKE_IMAGE:-postgres:17}"
require_docker="${REQUIRE_DOCKER:-0}"

for file in "${control_file}" "${sql_file}"; do
  if [[ ! -s "${file}" ]]; then
    echo "missing SQL extension smoke artifact: ${file}" >&2
    exit 1
  fi
done

if ! command -v docker >/dev/null 2>&1; then
  if [[ "${require_docker}" == "1" ]]; then
    echo "docker is required for SQL extension smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping SQL extension smoke"
  exit 0
fi

container="ai-blaise-sql-extension-smoke-${RANDOM}-$$"
cleanup() {
  docker rm -f "${container}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run \
  --name "${container}" \
  -e POSTGRES_PASSWORD=postgres \
  -v "${control_file}:/usr/share/postgresql/17/extension/ai_blaise_citus.control:ro" \
  -v "${sql_file}:/usr/share/postgresql/17/extension/ai_blaise_citus--0.1.0.sql:ro" \
  -d "${postgres_image}" >/dev/null

ready=0
for _ in $(seq 1 60); do
  if docker exec "${container}" pg_isready -U postgres >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 1
done

if [[ "${ready}" != "1" ]]; then
  docker logs "${container}" >&2 || true
  echo "postgres container did not become ready" >&2
  exit 1
fi

docker exec "${container}" psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
CREATE EXTENSION ai_blaise_citus;
CREATE TABLE timescale_smoke_metrics (
  metric_time timestamptz NOT NULL,
  value double precision NOT NULL
);

DO $$
DECLARE
  status_count integer;
  planned_count integer;
  plan_sql text;
BEGIN
  SELECT count(*) INTO status_count FROM companion_feature_status();
  IF status_count < 60 THEN
    RAISE EXCEPTION 'expected at least 60 companion feature rows, got %', status_count;
  END IF;

  SELECT count(*) INTO planned_count
  FROM companion_feature_status()
  WHERE status = 'planned';
  IF planned_count <> 0 THEN
    RAISE EXCEPTION 'companion_feature_status returned % planned rows', planned_count;
  END IF;

  IF NOT EXISTS (SELECT 1 FROM companion_feature_status() WHERE feature_id = 'TS1') THEN
    RAISE EXCEPTION 'companion_feature_status must include TS1';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM companion_feature_status() WHERE feature_id = 'TS5') THEN
    RAISE EXCEPTION 'companion_feature_status must include TS5';
  END IF;

  plan_sql := distribute_hypertable('timescale_smoke_metrics', 'metric_time', '1 day', 4);
  IF plan_sql NOT LIKE '%create_hypertable%' THEN
    RAISE EXCEPTION 'distribute_hypertable did not render create_hypertable plan: %', plan_sql;
  END IF;

  plan_sql := time_range_shard_pruner('timescale_smoke_metrics', 'metric_time');
  IF plan_sql NOT LIKE '%enable_time_range_shard_pruner%' THEN
    RAISE EXCEPTION 'time_range_shard_pruner did not render pruner plan: %', plan_sql;
  END IF;
END $$;
SQL

echo "ai_blaise_citus SQL extension smoke passed with ${postgres_image}"
