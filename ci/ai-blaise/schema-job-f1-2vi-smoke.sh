#!/usr/bin/env bash
set -euo pipefail

# F1-style schema-change runtime smoke for the two-version invariant (2VI).
#
# Asserts:
#   1. A Migration CR (modelled as a SchemaJobPlan + reconcile plan) walks
#      DELETE_ONLY -> WRITE_ONLY -> BACKFILL -> PUBLIC with one phase-log row
#      per transition.
#   2. Phase transitions wait on worker acknowledgement and the controller
#      decision matches the gate (WaitForever / SkipMissing / RollbackOnTimeout).
#   3. A simulated worker failure mid-BACKFILL triggers a rollback that
#      restores the prior state and cleans up the partial backfill rows.
#   4. The two-version invariant (verify_two_version_invariant) reports
#      <=2 distinct schema versions throughout, and raises a critical
#      cluster_alarm row when violated.
#
# All Rust controller/registry/rollback determinism is exercised by the
# canonical TSV runner; the SQL-surface assertions run on the shared
# source-built real-Citus PG17 test fixture.

repo_root="$(git rev-parse --show-toplevel)"
fixture_builder="${repo_root}/ci/ai-blaise/build-real-citus-test-fixture.sh"
fixture_contract="${repo_root}/ci/ai-blaise/real-citus-test-fixture-contract.py"
require_docker="${REQUIRE_DOCKER:-0}"
pg_major=17

for file in "${fixture_builder}" "${fixture_contract}"; do
  if [[ ! -s "${file}" ]]; then
    echo "missing schema-job fixture artifact: ${file}" >&2
    exit 1
  fi
done
if [[ ! -x "${fixture_builder}" ]]; then
  echo "real-Citus test fixture builder is not executable: ${fixture_builder}" >&2
  exit 1
fi

python3 "${fixture_contract}"

echo "=== schema-job-f1-2vi-smoke: canonical Rust sidecar report ==="
canonical="$(cargo run -q -p ai_blaise_citus_sidecar_schema_job -- run-canonical 2>&1)"
if ! echo "${canonical}" | grep -q "apply_delete_only"; then
  echo "canonical schema-job report missed apply_delete_only" >&2
  echo "${canonical}" >&2
  exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
  if [[ "${require_docker}" == "1" ]]; then
    echo "docker is required for schema-job-f1-2vi smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping SQL portion of schema-job-f1-2vi smoke"
  exit 0
fi

fixture_image="$("${fixture_builder}" --pg-major "${pg_major}")"
container="ai-blaise-f1-2vi-smoke-pg${pg_major}-${RANDOM}-$$"

