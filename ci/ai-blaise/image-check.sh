#!/usr/bin/env bash
set -euo pipefail

image_dir="images/citus-pg-overlay"
manifest="${image_dir}/extension-manifest.tsv"
dockerfile="${image_dir}/Dockerfile"
load_order="${image_dir}/shared-preload-libraries.conf"
init_sql="${image_dir}/initdb.d/00-ai-blaise-extensions.sql"
image_overview="images/README.ai-blaise.md"
runtime_dockerfile="images/rust-runtime/Dockerfile"
timescale_cohabitation_dockerfile="images/citus-timescale-cohabitation/Dockerfile"
build_app_images="scripts/citus-scale/build-app-images.sh"
dockerignore=".dockerignore"
pool_proxy_smoke="ci/ai-blaise/pool-proxy-smoke.sh"
timescale_bridge_smoke="ci/ai-blaise/timescale-bridge-smoke.sh"
timescale_cohabitation_smoke="ci/ai-blaise/timescale-cohabitation-smoke.sh"
observability_replication_smoke="ci/ai-blaise/observability-replication-smoke.sh"
app_digest_smoke="ci/ai-blaise/app-image-digest-manifest-smoke.sh"

for file in \
  "${dockerignore}" \
  "${manifest}" \
  "${dockerfile}" \
  "${load_order}" \
  "${init_sql}" \
  "${image_overview}" \
  "${image_dir}/README.md" \
  "${runtime_dockerfile}" \
  "${timescale_cohabitation_dockerfile}" \
  "${build_app_images}" \
  "${pool_proxy_smoke}" \
  "${timescale_bridge_smoke}" \
  "${timescale_cohabitation_smoke}" \
  "${observability_replication_smoke}" \
  "${app_digest_smoke}"; do
  if [[ ! -s "${file}" ]]; then
    echo "missing image contract artifact: ${file}" >&2
    exit 1
  fi
done

if [[ ! -x "${build_app_images}" ]]; then
  echo "missing executable app image build matrix: ${build_app_images}" >&2
  exit 1
fi
if [[ ! -x "${timescale_bridge_smoke}" ]]; then
  echo "missing executable Timescale bridge smoke: ${timescale_bridge_smoke}" >&2
  exit 1
fi
if [[ ! -x "${timescale_cohabitation_smoke}" ]]; then
  echo "missing executable Timescale/Citus cohabitation smoke: ${timescale_cohabitation_smoke}" >&2
  exit 1
fi
if [[ ! -x "${observability_replication_smoke}" ]]; then
  echo "missing executable observability replication smoke: ${observability_replication_smoke}" >&2
  exit 1
fi
if [[ ! -x "${app_digest_smoke}" ]]; then
  echo "missing executable app image digest manifest smoke: ${app_digest_smoke}" >&2
  exit 1
fi

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
grep -Fq "FEATURE: Bundle1" "${image_overview}"
grep -Fq "full required binary extension bundle is installed" "${image_overview}"
grep -Fq "build/initdb smoke" "${image_overview}"
grep -Fq "not production evidence" "${image_dir}/README.md"
grep -Fq "every binary package" "${image_dir}/README.md"
grep -Fq "FEATURE: D13" "${runtime_dockerfile}"
if [[ "$(grep -Fc "ARG DEFAULT_ARGS=serve" "${runtime_dockerfile}")" -lt 2 ]]; then
  echo "runtime Dockerfile must declare DEFAULT_ARGS in both builder and runtime stages" >&2
  exit 1
fi
grep -Fq "AI_BLAISE_DEFAULT_ARGS" "${runtime_dockerfile}"
grep -Fq "ai-blaise-entrypoint" "${runtime_dockerfile}"
grep -Fq "FEATURE: D13" "${build_app_images}"
grep -Fq "DIGEST_FILE" "${build_app_images}"
grep -Fq "push_output" "${build_app_images}"
grep -Fq "ai-blaise-image-digests.tsv" "${build_app_images}"
grep -Fq "pushed image" "${build_app_images}"
grep -Fq "did not report an immutable repo digest" "${build_app_images}"
grep -Fq "repository\\timage\\ttag\\tdigest\\tpackage\\tbinary\\tpushed" "${build_app_images}"
grep -Fq 'DEFAULT_ARGS=${default_args}' "${build_app_images}"
grep -Fq "citusctl|ai_blaise_citusctl|ai_blaise_citusctl|plan inspect cluster" "${build_app_images}"
grep -Fq "build-app-images.sh must fail a pushed image without an immutable digest" "${app_digest_smoke}"
grep -Fq "digest manifest must include header plus 20 image rows" "${app_digest_smoke}"
grep -Fq "FAKE_DOCKER_DIGEST_MODE=missing" "${app_digest_smoke}"
grep -Fq "FAKE_DOCKER_PUSH_DIGEST_MODE=missing" "${app_digest_smoke}"
grep -Fq 'cargo build --release -p "${PACKAGE}" --bin "${BIN}"' "${runtime_dockerfile}"
grep -Fq 'ENTRYPOINT ["/usr/local/bin/ai-blaise-entrypoint"]' "${runtime_dockerfile}"
grep -Fq 'CMD []' "${runtime_dockerfile}"
grep -Fxq 'target' "${dockerignore}"
grep -Fxq '.git' "${dockerignore}"

