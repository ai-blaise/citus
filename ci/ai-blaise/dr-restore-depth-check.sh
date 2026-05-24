#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if [[ -d "${HOME}/.cargo/bin" ]]; then
  export PATH="${HOME}/.cargo/bin:${PATH}"
fi

require_docker="${REQUIRE_DOCKER:-0}"
run_docker_smoke="${DR_RESTORE_DEPTH_DOCKER:-${require_docker}}"

assert_contains() {
  local file="$1"
  local needle="$2"

  if ! grep -Fq "${needle}" "${file}"; then
    echo "${file} is missing required DR restore-depth evidence: ${needle}" >&2
    exit 1
  fi
}

assert_output_line() {
  local description="$1"
  local expected="$2"
  shift 2

  local output
  if ! output="$("$@")"; then
    echo "${description} command failed" >&2
    exit 1
  fi
  if ! printf '%s\n' "${output}" | grep -Fqx "${expected}"; then
    echo "${description} did not emit expected evidence row" >&2
    echo "expected: ${expected}" >&2
    echo "actual output:" >&2
    printf '%s\n' "${output}" >&2
    exit 1
  fi
}

wait_for_psql() {
  local container="$1"
  local query="${2:-SELECT 1}"
  local attempts="${3:-90}"

  local _
  for _ in $(seq 1 "${attempts}"); do
    if docker exec "${container}" psql -U postgres -Atqc "${query}" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  docker logs "${container}" >&2 || true
  echo "postgres container ${container} did not become queryable" >&2
  return 1
}

wait_for_init_complete() {
  local container="$1"
  local attempts="${2:-120}"

  local _
  for _ in $(seq 1 "${attempts}"); do
    if docker logs "${container}" 2>&1 | grep -q "PostgreSQL init process complete"; then
      return 0
    fi
    sleep 1
  done
  docker logs "${container}" >&2 || true
  echo "postgres container ${container} did not finish init scripts" >&2
  return 1
}

wait_for_archived_wal() {
  local archive_dir="$1"
  local minimum_segments="$2"
  local attempts="${3:-90}"

  local _ segment_count
  for _ in $(seq 1 "${attempts}"); do
    segment_count="$(find "${archive_dir}" -type f | wc -l | tr -d ' ')"
    if [[ "${segment_count}" -ge "${minimum_segments}" ]]; then
      return 0
    fi
    sleep 1
  done
  echo "WAL archive did not reach ${minimum_segments} segment(s); found ${segment_count:-0}" >&2
  find "${archive_dir}" -maxdepth 1 -type f -print >&2 || true
  return 1
}

