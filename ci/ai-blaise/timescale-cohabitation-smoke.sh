#!/usr/bin/env bash
set -euo pipefail

# FEATURE: TS6 TS18

repo_root="$(git rev-parse --show-toplevel)"
dockerfile="${repo_root}/images/citus-timescale-cohabitation/Dockerfile"
base_image="${TIMESCALE_COHABITATION_BASE_IMAGE:-timescale/timescaledb:latest-pg17}"
image="${TIMESCALE_COHABITATION_IMAGE:-}"
tag="${TIMESCALE_COHABITATION_TAG:-ai-blaise-citus-timescale-cohabitation:local}"
make_jobs="${TIMESCALE_COHABITATION_MAKE_JOBS:-4}"
require_docker="${REQUIRE_DOCKER:-0}"
evidence_file="${TIMESCALE_COHABITATION_EVIDENCE:-artifacts/timescale-cohabitation-evidence.tsv}"
expected_ts_minor="${TIMESCALE_COHABITATION_EXPECTED_TS_MINOR:-}"
expected_pg_major="${TIMESCALE_COHABITATION_EXPECTED_PG_MAJOR:-}"

if [[ -z "${expected_ts_minor}" && "${base_image}" =~ :([0-9]+\.[0-9]+)(\.[0-9]+)?-pg[0-9]+$ ]]; then
  expected_ts_minor="${BASH_REMATCH[1]}"
fi
if [[ -z "${expected_pg_major}" && "${base_image}" =~ -pg([0-9]+)$ ]]; then
  expected_pg_major="${BASH_REMATCH[1]}"
fi

if [[ ! -s "${dockerfile}" ]]; then
  echo "missing Timescale cohabitation Dockerfile: ${dockerfile}" >&2
  exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
  if [[ "${require_docker}" == "1" ]]; then
    echo "docker is required for Timescale cohabitation smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping Timescale cohabitation smoke"
  exit 0
fi

if [[ -z "${image}" ]]; then
  docker build \
    --file "${dockerfile}" \
    --build-arg "BASE_IMAGE=${base_image}" \
    --build-arg "MAKE_JOBS=${make_jobs}" \
    --tag "${tag}" \
    "${repo_root}"
  image="${tag}"
fi

container="ai-blaise-timescale-cohabitation-smoke-${RANDOM}-$$"
cleanup() {
  docker rm -f "${container}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run \
  --name "${container}" \
  -e POSTGRES_PASSWORD=postgres \
  -d "${image}" \
  postgres \
  -c shared_preload_libraries=timescaledb,citus \
  -c citus.cohabit_extensions=timescaledb >/dev/null

init_complete=0
for _ in $(seq 1 180); do
  if docker logs "${container}" 2>&1 | grep -q "PostgreSQL init process complete"; then
    init_complete=1
    break
  fi
  sleep 1
done

if [[ "${init_complete}" != "1" ]]; then
  docker logs "${container}" >&2 || true
  echo "Timescale/Citus cohabitation container did not finish init scripts" >&2
  exit 1
fi

ready=0
for _ in $(seq 1 180); do
  if docker exec "${container}" psql -U postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 1
done

if [[ "${ready}" != "1" ]]; then
  docker logs "${container}" >&2 || true
  echo "Timescale/Citus cohabitation container did not become ready" >&2
  exit 1
fi

docker exec -i "${container}" psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
SHOW shared_preload_libraries;
SELECT current_setting('citus.cohabit_extensions', true) AS cohabit_extensions;

CREATE EXTENSION IF NOT EXISTS citus;
CREATE EXTENSION IF NOT EXISTS timescaledb;
CREATE EXTENSION IF NOT EXISTS ai_blaise_citus;

SELECT extname, extversion
FROM pg_extension
WHERE extname IN ('citus', 'timescaledb', 'ai_blaise_citus')
ORDER BY extname;

CREATE TABLE citus_smoke_events (
  tenant_id integer NOT NULL,
  metric_time timestamptz NOT NULL,
  value double precision NOT NULL
);
SELECT create_distributed_table('citus_smoke_events', 'tenant_id');
INSERT INTO citus_smoke_events VALUES (1, now(), 42.0);
SELECT count(*) FROM citus_smoke_events;

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
    FROM pg_dist_partition
    WHERE logicalrelid = 'citus_smoke_events'::regclass
  ) THEN
    RAISE EXCEPTION 'real Citus create_distributed_table did not register pg_dist_partition';
  END IF;

  IF NOT EXISTS (
    SELECT 1
    FROM _timescaledb_catalog.hypertable
    WHERE table_name = 'timescale_smoke_metrics'
  ) THEN
    RAISE EXCEPTION 'apply_distribute_hypertable did not create a real Timescale hypertable';
  END IF;

  IF NOT EXISTS (
    SELECT 1
    FROM pg_dist_partition
    WHERE logicalrelid = 'timescale_smoke_metrics'::regclass
  ) THEN
    RAISE EXCEPTION 'apply_distribute_hypertable did not call real Citus create_distributed_table';
  END IF;

  IF to_regclass('timescale_smoke_hourly') IS NULL THEN
    RAISE EXCEPTION 'apply_continuous_aggregate_distributed did not create the continuous aggregate';
  END IF;

  SELECT count(DISTINCT feature_id)
  INTO bridge_features
  FROM companion_timescale_bridge_state
  WHERE feature_id IN ('TS1', 'TS2', 'TS3', 'TS4', 'TS5', 'TS12');
  IF bridge_features <> 6 THEN
    RAISE EXCEPTION 'expected six Timescale bridge feature ids, got %', bridge_features;
  END IF;