required_app_images=(
  citus-operator
  citus-pool
  citus-sidecar-analytical
  citus-sidecar-auth
  citus-sidecar-backup
  citus-sidecar-cdc
  citus-sidecar-coldtier
  citus-sidecar-edge-functions
  citus-sidecar-graphql
  citus-sidecar-hlc
  citus-sidecar-mcp
  citus-sidecar-postgrest
  citus-sidecar-raft
  citus-sidecar-realtime
  citus-sidecar-repack
  citus-sidecar-schema-job
  citus-sidecar-storage
  citus-sidecar-txn-status
  citus-sidecar-vectorizer
  citusctl
)

for app_image in "${required_app_images[@]}"; do
  if ! grep -Fq "\"${app_image}|" "${build_app_images}"; then
    echo "missing app image from build matrix: ${app_image}" >&2
    exit 1
  fi
done

required_serve_mains=(
  operator/src/main.rs
  sidecar/analytical/src/main.rs
  sidecar/auth/src/main.rs
  sidecar/backup/src/main.rs
  sidecar/cdc/src/main.rs
  sidecar/coldtier/src/main.rs
  sidecar/edge_functions/src/main.rs
  sidecar/graphql/src/main.rs
  sidecar/hlc/src/main.rs
  sidecar/mcp/src/main.rs
  sidecar/postgrest/src/main.rs
  sidecar/raft/src/main.rs
  sidecar/realtime/src/main.rs
  sidecar/repack/src/main.rs
  sidecar/schema_job/src/main.rs
  sidecar/storage/src/main.rs
  sidecar/txn_status/src/main.rs
  sidecar/vectorizer/src/main.rs
)

for main_file in "${required_serve_mains[@]}"; do
  grep -Fq 'args == ["serve"]' "${main_file}"
  grep -Fq 'run_probe_server' "${main_file}"
done

