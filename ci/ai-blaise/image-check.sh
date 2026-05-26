#!/usr/bin/env bash
set -euo pipefail

image_dir="images/citus-pg-overlay"
manifest="${image_dir}/extension-manifest.tsv"
upgrade_manifest="${image_dir}/extensions/ai_blaise_citus-upgrade-manifest.tsv"
dockerfile="${image_dir}/Dockerfile"
load_order="${image_dir}/shared-preload-libraries.conf"
init_sql="${image_dir}/initdb.d/00-ai-blaise-extensions.sql"
image_overview="images/README.ai-blaise.md"
runtime_dockerfile="images/rust-runtime/Dockerfile"
timescale_cohabitation_dockerfile="images/citus-timescale-cohabitation/Dockerfile"
pg_cron_cohabitation_dockerfile="images/citus-pg-cron-cohabitation/Dockerfile"
build_app_images="scripts/citus-scale/build-app-images.sh"
dockerignore=".dockerignore"
pool_proxy_smoke="ci/ai-blaise/pool-proxy-smoke.sh"
timescale_bridge_smoke="ci/ai-blaise/timescale-bridge-smoke.sh"
timescale_cohabitation_smoke="ci/ai-blaise/timescale-cohabitation-smoke.sh"
pg_cron_cohabitation_smoke="ci/ai-blaise/pg-cron-cohabitation-smoke.sh"
ts_version_matrix_smoke="ci/ai-blaise/ts-version-matrix-smoke.sh"
cohab_matrix_dir="tests/cohab-matrix"
cohab_matrix_compare="${cohab_matrix_dir}/compare-hook-claims.sh"
observability_replication_smoke="ci/ai-blaise/observability-replication-smoke.sh"
app_digest_smoke="ci/ai-blaise/app-image-digest-manifest-smoke.sh"
observability_contracts_check="ci/ai-blaise/observability-contracts-check.sh"
ai_sql_contract_smoke="ci/ai-blaise/ai-sql-contract-smoke.sh"
bundle1_contract_check="ci/ai-blaise/bundle1-contract-check.py"
bundle1_source_lock="${image_dir}/bundle1-source-build.lock.tsv"
security_supply_chain_smoke="ci/ai-blaise/security-supply-chain-smoke.sh"
operator_reconcilers_batch_c_smoke="ci/ai-blaise/operator-reconcilers-batch-c-smoke.sh"
companion_runtime_depth_a_smoke="ci/ai-blaise/companion-runtime-depth-a-smoke.sh"

for file in \
  "${dockerignore}" \
  "${manifest}" \
  "${upgrade_manifest}" \
  "${dockerfile}" \
  "${load_order}" \
  "${init_sql}" \
  "${image_overview}" \
  "${image_dir}/README.md" \
  "${runtime_dockerfile}" \
  "${timescale_cohabitation_dockerfile}" \
  "${pg_cron_cohabitation_dockerfile}" \
  "${build_app_images}" \
  "${pool_proxy_smoke}" \
  "${timescale_bridge_smoke}" \
  "${timescale_cohabitation_smoke}" \
  "${pg_cron_cohabitation_smoke}" \
  "${ts_version_matrix_smoke}" \
  "${cohab_matrix_compare}" \
  "${cohab_matrix_dir}/README.md" \
  "${cohab_matrix_dir}/2.27/expected-hook-claims.tsv" \
  "${cohab_matrix_dir}/2.27/image-tag.txt" \
  "${cohab_matrix_dir}/2.27/notes.md" \
  "${cohab_matrix_dir}/2.28/expected-hook-claims.tsv" \
  "${cohab_matrix_dir}/2.28/image-tag.txt" \
  "${cohab_matrix_dir}/2.28/notes.md" \
  "${observability_replication_smoke}" \
  "${app_digest_smoke}" \
  "${observability_contracts_check}" \
  "${ai_sql_contract_smoke}" \
  "${bundle1_contract_check}" \
  "${bundle1_source_lock}" \
  "${security_supply_chain_smoke}" \
  "${operator_reconcilers_batch_c_smoke}" \
  "${companion_runtime_depth_a_smoke}"; do
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
if [[ ! -x "${pg_cron_cohabitation_smoke}" ]]; then
  echo "missing executable pg_cron cohabitation smoke: ${pg_cron_cohabitation_smoke}" >&2
  exit 1
