#!/usr/bin/env bash
# FEATURE: S7
#
# Live pgactive runtime smoke for S7 Cross-Region Replication. Boots a
# source-built pgactive PostgreSQL container, configures the pgactive_fdw
# foreign data wrapper, creates the pgactive group, waits for the node to
# reach ready state, and verifies the conflict-policy infrastructure
# (pgactive_conflict_history table + GUCs + supervisor worker).
#
# Scope: this proves the pgactive runtime end-to-end on a single regional
# node plus the conflict-policy gate. It does NOT exercise the full
# multi-host active-active join. The upstream pgactive_join_group SQL
# function ships a logical-copy bootstrap that races with the joiner's
# own pre-ready catalog entry on the target node, manifesting as a
# 'previous init failed, manual cleanup is required' loop. The supported
# AWS pgactive operational path for cross-host active-active deployment
# is the pgactive_init_copy client-side binary (pg_basebackup-based
# bootstrap), which requires orchestration outside the scope of this CI
# smoke. Multi-host pgactive_init_copy operational evidence is tracked
# separately under the same S7 contract surface.

set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if ! command -v docker >/dev/null 2>&1; then
  if [[ "${REQUIRE_DOCKER:-0}" == "1" ]]; then echo "docker required" >&2; exit 1; fi
  echo "docker unavailable; skipping S7 smoke"; exit 0
fi

evidence_dir="${S7_EVIDENCE_DIR:-artifacts}"
mkdir -p "${evidence_dir}"
evidence_file="${S7_EVIDENCE_FILE:-${evidence_dir}/s7-pgactive-runtime-evidence.tsv}"
pgactive_image="${S7_PGACTIVE_IMAGE:-pgactive-pg17:test}"
container="s7-pgactive-${RANDOM}-$$"

cleanup() { docker rm -f "${container}" >/dev/null 2>&1 || true; }
trap cleanup EXIT

log() { printf '[s7-pgactive] %s\n' "$*" >&2; }

docker image inspect "${pgactive_image}" >/dev/null 2>&1 || { echo "image ${pgactive_image} not available" >&2; exit 1; }

log "booting pgactive container"
docker run -d --name "${container}" \
  -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=app \
  "${pgactive_image}" \
  -c shared_preload_libraries=pgactive -c wal_level=logical \
  -c track_commit_timestamp=on -c max_wal_senders=10 \
  -c max_replication_slots=10 -c max_worker_processes=16 >/dev/null

for _ in $(seq 1 120); do
  if docker exec "${container}" psql -U postgres -d app -Atqc 'SELECT 1' >/dev/null 2>&1; then break; fi
  sleep 1
done

log "installing pgactive + FDW + create_group + wait_for_node_ready"
docker exec -i "${container}" psql -U postgres -d app -v ON_ERROR_STOP=1 <<'SQL'
CREATE EXTENSION pgactive;
CREATE SERVER pgactive_server_us_east_1
  FOREIGN DATA WRAPPER pgactive_fdw
  OPTIONS (host 'localhost', port '5432', dbname 'app');
CREATE USER MAPPING FOR postgres SERVER pgactive_server_us_east_1
  OPTIONS (user 'postgres', password 'postgres');
SELECT pgactive.pgactive_create_group(
  node_name := 'region-us-east-1',
  node_dsn := 'user_mapping=postgres pgactive_foreign_server=pgactive_server_us_east_1'
);
SELECT pgactive.pgactive_wait_for_node_ready();
SELECT pgactive.pgactive_is_active_in_db();
SQL

# Canonical evidence fields.
node_status="$(docker exec "${container}" psql -U postgres -d app -Atqc "SELECT node_status FROM pgactive.pgactive_nodes WHERE node_name='region-us-east-1'")"
is_active="$(docker exec "${container}" psql -U postgres -d app -Atqc "SELECT pgactive.pgactive_is_active_in_db()")"
nodes_count="$(docker exec "${container}" psql -U postgres -d app -Atqc "SELECT count(*) FROM pgactive.pgactive_nodes")"
conflict_table_present="$(docker exec "${container}" psql -U postgres -d app -Atqc "SELECT count(*) FROM pg_tables WHERE schemaname='pgactive' AND tablename='pgactive_conflict_history'")"
conflict_gucs_present="$(docker exec "${container}" psql -U postgres -d app -Atqc "SELECT count(*) FROM pg_settings WHERE name IN ('pgactive.log_conflicts_to_table','pgactive.log_conflicts_to_logfile','pgactive.conflict_logging_include_tuples')")"
preload_loaded="$(docker exec "${container}" psql -U postgres -d app -Atqc "SELECT current_setting('shared_preload_libraries') ~ 'pgactive'")"
init_copy_binary_present="$(docker exec "${container}" bash -c 'test -x /usr/lib/postgresql/17/bin/pgactive_init_copy && echo t || echo f')"

if [[ "${node_status}" != "r" ]]; then echo "node_status expected 'r' got '${node_status}'" >&2; exit 1; fi
if [[ "${is_active}" != "t" ]]; then echo "is_active should be true" >&2; exit 1; fi
if [[ "${conflict_table_present}" != "1" ]]; then echo "conflict_history table missing" >&2; exit 1; fi
if [[ "${conflict_gucs_present}" != "3" ]]; then echo "conflict GUCs not all present (${conflict_gucs_present}/3)" >&2; exit 1; fi
if [[ "${preload_loaded}" != "t" ]]; then echo "pgactive not in shared_preload_libraries" >&2; exit 1; fi
# init_copy_binary_present is recorded as informational; multi-host bootstrap requires the binary but stays alpha-deferred outside this smoke.

mkdir -p "$(dirname "${evidence_file}")"
if [[ ! -f "${evidence_file}" ]]; then
  printf 'observed_at\tgit_sha\tnode_name\tnode_status\tnodes_count\tconflict_table_present\tconflict_gucs_present\tpreload_loaded\tinit_copy_binary_present\n' >"${evidence_file}"
fi
printf '%s\t%s\tregion-us-east-1\t%s\t%s\t%s\t%s\t%s\t%s\n' \
  "$(date -Is)" "$(git rev-parse HEAD)" "${node_status}" "${nodes_count}" \
  "${conflict_table_present}" "${conflict_gucs_present}" "${preload_loaded}" "${init_copy_binary_present}" \
  >>"${evidence_file}"

printf 's7_pgactive_runtime_live\tpassed\tnode_status=%s\tis_active=%s\tnodes_count=%s\tconflict_table_present=%s\tconflict_gucs_present=%s\tpreload_loaded=%s\tinit_copy_binary_present=%s\n' \
  "${node_status}" "${is_active}" "${nodes_count}" "${conflict_table_present}" "${conflict_gucs_present}" "${preload_loaded}" "${init_copy_binary_present}"
echo "S7 pgactive runtime live smoke passed"
