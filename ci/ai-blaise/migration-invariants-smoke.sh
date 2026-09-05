#!/usr/bin/env bash
set -euo pipefail

# Focused migration durability smoke on the shared source-built real-Citus
# PG17 test fixture. This remains narrower than the full Citus matrix.

repo_root="$(git rev-parse --show-toplevel)"
fixture_builder="${repo_root}/ci/ai-blaise/build-real-citus-test-fixture.sh"
fixture_contract="${repo_root}/ci/ai-blaise/real-citus-test-fixture-contract.py"
require_docker="${REQUIRE_DOCKER:-0}"
pg_major=17

for file in "${fixture_builder}" "${fixture_contract}"; do
  if [[ ! -s "${file}" ]]; then
    echo "missing migration invariants fixture artifact: ${file}" >&2
    exit 1
  fi
done
if [[ ! -x "${fixture_builder}" ]]; then
  echo "real-Citus test fixture builder is not executable: ${fixture_builder}" >&2
  exit 1
fi

python3 "${fixture_contract}"

if ! command -v docker >/dev/null 2>&1; then
  if [[ "${require_docker}" == "1" ]]; then
    echo "docker is required for migration invariants smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping migration invariants smoke"
  exit 0
fi

fixture_image="$("${fixture_builder}" --pg-major "${pg_major}")"
container="ai-blaise-migration-invariants-pg${pg_major}-${RANDOM}-$$"

cleanup() {
  docker rm --force --volumes "${container}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "=== migration-invariants-smoke on immutable real-Citus PG${pg_major} fixture ==="

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

CREATE TABLE migration_invariant_orders (
  order_id bigserial PRIMARY KEY,
  tenant_id text NOT NULL,
  amount_cents integer NOT NULL,
  legacy_note text,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE migration_invariant_orders_conflict (
  order_id bigserial PRIMARY KEY,
  tenant_id text NOT NULL
);

INSERT INTO migration_invariant_orders(tenant_id, amount_cents, legacy_note)
SELECT 'tenant-' || (g % 4)::text, g * 100, 'note-' || g::text
FROM generate_series(1, 25) AS g;

DO $$
DECLARE
  migration_sql text;
  invariant_report jsonb;
BEGIN
  PERFORM companion_internal.migrate_start(
    'orders-durable-type-change',
    'migration_invariant_orders',
    500,
    100
  );

  BEGIN
    PERFORM companion_internal.migration_online_type_change(
      'amount_cents',
      'integer',
      'bigint',
      'amount_cents::bigint'
    );
    RAISE EXCEPTION 'destructive migration operation ran without an invariant';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'data invariant check is required before destructive migration operation' THEN
        RAISE;
      END IF;
  END;

  BEGIN
    PERFORM companion_internal.migration_register_invariant(
      'orders-durable-type-change',
      'mutating-check',
      'UPDATE migration_invariant_orders SET amount_cents = amount_cents'
    );
    RAISE EXCEPTION 'mutating invariant SQL was accepted';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'data invariant SQL must be a single read-only SELECT or WITH query' THEN
        RAISE;
      END IF;
  END;

  PERFORM companion_internal.migration_register_invariant(
    'orders-durable-type-change',
    'row-count-and-sum',
    'SELECT (count(*) = 25 AND sum(amount_cents) = 32500) AS passed, count(*) AS rows_checked, sum(amount_cents) AS amount_checksum FROM migration_invariant_orders'
  );

  migration_sql := companion_internal.migration_online_type_change(
    'amount_cents',
    'integer',
    'bigint',
    'amount_cents::bigint'
  );
  IF migration_sql NOT LIKE '%amount_cents__ai_blaise_new bigint%' THEN
    RAISE EXCEPTION 'online type-change did not render shadow-column DDL: %', migration_sql;
  END IF;

  PERFORM companion_internal.migration_online_type_change(
    'amount_cents',
    'integer',
    'bigint',
    'amount_cents::bigint'
  );

  IF (
    SELECT count(*)
    FROM companion_migration_operations
    WHERE migration_name = 'orders-durable-type-change'
      AND operation_type = 'online_type_change'
  ) <> 1 THEN
    RAISE EXCEPTION 'online type-change operation was not idempotent';
  END IF;

  invariant_report := companion_internal.migration_assert_invariants('orders-durable-type-change');
  IF (invariant_report->>'passed_checks')::int <> 1 THEN
    RAISE EXCEPTION 'expected one passing invariant, got %', invariant_report;
  END IF;

  PERFORM companion_internal.migrate_complete('orders-durable-type-change');
  PERFORM companion_internal.migrate_complete('orders-durable-type-change');

  IF NOT EXISTS (
    SELECT 1
    FROM companion_migration_runs
    WHERE migration_name = 'orders-durable-type-change'
      AND status = 'completed'
  ) THEN
    RAISE EXCEPTION 'completed migration was not visible';
  END IF;

  BEGIN
    PERFORM companion_internal.migrate_start(
      'orders-durable-type-change',
      'migration_invariant_orders_conflict',
      500,
      100
    );
    RAISE EXCEPTION 'conflicting migration re-entry was accepted';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'migration re-entry conflicts with existing run: orders-durable-type-change' THEN
        RAISE;
      END IF;
  END;
END;
$$;

CREATE TABLE schema_job_invariant_users (
  user_id bigserial PRIMARY KEY,
  email text NOT NULL
);

DO $$
DECLARE
  phase_completed_at timestamptz := now();
  phase_started_at timestamptz := now() - interval '1 second';
  first_log bigint;
  second_log bigint;
BEGIN
  PERFORM companion_internal.schema_job_start(
    'users-display-name-durable',
    'schema_job_invariant_users',
    60
  );

  PERFORM companion_internal.schema_job_add_operation(
    'users-display-name-durable',
    'add_column',
    'display_name',
    'text'
  );
  PERFORM companion_internal.schema_job_add_operation(
    'users-display-name-durable',
    'add_column',
    'display_name',
    'text'
  );

  IF (
    SELECT count(*)
    FROM companion_schema_job_operations
    WHERE job_name = 'users-display-name-durable'
      AND operation_type = 'add_column'
  ) <> 1 THEN
    RAISE EXCEPTION 'schema job operation re-entry was not idempotent';
  END IF;

  first_log := companion_internal.schema_job_phase_log_insert(
    'users-display-name-durable', 'delete_only', 'write_only',
    phase_started_at, phase_completed_at,
    ARRAY['worker-a']::text[],
    'wait_forever'
  );
  second_log := companion_internal.schema_job_phase_log_insert(
    'users-display-name-durable', 'delete_only', 'write_only',
    phase_started_at, phase_completed_at,
    ARRAY['worker-a']::text[],
    'wait_forever'
  );

  IF first_log <> second_log THEN
    RAISE EXCEPTION 'phase log re-entry inserted duplicate rows';
  END IF;

  PERFORM companion_internal.schema_job_advance('users-display-name-durable', 'write_only');
  PERFORM companion_internal.schema_job_advance('users-display-name-durable', 'write_only');

  BEGIN
    PERFORM companion_internal.schema_job_start(
      'users-display-name-durable',
      'schema_job_invariant_users',
      60
    );
    RAISE EXCEPTION 'schema job restart after phase advance was accepted';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'schema job cannot restart from state write_only' THEN
        RAISE;
      END IF;
  END;
END;
$$;
SQL

echo "migration-invariants-smoke passed"
