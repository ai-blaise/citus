#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
extension_dir="${repo_root}/images/citus-pg-overlay/extensions"
control_file="${extension_dir}/ai_blaise_citus.control"
sql_file="${extension_dir}/ai_blaise_citus--0.1.0.sql"
timescale_image="${TIMESCALE_BRIDGE_SMOKE_IMAGE:-timescale/timescaledb:latest-pg17}"
require_docker="${REQUIRE_DOCKER:-0}"
evidence_file="${TIMESCALE_BRIDGE_SMOKE_EVIDENCE:-artifacts/timescale-bridge-smoke.tsv}"

for file in "${control_file}" "${sql_file}"; do
  if [[ ! -s "${file}" ]]; then
    echo "missing Timescale bridge smoke artifact: ${file}" >&2
    exit 1
  fi
done

if ! command -v docker >/dev/null 2>&1; then
  if [[ "${require_docker}" == "1" ]]; then
    echo "docker is required for Timescale bridge smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping Timescale bridge smoke"
  exit 0
fi

container="ai-blaise-timescale-bridge-smoke-${RANDOM}-$$"
cleanup() {
  docker rm -f "${container}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run \
  --name "${container}" \
  -e POSTGRES_PASSWORD=postgres \
  -v "${control_file}:/usr/share/postgresql/17/extension/ai_blaise_citus.control:ro" \
  -v "${sql_file}:/usr/share/postgresql/17/extension/ai_blaise_citus--0.1.0.sql:ro" \
  -v "${control_file}:/usr/local/share/postgresql/extension/ai_blaise_citus.control:ro" \
  -v "${sql_file}:/usr/local/share/postgresql/extension/ai_blaise_citus--0.1.0.sql:ro" \
  -d "${timescale_image}" >/dev/null

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
  echo "TimescaleDB container did not finish init scripts" >&2
  exit 1
fi

ready=0
for _ in $(seq 1 120); do
  if docker exec "${container}" psql -U postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 1
done

if [[ "${ready}" != "1" ]]; then
  docker logs "${container}" >&2 || true
  echo "TimescaleDB container did not become ready" >&2
  exit 1
fi

docker exec -i "${container}" psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
CREATE EXTENSION IF NOT EXISTS timescaledb;
CREATE EXTENSION IF NOT EXISTS ai_blaise_citus;

CREATE TABLE timescale_missing_citus (
  metric_time timestamptz NOT NULL,
  tenant_id integer NOT NULL,
  value double precision NOT NULL
);

DO $$
BEGIN
  BEGIN
    PERFORM apply_distribute_hypertable('timescale_missing_citus', 'metric_time', '1 day', 2);
    RAISE EXCEPTION 'expected apply_distribute_hypertable to fail without Citus create_distributed_table';
  EXCEPTION WHEN OTHERS THEN
    IF SQLERRM NOT LIKE '%requires visible function create_distributed_table from extension citus%' THEN
      RAISE EXCEPTION 'apply_distribute_hypertable failed with unexpected message: %', SQLERRM;
    END IF;
  END;
END $$;

DROP TABLE timescale_missing_citus;

CREATE TABLE citus_bridge_call_log (
  function_name text NOT NULL,
  relation_name text,
  argument_summary jsonb NOT NULL DEFAULT '{}'::jsonb,
  called_at timestamptz NOT NULL DEFAULT now()
);

-- The Timescale image supplies real TimescaleDB functions. Citus is the only
-- external dependency stubbed here so the bridge can prove its call contract.
CREATE FUNCTION create_distributed_table(
  table_name regclass,
  distribution_column text,
  shard_count integer DEFAULT 32
)
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
  INSERT INTO citus_bridge_call_log(function_name, relation_name, argument_summary)
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

CREATE EXTENSION IF NOT EXISTS ai_blaise_citus;

CREATE TABLE timescale_smoke_metrics (
  metric_time timestamptz NOT NULL,
  tenant_id integer NOT NULL,
  value double precision NOT NULL
);
CREATE INDEX timescale_smoke_metrics_metric_time_idx
ON timescale_smoke_metrics(metric_time);

SELECT apply_distribute_hypertable('timescale_smoke_metrics', 'metric_time', '1 day', 2);
SELECT apply_compression_policy_distributed(
  'timescale_smoke_metrics',
  '7 days',
  'tenant_id',
  'metric_time DESC'
);
SELECT apply_retention_policy_distributed('timescale_smoke_metrics', '90 days');
SELECT apply_reorder_policy_distributed(
  'timescale_smoke_metrics',
  'timescale_smoke_metrics_metric_time_idx'
);
SELECT apply_continuous_aggregate_distributed(
  'timescale_smoke_hourly',
  'SELECT time_bucket(''1 hour'', metric_time) AS bucket, avg(value) FROM timescale_smoke_metrics GROUP BY 1',
  '7 days',
  '1 hour',
  '1 hour'
);
SELECT apply_time_range_shard_pruner('timescale_smoke_metrics', 'metric_time');

DO $$
DECLARE
  bridge_features integer;
BEGIN
  IF NOT EXISTS (
    SELECT 1
    FROM _timescaledb_catalog.hypertable
    WHERE table_name = 'timescale_smoke_metrics'
  ) THEN
    RAISE EXCEPTION 'apply_distribute_hypertable did not create a real Timescale hypertable';
  END IF;

  IF NOT EXISTS (
    SELECT 1
    FROM citus_bridge_call_log
    WHERE function_name = 'create_distributed_table'
      AND relation_name = 'timescale_smoke_metrics'
      AND argument_summary->>'distribution_column' = 'metric_time'
      AND argument_summary->>'shard_count' = '2'
  ) THEN
    RAISE EXCEPTION 'apply_distribute_hypertable did not call the Citus distribution entrypoint';
  END IF;

  IF to_regclass('timescale_smoke_hourly') IS NULL THEN
    RAISE EXCEPTION 'apply_continuous_aggregate_distributed did not create the continuous aggregate';
  END IF;

  SELECT count(DISTINCT feature_id)
  INTO bridge_features
  FROM companion_timescale_bridge_state
  WHERE feature_id IN ('TS1', 'TS2', 'TS3', 'TS4', 'TS5', 'TS12');
  IF bridge_features <> 6 THEN
    RAISE EXCEPTION 'expected six Timescale bridge feature ids, got %',
      bridge_features;
  END IF;
END $$;
SQL

mkdir -p "$(dirname "${evidence_file}")"
image_id="$(docker image inspect --format '{{.Id}}' "${timescale_image}")"
if [[ ! "${image_id}" =~ ^sha256:[0-9a-f]{64}$ ]]; then
  echo "Timescale bridge smoke image did not report a stable image identity: ${image_id}" >&2
  exit 1
fi
runtime_metadata="$(docker exec -i "${container}" psql -U postgres -AtX -v ON_ERROR_STOP=1 -F $'\t' <<'SQL'
SELECT
  current_setting('server_version_num', true),
  current_setting('server_version', true),
  (SELECT extversion FROM pg_extension WHERE extname = 'timescaledb'),
  (SELECT extversion FROM pg_extension WHERE extname = 'ai_blaise_citus'),
  (SELECT count(DISTINCT feature_id)::text FROM companion_timescale_bridge_state WHERE feature_id IN ('TS1', 'TS2', 'TS3', 'TS4', 'TS5', 'TS12'));
SQL
)"
IFS=$'\t' read -r server_version_num server_version timescaledb_extversion ai_blaise_citus_extversion bridge_features <<<"${runtime_metadata}"
if [[ ! "${server_version_num:-}" =~ ^[0-9]+$ || "${bridge_features:-}" != "6" ]]; then
  echo "Timescale bridge smoke metadata query returned unusable evidence: ${runtime_metadata}" >&2
  exit 1
fi
git_sha="$(git -C "${repo_root}" rev-parse --short=12 HEAD)"
{
  printf 'git_sha\timage\timage_id\tserver_version_num\tserver_version\ttimescaledb_extversion\tai_blaise_citus_extversion\treal_timescaledb\tstubbed_citus_distribution\tmissing_citus_fail_closed\tbridge_features\tpolicy_execution_scope\n'
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    "${git_sha}" \
    "${timescale_image}" \
    "${image_id}" \
    "${server_version_num}" \
    "${server_version}" \
    "${timescaledb_extversion}" \
    "${ai_blaise_citus_extversion}" \
    "true" \
    "true" \
    "true" \
    "${bridge_features}" \
    "entrypoints-and-catalog-state-only"
} >"${evidence_file}"

echo "ai_blaise_citus Timescale bridge smoke passed with ${timescale_image}; evidence at ${evidence_file}"
