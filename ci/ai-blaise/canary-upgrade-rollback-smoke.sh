#!/usr/bin/env bash
set -euo pipefail

# FEATURE: D9
# Live PostgreSQL canary drill for the reversible ai_blaise_citus companion SQL
# transition. The upstream Citus upgrade matrix remains a larger release gate;
# this smoke proves the local overlay extension can upgrade, record canary
# evidence, and roll back through PostgreSQL's ALTER EXTENSION mechanism.

repo_root="$(git rev-parse --show-toplevel)"
fixture_builder="${repo_root}/ci/ai-blaise/build-real-citus-test-fixture.sh"
fixture_contract="${repo_root}/ci/ai-blaise/real-citus-test-fixture-contract.py"
install_version="0.1.0"
reversible_version="0.1.1"
current_version="0.1.2"
release_mode="${AI_BLAISE_RELEASE_MODE:-0}"
require_docker="${REQUIRE_DOCKER:-${release_mode}}"
release_id="${CANARY_UPGRADE_RELEASE_ID:-canary-drill-001}"

if [[ -n "${CANARY_UPGRADE_PG_MAJOR:-}" ]]; then
  pg_majors=("${CANARY_UPGRADE_PG_MAJOR}")
else
  pg_majors=(17 18)
fi

if [[ -n "${CANARY_UPGRADE_IMAGE:-}" || -n "${CANARY_UPGRADE_IMAGE_17:-}" || -n "${CANARY_UPGRADE_IMAGE_18:-}" ]]; then
  echo "CANARY_UPGRADE_IMAGE overrides are retired; use source-verified CITUS_TEST_FIXTURE_IMAGE with CANARY_UPGRADE_PG_MAJOR" >&2
  exit 1
fi
if [[ -n "${CITUS_TEST_FIXTURE_IMAGE:-}" && "${#pg_majors[@]}" -ne 1 ]]; then
  echo "CITUS_TEST_FIXTURE_IMAGE requires one explicit CANARY_UPGRADE_PG_MAJOR" >&2
  exit 1
fi

for pg_major in "${pg_majors[@]}"; do
  if [[ "${pg_major}" != "17" && "${pg_major}" != "18" ]]; then
    echo "CANARY_UPGRADE_PG_MAJOR must be 17 or 18, got ${pg_major}" >&2
    exit 1
  fi
done

for file in "${fixture_builder}" "${fixture_contract}"; do
  if [[ ! -s "${file}" ]]; then
    echo "missing canary upgrade artifact: ${file}" >&2
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
    echo "docker is required for canary upgrade rollback smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping canary upgrade rollback smoke"
  exit 0
fi

active_container=""
active_evidence_file=""
cleanup() {
  if [[ -n "${active_container}" ]]; then
    docker rm --force --volumes "${active_container}" >/dev/null 2>&1 || true
  fi
  if [[ -n "${active_evidence_file}" ]]; then
    rm -f "${active_evidence_file}"
  fi
}
trap cleanup EXIT