fi
if [[ ! -x "${ts_version_matrix_smoke}" ]]; then
  echo "missing executable TS-version matrix smoke: ${ts_version_matrix_smoke}" >&2
  exit 1
fi
if [[ ! -x "${cohab_matrix_compare}" ]]; then
  echo "missing executable TS-version matrix comparator: ${cohab_matrix_compare}" >&2
  exit 1
fi
if [[ ! -x "${observability_replication_smoke}" ]]; then
  echo "missing executable observability replication smoke: ${observability_replication_smoke}" >&2
  exit 1
fi
if [[ ! -x "${ai_sql_contract_smoke}" ]]; then
  echo "missing executable AI SQL contract smoke: ${ai_sql_contract_smoke}" >&2
  exit 1
fi
if [[ ! -x "${app_digest_smoke}" ]]; then
  echo "missing executable app image digest manifest smoke: ${app_digest_smoke}" >&2
  exit 1
fi
if [[ ! -x "${observability_contracts_check}" ]]; then
  echo "missing executable observability contracts smoke: ${observability_contracts_check}" >&2
  exit 1
fi
if [[ ! -x "${security_supply_chain_smoke}" ]]; then
  echo "missing executable security supply-chain smoke: ${security_supply_chain_smoke}" >&2
  exit 1
fi
if [[ ! -x "${operator_reconcilers_batch_c_smoke}" ]]; then
  echo "missing executable operator reconcilers batch C smoke: ${operator_reconcilers_batch_c_smoke}" >&2
  exit 1
fi
if [[ ! -x "${companion_runtime_depth_a_smoke}" ]]; then
  echo "missing executable companion runtime depth A smoke: ${companion_runtime_depth_a_smoke}" >&2
  exit 1
fi

required_extensions=(
  timescaledb citus vector pg_cron pg_partman pgaudit pgauditlogtofile
  ai_blaise_citus pgsodium hll topn tdigest pgnodemx postgis pg_search pg_graphql
  pg_jsonschema age plv8 pg_uuidv7 pg_repack pg_failover_slots
  pg_warm pgcrypto pg_trgm citext rum
)

