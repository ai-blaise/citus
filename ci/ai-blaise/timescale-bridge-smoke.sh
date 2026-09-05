#!/usr/bin/env bash
set -euo pipefail

# FEATURE: TS1 TS2 TS3 TS4 TS5 TS12

repo_root="$(git rev-parse --show-toplevel)"
fixture_builder="${repo_root}/ci/ai-blaise/build-real-citus-timescale-test-fixture.sh"
fixture_contract="${repo_root}/ci/ai-blaise/real-citus-timescale-test-fixture-contract.py"
timescaledb_minor="${TIMESCALE_BRIDGE_EXPECTED_TS_MINOR:-${CITUS_TIMESCALE_TEST_FIXTURE_MINOR:-2.27}}"
require_docker="${REQUIRE_DOCKER:-0}"
evidence_file="${TIMESCALE_BRIDGE_SMOKE_EVIDENCE:-artifacts/timescale-bridge-smoke.tsv}"

if [[ -n "${TIMESCALE_BRIDGE_SMOKE_IMAGE:-}" ]]; then
  echo "TIMESCALE_BRIDGE_SMOKE_IMAGE is retired; use source-verified CITUS_TIMESCALE_TEST_FIXTURE_IMAGE" >&2
  exit 1
fi
case "${timescaledb_minor}" in
  2.27)
    expected_base_image="docker.io/timescale/timescaledb-ha:pg17-ts2.27@sha256:4f61167e11c7c95bedf96433c720d671a53aa29ad7f52b142b529a6d0e9f0b20"
    ;;
  2.28)
    expected_base_image="docker.io/timescale/timescaledb-ha:pg17-ts2.28@sha256:bc9e09875460aa69fb536362fef7c8e92c51ad6aab3d13f91a2487d3547dc71a"
    ;;
  *)
    echo "Timescale bridge smoke supports only the locked 2.27 and 2.28 lines" >&2
    exit 1
    ;;
esac
for file in "${fixture_builder}" "${fixture_contract}"; do
  if [[ ! -s "${file}" ]]; then
    echo "missing Timescale bridge smoke artifact: ${file}" >&2
    exit 1
  fi
done
if [[ ! -x "${fixture_builder}" ]]; then
  echo "Timescale bridge fixture builder is not executable: ${fixture_builder}" >&2
  exit 1
fi

python3 "${fixture_contract}"

if ! command -v docker >/dev/null 2>&1; then
  if [[ "${require_docker}" == "1" ]]; then
    echo "docker is required for Timescale bridge smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping Timescale bridge smoke"
  exit 0
fi

timescale_image="$("${fixture_builder}" --timescaledb-minor "${timescaledb_minor}")"
if [[ ! "${timescale_image}" =~ ^sha256:[0-9a-f]{64}$ ]]; then
  echo "Timescale bridge fixture builder did not return an immutable image ID" >&2
  exit 1
fi

container="ai-blaise-timescale-bridge-smoke-${RANDOM}-$$"
cleanup() {
  docker rm --force --volumes "${container}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run \
  --name "${container}" \
  --network none \
  -e POSTGRES_PASSWORD=postgres \
  -d "${timescale_image}" \
  postgres \
  -c shared_preload_libraries=timescaledb,citus \
  -c citus.cohabit_extensions=timescaledb >/dev/null

init_complete=0
for _ in $(seq 1 120); do
  container_logs="$(docker logs --tail 200 "${container}" 2>&1 || true)"
  if [[ "${container_logs}" == *"PostgreSQL init process complete"* ]]; then
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
CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS ai_blaise_citus;
DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'citus') THEN
    RAISE EXCEPTION 'negative bridge database unexpectedly contains Citus';
  END IF;
  IF (SELECT extversion FROM pg_extension WHERE extname = 'ai_blaise_citus')
      IS DISTINCT FROM '0.1.2' THEN
    RAISE EXCEPTION 'expected shipped ai_blaise_citus version 0.1.2';
  END IF;
END $$;

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

CREATE DATABASE timescale_bridge_positive;
SQL

docker exec -i "${container}" psql -U postgres -d timescale_bridge_positive -v ON_ERROR_STOP=1 <<'SQL'
CREATE EXTENSION IF NOT EXISTS citus;
CREATE EXTENSION IF NOT EXISTS timescaledb;
CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS ai_blaise_citus;
DO $$
BEGIN
  IF (SELECT extversion FROM pg_extension WHERE extname = 'ai_blaise_citus')
      IS DISTINCT FROM '0.1.2' THEN
    RAISE EXCEPTION 'expected shipped ai_blaise_citus version 0.1.2';
  END IF;
