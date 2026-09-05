#!/usr/bin/env bash
set -euo pipefail

# FEATURE: D9
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
fixture_builder="${repo_root}/ci/ai-blaise/build-real-citus-test-fixture.sh"
fixture_contract="${repo_root}/ci/ai-blaise/real-citus-test-fixture-contract.py"
sql_dir="${repo_root}/ci/ai-blaise/sql"
upgrade_script="${repo_root}/images/citus-pg-overlay/upgrades/ai_blaise_citus--0.1.2.sql"
pg_major="${EXTENSION_SECURITY_PG_MAJOR:?set EXTENSION_SECURITY_PG_MAJOR to 17 or 18}"
case "${pg_major}" in 17|18) ;; *) echo 'unsupported PostgreSQL major' >&2; exit 1 ;; esac
if [[ -n "${EXTENSION_SECURITY_IMAGE:-}" ]]; then
  echo 'EXTENSION_SECURITY_IMAGE is retired; use source-verified CITUS_TEST_FIXTURE_IMAGE' >&2
  exit 1
fi
for file in "${fixture_builder}" "${fixture_contract}" "${upgrade_script}"; do
  if [[ ! -s "${file}" ]]; then
    echo "missing security recovery fixture artifact: ${file}" >&2
    exit 1
  fi
done
if [[ ! -x "${fixture_builder}" ]]; then
  echo "real-Citus test fixture builder is not executable: ${fixture_builder}" >&2
  exit 1
fi
python3 "${fixture_contract}"
command -v docker >/dev/null 2>&1 || {
  echo 'docker is required for security recovery smoke' >&2
  exit 1
}
fixture_image="$("${fixture_builder}" --pg-major "${pg_major}")"

evidence_dir="$(mktemp -d -t ai-blaise-extension-security.XXXXXX)"
container=""
cleanup() {
  if [[ -n "${container}" ]]; then
    docker rm --force --volumes "${container}" >/dev/null 2>&1 || true
  fi
  rm -f "${evidence_dir}/before.tsv" "${evidence_dir}/after.tsv" "${evidence_dir}/backup.dump" "${evidence_dir}/negative.log"
  rmdir "${evidence_dir}"
}
trap cleanup EXIT

container="$(docker run --network none -d -e POSTGRES_HOST_AUTH_METHOD=trust "${fixture_image}")"
init_complete=0
for _ in $(seq 1 120); do
  if docker exec "${container}" pg_isready -U postgres >/dev/null 2>&1 &&
     [[ "$(docker inspect -f '{{.State.Running}}' "${container}")" == true ]] &&
     docker logs "${container}" 2>&1 | grep -q 'PostgreSQL init process complete'; then
    init_complete=1
    break
  fi
  sleep 1
done
if [[ "${init_complete}" != "1" ]]; then
  docker logs "${container}" >&2 || true
  echo 'real-Citus fixture did not complete PostgreSQL initialization' >&2
  exit 1
fi
actual_major="$(docker exec "${container}" psql -X -U postgres -Atqc "SELECT current_setting('server_version_num')::int / 10000")"
[[ "${actual_major}" == "${pg_major}" ]] || { echo 'PostgreSQL image major mismatch' >&2; exit 1; }

psql_db() {
  docker exec -i "${container}" psql -X -U postgres -d "$1" -Atq -v ON_ERROR_STOP=1
}
upgrade_state() {
  psql_db security_upgrade <<'SQL'
SELECT (SELECT extversion FROM pg_extension WHERE extname = 'ai_blaise_citus'),
       md5(string_agg(p.oid::text || ':' || COALESCE(p.proacl::text, 'null'), ',' ORDER BY p.oid)),
       (SELECT md5(COALESCE(string_agg(objoid::text || ':' || initprivs::text, ',' ORDER BY objoid), ''))
        FROM pg_init_privs WHERE classoid = 'pg_proc'::regclass)
FROM pg_proc p
JOIN pg_depend d ON d.classid = 'pg_proc'::regclass AND d.objid = p.oid AND d.deptype = 'e'
JOIN pg_extension e ON d.refclassid = 'pg_extension'::regclass AND d.refobjid = e.oid
WHERE e.extname = 'ai_blaise_citus';
SQL
}
assert_upgrade_rolls_back() {
  local stage="$1" expected="$2" before after
  before="$(upgrade_state)"
  if awk -v stage="${stage}" '
      stage == "before_update" && /^ALTER EXTENSION/ { print "SELECT 1 / 0;" }
      stage == "before_commit" && /^COMMIT;/ { print "SELECT 1 / 0;" }
      { print }
    ' "${upgrade_script}" | psql_db security_upgrade > "${evidence_dir}/negative.log" 2>&1; then
    echo 'negative upgrade unexpectedly committed' >&2
    exit 1
  fi
  grep -Fq "${expected}" "${evidence_dir}/negative.log"
  after="$(upgrade_state)"
  [[ "${before}" == "${after}" ]] || { echo 'failed upgrade changed version or ACL state' >&2; exit 1; }
}
psql_db postgres <<'SQL'
CREATE ROLE backup_unprivileged;
CREATE ROLE backup_explicit_grantee;
CREATE DATABASE security_upgrade;
CREATE DATABASE security_fresh;
CREATE DATABASE security_restore;
SQL
psql_db security_upgrade <<'SQL'
CREATE EXTENSION citus;
CREATE EXTENSION pgcrypto;
CREATE EXTENSION ai_blaise_citus VERSION '0.1.1';
CREATE FUNCTION public.backup_unrelated() RETURNS integer LANGUAGE sql AS 'SELECT 37';
CREATE FUNCTION public.companion_current_tracestate(integer) RETURNS integer LANGUAGE sql AS 'SELECT $1';
GRANT USAGE ON SCHEMA companion TO backup_unprivileged, backup_explicit_grantee;
GRANT EXECUTE ON FUNCTION companion.current_tracestate() TO backup_explicit_grantee WITH GRANT OPTION;
SQL
psql_db security_upgrade < "${sql_dir}/extension-backup-seed.sql"
psql_db security_upgrade < "${sql_dir}/extension-backup-state.sql" > "${evidence_dir}/before.tsv"
psql_db security_upgrade <<'SQL'
DO $$
BEGIN
    BEGIN
        EXECUTE 'ALTER EXTENSION ai_blaise_citus UPDATE TO ''0.1.2''';
        RAISE EXCEPTION 'unsafe bare upgrade unexpectedly accepted explicit grants';
    EXCEPTION WHEN object_not_in_prerequisite_state THEN
        NULL;
    END;
    IF (SELECT extversion FROM pg_extension WHERE extname = 'ai_blaise_citus') <> '0.1.1' THEN
        RAISE EXCEPTION 'rejected upgrade changed extension version';
    END IF;
