#!/usr/bin/env bash
set -euo pipefail

# FEATURE: O2 O3
repo_root="$(git rev-parse --show-toplevel)"
fixture_builder="${repo_root}/ci/ai-blaise/build-real-citus-test-fixture.sh"
fixture_contract="${repo_root}/ci/ai-blaise/real-citus-test-fixture-contract.py"
pg_major=17
require_docker="${REQUIRE_DOCKER:-0}"

if [[ -n "${OBSERVABILITY_REPLICATION_SMOKE_IMAGE:-}" ]]; then
  echo "OBSERVABILITY_REPLICATION_SMOKE_IMAGE is retired; use source-verified CITUS_TEST_FIXTURE_IMAGE" >&2
  exit 1
fi

for file in "${fixture_builder}" "${fixture_contract}"; do
  if [[ ! -s "${file}" ]]; then
    echo "missing observability replication smoke artifact: ${file}" >&2
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
    echo "docker is required for observability replication smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping observability replication smoke"
  exit 0
fi

fixture_image="$("${fixture_builder}" --pg-major "${pg_major}")"

network="ai-blaise-observability-replication-${RANDOM}-$$"
primary="${network}-primary"
standby="${network}-standby"

cleanup() {
  docker rm --force --volumes "${primary}" "${standby}" >/dev/null 2>&1 || true
  docker network rm "${network}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker network create "${network}" >/dev/null

docker run \
  --name "${primary}" \
  --network "${network}" \
  -e POSTGRES_PASSWORD=postgres \
  -d "${fixture_image}" \
  -c shared_preload_libraries=citus \
  -c wal_level=replica \
  -c max_wal_senders=5 \
  -c max_replication_slots=5 \
  -c listen_addresses='*' >/dev/null

primary_init_complete=0
for _ in $(seq 1 120); do
  if docker logs "${primary}" 2>&1 | grep -q "PostgreSQL init process complete"; then
    primary_init_complete=1
    break
  fi
  sleep 1
done

if [[ "${primary_init_complete}" != "1" ]]; then
  docker logs "${primary}" >&2 || true
  echo "primary postgres container did not finish init scripts" >&2
  exit 1
fi

primary_ready=0
for _ in $(seq 1 90); do
  if docker exec "${primary}" psql -U postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
    primary_ready=1
    break
  fi
  sleep 1
done

if [[ "${primary_ready}" != "1" ]]; then
  docker logs "${primary}" >&2 || true
  echo "primary postgres container did not become ready" >&2
  exit 1
fi

docker exec "${primary}" bash -lc \
  'echo "host replication replicator 0.0.0.0/0 scram-sha-256" >> "$PGDATA/pg_hba.conf"'
docker exec "${primary}" psql -U postgres -Atqc 'SELECT pg_reload_conf()' >/dev/null

docker exec -i "${primary}" psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
CREATE ROLE replicator WITH REPLICATION LOGIN PASSWORD 'replica';
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
CREATE TABLE observability_smoke(value integer);
INSERT INTO observability_smoke VALUES (1);

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1
    FROM companion_pg_stat_local_activity
    WHERE database_name = current_database()
      AND active_sessions >= 1
      AND idle_in_transaction_sessions >= 0
      AND waiting_sessions >= 0
  ) THEN
    RAISE EXCEPTION 'companion_pg_stat_local_activity did not report local activity';
  END IF;

  IF NOT EXISTS (
    SELECT 1
    FROM companion_pg_stat_distributed
    WHERE database_name = current_database()
      AND active_sessions >= 1
      AND idle_in_transaction_sessions >= 0
      AND waiting_sessions >= 0
  ) THEN
    RAISE EXCEPTION 'compatibility companion_pg_stat_distributed did not report local activity';
  END IF;
END $$;
SQL

docker run \
  --name "${standby}" \
  --network "${network}" \
  -e PGPASSWORD=replica \
  -e POSTGRES_PASSWORD=postgres \
  -d "${fixture_image}" \
  bash -lc "
set -euo pipefail
pg_basebackup -h \"${primary}\" -D \"\${PGDATA}\" -U replicator -Fp -Xs -R --checkpoint=fast
chown -R postgres:postgres \"\${PGDATA}\"
chmod 700 \"\${PGDATA}\"
exec gosu postgres \"\$(pg_config --bindir)/postgres\" -D \"\${PGDATA}\" \
  -c shared_preload_libraries=citus
" >/dev/null

standby_ready=0
for _ in $(seq 1 120); do
  if docker exec "${standby}" psql -U postgres -Atqc 'SELECT pg_is_in_recovery()' 2>/dev/null | grep -qx t; then
    standby_ready=1
    break
  fi
  sleep 1
done

if [[ "${standby_ready}" != "1" ]]; then
  docker logs "${standby}" >&2 || true
  echo "standby postgres container did not enter recovery" >&2
  exit 1
fi

replication_seen=0
for _ in $(seq 1 60); do
  rows="$(
    docker exec "${primary}" psql -U postgres -Atqc \
      "SELECT count(*) FROM companion_pg_dist_replication_lag WHERE state = 'streaming';" \
      2>/dev/null || true
  )"
  if [[ "${rows:-0}" =~ ^[1-9][0-9]*$ ]]; then
    replication_seen=1
    break
  fi
  sleep 1
done

if [[ "${replication_seen}" != "1" ]]; then
  docker exec "${primary}" psql -U postgres -v ON_ERROR_STOP=1 \
    -c "SELECT application_name, client_addr, state, replay_lsn FROM pg_stat_replication;" >&2 || true
  echo "companion_pg_dist_replication_lag did not report streaming replication" >&2
  exit 1
fi

docker exec -i "${primary}" psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1
    FROM companion_pg_dist_replication_lag
    WHERE state = 'streaming'
      AND lag_bytes >= 0
  ) THEN
    RAISE EXCEPTION 'companion_pg_dist_replication_lag did not expose nonnegative lag for streaming standby';
  END IF;
END $$;
SQL

docker exec "${primary}" psql -X -U postgres -v ON_ERROR_STOP=1 \
  -c 'INSERT INTO observability_smoke VALUES (2);' >/dev/null
replay_seen=0
for _ in $(seq 1 60); do
  rows="$(
    docker exec "${standby}" psql -X -U postgres -Atqv ON_ERROR_STOP=1 \
      -c 'SELECT count(*) FROM observability_smoke WHERE value = 2;' \
      2>/dev/null || true
  )"
  if [[ "${rows}" == "1" ]]; then
    replay_seen=1
    break
  fi
  sleep 1
done
if [[ "${replay_seen}" != "1" ]]; then
  echo "streaming standby did not replay the post-backup insert" >&2
  exit 1
fi

echo "ai_blaise_citus observability replication smoke passed with ${fixture_image}"
