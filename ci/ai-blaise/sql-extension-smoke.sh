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
  IF NOT EXISTS (
    SELECT 1
    FROM companion_feature_status()
    WHERE feature_id = 'TS18'
      AND status = 'sql-runtime'
  ) THEN
    RAISE EXCEPTION 'companion_feature_status must include sql-runtime TS18';
  END IF;
  IF (
    SELECT count(*)
    FROM companion_feature_status()
    WHERE feature_id IN ('O1', 'O2', 'O3', 'R4')
      AND status = 'sql-runtime'
  ) <> 4 THEN
    RAISE EXCEPTION 'companion_feature_status must mark observability features as sql-runtime';
  END IF;

  plan_sql := distribute_hypertable('timescale_smoke_metrics', 'metric_time', '1 day', 4);
  IF plan_sql NOT LIKE '%create_hypertable%' THEN
    RAISE EXCEPTION 'distribute_hypertable did not render create_hypertable plan: %', plan_sql;
  END IF;

  plan_sql := time_range_shard_pruner('timescale_smoke_metrics', 'metric_time');
  IF plan_sql NOT LIKE '%enable_time_range_shard_pruner%' THEN
    RAISE EXCEPTION 'time_range_shard_pruner did not render pruner plan: %', plan_sql;
  END IF;

  PERFORM companion_internal.create_worker_hypertables(
    'timescale_smoke_metrics'::regclass,
    'metric_time'::name,
    '1 day'::interval,
    4
  );
  PERFORM companion_internal.add_compression_policy_distributed(
    'timescale_smoke_metrics'::regclass,
    '7 days'::interval,
    ARRAY['metric_time']::text[],
    ARRAY['metric_time DESC']::text[]
  );
  PERFORM companion_internal.add_retention_policy_distributed(
    'timescale_smoke_metrics'::regclass,
    '90 days'::interval
  );
  PERFORM companion_internal.add_reorder_policy_distributed(
    'timescale_smoke_metrics'::regclass,
    'timescale_smoke_metrics_metric_time_idx'::name
  );
  PERFORM companion_internal.add_continuous_aggregate_distributed(
    'timescale_smoke_hourly',
    'SELECT time_bucket(''1 hour'', metric_time), avg(value) FROM timescale_smoke_metrics GROUP BY 1',
    '7 days'::interval,
    '1 hour'::interval,
    '1 hour'::interval
  );
  PERFORM companion_internal.enable_time_range_shard_pruner(
    'timescale_smoke_metrics'::regclass,
    'metric_time'::name
  );

  IF (
    SELECT count(*)
    FROM companion_timescale_bridge_state
    WHERE feature_id IN ('TS1', 'TS2', 'TS3', 'TS4', 'TS5', 'TS12')
  ) <> 6 THEN
    RAISE EXCEPTION 'expected six Timescale bridge state records, got %',
      (SELECT count(*) FROM companion_timescale_bridge_state);
  END IF;

  IF NOT EXISTS (SELECT 1 FROM companion_pg_stat_distributed) THEN
    RAISE EXCEPTION 'companion_pg_stat_distributed must report the local postgres node';
  END IF;

  PERFORM * FROM companion_pg_stat_statements_p95 LIMIT 1;
  PERFORM * FROM companion_pg_dist_replication_lag LIMIT 1;
  PERFORM * FROM companion_idle_transactions('1 second'::interval) LIMIT 1;
END $$;
SQL

echo "ai_blaise_citus SQL extension smoke passed with ${postgres_image}"
