#!/usr/bin/env bash
set -euo pipefail

image_dir="images/citus-pg-overlay"
manifest="${image_dir}/extension-manifest.tsv"
dockerfile="${image_dir}/Dockerfile"
load_order="${image_dir}/shared-preload-libraries.conf"
init_sql="${image_dir}/initdb.d/00-ai-blaise-extensions.sql"

for file in "${manifest}" "${dockerfile}" "${load_order}" "${init_sql}" "${image_dir}/README.md"; do
  if [[ ! -s "${file}" ]]; then
    echo "missing image contract artifact: ${file}" >&2
    exit 1
  fi
done

required_extensions=(
  timescaledb citus pgvector pg_cron pg_partman pgaudit pgauditlogtofile
  ai_blaise_citus pgsodium hll topn tdigest pgnodemx postgis pg_search pg_graphql
  pg_jsonschema age plrust plv8 pg_uuidv7 pg_repack pg_failover_slots
  pg_warm pgcrypto pg_trgm citext rum
)

optional_extensions=(
  hypopg pg_qualstats pg_stat_kcache pg_wait_sampling pgsentinel pgsql-http
  pg_net pgl_ddl_deploy pg_track_settings pg_lake pg_duckdb pgactive
  pg_subscription_pg_failover
  oracle_fdw mysql_fdw mongo_fdw tds_fdw pgmq pgque pg_parquet pg_squeeze
  pg_show_plans pg_stat_monitor pg_walinspect pg_safeupdate anon vchord
  pg_hint_plan sr_plan pgledger pglinter omnigres
)

hard_blocked_extensions=(
  orioledb pg_cryogen undam append_only_heap imcs hydra_columnar pg_strom
  vops vectorize_engine pg_pathman mmts pg_dtm pg_tsdtm pg_shardman gogudb
  plproxy postgres_fdw_plus
)

manifest_has() {
  local name="$1"
  local tier="$2"
  awk -F'|' -v name="${name}" -v tier="${tier}" \
    'BEGIN { found = 0 } !/^#/ && $1 == name && $2 == tier { found = 1 } END { exit found ? 0 : 1 }' \
    "${manifest}"
}

for extension in "${required_extensions[@]}"; do
  if ! manifest_has "${extension}" "required"; then
    echo "required extension missing from manifest: ${extension}" >&2
    exit 1
  fi

  if ! grep -Fq "CREATE EXTENSION IF NOT EXISTS ${extension};" "${init_sql}"; then
    echo "required extension missing from init SQL: ${extension}" >&2
    exit 1
  fi
done

for extension in "${optional_extensions[@]}"; do
  if ! manifest_has "${extension}" "optional"; then
    echo "optional extension missing from manifest: ${extension}" >&2
    exit 1
  fi
done

for extension in "${hard_blocked_extensions[@]}"; do
  if ! manifest_has "${extension}" "hard-block"; then
    echo "hard-blocked extension missing from manifest: ${extension}" >&2
    exit 1
  fi
done

required_count="$(awk -F'|' '!/^#/ && $2 == "required" { count++ } END { print count + 0 }' "${manifest}")"
optional_count="$(awk -F'|' '!/^#/ && $2 == "optional" { count++ } END { print count + 0 }' "${manifest}")"
hard_block_count="$(awk -F'|' '!/^#/ && $2 == "hard-block" { count++ } END { print count + 0 }' "${manifest}")"

if [[ "${required_count}" -ne "${#required_extensions[@]}" ]]; then
  echo "required extension count mismatch: manifest=${required_count} expected=${#required_extensions[@]}" >&2
  exit 1
fi

if [[ "${optional_count}" -ne "${#optional_extensions[@]}" ]]; then
  echo "optional extension count mismatch: manifest=${optional_count} expected=${#optional_extensions[@]}" >&2
  exit 1
fi

if [[ "${hard_block_count}" -ne "${#hard_blocked_extensions[@]}" ]]; then
  echo "hard-block extension count mismatch: manifest=${hard_block_count} expected=${#hard_blocked_extensions[@]}" >&2
  exit 1
fi

grep -Fq "shared_preload_libraries = 'citus,timescaledb,pgvector,pgaudit,pgsodium,pg_cron,age,plrust,companion,pg_hint_plan,sr_plan'" "${load_order}"
grep -Fq "citus.cohabit_extensions = 'timescaledb'" "${load_order}"
grep -Fq "COPY extension-manifest.tsv" "${dockerfile}"
grep -Fq "COPY extensions/ai_blaise_citus.control" "${dockerfile}"
grep -Fq "COPY extensions/ai_blaise_citus--0.1.0.sql" "${dockerfile}"
grep -Fq "00-ai-blaise-extensions.sql" "${dockerfile}"

for file in \
  "${image_dir}/extensions/ai_blaise_citus.control" \
  "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"; do
  if [[ ! -s "${file}" ]]; then
    echo "missing companion SQL extension artifact: ${file}" >&2
    exit 1
  fi
done

grep -Fq "CREATE FUNCTION companion_feature_status()" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_distribute_hypertable_plan" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION distribute_hypertable" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION add_compression_policy_distributed" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"

if grep -RIn "'planned'\\|planned" "${image_dir}/extensions"; then
  echo "companion SQL extension must not expose planned feature statuses" >&2
  exit 1
fi

if grep -RIn "^\\\\" "${image_dir}/extensions"; then
  echo "companion SQL extension files must be server-executable SQL, not psql meta-command scripts" >&2
  exit 1
fi

if grep -RIn "timescaledb.*/tsl\\|/tsl/" "${image_dir}"; then
  echo "image contract must not vendor or patch Timescale TSL source" >&2
  exit 1
fi