optional_extensions=(
  hypopg pg_qualstats pg_stat_kcache pg_wait_sampling pgsentinel pgsql-http
  pg_net pgl_ddl_deploy pg_track_settings pg_lake pg_duckdb pgactive
  pg_subscription_pg_failover
  oracle_fdw mysql_fdw mongo_fdw tds_fdw pgmq pgque pg_parquet pg_squeeze
  pg_show_plans pg_stat_monitor pg_walinspect pg_safeupdate anon vchord
  pg_hint_plan sr_plan pgledger pglinter omnigres
  plrust
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

# pg_failover_slots is shared_preload_libraries-only; no SQL extension surface.
preload_only_extensions=(pg_failover_slots)
is_preload_only() {
  local needle="$1" item
  for item in "${preload_only_extensions[@]}"; do
    if [[ "${item}" == "${needle}" ]]; then return 0; fi
  done
  return 1
}

for extension in "${required_extensions[@]}"; do
  if ! manifest_has "${extension}" "required"; then
    echo "required extension missing from manifest: ${extension}" >&2
    exit 1
  fi

  if is_preload_only "${extension}"; then
    continue
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

grep -Fq "shared_preload_libraries = 'citus,timescaledb,pgaudit,pgauditlogtofile,pgsodium,pg_cron,age,pg_failover_slots,pgnodemx'" "${load_order}"
grep -Fq "citus.cohabit_extensions = 'timescaledb,pg_cron'" "${load_order}"
grep -Fq "COPY images/citus-pg-overlay/extension-manifest.tsv" "${dockerfile}"
grep -Fq "COPY images/citus-pg-overlay/extensions/ai_blaise_citus.control" "${dockerfile}"
grep -Fq "COPY images/citus-pg-overlay/extensions/ai_blaise_citus-upgrade-manifest.tsv" "${dockerfile}"
grep -Fq "COPY images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql" "${dockerfile}"
grep -Fq "COPY images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0--0.1.1.sql" "${dockerfile}"
grep -Fq "COPY images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.1--0.1.0.sql" "${dockerfile}"
grep -Fq "00-ai-blaise-extensions.sql" "${dockerfile}"
grep -Fq "COPY images/citus-pg-overlay/extensions/pg_warm.control" "${dockerfile}"
grep -Fq "COPY images/citus-pg-overlay/extensions/pg_warm--0.1.0.sql" "${dockerfile}"
grep -Fq "COPY images/citus-pg-overlay/bin/pgsodium_getkey" "${dockerfile}"
grep -Fq 'install -m 0755 /usr/local/share/ai-blaise/citus/pgsodium_getkey "/usr/share/postgresql/${PG_MAJOR}/extension/pgsodium_getkey"' "${dockerfile}"
# Bundle1 source-build path: PGDG-missing PG17 extensions must have pinned
# builder stages or an explicit alpha boundary when upstream PG17 is blocked.
bundle1_source_build_stages=(
  build-pgsodium
  build-topn
  build-pg-jsonschema
  build-pg-graphql
  build-pg-search
  build-plv8
  build-citus
)
for stage in "${bundle1_source_build_stages[@]}"; do
  if ! grep -Fq "AS ${stage}" "${dockerfile}"; then
    echo "bundle1 source-build stage missing from Dockerfile: ${stage}" >&2
    exit 1
  fi
done
grep -Fq "ARG PGSODIUM_TAG=v3.1.9" "${dockerfile}"
grep -Fq "ARG PGSODIUM_REF=7222ebc5ed87084a68d526aef977be0f4eb319a2" "${dockerfile}"
grep -Fq "ARG TOPN_TAG=v2.7.0" "${dockerfile}"
grep -Fq "ARG TOPN_REF=f636ff1b3586025c81fb84c20483412f3991ed84" "${dockerfile}"
grep -Fq "ARG PG_JSONSCHEMA_TAG=v0.3.4" "${dockerfile}"
grep -Fq "ARG PG_JSONSCHEMA_REF=cbe74b570d38aa0c4d42914e7a118bcb3adaee7a" "${dockerfile}"
grep -Fq "ARG PG_GRAPHQL_TAG=v1.6.1" "${dockerfile}"
grep -Fq "ARG PG_GRAPHQL_REF=66d4c551db213000506fd858676269ba8f801a44" "${dockerfile}"
grep -Fq "ARG PG_SEARCH_TAG=v0.20.11" "${dockerfile}"
grep -Fq "ARG PG_SEARCH_REF=cd1ba46a116c5a98bd6fe9ae370a2f260aee1394" "${dockerfile}"
grep -Fq "ARG PLRUST_TAG=v1.2.8" "${dockerfile}"
grep -Fq "ARG PLRUST_REF=bd76906a43c05a2afdb7839263431a066f5b42fb" "${dockerfile}"
grep -Fq "alpha-upstream-pg17-blocked" "${dockerfile}"
grep -Fq "ARG PLV8_TAG=v3.2.4" "${dockerfile}"
grep -Fq "ARG PLV8_REF=cafc37f7aee850de5478773a4e56f7fadfad8e00" "${dockerfile}"
grep -Fq "ARG CITUS_TAG=v13.3.0" "${dockerfile}"
grep -Fq "AI_BLAISE_SOURCE_GIT_SHA" "${dockerfile}"
grep -Fq "ai-blaise.citus.source-git-sha" "${dockerfile}"
grep -Fq "ai-blaise.citus.source-tree-state" "${dockerfile}"
grep -Fq "bundle1-source-build.lock.tsv" "${dockerfile}"
grep -Fq "full-bundle-required-minus-plrust" "${dockerfile}"
python3 "${bundle1_contract_check}"
grep -Fq "AS bundle1-final-light" "${dockerfile}"
grep -Fq "AS bundle1-final-full" "${dockerfile}"
grep -Fq "BUNDLE1_BUILD_IMAGE" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "BUNDLE1_BUILD_HEAVY" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "BUNDLE1_EVIDENCE_FILE" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "PGSODIUM_KEY=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef" ci/ai-blaise/sql-extension-smoke.sh
grep -Fq "source-build-deferred|EF6|none" "${manifest}"
grep -Fq "local pg_prewarm-backed shim" "${manifest}"
grep -Fq "FEATURE: Bundle1" "${image_overview}"
grep -Fq "full required binary extension bundle is installed" "${image_overview}"
grep -Fq "build/initdb smoke" "${image_overview}"
grep -Fq "not production evidence" "${image_dir}/README.md"
grep -Fq "ai_blaise_citus-upgrade-manifest.tsv" "${image_dir}/README.md"
grep -Fq "bundle1-source-build.lock.tsv" "${image_dir}/README.md"
grep -Fq "structured Bundle1 contract check" "${image_dir}/README.md"
grep -Fq "every required extension" "${image_dir}/README.md"
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
grep -Fq "observability-contracts-check.sh" .github/workflows/ci-observability-contracts.yml
grep -Fq "log-schema-canonical" sidecar/shared/src/main.rs
grep -Fq "serve_surfaces=" "${observability_contracts_check}"
grep -Fq "sidecar_log_schemas=" "${observability_contracts_check}"
grep -Fq "pool_admin_metrics=true" "${observability_contracts_check}"
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

custom_probe_contract_file() {
  case "${1}" in
    sidecar/auth/src/main.rs) echo sidecar/auth/src/http.rs ;;
    sidecar/cdc/src/main.rs) echo sidecar/cdc/src/runtime.rs ;;
    sidecar/edge_functions/src/main.rs) echo sidecar/edge_functions/src/lib.rs ;;
    sidecar/graphql/src/main.rs) echo sidecar/graphql/src/lib.rs ;;
    sidecar/hlc/src/main.rs) echo sidecar/hlc/src/main.rs ;;
    sidecar/postgrest/src/main.rs) echo sidecar/postgrest/src/lib.rs ;;
    sidecar/storage/src/main.rs) echo sidecar/storage/src/lib.rs ;;
    sidecar/txn_status/src/main.rs) echo sidecar/txn_status/src/main.rs ;;
    sidecar/vectorizer/src/main.rs) echo sidecar/vectorizer/src/runtime/server.rs ;;
    *) return 1 ;;
  esac
}