END
$$;
SQL
assert_upgrade_rolls_back before_update 'division by zero'
assert_upgrade_rolls_back before_commit 'division by zero'
psql_db security_upgrade <<'SQL'
SET ROLE backup_explicit_grantee;
GRANT EXECUTE ON FUNCTION companion.current_tracestate() TO backup_unprivileged;
RESET ROLE;
SQL
assert_upgrade_rolls_back delegated 'routine ownership or delegated grants require manual upgrade review'
psql_db security_upgrade <<'SQL'
SET ROLE backup_explicit_grantee;
REVOKE EXECUTE ON FUNCTION companion.current_tracestate() FROM backup_unprivileged;
RESET ROLE;
SQL
psql_db security_upgrade < "${upgrade_script}"
psql_db security_upgrade <<'SQL'
DO $$
BEGIN
    IF NOT has_function_privilege('backup_unprivileged', 'public.backup_unrelated()', 'EXECUTE')
       OR NOT has_function_privilege('backup_unprivileged', 'public.companion_current_tracestate(integer)', 'EXECUTE')
       OR NOT has_function_privilege('backup_explicit_grantee', 'companion.current_tracestate()', 'EXECUTE') THEN
        RAISE EXCEPTION 'migration changed unrelated or explicit privileges';
    END IF;
END
$$;
SET ROLE backup_unprivileged;
DO $$
BEGIN
    BEGIN
        PERFORM companion.current_tracestate();
        RAISE EXCEPTION 'unprivileged extension call unexpectedly succeeded';
    EXCEPTION WHEN insufficient_privilege THEN
        NULL;
    END;
END
$$;
RESET ROLE;
SET ROLE backup_explicit_grantee;
SELECT companion.current_tracestate();
RESET ROLE;
DO $$
BEGIN
    BEGIN
        EXECUTE 'ALTER EXTENSION ai_blaise_citus UPDATE TO ''0.1.1''';
        RAISE EXCEPTION 'security floor downgrade unexpectedly succeeded';
    EXCEPTION WHEN invalid_parameter_value THEN
        NULL;
    END;
END
$$;
SQL
psql_db security_upgrade < "${sql_dir}/extension-security-assert.sql"
psql_db security_fresh <<'SQL'
CREATE EXTENSION citus;
CREATE EXTENSION pgcrypto;
CREATE EXTENSION ai_blaise_citus VERSION '0.1.2';
SQL
psql_db security_fresh < "${sql_dir}/extension-security-assert.sql"
docker exec "${container}" pg_dump -U postgres -Fc security_upgrade > "${evidence_dir}/backup.dump"
psql_db security_restore <<'SQL'
CREATE EXTENSION citus;
CREATE EXTENSION pgcrypto;
SQL
docker exec -i "${container}" pg_restore -U postgres -d security_restore --exit-on-error < "${evidence_dir}/backup.dump"
psql_db security_restore < "${sql_dir}/extension-security-assert.sql"
psql_db security_restore < "${sql_dir}/extension-backup-state.sql" > "${evidence_dir}/after.tsv"
diff -u "${evidence_dir}/before.tsv" "${evidence_dir}/after.tsv"
psql_db security_restore <<'SQL'
DO $$
DECLARE
    sequence_oid oid;
BEGIN
    IF NOT has_function_privilege('backup_explicit_grantee', 'companion.current_tracestate()', 'EXECUTE WITH GRANT OPTION') THEN
        RAISE EXCEPTION 'dump restore lost explicit pre-upgrade routine grant';
    END IF;
    FOR sequence_oid IN
        SELECT c.oid FROM pg_class c
        JOIN pg_extension e ON c.oid = ANY(e.extconfig)
        WHERE e.extname = 'ai_blaise_citus' AND c.relkind = 'S'
    LOOP
        IF nextval(sequence_oid::regclass) <> 987655 THEN
            RAISE EXCEPTION 'restored serial sequence did not resume after preserved state';
        END IF;
    END LOOP;
END
$$;
SQL
printf 'extension_security_backup_smoke\tpg_major=%s\timage=%s\tversion=0.1.2\ttables=44\tsequences=24\troutines=153\tpopulated_restore=passed\tpublic_deny=passed\texplicit_grants=preserved\tdowngrade=denied\tfailed_transaction_rollback=passed\tdelegated_grants=denied\n' "${pg_major}" "${fixture_image}"