run_canary() {
  local pg_major="$1"
  local fixture_image
  fixture_image="$("${fixture_builder}" --pg-major "${pg_major}")"

  active_container="ai-blaise-canary-upgrade-pg${pg_major}-${RANDOM}-$$"
  active_evidence_file="$(mktemp -t ai-blaise-canary-upgrade-evidence.XXXXXX)"

  echo "=== canary-upgrade-rollback-smoke on immutable real-Citus PG${pg_major} fixture ==="

  docker run \
    --name "${active_container}" \
    --network none \
    -e POSTGRES_PASSWORD=postgres \
    -d "${fixture_image}" >/dev/null

  local container_logs
  local init_complete=0
  for _ in $(seq 1 120); do
    container_logs="$(docker logs "${active_container}" 2>&1 || true)"
    if [[ "${container_logs}" == *"PostgreSQL init process complete"* ]]; then
      init_complete=1
      break
    fi
    sleep 1
  done
  if [[ "${init_complete}" != "1" ]]; then
    docker logs "${active_container}" >&2 || true
    echo "postgres container did not finish init scripts" >&2
    exit 1
  fi

  local ready=0
  for _ in $(seq 1 60); do
    if docker exec "${active_container}" psql -U postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
      ready=1
      break
    fi
    sleep 1
  done
  if [[ "${ready}" != "1" ]]; then
    docker logs "${active_container}" >&2 || true
    echo "postgres container did not become ready" >&2
    exit 1
  fi

  local actual_pg_major
  actual_pg_major="$(docker exec "${active_container}" \
    psql -U postgres -Atqc "SELECT current_setting('server_version_num')::integer / 10000")"
  if [[ "${actual_pg_major}" != "${pg_major}" ]]; then
    echo "canary image major mismatch: expected ${pg_major}, got ${actual_pg_major}" >&2
    exit 1
  fi

  docker exec -i "${active_container}" \
    psql -U postgres -Atq -v ON_ERROR_STOP=1 -v release_id="${release_id}" <<'SQL' \
    | tee "${active_evidence_file}"
CREATE DATABASE upgrade_path;
CREATE DATABASE default_install;
CREATE DATABASE explicit_current_install;

\connect upgrade_path
CREATE EXTENSION citus;
CREATE EXTENSION pgcrypto;
CREATE EXTENSION ai_blaise_citus VERSION '0.1.0';
SELECT 'version_before_upgrade' || E'\t' || extversion
FROM pg_extension
WHERE extname = 'ai_blaise_citus';
SELECT 'selected_upgrade_path' || E'\t' || COALESCE(path, 'absent')
FROM pg_extension_update_paths('ai_blaise_citus')
WHERE source = '0.1.0' AND target = '0.1.1';
SELECT 'selected_downgrade_path' || E'\t' || COALESCE(path, 'absent')
FROM pg_extension_update_paths('ai_blaise_citus')
WHERE source = '0.1.1' AND target = '0.1.0';

ALTER EXTENSION ai_blaise_citus UPDATE TO '0.1.1';
SELECT 'version_after_upgrade' || E'\t' || extversion
FROM pg_extension
WHERE extname = 'ai_blaise_citus';

SELECT 'upgrade_event_id' || E'\t' || companion_internal.record_extension_upgrade_event(
    :'release_id',
    '0.1.0',
    '0.1.1',
    'upgrade'
);
SELECT 'event_count_after_upgrade' || E'\t' || count(*)
FROM companion_extension_upgrade_events
WHERE release_id = :'release_id'
  AND previous_version = '0.1.0'
  AND target_version = '0.1.1'
  AND action = 'upgrade';
DO $$
BEGIN
    IF to_regclass('companion_internal.extension_upgrade_events') IS NULL THEN
        RAISE EXCEPTION 'upgrade evidence table was not installed';
    END IF;
    IF to_regprocedure('companion_internal.record_extension_upgrade_event(text,text,text,text)') IS NULL THEN
        RAISE EXCEPTION 'upgrade evidence recorder was not installed';
    END IF;
END;
$$;

ALTER EXTENSION ai_blaise_citus UPDATE TO '0.1.0';
SELECT 'version_after_rollback' || E'\t' || extversion
FROM pg_extension
WHERE extname = 'ai_blaise_citus';
SELECT 'event_table_after_rollback' || E'\t' || COALESCE(
    to_regclass('companion_internal.extension_upgrade_events')::text,
    'absent'
);
SELECT 'event_function_after_rollback' || E'\t' || COALESCE(
    to_regprocedure('companion_internal.record_extension_upgrade_event(text,text,text,text)')::text,
    'absent'
);
DO $$
BEGIN
    IF to_regclass('companion_internal.extension_upgrade_events') IS NOT NULL THEN
        RAISE EXCEPTION 'rollback did not drop upgrade evidence table';
    END IF;
    IF to_regprocedure('companion_internal.record_extension_upgrade_event(text,text,text,text)') IS NOT NULL THEN
        RAISE EXCEPTION 'rollback did not drop upgrade evidence recorder';
    END IF;
END;
$$;

ALTER EXTENSION ai_blaise_citus UPDATE;
SELECT 'version_after_default_update' || E'\t' || extversion
FROM pg_extension
WHERE extname = 'ai_blaise_citus';

\connect default_install
CREATE EXTENSION citus;
CREATE EXTENSION pgcrypto;
CREATE EXTENSION ai_blaise_citus;
SELECT 'version_after_bare_create' || E'\t' || extversion
FROM pg_extension
WHERE extname = 'ai_blaise_citus';
SELECT 'event_table_after_bare_create' || E'\t' || COALESCE(
    to_regclass('companion_internal.extension_upgrade_events')::text,
    'absent'
);

\connect explicit_current_install
CREATE EXTENSION citus;
CREATE EXTENSION pgcrypto;
CREATE EXTENSION ai_blaise_citus VERSION '0.1.2';
SELECT 'version_after_explicit_current_create' || E'\t' || extversion
FROM pg_extension
WHERE extname = 'ai_blaise_citus';
SELECT 'event_table_after_explicit_current_create' || E'\t' || COALESCE(
    to_regclass('companion_internal.extension_upgrade_events')::text,
    'absent'
);
SQL

  grep -Fq $'version_before_upgrade\t'"${install_version}" "${active_evidence_file}"
  grep -Fq $'selected_upgrade_path\t'"${install_version}--${reversible_version}" "${active_evidence_file}"
  grep -Fq $'selected_downgrade_path\t'"${reversible_version}--${install_version}" "${active_evidence_file}"
  grep -Fq $'version_after_upgrade\t'"${reversible_version}" "${active_evidence_file}"
  grep -Eq $'^upgrade_event_id\t[1-9][0-9]*$' "${active_evidence_file}"
  grep -Fq $'event_count_after_upgrade\t1' "${active_evidence_file}"
  grep -Fq $'version_after_rollback\t'"${install_version}" "${active_evidence_file}"
  grep -Fq $'event_table_after_rollback\tabsent' "${active_evidence_file}"
  grep -Fq $'event_function_after_rollback\tabsent' "${active_evidence_file}"
  grep -Fq $'version_after_default_update\t'"${current_version}" "${active_evidence_file}"
  grep -Fq $'version_after_bare_create\t'"${current_version}" "${active_evidence_file}"
  grep -Fq $'event_table_after_bare_create\tcompanion_internal.extension_upgrade_events' "${active_evidence_file}"
  grep -Fq $'version_after_explicit_current_create\t'"${current_version}" "${active_evidence_file}"
  grep -Fq $'event_table_after_explicit_current_create\tcompanion_internal.extension_upgrade_events' "${active_evidence_file}"

  printf 'canary_upgrade_rollback_smoke\tpg_major=%s\tdefault=%s\tupgrade=%s->%s\trollback=%s->%s\tpaths=exact\tevidence=recorded\n' \
    "${pg_major}" \
    "${current_version}" \
    "${install_version}" \
    "${reversible_version}" \
    "${reversible_version}" \
    "${install_version}"

  docker rm --force --volumes "${active_container}" >/dev/null
  rm -f "${active_evidence_file}"
  active_container=""
  active_evidence_file=""
}

for pg_major in "${pg_majors[@]}"; do
  run_canary "${pg_major}"
done