has_custom_http_probe() {
  local main_file="$1"
  local src_dir="${main_file%/main.rs}"
  local lib_file="${src_dir}/lib.rs"
  local runtime_file="${src_dir}/runtime.rs"
  local probe_files=("${main_file}")

  [[ -f "${lib_file}" ]] && probe_files+=("${lib_file}")
  [[ -f "${runtime_file}" ]] && probe_files+=("${runtime_file}")

  grep -Eq \
    'serve_[[:alnum:]_]*http(_forever)?|handle_[[:alnum:]_]*http|runtime::serve|axum::serve' \
    "${probe_files[@]}" || return 1

  if grep -Eq '/healthz|GET /healthz' "${probe_files[@]}" \
    && grep -Eq '/readyz|GET /readyz' "${probe_files[@]}" \
    && grep -Eq '/metrics|GET /metrics' "${probe_files[@]}"; then
    return 0
  fi

  grep -Eq 'SidecarRuntime::ready|handle_http_bytes' "${probe_files[@]}"
}

has_http_probe_contract() {
  local main_file="${1}"
  local probe_file
  local src_dir

  if grep -Fq 'run_probe_server' "${main_file}"; then
    return 0
  fi

  if has_custom_http_probe "${main_file}"; then
    return 0
  fi

  if probe_file="$(custom_probe_contract_file "${main_file}")"; then
    [[ -s "${probe_file}" ]] || return 1
    if grep -Fq 'SidecarRuntime::ready' "${probe_file}"; then
      if grep -Fq '/healthz' "${probe_file}" || grep -Fq 'handle_http_bytes' "${probe_file}"; then
        return 0
      fi
    fi
    if grep -Fq '/healthz' "${probe_file}" && grep -Fq '/readyz' "${probe_file}"; then
      return 0
    fi
  fi

  src_dir="${main_file%/main.rs}"
  if [[ -d "${src_dir}" ]]     && grep -R -Fq 'TcpListener::bind' "${src_dir}"     && grep -R -Fq '"/healthz"' "${src_dir}"     && grep -R -Fq '"/readyz"' "${src_dir}"     && grep -R -Fq '"/metrics"' "${src_dir}"; then
    return 0
  fi

  return 1
}

