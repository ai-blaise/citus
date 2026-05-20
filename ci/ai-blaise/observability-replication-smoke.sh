#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
extension_dir="${repo_root}/images/citus-pg-overlay/extensions"
control_file="${extension_dir}/ai_blaise_citus.control"
sql_file="${extension_dir}/ai_blaise_citus--0.1.0.sql"
postgres_image="${OBSERVABILITY_REPLICATION_SMOKE_IMAGE:-postgres:17}"
require_docker="${REQUIRE_DOCKER:-0}"

for file in "${control_file}" "${sql_file}"; do
  if [[ ! -s "${file}" ]]; then
    echo "missing observability replication smoke artifact: ${file}" >&2
    exit 1
  fi
done

if ! command -v docker >/dev/null 2>&1; then
  if [[ "${require_docker}" == "1" ]]; then
    echo "docker is required for observability replication smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping observability replication smoke"
  exit 0
fi

network="ai-blaise-observability-replication-${RANDOM}-$$"
primary="${network}-primary"
standby="${network}-standby"

cleanup() {
  docker rm -f "${primary}" "${standby}" >/dev/null 2>&1 || true
  docker network rm "${network}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker network create "${network}" >/dev/null

docker run \
  --name "${primary}" \
  --network "${network}" \
  -e POSTGRES_PASSWORD=postgres \
  -v "${control_file}:/usr/share/postgresql/17/extension/ai_blaise_citus.control:ro" \
  -v "${sql_file}:/usr/share/postgresql/17/extension/ai_blaise_citus--0.1.0.sql:ro" \
  -d "${postgres_image}" \
  -c wal_level=replica \
  -c max_wal_senders=5 \
  -c max_replication_slots=5 \
  -c listen_addresses='*' >/dev/null

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
CREATE EXTENSION ai_blaise_citus;
CREATE TABLE observability_smoke(value integer);
INSERT INTO observability_smoke VALUES (1);

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1
    FROM companion_pg_stat_distributed
    WHERE database_name = current_database()
      AND active_sessions >= 1
      AND idle_in_transaction_sessions >= 0
      AND waiting_sessions >= 0
  ) THEN
    RAISE EXCEPTION 'companion_pg_stat_distributed did not report local activity';
  END IF;
END $$;
SQL

docker run \
  --name "${standby}" \
  --network "${network}" \
  -e PGPASSWORD=replica \
  -e POSTGRES_PASSWORD=postgres \
  -d "${postgres_image}" \
  bash -lc "
set -euo pipefail
rm -rf \"\${PGDATA}\"/*
pg_basebackup -h \"${primary}\" -D \"\${PGDATA}\" -U replicator -Fp -Xs -R
chown -R postgres:postgres \"\${PGDATA}\"
chmod 700 \"\${PGDATA}\"
exec gosu postgres /usr/lib/postgresql/17/bin/postgres -D \"\${PGDATA}\"
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

echo "ai_blaise_citus observability replication smoke passed with ${postgres_image}"