run_postgres_pitr_smoke() {
  local postgres_image="${DR_RESTORE_DEPTH_POSTGRES_IMAGE:-postgres:17}"
  local workdir
  workdir="$(mktemp -d -t ai-blaise-dr-restore-depth.XXXXXX)"
  local archive_dir="${workdir}/archive"
  mkdir -p "${archive_dir}"
  chmod 0777 "${workdir}" "${archive_dir}"

  local primary="ai-blaise-dr-restore-primary-${RANDOM}-$$"
  local restored="ai-blaise-dr-restore-restored-${RANDOM}-$$"
  DR_RESTORE_DEPTH_SMOKE_IMAGE="${postgres_image}"
  DR_RESTORE_DEPTH_SMOKE_WORKDIR="${workdir}"
  DR_RESTORE_DEPTH_SMOKE_PRIMARY="${primary}"
  DR_RESTORE_DEPTH_SMOKE_RESTORED="${restored}"

  cleanup_smoke() {
    docker rm -f "${DR_RESTORE_DEPTH_SMOKE_PRIMARY:-}" \
      "${DR_RESTORE_DEPTH_SMOKE_RESTORED:-}" >/dev/null 2>&1 || true
    if [[ -n "${DR_RESTORE_DEPTH_SMOKE_WORKDIR:-}" ]]; then
      docker run --rm --entrypoint bash \
        -v "${DR_RESTORE_DEPTH_SMOKE_WORKDIR}:/dr" \
        "${DR_RESTORE_DEPTH_SMOKE_IMAGE:-postgres:17}" \
        -lc 'chmod -R 0777 /dr' >/dev/null 2>&1 || true
      rm -rf "${DR_RESTORE_DEPTH_SMOKE_WORKDIR}"
    fi
  }
  trap cleanup_smoke EXIT

  docker run \
    --name "${primary}" \
    -e POSTGRES_PASSWORD=postgres \
    -v "${workdir}:/dr" \
    -d "${postgres_image}" \
    -c wal_level=replica \
    -c archive_mode=on \
    -c "archive_command=test ! -f /dr/archive/%f && cp %p /dr/archive/%f || test -f /dr/archive/%f" \
    -c max_wal_senders=5 \
    -c listen_addresses='*' >/dev/null

  wait_for_init_complete "${primary}"
  wait_for_psql "${primary}"

  docker exec -i "${primary}" psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
CREATE TABLE dr_restore_depth_smoke (
  id bigserial PRIMARY KEY,
  label text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
INSERT INTO dr_restore_depth_smoke(label) VALUES ('base-before-backup');
SELECT pg_switch_wal();
SQL

  wait_for_archived_wal "${archive_dir}" 1

  docker exec -u postgres "${primary}" bash -lc \
    'rm -rf /dr/base && pg_basebackup -U postgres -D /dr/base -Fp -Xs'

  docker exec -i "${primary}" psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
INSERT INTO dr_restore_depth_smoke(label) VALUES ('before-target');
SQL

  sleep 2
  local target_time
  target_time="$(
    docker exec "${primary}" psql -U postgres -Atqc \
      "SELECT to_char(clock_timestamp(), 'YYYY-MM-DD HH24:MI:SS.USOF')"
  )"
  sleep 2

  docker exec -i "${primary}" psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
INSERT INTO dr_restore_depth_smoke(label) VALUES ('after-target');
SELECT pg_switch_wal();
CHECKPOINT;
SQL

  wait_for_archived_wal "${archive_dir}" 2
  docker stop -t 60 "${primary}" >/dev/null

  docker run --rm \
    --entrypoint bash \
    -v "${workdir}:/dr" \
    "${postgres_image}" \
    -lc "
set -euo pipefail
rm -rf /dr/restore
cp -a /dr/base /dr/restore
touch /dr/restore/recovery.signal
cat >> /dr/restore/postgresql.auto.conf <<EOF
restore_command = 'cp /dr/archive/%f %p'
recovery_target_time = '${target_time}'
recovery_target_action = 'promote'
EOF
chown -R postgres:postgres /dr/restore
chmod 700 /dr/restore
" >/dev/null

  docker run \
    --name "${restored}" \
    -e POSTGRES_PASSWORD=postgres \
    -v "${workdir}:/dr" \
    --entrypoint bash \
    -d "${postgres_image}" \
    -lc 'exec gosu postgres "$(pg_config --bindir)/postgres" -D /dr/restore' >/dev/null

  local restored_rows=""
  local _
  for _ in $(seq 1 120); do
    restored_rows="$(
      docker exec "${restored}" psql -U postgres -Atqc \
        "SELECT CASE WHEN pg_is_in_recovery() THEN 'recovery' ELSE 'promoted' END || '|' || string_agg(label, ',' ORDER BY id) FROM dr_restore_depth_smoke;" \
        2>/dev/null || true
    )"
    if [[ "${restored_rows}" == "promoted|base-before-backup,before-target" ]]; then
      break
    fi
    sleep 1
  done

  if [[ "${restored_rows}" != "promoted|base-before-backup,before-target" ]]; then
    docker logs "${restored}" >&2 || true
    docker exec "${restored}" psql -U postgres -v ON_ERROR_STOP=1 \
      -c "SELECT pg_is_in_recovery(), pg_last_wal_replay_lsn(), pg_last_xact_replay_timestamp();" \
      -c "TABLE dr_restore_depth_smoke;" >&2 || true
    echo "PITR smoke restored unexpected rows: ${restored_rows:-<none>}" >&2
    exit 1
  fi

  local archive_count
  archive_count="$(find "${archive_dir}" -type f | wc -l | tr -d ' ')"
  echo "dr_restore_depth_postgres_smoke\ttarget_time=${target_time}\tarchived_wal_segments=${archive_count}\trestored_rows=${restored_rows}"
}

cargo test -q -p ai_blaise_citus_e2e dr_restore_depth
assert_output_line \
  "DR restore-depth canonical report" \
  $'in-place\t8\t8\t6\t5\t2\t840\tsha256' \
  cargo run -q -p ai_blaise_citus_e2e --bin dr_restore_depth_report

assert_contains "docs/ai-blaise/RUNBOOKS/disaster-recovery.md" "ci/ai-blaise/dr-restore-depth-check.sh"
assert_contains "docs/ai-blaise/RUNBOOKS/pitr-restore.md" "dr_restore_depth_postgres_smoke"
assert_contains "docs/ai-blaise/NEW_FEATURES.md" "dr_restore_depth_report"
assert_contains "docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md" "restore-depth gate"
assert_contains "Makefile.ai-blaise" "dr-restore-depth-check:"

if [[ "${run_docker_smoke}" == "1" ]]; then
  if ! command -v docker >/dev/null 2>&1; then
    echo "docker is required for DR restore-depth smoke" >&2
    exit 1
  fi
  run_postgres_pitr_smoke
else
  echo "DR restore-depth Docker PITR smoke skipped; set REQUIRE_DOCKER=1 to require it"
fi

echo "ai_blaise_citus DR restore-depth check passed"