for main_file in "${required_serve_mains[@]}"; do
  grep -Fq 'args == ["serve"]' "${main_file}"
  if grep -Fq 'run_probe_server' "${main_file}"; then
    :
  elif has_custom_http_probe "${main_file}" || has_http_probe_contract "${main_file}"; then
    :
  else
    echo "${main_file} must use shared probes or a custom HTTP probe implementation" >&2
    exit 1
  fi

  if [[ "${main_file}" == "sidecar/mcp/src/main.rs" ]]; then
    grep -Fq 'serve_mcp_sidecar_http_forever' "${main_file}"
    grep -Fq 'handle_mcp_sidecar_http_bytes' sidecar/mcp/src/lib.rs
    grep -Fq 'GET /healthz' sidecar/mcp/src/lib.rs
    custom_http_probe_paths=(/healthz /readyz /metrics)
    for custom_http_probe_path in "${custom_http_probe_paths[@]}"; do
      grep -Fq 'request("GET", "'"${custom_http_probe_path}"'")' ci/ai-blaise/mcp-sidecar-http-smoke.sh
    done
    continue
  elif [[ "${main_file}" == "sidecar/cdc/src/main.rs" ]]; then
    grep -Fq 'runtime::serve' "${main_file}"
    grep -Fq 'serve("cdc", default_addr)' "${main_file}"
    grep -Fq 'DdlStreamEvent' sidecar/cdc/src/lib.rs
    grep -Fq 'parse_ddl_stream_event' sidecar/cdc/src/lib.rs
    grep -Fq 'ddl_events_total' "${main_file}"
    grep -Fq 'CREATE EVENT TRIGGER ai_blaise_capture_ddl' ci/ai-blaise/sidecar-cdc-smoke.sh
    grep -Fq 'OK cdc-sidecar live Postgres DDL capture parsed through /ingest' ci/ai-blaise/sidecar-cdc-smoke.sh
    grep -Fq 'route("/healthz", get(healthz))' sidecar/cdc/src/runtime.rs
    grep -Fq 'route("/readyz", get(readyz))' sidecar/cdc/src/runtime.rs
    grep -Fq 'route("/metrics", get(metrics))' sidecar/cdc/src/runtime.rs
    grep -Fq 'SidecarRuntime::ready(component)' sidecar/cdc/src/runtime.rs
    continue
  fi
  if grep -Fq 'run_probe_server' "${main_file}"; then
    continue
  fi
  if grep -Fq 'runtime.block_on(serve(' "${main_file}"; then
    grep -Fq 'runtime::serve' "${main_file}"
    continue
  fi
  if has_custom_http_probe "${main_file}" || has_http_probe_contract "${main_file}"; then
    continue
  fi
  echo "${main_file} must expose serve-mode HTTP probes through run_probe_server, a custom HTTP probe implementation, or a custom runtime::serve implementation" >&2
  exit 1
done

grep -Fq 'args == ["serve"]' pool/src/main.rs
grep -Fq 'run_pool_service_from_env' pool/src/main.rs
grep -Fq 'AI_BLAISE_POOL_UPSTREAM_ADDR' pool/src/proxy.rs
grep -Fq 'handle_proxy_connection' pool/src/proxy.rs
grep -Fq 'ai_blaise_citus_pool_upstream_ready' pool/src/proxy.rs
grep -Fq 'psql -h 127.0.0.1 -p "${pool_port}"' "${pool_proxy_smoke}"

for file in \
  "${image_dir}/extensions/ai_blaise_citus.control" \
  "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql" \
  "${image_dir}/extensions/ai_blaise_citus--0.1.0--0.1.1.sql" \
  "${image_dir}/extensions/ai_blaise_citus--0.1.1--0.1.0.sql"; do
  if [[ ! -s "${file}" ]]; then
    echo "missing companion SQL extension artifact: ${file}" >&2
    exit 1
  fi
done