grep -Fq 'args == ["serve"]' pool/src/main.rs
grep -Fq 'run_pool_service_from_env' pool/src/main.rs
grep -Fq 'AI_BLAISE_POOL_UPSTREAM_ADDR' pool/src/proxy.rs
grep -Fq 'handle_proxy_connection' pool/src/proxy.rs
grep -Fq 'ai_blaise_citus_pool_upstream_ready' pool/src/proxy.rs
grep -Fq 'psql -h 127.0.0.1 -p "${pool_port}"' "${pool_proxy_smoke}"

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
grep -Fq "CREATE FUNCTION apply_distribute_hypertable" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION add_compression_policy_distributed" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION apply_compression_policy_distributed" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "WITH NO DATA" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION time_range_shard_pruner" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE TABLE IF NOT EXISTS companion_internal.timescale_bridge_state" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE VIEW companion_timescale_bridge_state" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE VIEW companion_pg_stat_statements_p95" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_idle_transactions" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "FEATURE: TS18" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST" "${pool_proxy_smoke}"
grep -Fq "ai_blaise_citus_pool_rejected_connections_total" "${pool_proxy_smoke}"
grep -Fq "pool CIDR deny smoke unexpectedly allowed PostgreSQL traffic" "${pool_proxy_smoke}"
grep -Fq "PostgreSQL init process complete" "${pool_proxy_smoke}"
grep -Fq 'docker exec -i "${container}" psql' ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "shared_preload_libraries=pg_stat_statements" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "PostgreSQL init process complete" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "ai_blaise_pg_stat_statements_seed" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "timescale_bridge_call_log" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "apply_distribute_hypertable" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "apply_retention_policy_distributed" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "apply_reorder_policy_distributed" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "apply_time_range_shard_pruner" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "apply_compression_policy_distributed must require TimescaleDB dependency" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "apply_continuous_aggregate_distributed must require TimescaleDB dependency" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_idle_transactions('100 milliseconds'::interval)" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "timescale/timescaledb:latest-pg17" "${timescale_bridge_smoke}"
grep -Fq "PostgreSQL init process complete" "${timescale_bridge_smoke}"
grep -Fq "CREATE EXTENSION IF NOT EXISTS timescaledb" "${timescale_bridge_smoke}"
grep -Fq "CREATE FUNCTION create_distributed_table" "${timescale_bridge_smoke}"
grep -Fq "SELECT apply_distribute_hypertable" "${timescale_bridge_smoke}"
grep -Fq "SELECT apply_compression_policy_distributed" "${timescale_bridge_smoke}"
grep -Fq "SELECT apply_retention_policy_distributed" "${timescale_bridge_smoke}"
grep -Fq "SELECT apply_reorder_policy_distributed" "${timescale_bridge_smoke}"
grep -Fq "SELECT apply_continuous_aggregate_distributed" "${timescale_bridge_smoke}"
grep -Fq "SELECT apply_time_range_shard_pruner" "${timescale_bridge_smoke}"
grep -Fq "_timescaledb_catalog.hypertable" "${timescale_bridge_smoke}"
grep -Fq "companion_timescale_bridge_state" "${timescale_bridge_smoke}"
grep -Fq "FEATURE: TS6 TS18" "${timescale_cohabitation_dockerfile}"
grep -Fq "timescale/timescaledb:latest-pg17" "${timescale_cohabitation_dockerfile}"
grep -Fq "make install" "${timescale_cohabitation_dockerfile}"
grep -Fq "ai_blaise_citus--0.1.0.sql" "${timescale_cohabitation_dockerfile}"
grep -Fq "FEATURE: TS6 TS18" "${timescale_cohabitation_smoke}"
grep -Fq "TIMESCALE_COHABITATION_BASE_IMAGE" "${timescale_cohabitation_smoke}"
grep -Fq "shared_preload_libraries=timescaledb,citus" "${timescale_cohabitation_smoke}"
grep -Fq "citus.cohabit_extensions=timescaledb" "${timescale_cohabitation_smoke}"
grep -Fq "CREATE EXTENSION IF NOT EXISTS citus" "${timescale_cohabitation_smoke}"
grep -Fq "CREATE EXTENSION IF NOT EXISTS timescaledb" "${timescale_cohabitation_smoke}"
grep -Fq "CREATE EXTENSION IF NOT EXISTS ai_blaise_citus" "${timescale_cohabitation_smoke}"
grep -Fq "SELECT create_distributed_table('citus_smoke_events', 'tenant_id')" "${timescale_cohabitation_smoke}"
grep -Fq "SELECT apply_distribute_hypertable" "${timescale_cohabitation_smoke}"
grep -Fq "SELECT apply_compression_policy_distributed" "${timescale_cohabitation_smoke}"
grep -Fq "SELECT apply_retention_policy_distributed" "${timescale_cohabitation_smoke}"
grep -Fq "SELECT apply_reorder_policy_distributed" "${timescale_cohabitation_smoke}"
grep -Fq "SELECT apply_continuous_aggregate_distributed" "${timescale_cohabitation_smoke}"
grep -Fq "SELECT apply_time_range_shard_pruner" "${timescale_cohabitation_smoke}"
grep -Fq "pg_dist_partition" "${timescale_cohabitation_smoke}"
grep -Fq "expected six Timescale bridge feature ids" "${timescale_cohabitation_smoke}"
grep -Fq "timescale-cohabitation-evidence.tsv" "${timescale_cohabitation_smoke}"
if grep -Fq "CREATE FUNCTION create_distributed_table" "${timescale_cohabitation_smoke}"; then
  echo "real Timescale/Citus cohabitation smoke must not stub create_distributed_table" >&2
  exit 1
fi
grep -Fq "citus.cohabit_extensions" src/backend/distributed/shared_library_init.c
grep -Fq "ErrorIfHooksAlreadyRegistered" src/backend/distributed/shared_library_init.c
grep -Fq "IsTrustedHookCoextension" src/backend/distributed/shared_library_init.c
grep -Fq 'pg_strcasecmp(coextensionName, "timescaledb")' src/backend/distributed/shared_library_init.c
grep -Fq "PreviousPlannerHook = planner_hook" src/backend/distributed/shared_library_init.c
grep -Fq "PreviousExecutorStartHook = ExecutorStart_hook" src/backend/distributed/shared_library_init.c
grep -Fq "PreviousExecutorRunHook = ExecutorRun_hook" src/backend/distributed/shared_library_init.c
grep -Fq "PreviousExplainOneQueryHook = ExplainOneQuery_hook" src/backend/distributed/shared_library_init.c
grep -Fq "CallPreviousPlannerHook" src/backend/distributed/planner/distributed_planner.c
grep -Fq "RunPreviousExecutorStartHook" src/backend/distributed/executor/multi_executor.c
grep -Fq "RunPreviousExecutorRunHook" src/backend/distributed/executor/multi_executor.c
grep -Fq "PreviousExplainOneQueryHook" src/backend/distributed/planner/multi_explain.c
grep -Fq "wal_level=replica" "${observability_replication_smoke}"
grep -Fq "pg_basebackup" "${observability_replication_smoke}"
grep -Fq "PostgreSQL init process complete" "${observability_replication_smoke}"
grep -Fq "pg_is_in_recovery()" "${observability_replication_smoke}"
grep -Fq "companion_pg_stat_local_activity" "${observability_replication_smoke}"
grep -Fq "companion_pg_stat_distributed" "${observability_replication_smoke}"
grep -Fq "companion_pg_dist_replication_lag" "${observability_replication_smoke}"
grep -Fq "state = 'streaming'" "${observability_replication_smoke}"

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
