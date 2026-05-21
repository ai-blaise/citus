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
  -d "${postgres_image}" \
  -c shared_preload_libraries=pg_stat_statements >/dev/null

init_complete=0
for _ in $(seq 1 120); do
  if docker logs "${container}" 2>&1 | grep -q "PostgreSQL init process complete"; then
    init_complete=1
    break
  fi
  sleep 1
done

if [[ "${init_complete}" != "1" ]]; then
  docker logs "${container}" >&2 || true
  echo "postgres container did not finish init scripts" >&2
  exit 1
fi

ready=0
for _ in $(seq 1 60); do
  if docker exec "${container}" psql -U postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
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

docker exec -i "${container}" psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
CREATE EXTENSION pg_stat_statements;
SELECT pg_stat_statements_reset();
SELECT 1 AS ai_blaise_pg_stat_statements_seed;
CREATE EXTENSION ai_blaise_citus;
CREATE TABLE timescale_smoke_metrics (
  metric_time timestamptz NOT NULL,
  value double precision NOT NULL
);
CREATE INDEX timescale_smoke_metrics_metric_time_idx
ON timescale_smoke_metrics(metric_time);

CREATE TABLE timescale_bridge_call_log (
  function_name text NOT NULL,
  relation_name text,
  argument_summary jsonb NOT NULL DEFAULT '{}'::jsonb,
  called_at timestamptz NOT NULL DEFAULT now()
);

CREATE FUNCTION create_hypertable(
  table_name regclass,
  time_column text,
  chunk_time_interval interval DEFAULT NULL,
  if_not_exists boolean DEFAULT false
)
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
  INSERT INTO timescale_bridge_call_log(function_name, relation_name, argument_summary)
  VALUES (
    'create_hypertable',
    table_name::text,
    jsonb_build_object(
      'time_column', time_column,
      'chunk_time_interval', chunk_time_interval::text,
      'if_not_exists', if_not_exists
    )
  );
END;
$$;

CREATE FUNCTION create_distributed_table(
  table_name regclass,
  distribution_column text,
  shard_count integer DEFAULT 32
)
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
  INSERT INTO timescale_bridge_call_log(function_name, relation_name, argument_summary)
  VALUES (
    'create_distributed_table',
    table_name::text,
    jsonb_build_object(
      'distribution_column', distribution_column,
      'shard_count', shard_count
    )
  );
END;
$$;

CREATE FUNCTION add_retention_policy(
  table_name regclass,
  drop_after interval
)
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
  INSERT INTO timescale_bridge_call_log(function_name, relation_name, argument_summary)
  VALUES (
    'add_retention_policy',
    table_name::text,
    jsonb_build_object('drop_after', drop_after::text)
  );
END;
$$;

CREATE FUNCTION add_reorder_policy(
  table_name regclass,
  index_name text
)
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
  INSERT INTO timescale_bridge_call_log(function_name, relation_name, argument_summary)
  VALUES (
    'add_reorder_policy',
    table_name::text,
    jsonb_build_object('index_name', index_name)
  );
END;
$$;

