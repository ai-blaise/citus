#!/usr/bin/env bash
set -euo pipefail

# FEATURE: PGC1
# FEATURE: PGC2
# Live PostgreSQL core patch proof. This smoke builds PostgreSQL 17 from source,
# applies patches/postgres/series, builds Citus against the patched pg_config,
# starts that runtime, and verifies patch-only symbols through a C probe
# extension plus real commit_ts/WAL behavior. It does not claim pgactive or
# Spock apply traffic, multi-node active-active conflict resolution, or PG18.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if [[ "${REQUIRE_DOCKER:-0}" != "1" ]]; then
  echo "REQUIRE_DOCKER=1 is required for postgres core patches live smoke" >&2
  exit 2
fi

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "$1 is required for postgres core patches live smoke" >&2
    exit 1
  }
}

need_cmd docker
need_cmd git

pg_ref="${POSTGRES_CORE_PATCHES_REF:-REL_17_10}"
image="${POSTGRES_CORE_PATCHES_IMAGE:-ai-blaise-postgres-core-patches:pg17-${pg_ref}}"
source_git_sha="$(git rev-parse HEAD)"
source_tree_state="clean"
if [[ -n "$(git status --porcelain)" ]]; then
  source_tree_state="dirty"
fi

docker build \
  -f images/citus-pg-overlay/Dockerfile.pgcore-patches \
  --build-arg POSTGRES_CORE_REF="${pg_ref}" \
  --build-arg AI_BLAISE_SOURCE_GIT_SHA="${source_git_sha}" \
  --build-arg AI_BLAISE_SOURCE_TREE_STATE="${source_tree_state}" \
  -t "${image}" \
  .

observed_ref="$(docker image inspect -f '{{ index .Config.Labels "ai-blaise.citus.pg-core-ref" }}' "${image}")"
if [[ "${observed_ref}" != "${pg_ref}" ]]; then
  echo "pg-core-ref label mismatch: expected ${pg_ref}, observed ${observed_ref}" >&2
  exit 1
fi

observed_features="$(docker image inspect -f '{{ index .Config.Labels "ai-blaise.citus.pg-core-patch-features" }}' "${image}")"
if [[ "${observed_features}" != "PGC1,PGC2" ]]; then
  echo "pg-core-patch-features label mismatch: ${observed_features}" >&2
  exit 1
fi

container="ai-blaise-pgc-${RANDOM}-$$"
cleanup() {
  docker rm -f "${container}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run -d --name "${container}" "${image}" bash -lc 'sleep infinity' >/dev/null

docker exec "${container}" bash -s <<'CONTAINER_PGC_SMOKE'
set -euo pipefail
export PATH=/usr/local/pgsql/bin:${PATH}
export PGDATA=/tmp/pgc-data
initdb -D "${PGDATA}" -A trust --no-locale >/tmp/pgc-initdb.log
{
  echo "shared_preload_libraries = 'citus'"
  echo "track_commit_timestamp = on"
  echo "listen_addresses = ''"
} >>"${PGDATA}/postgresql.conf"
pg_ctl -D "${PGDATA}" -o "-k /tmp" -w start >/tmp/pgc-start.log
trap 'pg_ctl -D "${PGDATA}" -m fast -w stop >/tmp/pgc-stop.log' EXIT
createdb -h /tmp pgc_smoke
psql -h /tmp -d pgc_smoke -v ON_ERROR_STOP=1 -qAt <<'SQL' >/tmp/pgc-setup.out
CREATE EXTENSION citus;
CREATE EXTENSION ai_blaise_pgc_probe;
CREATE TABLE public.pgc_probe(id integer primary key, note text);
SQL
citus_version="$(psql -h /tmp -d pgc_smoke -qAt -v ON_ERROR_STOP=1 -c 'SELECT citus_version()')"
clock_match="$(psql -h /tmp -d pgc_smoke -qAt -v ON_ERROR_STOP=1 -c "SELECT ai_blaise_pgc_logical_clock_roundtrip('2030-01-02 03:04:05+00'::timestamptz) = '2030-01-02 03:04:05+00'::timestamptz")"
if [[ "${clock_match}" != "t" ]]; then
  echo "PGC1 logical clock roundtrip failed" >&2
  exit 1
fi
monotonic_xid="$(psql -h /tmp -d pgc_smoke -qAt -v ON_ERROR_STOP=1 <<'SQL'
BEGIN;
INSERT INTO public.pgc_probe VALUES (1, 'monotonic clock bump');
SELECT pg_current_xact_id();
COMMIT;
SQL
)"
monotonic_match="$(psql -h /tmp -d pgc_smoke -qAt -v ON_ERROR_STOP=1 -c "SELECT pg_xact_commit_timestamp('${monotonic_xid}'::xid) > '2030-01-02 03:04:05+00'::timestamptz")"
if [[ "${monotonic_match}" != "t" ]]; then
  echo "PGC1 monotonic commit timestamp hook did not bump local commit time" >&2
  exit 1
fi
override_xid="$(psql -h /tmp -d pgc_smoke -qAt -v ON_ERROR_STOP=1 <<'SQL'
BEGIN;
SELECT ai_blaise_pgc_subtrans_override('2030-01-02 03:04:06+00'::timestamptz, 7);
INSERT INTO public.pgc_probe VALUES (2, 'subtrans commit-ts override');
COMMIT;
SQL
)"
override_match="$(psql -h /tmp -d pgc_smoke -qAt -v ON_ERROR_STOP=1 -c "SELECT pg_xact_commit_timestamp('${override_xid}'::xid) = '2030-01-02 03:04:06+00'::timestamptz")"
if [[ "${override_match}" != "t" ]]; then
  echo "PGC2 subtransaction commit timestamp override did not persist" >&2
  exit 1
fi
pg_ctl -D "${PGDATA}" -m fast -w stop >/tmp/pgc-stop.log
trap - EXIT
first_wal="$(find "${PGDATA}/pg_wal" -maxdepth 1 -type f -name '0000000100000000000000*' | sort | head -1)"
if [[ -z "${first_wal}" ]]; then
  echo "no WAL segment found for pg_waldump" >&2
  exit 1
fi
pg_waldump "${first_wal}" >/tmp/pgc-waldump.out
if ! grep -Fq "SUBTRANS_TS" /tmp/pgc-waldump.out; then
  echo "pg_waldump did not identify COMMIT_TS_SUBTRANS_TS" >&2
  exit 1
fi
printf 'pgc_citus_version=%s\n' "${citus_version}"
printf 'pgc_monotonic_xid=%s\n' "${monotonic_xid}"
printf 'pgc_override_xid=%s\n' "${override_xid}"
printf 'pgc_waldump_subtrans_ts=true\n'
CONTAINER_PGC_SMOKE

echo "postgres_core_patches_live=passed"
echo "pgc_postgres_ref=${pg_ref}"
echo "pgc_patch_series=patches/postgres/series"
echo "pgc_citus_built_against_patched_pg=true"
echo "pgc_logical_clock_hook_executed=true"
echo "pgc_subtrans_commit_ts_override_executed=true"
echo "pgc_pgactive_traffic_exercised=false"
echo "pgc_spock_apply_traffic_exercised=false"
echo "pgc_pg18_exercised=false"