grep -Fq "CREATE TABLE companion_internal.extension_upgrade_events" "${image_dir}/extensions/ai_blaise_citus--0.1.0--0.1.1.sql"
grep -Fq "DROP TABLE IF EXISTS companion_internal.extension_upgrade_events" "${image_dir}/extensions/ai_blaise_citus--0.1.1--0.1.0.sql"
grep -Fq "CREATE OR REPLACE FUNCTION companion.current_traceparent()" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE OR REPLACE FUNCTION companion.project_traceparent_from_application_name" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE OR REPLACE FUNCTION companion.current_traceparent()" "${image_dir}/extensions/ai_blaise_citus--0.1.0--0.1.1.sql"
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
grep -Fq "'A10', 'streaming chat completion SQL contract', 'sql-intent-fail-closed'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "'A11', 'semantic catalog text-to-SQL SQL contract', 'sql-intent-fail-closed'" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
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
grep -Fq "CREATE TABLE IF NOT EXISTS companion_internal.ai_provider_bindings" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE TABLE IF NOT EXISTS companion_internal.semantic_catalog_objects" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE VIEW companion_ai_provider_bindings" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE VIEW companion_semantic_catalog_objects" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.register_ai_provider_binding" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_ai_chat_stream" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_internal.register_semantic_catalog_object" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "CREATE FUNCTION companion_semantic_text_to_sql_intent" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "sql-intent-fail-closed-only" "${ai_sql_contract_smoke}"
grep -Fq "AI provider runtime is unavailable" "${ai_sql_contract_smoke}"
grep -Fq "text-to-SQL execution is unavailable" "${ai_sql_contract_smoke}"
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
grep -Fq "raw PostgreSQL pipelined simple-query and settings-bucket smoke passed through pool proxy" "${pool_proxy_smoke}"
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
grep -Fq "run-security-supply-chain-canonical" "${security_supply_chain_smoke}"
grep -Fq "slsa.dev/provenance/v1" "${security_supply_chain_smoke}"
grep -Fq "security-supply-chain-smoke ok" "${security_supply_chain_smoke}"
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
grep -Fq "timescale/timescaledb-ha:pg17-ts2.27" "${timescale_bridge_smoke}"
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
grep -Fq "timescale/timescaledb-ha:pg17-ts2.27" "${timescale_cohabitation_dockerfile}"
grep -Fq "make install" "${timescale_cohabitation_dockerfile}"
grep -Fq 'with_llvm="${WITH_LLVM}"' "${timescale_cohabitation_dockerfile}"
grep -Fq "postgresql-server-dev-17" "${timescale_cohabitation_dockerfile}"
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
grep -Fq "FEATURE: Bundle1 TS19 TS20" "${pg_cron_cohabitation_dockerfile}"
grep -Fq "postgres:17-bookworm" "${pg_cron_cohabitation_dockerfile}"
grep -Fq "postgresql-17-cron" "${pg_cron_cohabitation_dockerfile}"
grep -Fq "make install" "${pg_cron_cohabitation_dockerfile}"
grep -Fq "ai_blaise_citus--0.1.0.sql" "${pg_cron_cohabitation_dockerfile}"
grep -Fq "FEATURE: Bundle1 T2 TS19 TS20" "${pg_cron_cohabitation_smoke}"
grep -Fq "placement_generation_after_first_distribution" "${pg_cron_cohabitation_smoke}"
grep -Fq "placement_generation_after_second_distribution" "${pg_cron_cohabitation_smoke}"
grep -Fq "placement_generation_placements" "${pg_cron_cohabitation_smoke}"
grep -Fq "citus_shard_count_parameter_status" "${pg_cron_cohabitation_smoke}"
grep -Fq "SET citus.shard_count TO 7" "${pg_cron_cohabitation_smoke}"
grep -Fq "POSTGRES_HOST_AUTH_METHOD=trust" "${pg_cron_cohabitation_smoke}"
grep -Fq "shared_preload_libraries=pg_cron,citus" "${pg_cron_cohabitation_smoke}"
grep -Fq "citus.cohabit_extensions=pg_cron" "${pg_cron_cohabitation_smoke}"
grep -Fq "CREATE EXTENSION IF NOT EXISTS pg_cron" "${pg_cron_cohabitation_smoke}"
grep -Fq "SELECT companion_internal.assert_cohabit_extension_ready('pg_cron')" "${pg_cron_cohabitation_smoke}"
grep -Fq "citus_cohabit_clock_tick_reserved" "${pg_cron_cohabitation_smoke}"
grep -Fq "citus_cohabit_extension_role" "${pg_cron_cohabitation_smoke}"
grep -Fq "citus_cohabit_extension_configured" "${pg_cron_cohabitation_smoke}"
grep -Fq "citus_cohabit_pg_cron_role" "${pg_cron_cohabitation_smoke}"
grep -Fq "citus_cohabit_timescaledb_role" "${pg_cron_cohabitation_smoke}"
grep -Fq "citus_cohabit_pg_partman_role" "${pg_cron_cohabitation_smoke}"
grep -Fq "negative_pg_cron_citus_configured" "${pg_cron_cohabitation_smoke}"
grep -Fq "cron.schedule" "${pg_cron_cohabitation_smoke}"
grep -Fq "ai_blaise_pg_cron_cohabit_smoke" "${pg_cron_cohabitation_smoke}"
grep -Fq "cron_clock_reserved_runs" "${pg_cron_cohabitation_smoke}"
grep -Fq "negative_clock_tick_reserved" "${pg_cron_cohabitation_smoke}"
grep -Fq "missing-citus-cohabit-extensions" "${pg_cron_cohabitation_smoke}"
grep -Fq "pg-cron-cohabitation-evidence.tsv" "${pg_cron_cohabitation_smoke}"
grep -Fq "run-conflict-policy-runtime-canonical" operator/src/main.rs
grep -Fq "run-conflict-policy-runtime-canonical" "${operator_reconcilers_batch_c_smoke}"
grep -Fq "CONFLICT_POLICY_IMAGE" "${operator_reconcilers_batch_c_smoke}"
grep -Fq "conflict_policy_live_row" "${operator_reconcilers_batch_c_smoke}"
grep -Fq "accounts-lww" "${operator_reconcilers_batch_c_smoke}"
grep -Fq "accounts-merge" "${operator_reconcilers_batch_c_smoke}"
grep -Fq "conflict_classes" "${operator_reconcilers_batch_c_smoke}"
grep -Fq "companion.replication_conflict_audit" "${operator_reconcilers_batch_c_smoke}"
grep -Fq "companion.replication_conflict_audit" companion/src/replication_conflict.rs
grep -Fq "cohabit_extension_detection_report" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "assert_cohabit_extension_ready" "${image_dir}/extensions/ai_blaise_citus--0.1.0.sql"
grep -Fq "stable image identity" "${timescale_cohabitation_smoke}"
grep -Fq "docker buildx imagetools inspect" "${timescale_cohabitation_smoke}"
grep -Fq "git_sha" "${timescale_cohabitation_smoke}"
grep -Fq "command_path" "${timescale_cohabitation_smoke}"
grep -Fq "TS_VERSION_MATRIX_REQUIRED" "${ts_version_matrix_smoke}"
grep -Fq "docker manifest inspect" "${ts_version_matrix_smoke}"
grep -Fq "TIMESCALE_COHABITATION_EVIDENCE" "${ts_version_matrix_smoke}"
grep -Fq "compare-hook-claims.sh" "${ts_version_matrix_smoke}"
grep -Fq "skip-with-note" "${ts_version_matrix_smoke}"
grep -Fq "TS_VERSION_MATRIX_ALLOW_UNKNOWN=1 only for exploratory local probes" "${cohab_matrix_compare}"
grep -Fxq "timescale/timescaledb-ha:pg17-ts2.27" "${cohab_matrix_dir}/2.27/image-tag.txt"
grep -Fxq "timescale/timescaledb-ha:pg17-ts2.28" "${cohab_matrix_dir}/2.28/image-tag.txt"
grep -Fq $'ExecutorStart_hook\tunknown\t' "${cohab_matrix_dir}/2.28/expected-hook-claims.tsv"
grep -Fq "does not promote TS 2.28 to production-ready" "${cohab_matrix_dir}/README.md"
if grep -Fq $'\tunknown\t' "${cohab_matrix_dir}/2.27/expected-hook-claims.tsv"; then
  echo "load-bearing TS 2.27 matrix must not contain unknown hook claims" >&2
  exit 1
fi
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