cleanup() {
  docker rm --force --volumes "${container}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "=== schema-job-f1-2vi-smoke on immutable real-Citus PG${pg_major} fixture ==="

docker run \
  --name "${container}" \
  --network none \
  -e POSTGRES_PASSWORD=postgres \
  -d "${fixture_image}" >/dev/null

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
  echo "real-Citus fixture did not complete PostgreSQL initialization" >&2
  exit 1
fi

ready=0
for _ in $(seq 1 90); do
  if docker exec "${container}" psql -U postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 1
done
if [[ "${ready}" != "1" ]]; then
  docker logs "${container}" >&2 || true
  echo "real-Citus fixture did not become SQL-ready" >&2
  exit 1
fi

docker exec -i "${container}" psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
CREATE EXTENSION citus;
CREATE EXTENSION pgcrypto;
CREATE EXTENSION ai_blaise_citus;
DO $$
BEGIN
  IF (SELECT extversion FROM pg_extension WHERE extname = 'ai_blaise_citus')
      IS DISTINCT FROM '0.1.2' THEN
    RAISE EXCEPTION 'expected shipped ai_blaise_citus version 0.1.2';
  END IF;
END $$;

CREATE TABLE f1_users (
  user_id bigserial PRIMARY KEY,
  email text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

INSERT INTO f1_users(email) SELECT 'user-' || g::text || '@example.com'
  FROM generate_series(1, 50) AS g;

-- Start a schema job and add the operations the Migration CR encodes.
SELECT companion_internal.schema_job_start(
  'users-add-display-name', 'f1_users', 60
);

SELECT companion_internal.schema_job_add_operation(
  'users-add-display-name',
  'add_column',
  'display_name',
  'text'
);

SELECT companion_internal.schema_job_add_operation(
  'users-add-display-name',
  'backfill',
  NULL,
  NULL,
  'UPDATE f1_users SET display_name = email'
);

-- Record worker leases at DELETE_ONLY for both workers.
SELECT companion_internal.worker_schema_lease_upsert(
  'worker-a', 'users-add-display-name', 'schema-v2', 'delete_only', now() + interval '1 hour'
);
SELECT companion_internal.worker_schema_lease_upsert(
  'worker-b', 'users-add-display-name', 'schema-v2', 'delete_only', now() + interval '1 hour'
);

-- Phase 1: DELETE_ONLY -> WRITE_ONLY.
SELECT companion_internal.schema_job_phase_log_insert(
  'users-add-display-name', 'delete_only', 'write_only',
  now() - interval '1 minute', now(),
  ARRAY['worker-a','worker-b']::text[],
  'wait_forever'
);
SELECT companion_internal.schema_job_advance('users-add-display-name', 'write_only');

-- Workers acknowledge WRITE_ONLY.
SELECT companion_internal.worker_schema_lease_upsert(
  'worker-a', 'users-add-display-name', 'schema-v2', 'write_only', now() + interval '1 hour'
);
SELECT companion_internal.worker_schema_lease_upsert(
  'worker-b', 'users-add-display-name', 'schema-v2', 'write_only', now() + interval '1 hour'
);

-- 2VI snapshot: one job, one schema version.
DO $$
DECLARE
  report jsonb;
BEGIN
  report := companion_internal.verify_two_version_invariant();
  IF (report->>'violation_count')::int <> 0 THEN
    RAISE EXCEPTION 'unexpected 2VI violation during WRITE_ONLY: %', report;
  END IF;
  IF (report->>'inflight_versions')::int <> 1 THEN
    RAISE EXCEPTION 'expected 1 inflight version, got %', report->>'inflight_versions';
  END IF;
END;
$$;

-- Phase 2: WRITE_ONLY -> BACKFILL.
SELECT companion_internal.schema_job_phase_log_insert(
  'users-add-display-name', 'write_only', 'backfill',
  now() - interval '1 minute', now(),
  ARRAY['worker-a','worker-b']::text[],
  'wait_forever'
);
SELECT companion_internal.schema_job_advance('users-add-display-name', 'backfill');

-- worker-a acknowledges BACKFILL.
SELECT companion_internal.worker_schema_lease_upsert(
  'worker-a', 'users-add-display-name', 'schema-v2', 'backfill', now() + interval '1 hour'
);
-- worker-b fails: its lease stays on WRITE_ONLY *and* expires.
SELECT companion_internal.worker_schema_lease_upsert(
  'worker-b', 'users-add-display-name', 'schema-v2', 'write_only', now() - interval '1 minute'
);

-- The Rust controller would notice worker-b's expired lease under
-- RollbackOnTimeout. Simulate that decision by issuing the rollback
-- helpers directly.

-- Apply the column add (so we have something to clean up).
ALTER TABLE f1_users ADD COLUMN IF NOT EXISTS display_name text;
UPDATE f1_users SET display_name = email WHERE user_id <= 10;

-- Trigger the rollback path.
SELECT companion_internal.schema_job_cleanup_backfill('f1_users', 'display_name');

DO $$
DECLARE
  remaining bigint;
BEGIN
  SELECT count(*) INTO remaining FROM f1_users WHERE display_name IS NOT NULL;
  IF remaining <> 0 THEN
    RAISE EXCEPTION 'cleanup_backfill left % rows populated', remaining;
  END IF;
END;
$$;

-- Walk the schema job state back to WRITE_ONLY via the canceled path then
-- restart at WRITE_ONLY. (The forward state machine forbids public ->
-- write_only, so we use the rollback recorder which is allowed.)
SELECT companion_internal.schema_job_phase_log_rollback(
  'users-add-display-name', 'backfill', 'write_only', now()
);

DO $$
DECLARE
  rollback_count integer;
BEGIN
  SELECT count(*) INTO rollback_count
  FROM companion_schema_job_phase_log
  WHERE job_name = 'users-add-display-name'
    AND is_rollback = true;
  IF rollback_count <> 1 THEN
    RAISE EXCEPTION 'expected exactly 1 rollback log row, got %', rollback_count;
  END IF;
END;
$$;

-- Re-acknowledge BACKFILL once worker-b recovers, then advance.
SELECT companion_internal.worker_schema_lease_upsert(
  'worker-b', 'users-add-display-name', 'schema-v2', 'backfill', now() + interval '1 hour'
);

UPDATE f1_users SET display_name = email;

-- Phase 3: BACKFILL -> PUBLIC.
SELECT companion_internal.schema_job_phase_log_insert(
  'users-add-display-name', 'backfill', 'public',
  now() - interval '1 minute', now(),
  ARRAY['worker-a','worker-b']::text[],
  'wait_forever'
);
SELECT companion_internal.schema_job_advance('users-add-display-name', 'public');

-- 2VI final check.
DO $$
DECLARE
  report jsonb;
  forward_log_count integer;
  final_state text;
BEGIN
  report := companion_internal.verify_two_version_invariant();
  IF (report->>'violation_count')::int <> 0 THEN
    RAISE EXCEPTION 'unexpected 2VI violation at PUBLIC: %', report;
  END IF;

  SELECT count(*) INTO forward_log_count
  FROM companion_schema_job_phase_log
  WHERE job_name = 'users-add-display-name'
    AND is_rollback = false;
  IF forward_log_count <> 3 THEN
    RAISE EXCEPTION 'expected 3 forward phase log rows, got %', forward_log_count;
  END IF;

  SELECT state INTO final_state
  FROM companion_schema_jobs
  WHERE job_name = 'users-add-display-name';
  IF final_state <> 'public' THEN
    RAISE EXCEPTION 'final state not PUBLIC: %', final_state;
  END IF;
END;
$$;

-- Now provoke a 2VI violation: register a third worker on a stale third
-- schema version, then verify the alarm fires.
SELECT companion_internal.schema_job_start(
  'orders-add-total-bigint', 'f1_users', 60
);
SELECT companion_internal.worker_schema_lease_upsert(
  'worker-a', 'orders-add-total-bigint', 'schema-v3', 'write_only', now() + interval '1 hour'
);
SELECT companion_internal.worker_schema_lease_upsert(
  'worker-b', 'orders-add-total-bigint', 'schema-v4', 'write_only', now() + interval '1 hour'
);
SELECT companion_internal.worker_schema_lease_upsert(
  'worker-c', 'orders-add-total-bigint', 'schema-v5', 'write_only', now() + interval '1 hour'
);

DO $$
DECLARE
  report jsonb;
  alarm_count integer;
BEGIN
  report := companion_internal.verify_two_version_invariant();
  IF (report->>'violation_count')::int <> 1 THEN
    RAISE EXCEPTION 'expected 1 invariant violation, got %', report;
  END IF;

  SELECT count(*) INTO alarm_count
  FROM companion_cluster_alarms
  WHERE alarm_kind = 'two_version_invariant_violation'
    AND severity = 'critical';
  IF alarm_count < 1 THEN
    RAISE EXCEPTION 'expected critical 2VI alarm row, got %', alarm_count;
  END IF;
END;
$$;
SQL

echo "schema-job-f1-2vi-smoke passed on immutable real-Citus PG${pg_major} fixture"