END $$;
SQL

mkdir -p "$(dirname "${evidence_file}")"
image_id="$(docker image inspect --format '{{.Id}}' "${image}")"
if [[ ! "${image_id}" =~ ^sha256:[0-9a-f]{64}$ ]]; then
  echo "Timescale/Citus cohabitation image did not report a stable image identity: ${image_id}" >&2
  exit 1
fi

runtime_metadata="$(docker exec -i "${container}" psql -U postgres -AtX -v ON_ERROR_STOP=1 -F $'\t' <<'SQL'
SELECT
  current_setting('server_version_num', true),
  current_setting('server_version', true),
  current_setting('shared_preload_libraries', true),
  current_setting('citus.cohabit_extensions', true),
  (SELECT extversion FROM pg_extension WHERE extname = 'timescaledb'),
  (SELECT extversion FROM pg_extension WHERE extname = 'citus'),
  (SELECT extversion FROM pg_extension WHERE extname = 'ai_blaise_citus'),
  (SELECT count(DISTINCT feature_id)::text FROM companion_timescale_bridge_state WHERE feature_id IN ('TS1', 'TS2', 'TS3', 'TS4', 'TS5', 'TS12'));
SQL
)"
IFS=$'\t' read -r server_version_num server_version shared_preload_libraries cohabit_extensions timescaledb_extversion citus_extversion ai_blaise_citus_extversion bridge_features <<<"${runtime_metadata}"
if [[ ! "${server_version_num:-}" =~ ^[0-9]+$ || "${bridge_features:-}" != "6" ]]; then
  echo "Timescale/Citus cohabitation metadata query returned unusable evidence: ${runtime_metadata}" >&2
  exit 1
fi
actual_pg_major="$((server_version_num / 10000))"
if [[ -n "${expected_pg_major}" && "${actual_pg_major}" != "${expected_pg_major}" ]]; then
  echo "Timescale/Citus cohabitation PG major mismatch: expected ${expected_pg_major}, got ${actual_pg_major} from server_version_num=${server_version_num}" >&2
  exit 1
fi
if [[ -n "${expected_ts_minor}" && "${timescaledb_extversion}" != "${expected_ts_minor}".* && "${timescaledb_extversion}" != "${expected_ts_minor}" ]]; then
  echo "Timescale/Citus cohabitation TimescaleDB minor mismatch: expected ${expected_ts_minor}.x, got ${timescaledb_extversion}" >&2
  exit 1
fi

base_digest="$(
  { docker image inspect --format '{{range .RepoDigests}}{{println .}}{{end}}' "${base_image}" 2>/dev/null || true; } |
    awk 'NR == 1 { print; exit }'
)"
if [[ -z "${base_digest}" ]]; then
  base_digest="$(
    docker buildx imagetools inspect "${base_image}" 2>/dev/null |
      awk '/^Digest:/ { print $2; exit }'
  )" || base_digest=""
fi
git_sha="$(git -C "${repo_root}" rev-parse --short=12 HEAD)"
command_path="postgres -c shared_preload_libraries=timescaledb,citus -c citus.cohabit_extensions=timescaledb"
{
  printf 'git_sha\timage\timage_id\tbase_image\tbase_digest\tcommand_path\tserver_version_num\tserver_version\ttimescaledb_extversion\tcitus_extversion\tai_blaise_citus_extversion\tshared_preload_libraries\tcohabit_extensions\texpected_pg_major\texpected_ts_minor\treal_citus_distribution\tstubbed_citus_distribution\tbridge_features\tpolicy_execution_scope\n'
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    "${git_sha}" \
    "${image}" \
    "${image_id}" \
    "${base_image}" \
    "${base_digest}" \
    "${command_path}" \
    "${server_version_num}" \
    "${server_version}" \
    "${timescaledb_extversion}" \
    "${citus_extversion}" \
    "${ai_blaise_citus_extversion}" \
    "${shared_preload_libraries}" \
    "${cohabit_extensions}" \
    "${expected_pg_major:-}" \
    "${expected_ts_minor:-}" \
    "true" \
    "false" \
    "${bridge_features}" \
    "entrypoints-and-catalog-state-only"
} >"${evidence_file}"

echo "ai_blaise_citus Timescale/Citus cohabitation smoke passed with ${image}"
