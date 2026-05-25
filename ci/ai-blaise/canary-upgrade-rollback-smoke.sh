#!/usr/bin/env bash
set -euo pipefail

# FEATURE: D9
# Live PostgreSQL canary drill for the reversible ai_blaise_citus companion SQL
# transition. The upstream Citus upgrade matrix remains a larger release gate;
# this smoke proves the local overlay extension can upgrade, record canary
# evidence, and roll back through PostgreSQL's ALTER EXTENSION mechanism.

repo_root="$(git rev-parse --show-toplevel)"
extension_dir="${repo_root}/images/citus-pg-overlay/extensions"
control_file="${extension_dir}/ai_blaise_citus.control"
install_sql="${extension_dir}/ai_blaise_citus--0.1.0.sql"
upgrade_sql="${extension_dir}/ai_blaise_citus--0.1.0--0.1.1.sql"
downgrade_sql="${extension_dir}/ai_blaise_citus--0.1.1--0.1.0.sql"
release_mode="${AI_BLAISE_RELEASE_MODE:-0}"
require_docker="${REQUIRE_DOCKER:-${release_mode}}"
pg_major="${CANARY_UPGRADE_PG_MAJOR:-17}"
postgres_image="${CANARY_UPGRADE_IMAGE:-postgres:${pg_major}}"
release_id="${CANARY_UPGRADE_RELEASE_ID:-canary-drill-001}"

for file in "${control_file}" "${install_sql}" "${upgrade_sql}" "${downgrade_sql}"; do
  if [[ ! -s "${file}" ]]; then
    echo "missing canary upgrade artifact: ${file}" >&2
    exit 1
  fi
done

if ! command -v docker >/dev/null 2>&1; then
  if [[ "${require_docker}" == "1" ]]; then
    echo "docker is required for canary upgrade rollback smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping canary upgrade rollback smoke"
  exit 0
fi

container="ai-blaise-canary-upgrade-pg${pg_major}-${RANDOM}-$$"
evidence_file="$(mktemp -t ai-blaise-canary-upgrade-evidence.XXXXXX)"
cleanup() {
  docker rm -f "${container}" >/dev/null 2>&1 || true
  rm -f "${evidence_file}"
}
trap cleanup EXIT

echo "=== canary-upgrade-rollback-smoke vs ${postgres_image} ==="

docker run \
  --name "${container}" \
  -e POSTGRES_PASSWORD=postgres \
  -v "${control_file}:/usr/share/postgresql/${pg_major}/extension/ai_blaise_citus.control:ro" \
  -v "${install_sql}:/usr/share/postgresql/${pg_major}/extension/ai_blaise_citus--0.1.0.sql:ro" \
  -v "${upgrade_sql}:/usr/share/postgresql/${pg_major}/extension/ai_blaise_citus--0.1.0--0.1.1.sql:ro" \
  -v "${downgrade_sql}:/usr/share/postgresql/${pg_major}/extension/ai_blaise_citus--0.1.1--0.1.0.sql:ro" \
  -d "${postgres_image}" >/dev/null

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
  echo "postgres container did not finish init scripts" >&2
  exit 1
fi

ready=0
for _ in $(seq 1 60); do
  if docker exec "${container}" psql -U postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 1
done
if [[ "${ready}" != "1" ]]; then
  docker logs "${container}" >&2 || true
  echo "postgres container did not become ready" >&2
  exit 1
fi

docker exec -i "${container}" \
  psql -U postgres -Atq -v ON_ERROR_STOP=1 -v release_id="${release_id}" <<'SQL' \
  | tee "${evidence_file}"
CREATE EXTENSION ai_blaise_citus;
SELECT 'version_before_upgrade' || E'\t' || extversion
FROM pg_extension
WHERE extname = 'ai_blaise_citus';

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
SQL

grep -Fq $'version_before_upgrade\t0.1.0' "${evidence_file}"
grep -Fq $'version_after_upgrade\t0.1.1' "${evidence_file}"
grep -Eq $'^upgrade_event_id\t[1-9][0-9]*$' "${evidence_file}"
grep -Fq $'event_count_after_upgrade\t1' "${evidence_file}"
grep -Fq $'version_after_rollback\t0.1.0' "${evidence_file}"
grep -Fq $'event_table_after_rollback\tabsent' "${evidence_file}"
grep -Fq $'event_function_after_rollback\tabsent' "${evidence_file}"

printf 'canary_upgrade_rollback_smoke\tpg_major=%s\tupgrade=0.1.0->0.1.1\trollback=0.1.1->0.1.0\tevidence=recorded\n' \
  "${pg_major}"
