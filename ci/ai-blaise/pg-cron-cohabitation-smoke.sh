#!/usr/bin/env bash
set -euo pipefail

# FEATURE: Bundle1 T2 TS19 TS20
# Live PG17 smoke for the pg_cron cohabitation boundary and the T2 Citus patch
# runtime boundary. This proves startup parsing, pg_cron package availability,
# real Citus + pg_cron extension load, SQL-visible cohabit detection, the TS19
# in-shmem clock-reservation flag, real scheduled pg_cron worker execution,
# fail-closed mismatch handling, live placement-generation UDF advancement under
# placement mutations, and GUC_REPORT ParameterStatus emission for citus.* GUCs.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

require_docker="${REQUIRE_DOCKER:-0}"
image_tag="${PG_CRON_COHABITATION_IMAGE:-ai-blaise-citus-pg-cron-cohabitation:local}"
base_image="${PG_CRON_COHABITATION_BASE_IMAGE:-postgres:17-bookworm}"
evidence_file="${PG_CRON_COHABITATION_EVIDENCE_FILE:-artifacts/pg-cron-cohabitation-evidence.tsv}"
make_jobs="${MAKE_JOBS:-2}"
skip_build="${PG_CRON_COHABITATION_SKIP_BUILD:-0}"

if ! command -v docker >/dev/null 2>&1; then
  if [[ "${require_docker}" == "1" ]]; then
    echo "docker is required for pg_cron cohabitation smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping pg_cron cohabitation smoke"
  exit 0
fi

