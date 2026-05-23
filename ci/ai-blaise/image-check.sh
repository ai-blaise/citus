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
  if [[ "${main_file}" == "sidecar/mcp/src/main.rs" ]]; then
    grep -Fq 'serve_mcp_sidecar_http_forever' "${main_file}"
    grep -Fq 'handle_mcp_sidecar_http_bytes' sidecar/mcp/src/lib.rs
    grep -Fq 'GET /healthz' sidecar/mcp/src/lib.rs
    grep -Fq 'request("GET", "/readyz")' ci/ai-blaise/mcp-sidecar-http-smoke.sh
    grep -Fq 'request("GET", "/metrics")' ci/ai-blaise/mcp-sidecar-http-smoke.sh
  else
    if grep -Fq 'run_probe_server' "${main_file}"; then
      :
    else
      runtime_file="${main_file%/main.rs}/runtime.rs"
      grep -Fq 'SidecarRuntime' "${runtime_file}"
      grep -Fq 'route("/healthz"' "${runtime_file}"
      grep -Fq 'route("/readyz"' "${runtime_file}"
      grep -Fq 'route("/metrics"' "${runtime_file}"
      grep -Fq 'route("/drain"' "${runtime_file}"
    fi
  fi
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
grep -Fq "CREATE FUNCTION companion_set_session_claims" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_current_session_claims" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_current_tenant_id" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE TABLE IF NOT EXISTS companion_internal.plan_freezes" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE TABLE IF NOT EXISTS companion_internal.plan_promotion_policies" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE TABLE IF NOT EXISTS companion_internal.plan_regression_policies" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE TABLE IF NOT EXISTS companion_internal.plan_regression_samples" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE VIEW companion_plan_freezes" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.plan_freeze" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.plan_auto_promote" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.plan_regression_guard" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_plan_regression_violates" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE TABLE IF NOT EXISTS companion_internal.migration_runs" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE TABLE IF NOT EXISTS companion_internal.migration_operations" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE VIEW companion_migration_runs" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE VIEW companion_migration_operations" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.migrate_start" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.migration_add_column" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.migration_online_type_change" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.migrate_complete" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE TABLE IF NOT EXISTS companion_internal.index_advisor_candidates" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE VIEW companion_index_advisor_candidates" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.index_advisor_record_candidate" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_index_advisor_ranked" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE TABLE IF NOT EXISTS companion_internal.webhook_registrations" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE TABLE IF NOT EXISTS companion_internal.webhook_triggers" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE TABLE IF NOT EXISTS companion_internal.webhook_events" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE VIEW companion_webhook_registrations" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE VIEW companion_webhook_events" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.webhook_register" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.install_webhook_trigger" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE TABLE IF NOT EXISTS companion_internal.shard_placement_generations" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE VIEW companion_shard_placement_generations" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.bump_placement_generation" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_placement_generation" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_local_placement_matches" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_hash_shard_index" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_range_shard_index" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.base64url_encode" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.base64url_decode" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.jwt_audience_matches" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_verify_jwt_hs256" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_require_tenant_id" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_tenant_id_matches(row_tenant_id text)" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_tenant_id_matches(row_tenant_id uuid)" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE TABLE IF NOT EXISTS companion_internal.ledger_entries" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE TABLE IF NOT EXISTS companion_internal.ledger_seals" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.ledger_transfer" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_ledger_chain_valid" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_ledger_seal" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE VIEW companion_ledger_entries" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'Sec5', 'immutable ledger', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'Sec6', 'ledger HMAC tamper evidence', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'Auth2', 'tenant-aware claims', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'PM3', 'plan freeze companion module', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'PM4', 'plan regression detection', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'M1', 'pgroll-style expand-contract migrations', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'M11', 'online column-type migration', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'IA3', 'companion index advisor', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'WH2', 'companion webhook helpers', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'Search2', 'distributed BM25 search index', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'Search3', 'hybrid BM25 and vector ranking', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'Search9', 'reranker UDF plan', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'G2', 'distributed graph bridge', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'G3', 'graph colocation policy', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'API4', 'GraphQL distributed graph metadata', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'JS2', 'distributed JSON Schema validation', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'M13', 'JSON Schema validation triggers', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'Geo2', 'geo-aware distribution', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'Geo3', 'geo shard pruning', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'A1', 'pgai-compatible vectorizer DSL', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'TS9', 'doctor rules for cohabitation', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'M7', 'pre-flight cohabit-extension check', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'T8', 'toolkit two-step aggregate pushdown', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'L9', 'worker partial aggregate pushdown', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'TS13', 'distributed time_bucket_gapfill', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'TS14', 'distributed metric toolkit aggregates', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'TS15', 'distributed approximate toolkit aggregates', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'TS16', 'distributed downsampler toolkit aggregates', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'TS17', 'distributed state toolkit aggregates', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'C10', 'online schema job state machine', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'M2', 'gh-ost-style online DDL', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'S14', 'tenant migration online', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'TO3', 'tenant migration online', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'TO4', 'tenant archive', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'TO5', 'tenant region affinity', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.register_search_index" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.hybrid_rank" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.rerank_search" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.ensure_graph_colocation" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.register_graphql_distributed_graph" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.register_json_schema" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.install_jsonschema_trigger" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.add_geohash_column" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.enable_geo_shard_pruning" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.register_vectorizer" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.vectorizer_enqueue" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.assert_shared_preload_libraries" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.get_violations" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.register_toolkit_aggregate_plan" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.schema_job_start" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.schema_job_advance" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.plan_tenant_move" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.plan_tenant_archive" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.set_tenant_region_affinity" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE TABLE IF NOT EXISTS companion_internal.extension_catalog_contracts" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE VIEW companion_extension_catalog" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE VIEW companion_extension_feature_coverage" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.register_extension_contract" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.seed_extension_catalog" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_extension_required" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_required_preload_libraries" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_extension_conflicts" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.assert_extension_allowed" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'A7', 'pgvector cohabitation', 'extension-catalog-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'Search1', 'pg_search bundled', 'extension-catalog-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'Sec15', 'encryption-at-rest with CMK', 'extension-catalog-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'S6', 'placement generation helpers', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'S13', 'range routing helpers', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'Sec1', 'RLS helpers', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'Sec2', 'JWT verification UDF', 'sql-runtime'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "FEATURE: TS18" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST" "${pool_proxy_smoke}"
grep -Fq "ai_blaise_citus_pool_requests_total" "${pool_proxy_smoke}"
grep -Fq "ai_blaise_citus_pool_rejected_connections_total" "${pool_proxy_smoke}"
grep -Fq "pool CIDR deny smoke unexpectedly allowed PostgreSQL traffic" "${pool_proxy_smoke}"
grep -Fq "PostgreSQL init process complete" "${pool_proxy_smoke}"
grep -Fq "raw PostgreSQL pipelined simple-query smoke passed through pool proxy" "${pool_proxy_smoke}"
grep -Fq "pack_simple_query(\"SELECT 'pipeline_one'::text\")" "${pool_proxy_smoke}"
grep -Fq "pack_simple_query(\"SELECT 'pipeline_two'::text\")" "${pool_proxy_smoke}"
grep -Fq 'expected = [["pipeline_one"], ["pipeline_two"]]' "${pool_proxy_smoke}"
grep -Fq 'docker exec -i "${container}" psql' ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "shared_preload_libraries=pg_stat_statements" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "PostgreSQL init process complete" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "CREATE EXTENSION pgcrypto" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "ai_blaise_pg_stat_statements_seed" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_set_session_claims" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_current_session_claims" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_current_tenant_id" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_internal.plan_freeze" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_internal.plan_auto_promote" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_internal.plan_regression_guard" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_plan_regression_violates" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_plan_freezes" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "PM3 plan freeze state was not visible with policy metadata" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "PM4 regression guard did not flag latency regression" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "PM4 regression guard flagged an allowed candidate" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "PM4 regression samples were not recorded" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "PM3 plan_freeze accepted an empty query hash" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "PM4 regression guard accepted an unknown frozen plan" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_internal.migrate_start" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_internal.migration_add_column" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_internal.migration_online_type_change" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_migration_runs" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "M1 migration run was not completed and visible" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "M1/M11 migration operations were not recorded" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "M11 online type-change accepted identical types" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_internal.index_advisor_record_candidate" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_index_advisor_ranked" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "IA3 ranked advisor did not render CREATE INDEX CONCURRENTLY SQL" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "IA3 accepted a non-improving candidate" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_internal.webhook_register" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_internal.install_webhook_trigger" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_webhook_events" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "WH2 webhook trigger did not enqueue INSERT and UPDATE rows" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "WH2 accepted a non-http webhook URL" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_internal.bump_placement_generation" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_placement_generation" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_local_placement_matches" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_hash_shard_index" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_range_shard_index" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "S6 placement generation did not advance from 1 to 2" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "S6 unknown shard should return generation zero" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "S13 hash routing helper was not deterministic and bounded" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "S13 range routing helper returned" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "S13 range routing helper accepted an out-of-bounds value" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_verify_jwt_hs256" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "Sec2 JWT verification did not return expected claims" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "Sec2 verified JWT claims did not feed Auth2 tenant claims" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "Sec2 JWT verification accepted a bad signature" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "Sec2 JWT verification accepted a wrong audience" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "Sec2 JWT verification accepted an expired token" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "Sec2 JWT verification accepted a missing tenant_id claim" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_require_tenant_id" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_tenant_id_matches" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "ALTER TABLE rls_smoke_orders ENABLE ROW LEVEL SECURITY" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "CREATE POLICY rls_smoke_tenant_isolation" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "SET ROLE ai_blaise_rls_smoke" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "Sec1 RLS WITH CHECK allowed a cross-tenant insert" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_internal.ledger_transfer" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_ledger_chain_valid" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_ledger_seal" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "Sec5 ledger transfer did not return a sha256 entry hash" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "Sec5 ledger entries must reject mutation" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "Sec6 ledger seals must reject deletion" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "Sec6 ledger seal accepted an unsupported algorithm" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "uid claim must not be empty" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_internal.seed_extension_catalog" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_extension_feature_coverage" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_extension_required('A7')" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "companion_required_preload_libraries" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "extension catalog hard-block conflict check did not flag orioledb" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "extension catalog accepted empty feature ids" ci/ai-blaise/sql-extension-smoke.sh
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
grep -Fq "stable image identity" "${timescale_cohabitation_smoke}"
grep -Fq "git_sha" "${timescale_cohabitation_smoke}"
grep -Fq "command_path" "${timescale_cohabitation_smoke}"
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