END $$;

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

INSERT INTO timescale_smoke_metrics(metric_time, tenant_id, value)
VALUES (clock_timestamp(), 7, 42.0);

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
    FROM pg_dist_partition
    WHERE logicalrelid = 'timescale_smoke_metrics'::regclass
  ) THEN
    RAISE EXCEPTION 'apply_distribute_hypertable did not register real Citus distribution metadata';
  END IF;

  IF (SELECT count(*) FROM pg_dist_shard
      WHERE logicalrelid = 'timescale_smoke_metrics'::regclass) <> 2 THEN
    RAISE EXCEPTION 'apply_distribute_hypertable did not create exactly two real Citus shards';
  END IF;

  IF (SELECT count(*) FROM timescale_smoke_metrics WHERE tenant_id = 7 AND value = 42.0) <> 1 THEN
    RAISE EXCEPTION 'real Timescale/Citus bridge row did not round trip';
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
base_image="$(docker image inspect --format '{{ index .Config.Labels "ai-blaise.citus.test-fixture.base-image" }}' "${image_id}")"
if [[ "${base_image}" != "${expected_base_image}" ]]; then
  echo "Timescale bridge fixture base label did not match the selected minor" >&2
  exit 1
fi
runtime_metadata="$(docker exec -i "${container}" psql -U postgres -d timescale_bridge_positive -AtX -v ON_ERROR_STOP=1 -F $'\t' <<'SQL'
SELECT
  current_setting('server_version_num', true),
  current_setting('server_version', true),
  (SELECT extversion FROM pg_extension WHERE extname = 'timescaledb'),
  (SELECT extversion FROM pg_extension WHERE extname = 'citus'),
  (SELECT extversion FROM pg_extension WHERE extname = 'ai_blaise_citus'),
  (SELECT count(DISTINCT feature_id)::text FROM companion_timescale_bridge_state WHERE feature_id IN ('TS1', 'TS2', 'TS3', 'TS4', 'TS5', 'TS12')),
  (SELECT count(*)::text FROM pg_dist_partition WHERE logicalrelid = 'timescale_smoke_metrics'::regclass),
  (SELECT count(*)::text FROM timescale_smoke_metrics WHERE tenant_id = 7 AND value = 42.0);
SQL
)"
IFS=$'\t' read -r server_version_num server_version timescaledb_extversion citus_extversion ai_blaise_citus_extversion bridge_features distributed_relations round_trip_rows <<<"${runtime_metadata}"
if [[ ! "${server_version_num:-}" =~ ^[0-9]+$ || "${bridge_features:-}" != "6" || "${distributed_relations:-}" != "1" || "${round_trip_rows:-}" != "1" ]]; then
  echo "Timescale bridge smoke metadata query returned unusable evidence: ${runtime_metadata}" >&2
  exit 1
fi
if [[ "${timescaledb_extversion}" != "${timescaledb_minor}" && "${timescaledb_extversion}" != "${timescaledb_minor}".* ]]; then
  echo "Timescale bridge smoke TimescaleDB minor mismatch" >&2
  exit 1
fi
git_sha="$(git -C "${repo_root}" rev-parse --short=12 HEAD)"
{
  printf 'git_sha\timage\timage_id\tbase_image\tserver_version_num\tserver_version\ttimescaledb_extversion\tcitus_extversion\tai_blaise_citus_extversion\treal_timescaledb\treal_citus_distribution\tstubbed_citus_distribution\tmissing_citus_fail_closed\tbridge_features\tdistributed_relations\tround_trip_rows\tpolicy_execution_scope\n'
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    "${git_sha}" \
    "${timescale_image}" \
    "${image_id}" \
    "${base_image}" \
    "${server_version_num}" \
    "${server_version}" \
    "${timescaledb_extversion}" \
    "${citus_extversion}" \
    "${ai_blaise_citus_extversion}" \
    "true" \
    "true" \
    "false" \
    "true" \
    "${bridge_features}" \
    "${distributed_relations}" \
    "${round_trip_rows}" \
    "entrypoints-and-catalog-state-only"
} >"${evidence_file}"

echo "ai_blaise_citus Timescale bridge smoke passed with ${timescale_image}; evidence at ${evidence_file}"