positive_container=""
negative_container=""
guc_container=""
cleanup() {
  if [[ -n "${positive_container}" ]]; then
    docker rm -f "${positive_container}" >/dev/null 2>&1 || true
  fi
  if [[ -n "${negative_container}" ]]; then
    docker rm -f "${negative_container}" >/dev/null 2>&1 || true
  fi
  if [[ -n "${guc_container}" ]]; then
    docker rm -f "${guc_container}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

wait_for_postgres() {
  local container="$1"
  local init_complete=0
  local _
  for _ in $(seq 1 180); do
    if docker logs "${container}" 2>&1 | grep -q "PostgreSQL init process complete"; then
      init_complete=1
      break
    fi
    sleep 1
  done
  if [[ "${init_complete}" != "1" ]]; then
    docker logs "${container}" >&2 || true
    echo "postgres container did not finish init scripts: ${container}" >&2
    exit 1
  fi

  local ready=0
  for _ in $(seq 1 90); do
    if docker exec "${container}" psql -U postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
      ready=1
      break
    fi
    sleep 1
  done
  if [[ "${ready}" != "1" ]]; then
    docker logs "${container}" >&2 || true
    echo "postgres container did not become ready: ${container}" >&2
    exit 1
  fi
}

run_sql() {
  local container="$1"
  docker exec -i "${container}" psql -U postgres -v ON_ERROR_STOP=1
}

wait_for_cron_clock_run() {
  local container="$1"
  local observed=0
  local rows
  local _
  for _ in $(seq 1 90); do
    rows="$(docker exec "${container}" psql -U postgres -Atqc "SELECT count(*) FROM public.ai_blaise_pg_cron_cohabit_runs WHERE clock_reserved IS TRUE")"
    if [[ "${rows}" =~ ^[1-9][0-9]*$ ]]; then
      observed=1
      break
    fi
    sleep 2
  done
  if [[ "${observed}" != "1" ]]; then
    docker exec "${container}" psql -U postgres -Atqc "SELECT jobid, job_pid, status, coalesce(return_message, '') FROM cron.job_run_details ORDER BY start_time DESC LIMIT 5" >&2 || true
    docker logs "${container}" >&2 || true
    echo "pg_cron scheduled job did not observe a reserved Citus clock tick" >&2
    exit 1
  fi
}

mkdir -p "$(dirname "${evidence_file}")"

if [[ "${skip_build}" != "1" ]]; then
  docker build \
    -f images/citus-pg-cron-cohabitation/Dockerfile \
    --build-arg BASE_IMAGE="${base_image}" \
    --build-arg MAKE_JOBS="${make_jobs}" \
    -t "${image_tag}" \
    .
fi

positive_container="ai-blaise-pg-cron-cohabit-${RANDOM}-$$"
docker run \
  --name "${positive_container}" \
  -e POSTGRES_PASSWORD=postgres \
  -d "${image_tag}" \
  postgres \
    -c shared_preload_libraries=pg_cron,citus \
    -c citus.cohabit_extensions=pg_cron \
    -c cron.database_name=postgres >/dev/null
wait_for_postgres "${positive_container}"

run_sql "${positive_container}" <<'SQL'
CREATE EXTENSION IF NOT EXISTS citus;
CREATE EXTENSION IF NOT EXISTS pg_cron;
CREATE EXTENSION IF NOT EXISTS ai_blaise_citus;
SELECT companion_internal.assert_shared_preload_libraries(
  string_to_array(current_setting('shared_preload_libraries', true), ','),
  ARRAY['pg_cron']
);
SELECT companion_internal.assert_cohabit_extension_ready('pg_cron');
DO $$
BEGIN
  IF NOT pg_catalog.citus_cohabit_clock_tick_reserved() THEN
    RAISE EXCEPTION 'Citus did not reserve the pg_cron cohabit clock tick';
  END IF;
  IF pg_catalog.citus_cohabit_extension_role('pg_cron') <> 'clock-worker' THEN
    RAISE EXCEPTION 'Citus did not classify pg_cron as a clock-worker cohabitant';
  END IF;
  IF NOT pg_catalog.citus_cohabit_extension_configured('pg_cron') THEN
    RAISE EXCEPTION 'Citus did not report pg_cron as configured';
  END IF;
  IF pg_catalog.citus_cohabit_extension_role('timescaledb') <> 'trusted-hook' THEN
    RAISE EXCEPTION 'Citus did not classify timescaledb as trusted-hook';
  END IF;
  IF pg_catalog.citus_cohabit_extension_role('pg_partman') <> 'partition-manager' THEN
    RAISE EXCEPTION 'Citus did not classify pg_partman as partition-manager';
  END IF;
  IF pg_catalog.citus_cohabit_extension_role('unknown_extension') <> 'unsupported' THEN
    RAISE EXCEPTION 'Citus did not classify unknown extensions as unsupported';
  END IF;
  IF pg_catalog.citus_cohabit_extension_configured('unknown_extension') THEN
    RAISE EXCEPTION 'Citus reported an unsupported cohabitant as configured';
  END IF;
END;
$$;
SET citus.shard_count TO 4;
CREATE TABLE public.ai_blaise_placement_generation_evidence(
  key text PRIMARY KEY,
  value bigint NOT NULL
);
INSERT INTO public.ai_blaise_placement_generation_evidence(key, value)
SELECT 'initial', pg_catalog.citus_placement_generation();
CREATE TABLE public.ai_blaise_placement_probe_a(id int, tenant_id int);
SELECT create_distributed_table('public.ai_blaise_placement_probe_a', 'tenant_id');
INSERT INTO public.ai_blaise_placement_generation_evidence(key, value)
SELECT 'after_first_distribution', pg_catalog.citus_placement_generation();
CREATE TABLE public.ai_blaise_placement_probe_b(id int, tenant_id int);
SELECT create_distributed_table('public.ai_blaise_placement_probe_b', 'tenant_id');
INSERT INTO public.ai_blaise_placement_generation_evidence(key, value)
SELECT 'after_second_distribution', pg_catalog.citus_placement_generation();
INSERT INTO public.ai_blaise_placement_generation_evidence(key, value)
SELECT 'placements', count(*)::bigint FROM pg_dist_placement;
DO $$
DECLARE
  initial_generation bigint;
  after_first_generation bigint;
  after_second_generation bigint;
  placement_count bigint;
BEGIN
  SELECT value INTO initial_generation
  FROM public.ai_blaise_placement_generation_evidence
  WHERE key = 'initial';
  SELECT value INTO after_first_generation
  FROM public.ai_blaise_placement_generation_evidence
  WHERE key = 'after_first_distribution';
  SELECT value INTO after_second_generation
  FROM public.ai_blaise_placement_generation_evidence
  WHERE key = 'after_second_distribution';
  SELECT value INTO placement_count
  FROM public.ai_blaise_placement_generation_evidence
  WHERE key = 'placements';
  IF after_first_generation <= initial_generation THEN
    RAISE EXCEPTION 'placement generation did not advance after first distributed table: % -> %',
      initial_generation, after_first_generation;
  END IF;
  IF after_second_generation <= after_first_generation THEN
    RAISE EXCEPTION 'placement generation did not advance after second distributed table: % -> %',
      after_first_generation, after_second_generation;
  END IF;
  IF placement_count <= 0 THEN
    RAISE EXCEPTION 'Citus did not create placement metadata for distributed tables';
  END IF;
END;
$$;
CREATE TABLE public.ai_blaise_pg_cron_cohabit_runs(
  run_id bigserial PRIMARY KEY,
  clock_reserved boolean NOT NULL,
  node_clock pg_catalog.cluster_clock NOT NULL,
  ran_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
SELECT cron.schedule(
  'ai_blaise_pg_cron_cohabit_smoke',
  '* * * * *',
  $$INSERT INTO public.ai_blaise_pg_cron_cohabit_runs(clock_reserved, node_clock)
    SELECT pg_catalog.citus_cohabit_clock_tick_reserved(), pg_catalog.citus_get_node_clock()$$
);
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1
    FROM cron.job
    WHERE jobname = 'ai_blaise_pg_cron_cohabit_smoke'
  ) THEN
    RAISE EXCEPTION 'pg_cron smoke job was not registered';
  END IF;
END;
$$;
SQL

wait_for_cron_clock_run "${positive_container}"

{
  printf 'key\tvalue\n'
  printf 'git_sha\t%s\n' "$(git rev-parse HEAD)"
  printf 'image\t%s\n' "${image_tag}"
  printf 'base_image\t%s\n' "${base_image}"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'server_version_num' || E'\t' || current_setting('server_version_num')"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'shared_preload_libraries' || E'\t' || current_setting('shared_preload_libraries')"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'citus_cohabit_extensions' || E'\t' || current_setting('citus.cohabit_extensions')"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'pg_cron_extversion' || E'\t' || extversion FROM pg_extension WHERE extname = 'pg_cron'"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'citus_extversion' || E'\t' || extversion FROM pg_extension WHERE extname = 'citus'"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'pg_cron_detection' || E'\t' || role || ':' || ready || ':' || coalesce(reason, 'ok') FROM companion_internal.cohabit_extension_detection_report() WHERE extension_name = 'pg_cron'"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'citus_cohabit_pg_cron_role' || E'\t' || pg_catalog.citus_cohabit_extension_role('pg_cron')"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'citus_cohabit_pg_cron_configured' || E'\t' || pg_catalog.citus_cohabit_extension_configured('pg_cron')"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'citus_cohabit_timescaledb_role' || E'\t' || pg_catalog.citus_cohabit_extension_role('timescaledb')"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'citus_cohabit_pg_partman_role' || E'\t' || pg_catalog.citus_cohabit_extension_role('pg_partman')"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'citus_cohabit_unknown_role' || E'\t' || pg_catalog.citus_cohabit_extension_role('unknown_extension')"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'placement_generation_initial' || E'\t' || value FROM public.ai_blaise_placement_generation_evidence WHERE key = 'initial'"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'placement_generation_after_first_distribution' || E'\t' || value FROM public.ai_blaise_placement_generation_evidence WHERE key = 'after_first_distribution'"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'placement_generation_after_second_distribution' || E'\t' || value FROM public.ai_blaise_placement_generation_evidence WHERE key = 'after_second_distribution'"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'placement_generation_placements' || E'\t' || value FROM public.ai_blaise_placement_generation_evidence WHERE key = 'placements'"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'clock_tick_reserved' || E'\t' || pg_catalog.citus_cohabit_clock_tick_reserved()"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'cron_job_registered' || E'\t' || count(*) FROM cron.job WHERE jobname = 'ai_blaise_pg_cron_cohabit_smoke'"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'cron_clock_reserved_runs' || E'\t' || count(*) FROM public.ai_blaise_pg_cron_cohabit_runs WHERE clock_reserved IS TRUE"
  docker exec "${positive_container}" psql -U postgres -Atqc \
    "SELECT 'cron_node_clock_samples' || E'\t' || count(*) FROM public.ai_blaise_pg_cron_cohabit_runs WHERE node_clock IS NOT NULL"
} >"${evidence_file}"

grep -Fq $'pg_cron_detection\tclock-worker:true:ok' "${evidence_file}"
grep -Fq $'citus_cohabit_pg_cron_role\tclock-worker' "${evidence_file}"
grep -Fq $'citus_cohabit_pg_cron_configured\tt' "${evidence_file}"
grep -Fq $'citus_cohabit_timescaledb_role\ttrusted-hook' "${evidence_file}"
grep -Fq $'citus_cohabit_pg_partman_role\tpartition-manager' "${evidence_file}"
grep -Fq $'citus_cohabit_unknown_role\tunsupported' "${evidence_file}"
grep -Fq $'clock_tick_reserved\tt' "${evidence_file}"
grep -Eq $'^placement_generation_initial\t[0-9]+$' "${evidence_file}"
grep -Eq $'^placement_generation_after_first_distribution\t[0-9]+$' "${evidence_file}"
grep -Eq $'^placement_generation_after_second_distribution\t[0-9]+$' "${evidence_file}"
grep -Eq $'^placement_generation_placements\t[1-9][0-9]*$' "${evidence_file}"
placement_generation_initial="$(awk -F '\t' '$1 == "placement_generation_initial" {print $2}' "${evidence_file}")"
placement_generation_after_first="$(awk -F '\t' '$1 == "placement_generation_after_first_distribution" {print $2}' "${evidence_file}")"
placement_generation_after_second="$(awk -F '\t' '$1 == "placement_generation_after_second_distribution" {print $2}' "${evidence_file}")"
placement_generation_placements="$(awk -F '\t' '$1 == "placement_generation_placements" {print $2}' "${evidence_file}")"
if (( placement_generation_after_first <= placement_generation_initial )); then
  echo "placement generation did not advance after first distributed table" >&2
  exit 1
fi
if (( placement_generation_after_second <= placement_generation_after_first )); then
  echo "placement generation did not advance after second distributed table" >&2
  exit 1
fi
if (( placement_generation_placements <= 0 )); then
  echo "placement generation smoke did not create Citus placements" >&2
  exit 1
fi
grep -Fq $'cron_job_registered\t1' "${evidence_file}"
grep -Eq $'^cron_clock_reserved_runs\t[1-9][0-9]*$' "${evidence_file}"
grep -Eq $'^cron_node_clock_samples\t[1-9][0-9]*$' "${evidence_file}"

negative_container="ai-blaise-pg-cron-cohabit-negative-${RANDOM}-$$"
docker run \
  --name "${negative_container}" \
  -e POSTGRES_PASSWORD=postgres \
  -d "${image_tag}" \
  postgres \
    -c shared_preload_libraries=pg_cron,citus \
    -c cron.database_name=postgres >/dev/null
wait_for_postgres "${negative_container}"

run_sql "${negative_container}" <<'SQL'
CREATE EXTENSION IF NOT EXISTS citus;
CREATE EXTENSION IF NOT EXISTS pg_cron;
CREATE EXTENSION IF NOT EXISTS ai_blaise_citus;
DO $$
BEGIN
  IF pg_catalog.citus_cohabit_clock_tick_reserved() THEN
    RAISE EXCEPTION 'Citus reserved the pg_cron cohabit clock tick without allowlist';
  END IF;
  IF pg_catalog.citus_cohabit_extension_role('pg_cron') <> 'clock-worker' THEN
    RAISE EXCEPTION 'Citus lost the pg_cron role classifier without allowlist';
  END IF;
  IF pg_catalog.citus_cohabit_extension_configured('pg_cron') THEN
    RAISE EXCEPTION 'Citus reported pg_cron as configured without allowlist';
  END IF;
END;
$$;
SQL

docker exec "${negative_container}" psql -U postgres -Atqc \
  "SELECT 'negative_pg_cron_citus_role' || E'\t' || pg_catalog.citus_cohabit_extension_role('pg_cron')" >>"${evidence_file}"
docker exec "${negative_container}" psql -U postgres -Atqc \
  "SELECT 'negative_pg_cron_citus_configured' || E'\t' || pg_catalog.citus_cohabit_extension_configured('pg_cron')" >>"${evidence_file}"
printf 'negative_clock_tick_reserved\tfalse\n' >>"${evidence_file}"
grep -Fq $'negative_pg_cron_citus_role\tclock-worker' "${evidence_file}"
grep -Fq $'negative_pg_cron_citus_configured\tf' "${evidence_file}"

if docker exec "${negative_container}" psql -U postgres -v ON_ERROR_STOP=1 -c \
  "SELECT companion_internal.assert_cohabit_extension_ready('pg_cron');" >/tmp/pg-cron-negative-$$.out 2>&1; then
  cat /tmp/pg-cron-negative-$$.out >&2 || true
  rm -f /tmp/pg-cron-negative-$$.out
  echo "pg_cron cohabit detector did not fail closed without citus.cohabit_extensions" >&2
  exit 1
fi
if ! grep -Fq "missing-citus-cohabit-extensions" /tmp/pg-cron-negative-$$.out; then
  cat /tmp/pg-cron-negative-$$.out >&2 || true
  rm -f /tmp/pg-cron-negative-$$.out
  echo "pg_cron negative smoke failed for the wrong reason" >&2
  exit 1
fi
rm -f /tmp/pg-cron-negative-$$.out
printf 'negative_missing_cohabit_guc\tpass\n' >>"${evidence_file}"


guc_container="ai-blaise-guc-report-${RANDOM}-$$"
docker run \
  --name "${guc_container}" \
  -e POSTGRES_HOST_AUTH_METHOD=trust \
  -p 127.0.0.1::5432 \
  -d "${image_tag}" \
  postgres \
    -c shared_preload_libraries=citus >/dev/null
wait_for_postgres "${guc_container}"

run_sql "${guc_container}" <<'SQL'
CREATE EXTENSION IF NOT EXISTS citus;
SQL

guc_port="$(docker port "${guc_container}" 5432/tcp | sed -n 's/.*://p' | head -1)"
if [[ -z "${guc_port}" ]]; then
  docker logs "${guc_container}" >&2 || true
  echo "could not determine host port for GUC_REPORT smoke" >&2
  exit 1
fi
python3 - "${guc_port}" >>"${evidence_file}" <<'PYRAW'
import socket
import struct
import sys

port = int(sys.argv[1])

def cstring(value: str) -> bytes:
    return value.encode("utf-8") + b"\x00"


def recv_exact(sock: socket.socket, size: int) -> bytes:
    data = b""
    while len(data) < size:
        chunk = sock.recv(size - len(data))
        if not chunk:
            raise RuntimeError("connection closed while reading PostgreSQL message")
        data += chunk
    return data


def read_message(sock: socket.socket):
    msg_type = recv_exact(sock, 1)
    length = struct.unpack("!I", recv_exact(sock, 4))[0]
    payload = recv_exact(sock, length - 4)
    return msg_type, payload


def parse_error(payload: bytes) -> str:
    fields = {}
    for field in payload.split(b"\x00"):
        if len(field) >= 2:
            fields[field[:1].decode("ascii", errors="ignore")] = field[1:].decode("utf-8", errors="replace")
    return fields.get("M", repr(payload))

startup_parameters = {
    "user": "postgres",
    "database": "postgres",
    "application_name": "ai-blaise-guc-report-smoke",
}
startup_body = struct.pack("!I", 196608) + b"".join(
    cstring(key) + cstring(value) for key, value in startup_parameters.items()
) + b"\x00"

with socket.create_connection(("127.0.0.1", port), timeout=10) as sock:
    sock.sendall(struct.pack("!I", len(startup_body) + 4) + startup_body)
    while True:
        msg_type, payload = read_message(sock)
        if msg_type == b"R":
            auth_code = struct.unpack("!I", payload[:4])[0]
            if auth_code != 0:
                raise RuntimeError(f"unexpected PostgreSQL auth request {auth_code}")
        elif msg_type == b"E":
            raise RuntimeError(parse_error(payload))
        elif msg_type == b"Z":
            break

    query = b"SET citus.shard_count TO 7;\x00"
    sock.sendall(b"Q" + struct.pack("!I", len(query) + 4) + query)
    reported = None
    while True:
        msg_type, payload = read_message(sock)
        if msg_type == b"S":
            parts = payload.split(b"\x00")
            if len(parts) >= 2:
                key = parts[0].decode("utf-8", errors="replace")
                value = parts[1].decode("utf-8", errors="replace")
                if key == "citus.shard_count":
                    reported = value
        elif msg_type == b"E":
            raise RuntimeError(parse_error(payload))
        elif msg_type == b"Z":
            break

    sock.sendall(b"X" + struct.pack("!I", 4))

if reported != "7":
    raise RuntimeError(f"expected citus.shard_count ParameterStatus=7, got {reported!r}")
print("citus_shard_count_parameter_status\t7")
PYRAW
grep -Fq $'citus_shard_count_parameter_status\t7' "${evidence_file}"

cat "${evidence_file}"
echo "pg_cron cohabitation smoke passed"