DO $$
DECLARE
  status_count integer;
  planned_count integer;
  plan_sql text;
  bridge_features integer;
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
    WHERE feature_id IN ('Auth2', 'O1', 'O2', 'O3', 'R4')
      AND status = 'sql-runtime'
  ) <> 5 THEN
    RAISE EXCEPTION 'companion_feature_status must mark Auth2 and observability features as sql-runtime';
  END IF;

  PERFORM companion_set_session_claims(
    'user-123',
    'authenticated',
    'tenant-a',
    'jti-123'
  );
  IF companion_current_tenant_id() <> 'tenant-a' THEN
    RAISE EXCEPTION 'companion_current_tenant_id did not return tenant-a';
  END IF;
  IF NOT EXISTS (
    SELECT 1
    FROM companion_current_session_claims()
    WHERE uid = 'user-123'
      AND role = 'authenticated'
      AND tenant_id = 'tenant-a'
      AND jwt_id = 'jti-123'
  ) THEN
    RAISE EXCEPTION 'companion_current_session_claims did not return expected Auth2 claims';
  END IF;
  BEGIN
    PERFORM companion_set_session_claims('', 'authenticated', 'tenant-a');
    RAISE EXCEPTION 'companion_set_session_claims must reject empty uid claim';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'uid claim must not be empty' THEN
        RAISE;
      END IF;
  END;

  plan_sql := distribute_hypertable('timescale_smoke_metrics', 'metric_time', '1 day', 4);
  IF plan_sql NOT LIKE '%create_hypertable%' THEN
    RAISE EXCEPTION 'distribute_hypertable did not render create_hypertable plan: %', plan_sql;
  END IF;

  plan_sql := time_range_shard_pruner('timescale_smoke_metrics', 'metric_time');
  IF plan_sql NOT LIKE '%enable_time_range_shard_pruner%' THEN
    RAISE EXCEPTION 'time_range_shard_pruner did not render pruner plan: %', plan_sql;
  END IF;

  PERFORM apply_distribute_hypertable(
    'timescale_smoke_metrics',
    'metric_time',
    '1 day',
    2
  );
  IF (
    SELECT count(*)
    FROM timescale_bridge_call_log
    WHERE function_name IN ('create_hypertable', 'create_distributed_table')
  ) <> 2 THEN
    RAISE EXCEPTION 'apply_distribute_hypertable did not call both dependency entrypoints';
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
  PERFORM apply_retention_policy_distributed(
    'timescale_smoke_metrics',
    '90 days'
  );
  IF NOT EXISTS (
    SELECT 1
    FROM timescale_bridge_call_log
    WHERE function_name = 'add_retention_policy'
      AND relation_name = 'timescale_smoke_metrics'
  ) THEN
    RAISE EXCEPTION 'apply_retention_policy_distributed did not call dependency entrypoint';
  END IF;

  PERFORM companion_internal.add_reorder_policy_distributed(
    'timescale_smoke_metrics'::regclass,
    'timescale_smoke_metrics_metric_time_idx'::name
  );
  PERFORM apply_reorder_policy_distributed(
    'timescale_smoke_metrics',
    'timescale_smoke_metrics_metric_time_idx'
  );
  IF NOT EXISTS (
    SELECT 1
    FROM timescale_bridge_call_log
    WHERE function_name = 'add_reorder_policy'
      AND relation_name = 'timescale_smoke_metrics'
  ) THEN
    RAISE EXCEPTION 'apply_reorder_policy_distributed did not call dependency entrypoint';
  END IF;

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
  PERFORM apply_time_range_shard_pruner(
    'timescale_smoke_metrics',
    'metric_time'
  );

  BEGIN
    PERFORM apply_compression_policy_distributed(
      'timescale_smoke_metrics',
      '7 days',
      'metric_time',
      'metric_time DESC'
    );
    RAISE EXCEPTION 'apply_compression_policy_distributed must require TimescaleDB dependency';
  EXCEPTION WHEN OTHERS THEN
    IF SQLERRM NOT LIKE '%requires visible function add_compression_policy from extension timescaledb%' THEN
      RAISE;
    END IF;
  END;

  BEGIN
    PERFORM apply_continuous_aggregate_distributed(
      'timescale_smoke_hourly_apply',
      'SELECT metric_time, avg(value) FROM timescale_smoke_metrics GROUP BY 1',
      '7 days',
      '1 hour',
      '1 hour'
    );
    RAISE EXCEPTION 'apply_continuous_aggregate_distributed must require TimescaleDB dependency';
  EXCEPTION WHEN OTHERS THEN
    IF SQLERRM NOT LIKE '%requires visible function add_continuous_aggregate_policy from extension timescaledb%' THEN
      RAISE;
    END IF;
  END;

  SELECT count(DISTINCT feature_id)
  INTO bridge_features
  FROM companion_timescale_bridge_state
  WHERE feature_id IN ('TS1', 'TS2', 'TS3', 'TS4', 'TS5', 'TS12');
  IF bridge_features <> 6 THEN
    RAISE EXCEPTION 'expected six Timescale bridge state feature ids, got %',
      bridge_features;
  END IF;

  IF NOT EXISTS (
    SELECT 1
    FROM companion_timescale_bridge_state
    WHERE feature_id = 'TS1'
      AND object_name = 'timescale_smoke_metrics'
      AND parameters->>'shard_count' = '2'
  ) THEN
    RAISE EXCEPTION 'public apply_distribute_hypertable state was not recorded';
  END IF;

  IF NOT EXISTS (SELECT 1 FROM companion_pg_stat_local_activity) THEN
    RAISE EXCEPTION 'companion_pg_stat_local_activity must report the local postgres node';
  END IF;

  IF NOT EXISTS (SELECT 1 FROM companion_pg_stat_distributed) THEN
    RAISE EXCEPTION 'compatibility companion_pg_stat_distributed view must report the local postgres node';
  END IF;

  IF NOT EXISTS (
    SELECT 1
    FROM companion_pg_stat_statements_p95
    WHERE query LIKE '%ai_blaise_pg_stat_statements_seed%'
      AND calls >= 1
      AND p95_ms >= 0
  ) THEN
    RAISE EXCEPTION 'companion_pg_stat_statements_p95 must report pg_stat_statements rows';
  END IF;

  PERFORM * FROM companion_pg_dist_replication_lag LIMIT 1;
END $$;
SQL

docker exec -d "${container}" sh -c \
  "(printf 'BEGIN;\nSELECT pg_backend_pid();\n'; sleep 60; printf 'COMMIT;\n') | psql -U postgres -v ON_ERROR_STOP=1"

idle_seen=0
for _ in $(seq 1 20); do
  idle_count="$(
    docker exec "${container}" psql -U postgres -Atqv ON_ERROR_STOP=1 \
      -c "SELECT count(*) FROM companion_idle_transactions('100 milliseconds'::interval) WHERE state = 'idle in transaction';"
  )"
  if [[ "${idle_count}" =~ ^[1-9][0-9]*$ ]]; then
    idle_seen=1
    break
  fi
  sleep 1
done

if [[ "${idle_seen}" != "1" ]]; then
  docker exec "${container}" psql -U postgres -v ON_ERROR_STOP=1 \
    -c "SELECT pid, state, xact_start, query FROM pg_stat_activity WHERE datname = current_database() ORDER BY pid;" >&2 || true
  echo "companion_idle_transactions did not detect a real idle transaction" >&2
  exit 1
fi

echo "ai_blaise_citus SQL extension smoke passed with ${postgres_image}"
