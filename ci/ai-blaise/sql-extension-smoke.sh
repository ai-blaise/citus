#!/usr/bin/env bash
set -euo pipefail

# Smoke matrix: ai_blaise_citus companion SQL extension across the PostgreSQL
# major versions ai-blaise/citus supports. The same SQL surface must come up
# against PG16, PG17, and PG18 operand bases. PG18 adds `io_method` as a
# configured GUC; this harness asserts it accepts the contract value without
# breaking Citus or any bundled extension.

repo_root="$(git rev-parse --show-toplevel)"
extension_dir="${repo_root}/images/citus-pg-overlay/extensions"
control_file="${extension_dir}/ai_blaise_citus.control"
sql_file="${extension_dir}/ai_blaise_citus--0.1.0.sql"
bundle1_lock_file="${repo_root}/images/citus-pg-overlay/bundle1-source-build.lock.tsv"
require_docker="${REQUIRE_DOCKER:-0}"

# PG_MAJOR matrix is explicit: PG16 and PG17 production operands plus PG18
# forward coverage for T6 io_method. Override with SQL_EXTENSION_SMOKE_PG_MAJORS
# (whitespace-separated) for local repro, e.g.
#   SQL_EXTENSION_SMOKE_PG_MAJORS=18 bash ci/ai-blaise/sql-extension-smoke.sh
pg_majors_default="16 17 18"
read -r -a pg_majors <<<"${SQL_EXTENSION_SMOKE_PG_MAJORS:-${pg_majors_default}}"

# PG18 ships io_method as a real GUC. Default to the safe `worker` value (also
# the upstream PG18 default) so the smoke matches stock container kernels.
# Operators verifying io_uring kernels can set io_method=io_uring explicitly.
pg18_io_method="${SQL_EXTENSION_SMOKE_IO_METHOD:-worker}"
bundle1_build_image="${BUNDLE1_BUILD_IMAGE:-0}"
bundle1_build_heavy="${BUNDLE1_BUILD_HEAVY:-0}"
bundle1_image="${BUNDLE1_IMAGE:-ai-blaise-citus-overlay:bundle1-source-smoke-pg17}"
bundle1_evidence_file="${BUNDLE1_EVIDENCE_FILE:-}"

for file in "${control_file}" "${sql_file}" "${bundle1_lock_file}"; do
  if [[ ! -s "${file}" ]]; then
    echo "missing SQL extension smoke artifact: ${file}" >&2
    exit 1
  fi
done

if ! command -v docker >/dev/null 2>&1; then
  if [[ "${require_docker}" == "1" ]]; then
    echo "docker is required for SQL extension smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping SQL extension smoke"
  exit 0
fi

active_container=""
cleanup() {
  if [[ -n "${active_container}" ]]; then
    docker rm -f "${active_container}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

run_smoke_for_pg_major() {
  local pg_major="$1"
  local postgres_image="${SQL_EXTENSION_SMOKE_IMAGE:-postgres:${pg_major}}"
  local container="ai-blaise-sql-extension-smoke-pg${pg_major}-${RANDOM}-$$"
  active_container="${container}"

  echo "=== sql-extension-smoke vs ${postgres_image} (PG${pg_major}) ==="

  # PG18 introduces io_method. Verify the GUC accepts the contract value and
  # does not break Citus or any bundled extension. PG17 does not expose
  # io_method, so the run-args stay version-conditioned.
  local -a postgres_args
  postgres_args=(-c "shared_preload_libraries=pg_stat_statements")
  if [[ "${pg_major}" -ge 18 ]]; then
    postgres_args+=(-c "io_method=${pg18_io_method}")
  fi

  # Pre-pull with bounded retry — registry-1.docker.io transients
  # have flaked sibling smokes (sidecar-cdc, pool-*). Matches the
  # 3-attempt/5s pattern applied across smokes in PRs #170/#171.
  for attempt in 1 2 3; do
    if docker pull "${postgres_image}" >/dev/null; then break; fi
    if [ "${attempt}" = "3" ]; then
      echo "docker pull ${postgres_image} failed after 3 attempts" >&2; exit 1
    fi
    sleep 5
  done
  docker run \
    --name "${container}" \
    -e POSTGRES_PASSWORD=postgres \
    -v "${control_file}:/usr/share/postgresql/${pg_major}/extension/ai_blaise_citus.control:ro" \
    -v "${sql_file}:/usr/share/postgresql/${pg_major}/extension/ai_blaise_citus--0.1.0.sql:ro" \
    -d "${postgres_image}" \
    "${postgres_args[@]}" >/dev/null

  local init_complete=0
  local _
  for _ in $(seq 1 120); do
    if docker logs "${container}" 2>&1 | grep -q "PostgreSQL init process complete"; then
      init_complete=1
      break
    fi
    sleep 1
  done

  if [[ "${init_complete}" != "1" ]]; then
    docker logs "${container}" >&2 || true
    echo "postgres container did not finish init scripts (PG${pg_major})" >&2
    exit 1
  fi

  local ready=0
  for _ in $(seq 1 60); do
    if docker exec "${container}" psql -U postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
      ready=1
      break
    fi
    sleep 1
  done

  if [[ "${ready}" != "1" ]]; then
    docker logs "${container}" >&2 || true
    echo "postgres container did not become ready (PG${pg_major})" >&2
    exit 1
  fi

  if [[ "${pg_major}" -ge 18 ]]; then
    local io_method_observed
    io_method_observed="$(
      docker exec "${container}" psql -U postgres -Atqc "SHOW io_method"
    )"
    if [[ "${io_method_observed}" != "${pg18_io_method}" ]]; then
      docker logs "${container}" >&2 || true
      echo "PG${pg_major} io_method GUC did not accept '${pg18_io_method}' (observed: '${io_method_observed}')" >&2
      exit 1
    fi
  fi

  docker exec -i "${container}" psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
CREATE EXTENSION pg_stat_statements;
CREATE EXTENSION pgcrypto;
SELECT pg_stat_statements_reset();
SELECT 1 AS ai_blaise_pg_stat_statements_seed;
CREATE EXTENSION ai_blaise_citus;
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1
    FROM companion_internal.cohabit_extension_detection_report(
      ARRAY['timescaledb','pg_cron'],
      ARRAY['timescaledb','pg_cron'],
      ARRAY['timescaledb','pg_cron','pg_partman']
    )
    WHERE extension_name = 'pg_cron'
      AND role = 'clock-worker'
      AND ready
      AND reason IS NULL
  ) THEN
    RAISE EXCEPTION 'expected pg_cron cohabit detector to report ready clock-worker';
  END IF;

  IF NOT EXISTS (
    SELECT 1
    FROM companion_internal.cohabit_extension_detection_report(
      ARRAY['pg_cron'],
      ARRAY['pg_cron','pg_stat_statements'],
      ARRAY['pg_cron']
    )
    WHERE extension_name = 'pg_stat_statements'
      AND role = 'unsupported'
      AND NOT ready
      AND reason = 'unsupported-configured-extension'
  ) THEN
    RAISE EXCEPTION 'expected cohabit detector to fail closed for unsupported configured extension';
  END IF;
END;
$$;
DO $$
DECLARE
  traceparent text := '00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01';
  tracestate text := 'vendor=ai-blaise';
  projection jsonb;
BEGIN
  PERFORM set_config('trace.parent', traceparent, false);
  PERFORM set_config('trace.state', tracestate, false);
  IF companion.current_traceparent() <> traceparent THEN
    RAISE EXCEPTION 'O14 companion.current_traceparent did not return active traceparent';
  END IF;
  IF companion.current_tracestate() <> tracestate THEN
    RAISE EXCEPTION 'O14 companion.current_tracestate did not return active tracestate';
  END IF;

  projection := companion.project_traceparent_from_application_name(
    'application=companion-sql;traceparent=' || traceparent || ';tracestate=vendor=companion'
  );
  IF projection->>'projected' <> 'true' THEN
    RAISE EXCEPTION 'O14 companion projection did not project traceparent: %', projection;
  END IF;
  IF companion.current_traceparent() <> traceparent THEN
    RAISE EXCEPTION 'O14 companion projection did not set trace.parent';
  END IF;
  IF companion.current_tracestate() <> 'vendor=companion' THEN
    RAISE EXCEPTION 'O14 companion projection did not set trace.state';
  END IF;

  projection := companion.project_traceparent_from_application_name(
    'application=companion-sql;traceparent=not-a-traceparent'
  );
  IF projection->>'projected' <> 'false' THEN
    RAISE EXCEPTION 'O14 invalid traceparent projection did not fail closed: %', projection;
  END IF;
END;
$$;
CREATE TABLE timescale_smoke_metrics (
  metric_time timestamptz NOT NULL,
  value double precision NOT NULL
);
CREATE INDEX timescale_smoke_metrics_metric_time_idx
ON timescale_smoke_metrics(metric_time);

CREATE TABLE timescale_bridge_call_log (
  function_name text NOT NULL,
  relation_name text,
  argument_summary jsonb NOT NULL DEFAULT '{}'::jsonb,
  called_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE control_plane_smoke_orders (
  order_id bigserial PRIMARY KEY,
  tenant_id text NOT NULL,
  amount_cents integer NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  metadata jsonb NOT NULL DEFAULT '{}'::jsonb
);

CREATE TABLE search_smoke_documents (
  doc_id text PRIMARY KEY,
  tenant_id text NOT NULL,
  body text NOT NULL,
  embedding_score numeric NOT NULL DEFAULT 0
);

CREATE TABLE graph_smoke_vertices (
  vertex_id text PRIMARY KEY,
  tenant_id text NOT NULL
);

CREATE TABLE graph_smoke_edges (
  edge_id bigserial PRIMARY KEY,
  from_vertex text NOT NULL,
  to_vertex text NOT NULL,
  tenant_id text NOT NULL
);

CREATE TABLE graph_smoke_edges_unregistered (
  edge_id bigserial PRIMARY KEY,
  from_vertex text NOT NULL,
  to_vertex text NOT NULL,
  tenant_id text NOT NULL
);

CREATE TABLE jsonschema_smoke_documents (
  document_id bigserial PRIMARY KEY,
  payload jsonb NOT NULL
);

CREATE TABLE geo_smoke_places (
  place_id bigserial PRIMARY KEY,
  geom_text text NOT NULL,
  latitude numeric NOT NULL,
  longitude numeric NOT NULL
);

CREATE TABLE vectorizer_smoke_documents (
  doc_id text PRIMARY KEY,
  tenant_id text NOT NULL,
  body text NOT NULL
);

CREATE TABLE toolkit_smoke_metrics (
  tenant_id text NOT NULL,
  metric_time timestamptz NOT NULL,
  value double precision NOT NULL,
  state text NOT NULL DEFAULT 'ok'
);

CREATE SCHEMA doctor_smoke;

CREATE TABLE schema_tenant_smoke_accounts (
  account_id bigint PRIMARY KEY,
  tenant_id text NOT NULL,
  balance_cents bigint NOT NULL
);

CREATE FUNCTION create_hypertable(
  table_name regclass,
  time_column text,
  chunk_time_interval interval DEFAULT NULL,
  if_not_exists boolean DEFAULT false
)
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
  INSERT INTO timescale_bridge_call_log(function_name, relation_name, argument_summary)
  VALUES (
    'create_hypertable',
    table_name::text,
    jsonb_build_object(
      'time_column', time_column,
      'chunk_time_interval', chunk_time_interval::text,
      'if_not_exists', if_not_exists
    )
  );
END;
$$;

CREATE FUNCTION create_distributed_table(
  table_name regclass,
  distribution_column text,
  shard_count integer DEFAULT 32
)
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
  INSERT INTO timescale_bridge_call_log(function_name, relation_name, argument_summary)
  VALUES (
    'create_distributed_table',
    table_name::text,
    jsonb_build_object(
      'distribution_column', distribution_column,
      'shard_count', shard_count
    )
  );
END;
$$;

CREATE FUNCTION add_retention_policy(
  table_name regclass,
  drop_after interval
)
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
  INSERT INTO timescale_bridge_call_log(function_name, relation_name, argument_summary)
  VALUES (
    'add_retention_policy',
    table_name::text,
    jsonb_build_object('drop_after', drop_after::text)
  );
END;
$$;

CREATE FUNCTION add_reorder_policy(
  table_name regclass,
  index_name text
)
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
  INSERT INTO timescale_bridge_call_log(function_name, relation_name, argument_summary)
  VALUES (
    'add_reorder_policy',
    table_name::text,
    jsonb_build_object('index_name', index_name)
  );
END;
$$;

DO $$
DECLARE
  status_count integer;
  planned_count integer;
  plan_sql text;
  bridge_features integer;
  jwt_header_segment text;
  jwt_payload_segment text;
  jwt_signing_input text;
  jwt_token text;
  expired_payload_segment text;
  expired_token text;
  missing_tenant_payload_segment text;
  missing_tenant_token text;
  jwt_claims record;
  generation_one bigint;
  generation_two bigint;
  hash_index integer;
  hash_index_again integer;
  range_index integer;
  plan_violation boolean;
  migration_sql text;
  advisor_sql text;
  webhook_trigger_sql text;
  webhook_event_count integer;
  search_sql text;
  search_doc_id bigint;
  search_rank_count integer;
  json_trigger_sql text;
  json_total_rows bigint;
  json_invalid_rows bigint;
  geo_sql text;
  geo_bucket text;
  vectorizer_sql text;
  vectorizer_queue_table text;
  vectorizer_usage_id bigint;
  doctor_violations jsonb;
  toolkit_sql text;
  toolkit_feature_count integer;
  schema_plan text;
  tenant_move_id bigint;
  tenant_archive_id bigint;
  extension_seed_count integer;
  extension_preload_libraries text[];
  sto2_attachment storage.file_attachment;
  sto2_ref_id bigint;
  sto2_uri text;
  sto2_metadata jsonb;
BEGIN
  SELECT count(*) INTO status_count FROM companion_feature_status();
  IF status_count < 60 THEN
    RAISE EXCEPTION 'expected at least 60 companion feature rows, got %', status_count;
  END IF;

  SELECT count(*) INTO planned_count
  FROM companion_feature_status()
  WHERE status = 'planned';
  IF planned_count <> 0 THEN
    RAISE EXCEPTION 'companion_feature_status returned % planned rows', planned_count;
  END IF;

  IF NOT EXISTS (SELECT 1 FROM companion_feature_status() WHERE feature_id = 'TS1') THEN
    RAISE EXCEPTION 'companion_feature_status must include TS1';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM companion_feature_status() WHERE feature_id = 'TS5') THEN
    RAISE EXCEPTION 'companion_feature_status must include TS5';
  END IF;
  IF NOT EXISTS (
    SELECT 1
    FROM companion_feature_status()
    WHERE feature_id = 'TS18'
      AND status = 'sql-runtime'
  ) THEN
    RAISE EXCEPTION 'companion_feature_status must include sql-runtime TS18';
  END IF;
  IF (
    SELECT count(*)
    FROM companion_feature_status()
    WHERE feature_id IN (
      'Auth2', 'Sec1', 'Sec2', 'Sec5', 'Sec6', 'S6', 'S13',
      'PM3', 'PM4', 'M1', 'M11', 'IA3', 'WH2',
      'Search2', 'Search3', 'Search9', 'G2', 'G3', 'API4',
      'JS2', 'M13', 'Geo2', 'Geo3',
      'A1', 'TS9', 'M7', 'T8', 'L9', 'TS13', 'TS14', 'TS15', 'TS16', 'TS17',
      'C10', 'M2', 'S14', 'TO3', 'TO4', 'TO5',
      'O1', 'O2', 'O3', 'R4', 'Sto2'
    )
      AND status = 'sql-runtime'
  ) <> 44 THEN
    RAISE EXCEPTION 'companion_feature_status must mark custom SQL runtime features as sql-runtime';
  END IF;
  IF (
    SELECT count(*)
    FROM companion_feature_status()
    WHERE feature_id IN (
      'A7', 'A12', 'C11', 'C12', 'C13', 'EF6', 'F2', 'F5',
      'G1', 'Geo1', 'IA1', 'IA2', 'JS1', 'L11', 'M6', 'M10',
      'M12', 'MR7', 'O7', 'O8', 'O9', 'O11', 'O12', 'PM1',
      'PM2', 'R6', 'R11', 'Search1', 'Search4', 'Search5',
      'Search6', 'Sec3', 'Sec4', 'Sec10', 'Sec11', 'Sec14',
      'Sec15', 'WF1'
    )
      AND status = 'extension-catalog-runtime'
  ) <> 38 THEN
    RAISE EXCEPTION 'companion_feature_status must mark extension catalog features as extension-catalog-runtime';
  END IF;

  extension_seed_count := companion_internal.seed_extension_catalog();
  IF extension_seed_count <> 45 THEN
    RAISE EXCEPTION 'extension catalog seed inserted % rows, expected 45', extension_seed_count;
  END IF;
  IF (
    SELECT count(DISTINCT feature_id)
    FROM companion_extension_feature_coverage
  ) <> 38 THEN
    RAISE EXCEPTION 'extension catalog feature coverage did not include 38 features';
  END IF;
  IF NOT EXISTS (
    SELECT 1
    FROM companion_extension_required('A7')
    WHERE extension_name = 'pgvector'
      AND tier = 'required'
  ) THEN
    RAISE EXCEPTION 'A7 extension catalog did not expose required pgvector';
  END IF;
  IF NOT EXISTS (
    SELECT 1
    FROM companion_extension_required('F2')
    WHERE extension_name IN ('oracle_fdw', 'mysql_fdw', 'mongo_fdw', 'tds_fdw')
    HAVING count(*) = 4
  ) THEN
    RAISE EXCEPTION 'F2 extension catalog did not expose all FDW options';
  END IF;
  extension_preload_libraries := companion_required_preload_libraries();
  IF array_position(extension_preload_libraries, 'age') IS NULL
     OR array_position(extension_preload_libraries, 'pg_search') IS NULL
     OR array_position(extension_preload_libraries, 'pg_hint_plan') IS NULL THEN
    RAISE EXCEPTION 'extension catalog preload list omitted required preload contracts: %', extension_preload_libraries;
  END IF;
  PERFORM companion_internal.assert_extension_allowed('pgvector');
  PERFORM companion_internal.register_extension_contract(
    'orioledb',
    'hard-block',
    ARRAY['Bundle1'],
    false,
    'Heap access method conflicts with this stack'
  );
  IF NOT companion_extension_conflicts('orioledb') THEN
    RAISE EXCEPTION 'extension catalog hard-block conflict check did not flag orioledb';
  END IF;
  BEGIN
    PERFORM companion_internal.assert_extension_allowed('orioledb');
    RAISE EXCEPTION 'extension catalog allowed a hard-blocked extension';
  EXCEPTION WHEN OTHERS THEN
    IF SQLERRM <> 'extension is hard-blocked: orioledb' THEN
      RAISE;
    END IF;
  END;
  BEGIN
    PERFORM companion_internal.register_extension_contract('bad_ext', 'required', ARRAY[]::text[]);
    RAISE EXCEPTION 'extension catalog accepted empty feature ids';
  EXCEPTION WHEN OTHERS THEN
    IF SQLERRM <> 'feature_ids must not be empty' THEN
      RAISE;
    END IF;
  END;
  BEGIN
    PERFORM companion_internal.register_extension_contract('bad_tier', 'experimental', ARRAY['A7']);
    RAISE EXCEPTION 'extension catalog accepted an unsupported tier';
  EXCEPTION WHEN OTHERS THEN
    IF SQLERRM <> 'unsupported extension tier: experimental' THEN
      RAISE;
    END IF;
  END;


  sto2_attachment := storage.file_attachment(
    'tenant-files',
    'uploads/report.pdf',
    'application/pdf',
    1024,
    repeat('a', 64),
    jsonb_build_object('original_name', 'report.pdf', 'scan_status', 'pending')
  );
  IF storage.file_attachment_bucket(sto2_attachment) <> 'tenant-files'
     OR storage.file_attachment_object_key(sto2_attachment) <> 'uploads/report.pdf'
     OR storage.file_attachment_content_type(sto2_attachment) <> 'application/pdf'
     OR storage.file_attachment_size_bytes(sto2_attachment) <> 1024
     OR storage.file_attachment_sha256(sto2_attachment) <> repeat('a', 64) THEN
    RAISE EXCEPTION 'Sto2 file_attachment accessors returned unexpected values';
  END IF;
  sto2_metadata := storage.file_attachment_metadata(sto2_attachment);
  IF sto2_metadata->>'scan_status' <> 'pending' THEN
    RAISE EXCEPTION 'Sto2 metadata accessor returned unexpected value: %', sto2_metadata;
  END IF;
  sto2_uri := storage.file_attachment_uri(sto2_attachment);
  IF sto2_uri <> 'storage://tenant-files/uploads/report.pdf' THEN
    RAISE EXCEPTION 'Sto2 file_attachment URI mismatch: %', sto2_uri;
  END IF;
  INSERT INTO storage.file_attachment_refs(tenant_id, owner_id, owner_kind, attachment)
  VALUES ('tenant-a', 'user-123', 'user', sto2_attachment)
  RETURNING ref_id INTO sto2_ref_id;
  IF sto2_ref_id IS NULL OR NOT EXISTS (
    SELECT 1
    FROM storage.file_attachment_refs
    WHERE ref_id = sto2_ref_id
      AND tenant_id = 'tenant-a'
      AND owner_id = 'user-123'
      AND bucket = 'tenant-files'
      AND object_key = 'uploads/report.pdf'
      AND content_type = 'application/pdf'
      AND size_bytes = 1024
      AND sha256 = repeat('a', 64)
      AND object_metadata->>'original_name' = 'report.pdf'
  ) THEN
    RAISE EXCEPTION 'Sto2 file_attachment_refs persistence failed';
  END IF;
  BEGIN
    PERFORM storage.file_attachment(
      'Tenant_Files',
      'uploads/report.pdf',
      'application/pdf',
      1024,
      repeat('a', 64)
    );
    RAISE EXCEPTION 'Sto2 accepted invalid bucket';
  EXCEPTION WHEN check_violation THEN
  END;
  BEGIN
    PERFORM storage.file_attachment(
      'tenant-files',
      '../secrets.txt',
      'text/plain',
      8,
      repeat('b', 64)
    );
    RAISE EXCEPTION 'Sto2 accepted path traversal';
  EXCEPTION WHEN check_violation THEN
  END;
  BEGIN
    PERFORM storage.file_attachment(
      'tenant-files',
      'uploads/bad-sha.txt',
      'text/plain',
      8,
      upper(repeat('c', 64))
    );
    RAISE EXCEPTION 'Sto2 accepted malformed sha256';
  EXCEPTION WHEN check_violation THEN
  END;
  BEGIN
    PERFORM storage.file_attachment(
      'tenant-files',
      'uploads/negative.txt',
      'text/plain',
      -1,
      repeat('d', 64)
    );
    RAISE EXCEPTION 'Sto2 accepted negative size_bytes';
  EXCEPTION WHEN check_violation THEN
  END;

  vectorizer_sql := companion_internal.register_vectorizer(
    'documents_body',
    'vectorizer_smoke_documents',
    'doc_id',
    'body',
    512,
    64,
    'openai',
    'text-embedding-3-small',
    'secret://vectorizer/openai',
    'vectorizer_smoke_embeddings',
    'embedding',
    1536,
    '5 minutes',
    2,
    100000
  );
  IF vectorizer_sql NOT LIKE 'SELECT ai.create_vectorizer(%ai.loading_table%' THEN
    RAISE EXCEPTION 'A1 register_vectorizer did not render pgai-compatible SQL: %', vectorizer_sql;
  END IF;
  SELECT queue_table::text
  INTO vectorizer_queue_table
  FROM companion_vectorizer_definitions
  WHERE vectorizer_name = 'documents_body'
    AND source_table = 'vectorizer_smoke_documents'
    AND provider = 'openai'
    AND dimensions = 1536;
  IF vectorizer_queue_table IS NULL THEN
    RAISE EXCEPTION 'A1 vectorizer definition was not visible';
  END IF;
  PERFORM companion_internal.vectorizer_enqueue(
    'documents_body',
    'tenant-a',
    'doc-1',
    'hello vectorizer'
  );
  vectorizer_usage_id := companion_internal.vectorizer_record_usage(
    'documents_body',
    'tenant-a',
    32
  );
  IF vectorizer_usage_id IS NULL OR NOT EXISTS (
    SELECT 1
    FROM companion_vectorizer_usage_log
    WHERE vectorizer_name = 'documents_body'
      AND tenant_id = 'tenant-a'
      AND tokens = 32
  ) THEN
    RAISE EXCEPTION 'A1 vectorizer usage was not recorded';
  END IF;
  BEGIN
    PERFORM companion_internal.register_vectorizer(
      'bad_vectorizer',
      'vectorizer_smoke_documents',
      'doc_id',
      'missing_body',
      512,
      64,
      'openai',
      'text-embedding-3-small',
      'secret://vectorizer/openai',
      'vectorizer_smoke_embeddings',
      'embedding',
      1536,
      '5 minutes',
      2
    );
    RAISE EXCEPTION 'A1 accepted a missing source column';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'source_column does not exist on source table' THEN
        RAISE;
      END IF;
  END;
  BEGIN
    PERFORM companion_internal.register_vectorizer(
      'bad_overlap',
      'vectorizer_smoke_documents',
      'doc_id',
      'body',
      64,
      64,
      'openai',
      'text-embedding-3-small',
      'secret://vectorizer/openai',
      'vectorizer_smoke_embeddings',
      'embedding',
      1536,
      '5 minutes',
      2
    );
    RAISE EXCEPTION 'A1 accepted invalid chunk overlap';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'chunk_overlap_tokens must be less than chunk_max_tokens' THEN
        RAISE;
      END IF;
  END;

  PERFORM companion_internal.assert_shared_preload_libraries(
    ARRAY['timescaledb', 'citus'],
    ARRAY['timescaledb']
  );
  PERFORM companion_internal.assert_citus_cohabit_extension_order(
    ARRAY['timescaledb', 'citus']
  );
  doctor_violations := companion_internal.get_violations(
    ARRAY['doctor_smoke'],
    ARRAY['cohabit_extensions', 'missing_distribution_column']
  );
  IF doctor_violations <> '[]'::jsonb THEN
    RAISE EXCEPTION 'TS9 doctor reported violations for existing schema: %', doctor_violations;
  END IF;
  doctor_violations := companion_internal.get_violations(
    ARRAY['doctor_missing_schema'],
    ARRAY['cohabit_extensions']
  );
  IF jsonb_array_length(doctor_violations) <> 1 THEN
    RAISE EXCEPTION 'TS9 doctor did not report missing schema violation: %', doctor_violations;
  END IF;
  BEGIN
    PERFORM companion_internal.assert_shared_preload_libraries(
      ARRAY['timescaledb'],
      ARRAY['timescaledb']
    );
    RAISE EXCEPTION 'M7 accepted shared_preload_libraries without citus';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'citus must be preloaded' THEN
        RAISE;
      END IF;
  END;
  BEGIN
    PERFORM companion_internal.assert_citus_cohabit_extension_order(
      ARRAY['citus', 'timescaledb']
    );
    RAISE EXCEPTION 'M7 accepted citus before trusted cohabiting extensions';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'citus must be loaded after trusted cohabiting extensions' THEN
        RAISE;
      END IF;
  END;
  BEGIN
    PERFORM companion_internal.get_violations(
      ARRAY['doctor_smoke'],
      ARRAY['unknown_rule']
    );
    RAISE EXCEPTION 'TS9 accepted an unsupported doctor rule';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'unsupported doctor rule: unknown_rule' THEN
        RAISE;
      END IF;
  END;

  toolkit_sql := companion_internal.register_toolkit_aggregate_plan(
    'toolkit_smoke_metrics',
    'toolkit_gapfill_worker',
    'toolkit_gapfill_coordinator',
    'tenant_id',
    'value',
    'time_bucket_gapfill',
    'metric_time',
    '1 hour'
  );
  IF toolkit_sql NOT LIKE '%time_bucket_gapfill%' OR toolkit_sql NOT LIKE '%locf(interpolate(partial_state))%' THEN
    RAISE EXCEPTION 'TS13 toolkit gapfill plan did not render partial/final SQL: %', toolkit_sql;
  END IF;
  PERFORM companion_internal.register_toolkit_aggregate_plan(
    'toolkit_smoke_metrics',
    'toolkit_counter_worker',
    'toolkit_counter_coordinator',
    'tenant_id',
    'value',
    'counter_agg'
  );
  PERFORM companion_internal.register_toolkit_aggregate_plan(
    'toolkit_smoke_metrics',
    'toolkit_percentile_worker',
    'toolkit_percentile_coordinator',
    'tenant_id',
    'value',
    'percentile_agg'
  );
  PERFORM companion_internal.register_toolkit_aggregate_plan(
    'toolkit_smoke_metrics',
    'toolkit_asap_worker',
    'toolkit_asap_coordinator',
    'tenant_id',
    'value',
    'asap_smooth',
    'metric_time'
  );
  PERFORM companion_internal.register_toolkit_aggregate_plan(
    'toolkit_smoke_metrics',
    'toolkit_state_worker',
    'toolkit_state_coordinator',
    'tenant_id',
    'state',
    'state_agg'
  );
  PERFORM companion_internal.register_toolkit_aggregate_plan(
    'toolkit_smoke_metrics',
    'toolkit_hll_worker',
    'toolkit_hll_coordinator',
    'tenant_id',
    'tenant_id',
    'hyperloglog'
  );
  SELECT count(DISTINCT feature_id)
  INTO toolkit_feature_count
  FROM companion_toolkit_aggregate_plans
  WHERE feature_id IN ('TS13', 'TS14', 'TS15', 'TS16', 'TS17', 'T8');
  IF toolkit_feature_count <> 6 THEN
    RAISE EXCEPTION 'expected six toolkit feature ids, got %', toolkit_feature_count;
  END IF;
  IF NOT EXISTS (
    SELECT 1
    FROM companion_toolkit_aggregate_plans
    WHERE aggregate_kind = 'counter_agg'
      AND worker_sql LIKE '%counter_agg%'
      AND coordinator_sql LIKE '%rollup(partial_state)%'
  ) THEN
    RAISE EXCEPTION 'L9/TS14 toolkit worker partial plan was not visible';
  END IF;
  BEGIN
    PERFORM companion_internal.register_toolkit_aggregate_plan(
      'toolkit_smoke_metrics',
      'toolkit_bad_worker',
      'toolkit_bad_coordinator',
      'tenant_id',
      'value',
      'time_bucket_gapfill',
      'metric_time'
    );
    RAISE EXCEPTION 'TS13 accepted gapfill without bucket_width';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'bucket_width must not be empty for time_bucket_gapfill' THEN
        RAISE;
      END IF;
  END;
  BEGIN
    PERFORM companion_internal.register_toolkit_aggregate_plan(
      'toolkit_smoke_metrics',
      'toolkit_bad_worker',
      'toolkit_bad_coordinator',
      'tenant_id',
      'value',
      'lttb'
    );
    RAISE EXCEPTION 'TS16 accepted downsampler without time_column';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'time_column must not be empty for aggregate lttb' THEN
        RAISE;
      END IF;
  END;
  BEGIN
    PERFORM companion_internal.register_toolkit_aggregate_plan(
      'toolkit_smoke_metrics',
      'toolkit_bad_worker',
      'toolkit_bad_coordinator',
      'tenant_id',
      'value',
      'unknown_agg'
    );
    RAISE EXCEPTION 'T8 accepted an unsupported aggregate';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'unsupported toolkit aggregate: unknown_agg' THEN
        RAISE;
      END IF;
  END;

  PERFORM companion_internal.schema_job_start(
    'accounts-online-balance',
    'schema_tenant_smoke_accounts',
    60
  );
  schema_plan := companion_internal.schema_job_add_operation(
    'accounts-online-balance',
    'add_column',
    'balance_cents_shadow',
    'bigint'
  );
  IF schema_plan <> 'ALTER TABLE schema_tenant_smoke_accounts ADD COLUMN IF NOT EXISTS balance_cents_shadow bigint;' THEN
    RAISE EXCEPTION 'C10/M2 add-column operation rendered unexpected SQL: %', schema_plan;
  END IF;
  PERFORM companion_internal.schema_job_add_operation(
    'accounts-online-balance',
    'backfill',
    NULL,
    NULL,
    'UPDATE schema_tenant_smoke_accounts SET balance_cents_shadow = balance_cents'
  );
  IF companion_internal.schema_job_advance('accounts-online-balance', 'write_only') <> 'write_only' THEN
    RAISE EXCEPTION 'C10 schema job did not advance to write_only';
  END IF;
  PERFORM companion_internal.schema_job_advance('accounts-online-balance', 'backfill');
  PERFORM companion_internal.schema_job_advance('accounts-online-balance', 'public');
  schema_plan := companion_internal.schema_job_render_plan('accounts-online-balance');
  IF schema_plan NOT LIKE '%ADD COLUMN IF NOT EXISTS balance_cents_shadow bigint%'
     OR schema_plan NOT LIKE '%UPDATE schema_tenant_smoke_accounts SET balance_cents_shadow = balance_cents%' THEN
    RAISE EXCEPTION 'M2 schema job render plan missed expected operations: %', schema_plan;
  END IF;
  IF NOT EXISTS (
    SELECT 1
    FROM companion_schema_jobs
    WHERE job_name = 'accounts-online-balance'
      AND state = 'public'
  ) THEN
    RAISE EXCEPTION 'C10 schema job final state was not visible';
  END IF;
  BEGIN
    PERFORM companion_internal.schema_job_advance(
      'accounts-online-balance',
      'delete_only'
    );
    RAISE EXCEPTION 'C10 accepted an invalid state transition';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'invalid schema job transition: public -> delete_only' THEN
        RAISE;
      END IF;
  END;
  BEGIN
    PERFORM companion_internal.schema_job_start(
      'bad-schema-job',
      'schema_tenant_smoke_accounts',
      0
    );
    RAISE EXCEPTION 'C10 accepted a zero lease';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'lease_seconds must be greater than zero' THEN
        RAISE;
      END IF;
  END;

  tenant_move_id := companion_internal.plan_tenant_move(
    'tenant-a',
    'worker-1',
    'worker-2',
    'us-east1'
  );
  IF tenant_move_id IS NULL OR NOT EXISTS (
    SELECT 1
    FROM companion_tenant_moves
    WHERE move_id = tenant_move_id
      AND tenant_name = 'tenant-a'
      AND source_worker = 'worker-1'
      AND target_worker = 'worker-2'
      AND region_affinity = 'us-east1'
      AND status = 'queued'
  ) THEN
    RAISE EXCEPTION 'S14/TO3 tenant move was not visible';
  END IF;
  PERFORM companion_internal.set_tenant_quota('tenant-a', 25, 500);
  IF NOT EXISTS (
    SELECT 1
    FROM companion_tenant_quotas
    WHERE tenant_name = 'tenant-a'
      AND max_connections = 25
      AND max_qps = 500
  ) THEN
    RAISE EXCEPTION 'S14 tenant quota was not visible';
  END IF;
  tenant_archive_id := companion_internal.plan_tenant_archive(
    'tenant-a',
    's3://archives/tenant-a',
    90
  );
  IF tenant_archive_id IS NULL OR NOT EXISTS (
    SELECT 1
    FROM companion_tenant_archives
    WHERE archive_id = tenant_archive_id
      AND tenant_name = 'tenant-a'
      AND retention_days = 90
  ) THEN
    RAISE EXCEPTION 'TO4 tenant archive was not visible';
  END IF;
  PERFORM companion_internal.set_tenant_region_affinity('tenant-a', 'us-east1');
  IF NOT EXISTS (
    SELECT 1
    FROM companion_tenant_region_affinities
    WHERE tenant_name = 'tenant-a'
      AND region_affinity = 'us-east1'
  ) THEN
    RAISE EXCEPTION 'TO5 tenant region affinity was not visible';
  END IF;
  BEGIN
    PERFORM companion_internal.plan_tenant_move(
      'tenant-b',
      'worker-1',
      'worker-1'
    );
    RAISE EXCEPTION 'TO3 accepted a same-worker tenant move';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'source_worker and target_worker must differ' THEN
        RAISE;
      END IF;
  END;
  BEGIN
    PERFORM companion_internal.set_tenant_quota('tenant-a', 0, 500);
    RAISE EXCEPTION 'S14 accepted a zero connection quota';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'max_connections must be greater than zero' THEN
        RAISE;
      END IF;
  END;
  BEGIN
    PERFORM companion_internal.plan_tenant_archive(
      'tenant-a',
      's3://archives/tenant-a',
      0
    );
    RAISE EXCEPTION 'TO4 accepted zero retention';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'retention_days must be greater than zero' THEN
        RAISE;
      END IF;
  END;
  BEGIN
    PERFORM companion_internal.set_tenant_region_affinity('tenant-a', '');
    RAISE EXCEPTION 'TO5 accepted empty region affinity';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'region_affinity must not be empty' THEN
        RAISE;
      END IF;
  END;

  PERFORM companion_internal.migrate_start(
    'orders-expand-contract',
    'control_plane_smoke_orders',
    5000,
    1000
  );
  migration_sql := companion_internal.migration_add_column(
    'region',
    'text',
    '''us-east1'''
  );
  IF migration_sql NOT LIKE 'ALTER TABLE control_plane_smoke_orders ADD COLUMN IF NOT EXISTS region text DEFAULT %' THEN
    RAISE EXCEPTION 'M1 migration_add_column did not render bounded expand DDL: %', migration_sql;
  END IF;
  PERFORM companion_internal.migration_register_invariant(
    'orders-expand-contract',
    'amount-cents-shadow-check',
    'SELECT true AS passed, count(*) AS rows_checked FROM control_plane_smoke_orders'
  );
  migration_sql := companion_internal.migration_online_type_change(
    'amount_cents',
    'integer',
    'bigint',
    'amount_cents::bigint'
  );
  IF migration_sql NOT LIKE '%amount_cents__ai_blaise_new bigint%' THEN
    RAISE EXCEPTION 'M11 online type-change helper did not render shadow-column DDL: %', migration_sql;
  END IF;
  PERFORM companion_internal.migrate_complete('orders-expand-contract');
  IF NOT EXISTS (
    SELECT 1
    FROM companion_migration_runs
    WHERE migration_name = 'orders-expand-contract'
      AND table_name = 'control_plane_smoke_orders'
      AND status = 'completed'
  ) THEN
    RAISE EXCEPTION 'M1 migration run was not completed and visible';
  END IF;
  IF (
    SELECT count(*)
    FROM companion_migration_operations
    WHERE migration_name = 'orders-expand-contract'
  ) <> 2 THEN
    RAISE EXCEPTION 'M1/M11 migration operations were not recorded idempotently';
  END IF;
  IF NOT EXISTS (
    SELECT 1 FROM companion_migration_invariant_checks
    WHERE migration_name = 'orders-expand-contract'
      AND check_name = 'amount-cents-shadow-check'
      AND passed_at IS NOT NULL
  ) THEN
    RAISE EXCEPTION 'M1/M11 migration invariant did not pass';
  END IF;
  BEGIN
    PERFORM companion_internal.migration_drop_column('orphan_column');
    RAISE EXCEPTION 'M1 migration operation ran without migrate_start';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'no active companion migration; call companion_internal.migrate_start first' THEN
        RAISE;
      END IF;
  END;
  PERFORM companion_internal.migrate_start(
    'orders-bad-type-change',
    'control_plane_smoke_orders',
    5000,
    1000
  );
  BEGIN
    PERFORM companion_internal.migration_online_type_change(
      'amount_cents',
      'integer',
      'integer',
      'amount_cents'
    );
    RAISE EXCEPTION 'M11 online type-change accepted identical types';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'from_type and to_type must differ' THEN
        RAISE;
      END IF;
  END;

  PERFORM companion_internal.index_advisor_record_candidate(
    '15 minutes',
    'control_plane_smoke_orders',
    'control_plane_orders_tenant_created_idx',
    ARRAY['tenant_id', 'created_at'],
    'btree',
    1000,
    700,
    12
  );
  SELECT create_index_sql
  INTO advisor_sql
  FROM companion_index_advisor_ranked(10)
  WHERE index_name = 'control_plane_orders_tenant_created_idx'::name;
  IF advisor_sql NOT LIKE 'CREATE INDEX CONCURRENTLY IF NOT EXISTS control_plane_orders_tenant_created_idx ON control_plane_smoke_orders USING btree %' THEN
    RAISE EXCEPTION 'IA3 ranked advisor did not render CREATE INDEX CONCURRENTLY SQL: %', advisor_sql;
  END IF;
  BEGIN
    PERFORM companion_internal.index_advisor_record_candidate(
      '15 minutes',
      'control_plane_smoke_orders',
      'control_plane_orders_bad_idx',
      ARRAY['tenant_id'],
      'btree',
      1000,
      1200,
      1
    );
    RAISE EXCEPTION 'IA3 accepted a non-improving candidate';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'estimated_cost_after must be lower than estimated_cost_before' THEN
        RAISE;
      END IF;
  END;

  PERFORM companion_internal.webhook_register(
    'orders-webhook',
    'control_plane_smoke_orders',
    'https://hooks.example.test/orders',
    '{"Authorization": "secret://webhooks/orders"}'::jsonb,
    5
  );
  webhook_trigger_sql := companion_internal.install_webhook_trigger(
    'control_plane_smoke_orders',
    ARRAY['INSERT', 'UPDATE'],
    'companion.webhook_queue',
    'orders-webhook'
  );
  IF webhook_trigger_sql NOT LIKE 'CREATE TRIGGER companion_webhook_% AFTER INSERT OR UPDATE ON control_plane_smoke_orders%' THEN
    RAISE EXCEPTION 'WH2 install_webhook_trigger did not render/install trigger SQL: %', webhook_trigger_sql;
  END IF;
  INSERT INTO control_plane_smoke_orders(tenant_id, amount_cents, metadata)
  VALUES ('tenant-a', 100, '{"source":"insert"}'::jsonb);
  UPDATE control_plane_smoke_orders
  SET amount_cents = 125
  WHERE tenant_id = 'tenant-a';
  SELECT count(*)
  INTO webhook_event_count
  FROM companion_webhook_events
  WHERE webhook_name = 'orders-webhook'
    AND queue_name = 'companion.webhook_queue';
  IF webhook_event_count <> 2 THEN
    RAISE EXCEPTION 'WH2 webhook trigger did not enqueue INSERT and UPDATE rows';
  END IF;
  BEGIN
    PERFORM companion_internal.webhook_register(
      'bad-webhook',
      'control_plane_smoke_orders',
      'secret://orders',
      '{}'::jsonb,
      1
    );
    RAISE EXCEPTION 'WH2 accepted a non-http webhook URL';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'url must be http or https' THEN
        RAISE;
      END IF;
  END;

  search_sql := companion_internal.register_search_index(
    'search_smoke_documents',
    'search_smoke_documents_body_idx',
    ARRAY['body'],
    'tenant_id',
    ARRAY['embedding_score']
  );
  IF search_sql NOT LIKE 'CREATE INDEX IF NOT EXISTS search_smoke_documents_body_idx ON search_smoke_documents USING gin%' THEN
    RAISE EXCEPTION 'Search2 register_search_index did not render worker index DDL: %', search_sql;
  END IF;
  search_doc_id := companion_internal.search_document_upsert(
    'search_smoke_documents',
    'doc-1',
    'citus distributed search bridge',
    0.7
  );
  IF search_doc_id IS NULL THEN
    RAISE EXCEPTION 'Search2 search_document_upsert did not return a document id';
  END IF;
  PERFORM companion_internal.search_document_upsert(
    'search_smoke_documents',
    'doc-2',
    'unrelated analytics note',
    0.2
  );
  SELECT count(*)
  INTO search_rank_count
  FROM companion_internal.hybrid_rank(
    'search_smoke_documents',
    'distributed',
    'embedding_score',
    '[0.1]'
  )
  WHERE document_key = 'doc-1'
    AND bm25_score > 0
    AND vector_score = 0.7;
  IF search_rank_count <> 1 THEN
    RAISE EXCEPTION 'Search3 hybrid_rank did not return the expected ranked document';
  END IF;
  SELECT rerank_sql
  INTO search_sql
  FROM companion_internal.rerank_search(
    'companion_search_documents',
    'local',
    'identity'
  )
  LIMIT 1;
  IF search_sql <> 'SELECT * FROM companion_search_documents' THEN
    RAISE EXCEPTION 'Search9 rerank_search did not record/render deterministic rerank SQL: %', search_sql;
  END IF;
  IF NOT EXISTS (
    SELECT 1
    FROM companion_search_rerank_requests
    WHERE input_view = 'companion_search_documents'
      AND provider = 'local'
      AND model = 'identity'
  ) THEN
    RAISE EXCEPTION 'Search9 rerank_search request was not visible';
  END IF;
  BEGIN
    PERFORM companion_internal.register_search_index(
      'search_smoke_documents',
      'search_bad_idx',
      ARRAY['body'],
      'missing_tenant',
      ARRAY['embedding_score']
    );
    RAISE EXCEPTION 'Search2 accepted a missing distribution column';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'distribution column does not exist on table' THEN
        RAISE;
      END IF;
  END;
  BEGIN
    PERFORM companion_internal.hybrid_rank(
      'search_smoke_documents',
      'distributed',
      'missing_vector',
      '[0.1]'
    );
    RAISE EXCEPTION 'Search3 accepted a missing vector column';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'vector_column does not exist on table' THEN
        RAISE;
      END IF;
  END;
  BEGIN
    PERFORM companion_internal.rerank_search(
      'missing_rerank_view',
      'local',
      'identity'
    );
    RAISE EXCEPTION 'Search9 accepted a missing rerank input relation';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'input_view must reference an existing relation' THEN
        RAISE;
      END IF;
  END;

  PERFORM companion_internal.ensure_graph_colocation(
    'graph_smoke_vertices',
    'graph_smoke_edges',
    'vertex_id',
    'tenant_id'
  );
  PERFORM companion_internal.register_graphql_distributed_graph(
    'smoke_graph',
    'graph_smoke_vertices',
    'graph_smoke_edges'
  );
  IF NOT EXISTS (
    SELECT 1
    FROM companion_graph_colocations
    WHERE vertex_table = 'graph_smoke_vertices'
      AND edge_table = 'graph_smoke_edges'
      AND vertex_key = 'vertex_id'
  ) THEN
    RAISE EXCEPTION 'G2/G3 graph colocation metadata was not visible';
  END IF;
  IF NOT EXISTS (
    SELECT 1
    FROM companion_graphql_distributed_graphs
    WHERE graph_name = 'smoke_graph'
      AND vertex_table = 'graph_smoke_vertices'
      AND edge_table = 'graph_smoke_edges'
  ) THEN
    RAISE EXCEPTION 'API4 GraphQL distributed graph metadata was not visible';
  END IF;
  BEGIN
    PERFORM companion_internal.ensure_graph_colocation(
      'graph_smoke_vertices',
      'graph_smoke_edges',
      'missing_vertex',
      'tenant_id'
    );
    RAISE EXCEPTION 'G3 accepted a missing vertex key';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'vertex_key column does not exist on vertex table' THEN
        RAISE;
      END IF;
  END;
  BEGIN
    PERFORM companion_internal.register_graphql_distributed_graph(
      'unregistered_graph',
      'graph_smoke_vertices',
      'graph_smoke_edges_unregistered'
    );
    RAISE EXCEPTION 'API4 accepted GraphQL graph metadata without colocation';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'graph colocation must be registered before GraphQL graph metadata' THEN
        RAISE;
      END IF;
  END;

  PERFORM companion_internal.register_json_schema(
    'payload-kind',
    '{"type":"object","required":["kind"]}'::jsonb
  );
  json_trigger_sql := companion_internal.install_jsonschema_trigger(
    'jsonschema_smoke_documents',
    'payload',
    'payload-kind',
    'BEFORE INSERT OR UPDATE'
  );
  IF json_trigger_sql NOT LIKE 'CREATE TRIGGER companion_jsonschema_% BEFORE INSERT OR UPDATE ON jsonschema_smoke_documents%' THEN
    RAISE EXCEPTION 'M13 install_jsonschema_trigger did not render/install trigger SQL: %', json_trigger_sql;
  END IF;
  INSERT INTO jsonschema_smoke_documents(payload)
  VALUES ('{"kind":"event","value":1}'::jsonb);
  BEGIN
    INSERT INTO jsonschema_smoke_documents(payload)
    VALUES ('{"value":2}'::jsonb);
    RAISE EXCEPTION 'M13 JSON schema trigger accepted a document missing a required field';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'json document does not match registered schema' THEN
        RAISE;
      END IF;
  END;
  SELECT total_rows, invalid_rows
  INTO json_total_rows, json_invalid_rows
  FROM companion_internal.validate_jsonschema_shard(
    'jsonschema_smoke_documents'::regclass,
    'payload',
    'payload-kind'
  );
  IF json_total_rows <> 1 OR json_invalid_rows <> 0 THEN
    RAISE EXCEPTION 'JS2 validate_jsonschema_shard returned total %, invalid %',
      json_total_rows,
      json_invalid_rows;
  END IF;
  BEGIN
    PERFORM companion_internal.register_json_schema(
      'bad-json-schema',
      '[]'::jsonb
    );
    RAISE EXCEPTION 'JS2 accepted a non-object JSON schema';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'schema_document must be a JSON object' THEN
        RAISE;
      END IF;
  END;

  geo_sql := companion_internal.add_geohash_column(
    'geo_smoke_places',
    'geom_text',
    'geo_bucket',
    5
  );
  IF geo_sql <> 'ALTER TABLE geo_smoke_places ADD COLUMN IF NOT EXISTS geo_bucket text;' THEN
    RAISE EXCEPTION 'Geo2 add_geohash_column did not render/install expected column DDL: %', geo_sql;
  END IF;
  geo_bucket := companion_geo_bucket(42.3601, -71.0589, 5);
  IF length(geo_bucket) <> 5 THEN
    RAISE EXCEPTION 'Geo2 companion_geo_bucket returned unexpected bucket %', geo_bucket;
  END IF;
  PERFORM companion_internal.enable_geo_shard_pruning(
    'geo_smoke_places',
    'geom_text',
    5
  );
  IF NOT EXISTS (
    SELECT 1
    FROM companion_geo_distributions
    WHERE table_name = 'geo_smoke_places'
      AND geometry_column = 'geom_text'
      AND distribution_column = 'geo_bucket'
      AND precision = 5
  ) THEN
    RAISE EXCEPTION 'Geo2 distribution metadata was not visible';
  END IF;
  IF NOT EXISTS (
    SELECT 1
    FROM companion_geo_pruning_policies
    WHERE table_name = 'geo_smoke_places'
      AND geometry_column = 'geom_text'
      AND precision = 5
  ) THEN
    RAISE EXCEPTION 'Geo3 pruning metadata was not visible';
  END IF;
  BEGIN
    PERFORM companion_geo_bucket(95, 0, 5);
    RAISE EXCEPTION 'Geo2 accepted an out-of-range latitude';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'latitude must be between -90 and 90' THEN
        RAISE;
      END IF;
  END;
  BEGIN
    PERFORM companion_internal.add_geohash_column(
      'geo_smoke_places',
      'geom_text',
      'geo_bad_bucket',
      0
    );
    RAISE EXCEPTION 'Geo3 accepted an out-of-range precision';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'precision must be between 1 and 12' THEN
        RAISE;
      END IF;
  END;

  PERFORM companion_internal.plan_freeze('query-hash-1', '<Plan><Node /></Plan>', 'orders_hint');
  PERFORM companion_internal.plan_auto_promote('query-hash-1', 100, 7);
  PERFORM companion_internal.plan_regression_guard('query-hash-1', 10, 20);
  IF NOT EXISTS (
    SELECT 1
    FROM companion_plan_freezes
    WHERE query_hash = 'query-hash-1'
      AND hint_set_name = 'orders_hint'
      AND min_executions = 100
      AND stable_days = 7
      AND max_latency_regression_percent = 10
      AND max_cost_regression_percent = 20
  ) THEN
    RAISE EXCEPTION 'PM3 plan freeze state was not visible with policy metadata';
  END IF;
  plan_violation := companion_plan_regression_violates(
    'query-hash-1',
    100,
    112,
    1000,
    1000
  );
  IF NOT plan_violation THEN
    RAISE EXCEPTION 'PM4 regression guard did not flag latency regression';
  END IF;
  plan_violation := companion_plan_regression_violates(
    'query-hash-1',
    100,
    105,
    1000,
    1100
  );
  IF plan_violation THEN
    RAISE EXCEPTION 'PM4 regression guard flagged an allowed candidate';
  END IF;
  IF (
    SELECT count(*)
    FROM companion_internal.plan_regression_samples
    WHERE query_hash = 'query-hash-1'
  ) <> 2 THEN
    RAISE EXCEPTION 'PM4 regression samples were not recorded';
  END IF;
  BEGIN
    PERFORM companion_internal.plan_freeze('', '<Plan />', 'orders_hint');
    RAISE EXCEPTION 'PM3 plan_freeze accepted an empty query hash';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'query_hash must not be empty' THEN
        RAISE;
      END IF;
  END;
  BEGIN
    PERFORM companion_internal.plan_regression_guard('missing-query-hash', 10, 20);
    RAISE EXCEPTION 'PM4 regression guard accepted an unknown frozen plan';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'query_hash does not reference a frozen plan' THEN
        RAISE;
      END IF;
  END;

  generation_one := companion_internal.bump_placement_generation(102008, 'worker-a');
  generation_two := companion_internal.bump_placement_generation(102008, 'worker-a');
  IF generation_one <> 1 OR generation_two <> 2 THEN
    RAISE EXCEPTION 'S6 placement generation did not advance from 1 to 2';
  END IF;
  IF companion_placement_generation(102008) <> 2 THEN
    RAISE EXCEPTION 'S6 companion_placement_generation did not return the latest generation';
  END IF;
  IF companion_placement_generation(102009) <> 0 THEN
    RAISE EXCEPTION 'S6 unknown shard should return generation zero';
  END IF;
  IF NOT companion_local_placement_matches(102008, 'worker-a') THEN
    RAISE EXCEPTION 'S6 local placement helper did not match the recorded worker';
  END IF;
  IF companion_local_placement_matches(102008, 'worker-b') THEN
    RAISE EXCEPTION 'S6 local placement helper matched the wrong worker';
  END IF;
  BEGIN
    PERFORM companion_internal.bump_placement_generation(0, 'worker-a');
    RAISE EXCEPTION 'S6 placement generation accepted shard zero';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'shard_id must be greater than zero' THEN
        RAISE;
      END IF;
  END;

  hash_index := companion_hash_shard_index('tenant-a', 8);
  hash_index_again := companion_hash_shard_index('tenant-a', 8);
  IF hash_index <> hash_index_again OR hash_index < 0 OR hash_index >= 8 THEN
    RAISE EXCEPTION 'S13 hash routing helper was not deterministic and bounded';
  END IF;
  range_index := companion_range_shard_index(25, 0, 100, 4);
  IF range_index <> 1 THEN
    RAISE EXCEPTION 'S13 range routing helper returned %, expected 1', range_index;
  END IF;
  BEGIN
    PERFORM companion_hash_shard_index('tenant-a', 0);
    RAISE EXCEPTION 'S13 hash routing helper accepted zero shards';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'shard_count must be greater than zero' THEN
        RAISE;
      END IF;
  END;
  BEGIN
    PERFORM companion_range_shard_index(100, 0, 100, 4);
    RAISE EXCEPTION 'S13 range routing helper accepted an out-of-bounds value';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'range routing value is outside shard bounds' THEN
        RAISE;
      END IF;
  END;

  PERFORM companion_set_session_claims(
    'user-123',
    'authenticated',
    'tenant-a',
    'jti-123'
  );
  IF companion_current_tenant_id() <> 'tenant-a' THEN
    RAISE EXCEPTION 'companion_current_tenant_id did not return tenant-a';
  END IF;
  IF NOT EXISTS (
    SELECT 1
    FROM companion_current_session_claims()
    WHERE uid = 'user-123'
      AND role = 'authenticated'
      AND tenant_id = 'tenant-a'
      AND jwt_id = 'jti-123'
  ) THEN
    RAISE EXCEPTION 'companion_current_session_claims did not return expected Auth2 claims';
  END IF;
  BEGIN
    PERFORM companion_set_session_claims('', 'authenticated', 'tenant-a');
    RAISE EXCEPTION 'companion_set_session_claims must reject empty uid claim';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'uid claim must not be empty' THEN
        RAISE;
      END IF;
  END;

  jwt_header_segment := companion_internal.base64url_encode(
    convert_to('{"alg":"HS256","typ":"JWT"}', 'UTF8')
  );
  jwt_payload_segment := companion_internal.base64url_encode(
    convert_to(
      jsonb_build_object(
        'iss', 'https://auth.example.test',
        'aud', jsonb_build_array('ai-blaise-citus', 'analytics'),
        'sub', 'user-789',
        'role', 'authenticated',
        'tenant_id', 'tenant-c',
        'jti', 'jti-789',
        'exp', floor(extract(epoch FROM clock_timestamp() + interval '1 hour'))::bigint,
        'nbf', floor(extract(epoch FROM clock_timestamp() - interval '1 minute'))::bigint
      )::text,
      'UTF8'
    )
  );
  jwt_signing_input := jwt_header_segment || '.' || jwt_payload_segment;
  jwt_token := jwt_signing_input || '.' || companion_internal.base64url_encode(
    hmac(jwt_signing_input, 'jwt-secret', 'sha256')
  );

  SELECT * INTO jwt_claims
  FROM companion_verify_jwt_hs256(
    jwt_token,
    'jwt-secret',
    'https://auth.example.test',
    'ai-blaise-citus'
  );
  IF jwt_claims.uid <> 'user-789'
     OR jwt_claims.role <> 'authenticated'
     OR jwt_claims.tenant_id <> 'tenant-c'
     OR jwt_claims.jwt_id <> 'jti-789'
     OR jwt_claims.audience <> 'ai-blaise-citus' THEN
    RAISE EXCEPTION 'Sec2 JWT verification did not return expected claims';
  END IF;

  PERFORM companion_set_session_claims(
    jwt_claims.uid,
    jwt_claims.role,
    jwt_claims.tenant_id,
    jwt_claims.jwt_id
  );
  IF companion_current_tenant_id() <> 'tenant-c' THEN
    RAISE EXCEPTION 'Sec2 verified JWT claims did not feed Auth2 tenant claims';
  END IF;

  BEGIN
    PERFORM companion_verify_jwt_hs256(
      jwt_signing_input || '.bad-signature',
      'jwt-secret',
      'https://auth.example.test',
      'ai-blaise-citus'
    );
    RAISE EXCEPTION 'Sec2 JWT verification accepted a bad signature';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'JWT signature verification failed' THEN
        RAISE;
      END IF;
  END;

  BEGIN
    PERFORM companion_verify_jwt_hs256(
      jwt_token,
      'jwt-secret',
      'https://auth.example.test',
      'wrong-audience'
    );
    RAISE EXCEPTION 'Sec2 JWT verification accepted a wrong audience';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'JWT audience mismatch' THEN
        RAISE;
      END IF;
  END;

  expired_payload_segment := companion_internal.base64url_encode(
    convert_to(
      jsonb_build_object(
        'iss', 'https://auth.example.test',
        'aud', 'ai-blaise-citus',
        'sub', 'user-789',
        'role', 'authenticated',
        'tenant_id', 'tenant-c',
        'exp', floor(extract(epoch FROM clock_timestamp() - interval '1 minute'))::bigint
      )::text,
      'UTF8'
    )
  );
  jwt_signing_input := jwt_header_segment || '.' || expired_payload_segment;
  expired_token := jwt_signing_input || '.' || companion_internal.base64url_encode(
    hmac(jwt_signing_input, 'jwt-secret', 'sha256')
  );
  BEGIN
    PERFORM companion_verify_jwt_hs256(
      expired_token,
      'jwt-secret',
      'https://auth.example.test',
      'ai-blaise-citus'
    );
    RAISE EXCEPTION 'Sec2 JWT verification accepted an expired token';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'JWT has expired' THEN
        RAISE;
      END IF;
  END;

  missing_tenant_payload_segment := companion_internal.base64url_encode(
    convert_to(
      jsonb_build_object(
        'iss', 'https://auth.example.test',
        'aud', 'ai-blaise-citus',
        'sub', 'user-789',
        'role', 'authenticated',
        'exp', floor(extract(epoch FROM clock_timestamp() + interval '1 hour'))::bigint
      )::text,
      'UTF8'
    )
  );
  jwt_signing_input := jwt_header_segment || '.' || missing_tenant_payload_segment;
  missing_tenant_token := jwt_signing_input || '.' || companion_internal.base64url_encode(
    hmac(jwt_signing_input, 'jwt-secret', 'sha256')
  );
  BEGIN
    PERFORM companion_verify_jwt_hs256(
      missing_tenant_token,
      'jwt-secret',
      'https://auth.example.test',
      'ai-blaise-citus'
    );
    RAISE EXCEPTION 'Sec2 JWT verification accepted a missing tenant_id claim';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'JWT tenant_id claim must not be empty' THEN
        RAISE;
      END IF;
  END;

  plan_sql := distribute_hypertable('timescale_smoke_metrics', 'metric_time', '1 day', 4);
  IF plan_sql NOT LIKE '%create_hypertable%' THEN
    RAISE EXCEPTION 'distribute_hypertable did not render create_hypertable plan: %', plan_sql;
  END IF;

  plan_sql := time_range_shard_pruner('timescale_smoke_metrics', 'metric_time');
  IF plan_sql NOT LIKE '%enable_time_range_shard_pruner%' THEN
    RAISE EXCEPTION 'time_range_shard_pruner did not render pruner plan: %', plan_sql;
  END IF;

  PERFORM apply_distribute_hypertable(
    'timescale_smoke_metrics',
    'metric_time',
    '1 day',
    2
  );
  IF (
    SELECT count(*)
    FROM timescale_bridge_call_log
    WHERE function_name IN ('create_hypertable', 'create_distributed_table')
  ) <> 2 THEN
    RAISE EXCEPTION 'apply_distribute_hypertable did not call both dependency entrypoints';
  END IF;

  PERFORM companion_internal.create_worker_hypertables(
    'timescale_smoke_metrics'::regclass,
    'metric_time'::name,
    '1 day'::interval,
    4
  );
  PERFORM companion_internal.add_compression_policy_distributed(
    'timescale_smoke_metrics'::regclass,
    '7 days'::interval,
    ARRAY['metric_time']::text[],
    ARRAY['metric_time DESC']::text[]
  );
  PERFORM companion_internal.add_retention_policy_distributed(
    'timescale_smoke_metrics'::regclass,
    '90 days'::interval
  );
  PERFORM apply_retention_policy_distributed(
    'timescale_smoke_metrics',
    '90 days'
  );
  IF NOT EXISTS (
    SELECT 1
    FROM timescale_bridge_call_log
    WHERE function_name = 'add_retention_policy'
      AND relation_name = 'timescale_smoke_metrics'
  ) THEN
    RAISE EXCEPTION 'apply_retention_policy_distributed did not call dependency entrypoint';
  END IF;

  PERFORM companion_internal.add_reorder_policy_distributed(
    'timescale_smoke_metrics'::regclass,
    'timescale_smoke_metrics_metric_time_idx'::name
  );
  PERFORM apply_reorder_policy_distributed(
    'timescale_smoke_metrics',
    'timescale_smoke_metrics_metric_time_idx'
  );
  IF NOT EXISTS (
    SELECT 1
    FROM timescale_bridge_call_log
    WHERE function_name = 'add_reorder_policy'
      AND relation_name = 'timescale_smoke_metrics'
  ) THEN
    RAISE EXCEPTION 'apply_reorder_policy_distributed did not call dependency entrypoint';
  END IF;

  PERFORM companion_internal.add_continuous_aggregate_distributed(
    'timescale_smoke_hourly',
    'SELECT time_bucket(''1 hour'', metric_time), avg(value) FROM timescale_smoke_metrics GROUP BY 1',
    '7 days'::interval,
    '1 hour'::interval,
    '1 hour'::interval
  );
  PERFORM companion_internal.enable_time_range_shard_pruner(
    'timescale_smoke_metrics'::regclass,
    'metric_time'::name
  );
  PERFORM apply_time_range_shard_pruner(
    'timescale_smoke_metrics',
    'metric_time'
  );

  BEGIN
    PERFORM apply_compression_policy_distributed(
      'timescale_smoke_metrics',
      '7 days',
      'metric_time',
      'metric_time DESC'
    );
    RAISE EXCEPTION 'apply_compression_policy_distributed must require TimescaleDB dependency';
  EXCEPTION WHEN OTHERS THEN
    IF SQLERRM NOT LIKE '%requires visible function add_compression_policy from extension timescaledb%' THEN
      RAISE;
    END IF;
  END;

  BEGIN
    PERFORM apply_continuous_aggregate_distributed(
      'timescale_smoke_hourly_apply',
      'SELECT metric_time, avg(value) FROM timescale_smoke_metrics GROUP BY 1',
      '7 days',
      '1 hour',
      '1 hour'
    );
    RAISE EXCEPTION 'apply_continuous_aggregate_distributed must require TimescaleDB dependency';
  EXCEPTION WHEN OTHERS THEN
    IF SQLERRM NOT LIKE '%requires visible function add_continuous_aggregate_policy from extension timescaledb%' THEN
      RAISE;
    END IF;
  END;

  SELECT count(DISTINCT feature_id)
  INTO bridge_features
  FROM companion_timescale_bridge_state
  WHERE feature_id IN ('TS1', 'TS2', 'TS3', 'TS4', 'TS5', 'TS12');
  IF bridge_features <> 6 THEN
    RAISE EXCEPTION 'expected six Timescale bridge state feature ids, got %',
      bridge_features;
  END IF;

  IF NOT EXISTS (
    SELECT 1
    FROM companion_timescale_bridge_state
    WHERE feature_id = 'TS1'
      AND object_name = 'timescale_smoke_metrics'
      AND parameters->>'shard_count' = '2'
  ) THEN
    RAISE EXCEPTION 'public apply_distribute_hypertable state was not recorded';
  END IF;

  IF NOT EXISTS (SELECT 1 FROM companion_pg_stat_local_activity) THEN
    RAISE EXCEPTION 'companion_pg_stat_local_activity must report the local postgres node';
  END IF;

  IF NOT EXISTS (SELECT 1 FROM companion_pg_stat_distributed) THEN
    RAISE EXCEPTION 'compatibility companion_pg_stat_distributed view must report the local postgres node';
  END IF;

  IF NOT EXISTS (
    SELECT 1
    FROM companion_pg_stat_statements_p95
    WHERE query LIKE '%ai_blaise_pg_stat_statements_seed%'
      AND calls >= 1
      AND p95_ms >= 0
  ) THEN
    RAISE EXCEPTION 'companion_pg_stat_statements_p95 must report pg_stat_statements rows';
  END IF;

  PERFORM * FROM companion_pg_dist_replication_lag LIMIT 1;
END $$;

DO $$
DECLARE
  first_hash text;
  second_hash text;
  computed_seal text;
BEGIN
  first_hash := companion_internal.ledger_transfer(
    'tr_001',
    'cash',
    'revenue',
    5000,
    'USD',
    'genesis'
  );
  IF first_hash IS NULL OR length(first_hash) <> 64 THEN
    RAISE EXCEPTION 'Sec5 ledger transfer did not return a sha256 entry hash';
  END IF;

  second_hash := companion_internal.ledger_transfer(
    'tr_002',
    'cash',
    'deferred_revenue',
    2500,
    'USD',
    first_hash
  );
  IF second_hash IS NULL OR second_hash = first_hash THEN
    RAISE EXCEPTION 'Sec5 second ledger transfer did not advance the hash chain';
  END IF;
  IF NOT companion_ledger_chain_valid() THEN
    RAISE EXCEPTION 'Sec5 ledger chain should verify after ordered transfers';
  END IF;

  computed_seal := companion_ledger_seal('tr_001', 'ledger-secret', 'hmac-sha256');
  IF computed_seal IS NULL OR length(computed_seal) <> 64 THEN
    RAISE EXCEPTION 'Sec6 ledger seal did not return a sha256 HMAC';
  END IF;
  IF NOT EXISTS (
    SELECT 1
    FROM companion_ledger_entries
    WHERE transfer_id = 'tr_001'
      AND entry_hash = first_hash
      AND hmac_algorithm = 'hmac-sha256'
      AND companion_ledger_entries.seal = computed_seal
  ) THEN
    RAISE EXCEPTION 'Sec6 ledger seal was not visible through companion_ledger_entries';
  END IF;

  BEGIN
    PERFORM companion_internal.ledger_transfer(
      'tr_bad_prev',
      'cash',
      'revenue',
      100,
      'USD',
      'missing-hash'
    );
    RAISE EXCEPTION 'Sec5 ledger transfer accepted a missing previous hash';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'previous_hash does not reference an existing ledger entry' THEN
        RAISE;
      END IF;
  END;

  BEGIN
    UPDATE companion_internal.ledger_entries
    SET amount_cents = 1
    WHERE transfer_id = 'tr_001';
    RAISE EXCEPTION 'Sec5 ledger entries must reject mutation';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'companion ledger is append-only' THEN
        RAISE;
      END IF;
  END;

  BEGIN
    DELETE FROM companion_internal.ledger_seals
    WHERE transfer_id = 'tr_001';
    RAISE EXCEPTION 'Sec6 ledger seals must reject deletion';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'companion ledger is append-only' THEN
        RAISE;
      END IF;
  END;

  BEGIN
    PERFORM companion_ledger_seal('tr_002', 'ledger-secret', 'hmac-md5');
    RAISE EXCEPTION 'Sec6 ledger seal accepted an unsupported algorithm';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'unsupported ledger HMAC algorithm: hmac-md5' THEN
        RAISE;
      END IF;
  END;
END $$;


DO $$
DECLARE
  staged jsonb;
  finalized jsonb;
  waiting jsonb;
  aborted jsonb;
BEGIN
  staged := companion.txn_stage(
    'txn-sql-1',
    'worker-a',
    1700000000,
    '[{"shard_id":10,"key_range":"[a,m)","required_acks":2,"replica_acks":2},{"shard_id":11,"key_range":"[m,z)","required_acks":2,"replica_acks":2}]'::jsonb
  );
  IF staged->>'status' <> 'staging' THEN
    RAISE EXCEPTION 'T5 txn_stage did not return staging status: %', staged;
  END IF;
  finalized := companion.txn_finalize('txn-sql-1', 1700000010);
  IF finalized->>'decision' <> 'commit' OR finalized->>'status' <> 'committed' THEN
    RAISE EXCEPTION 'T5 txn_finalize did not commit with full evidence: %', finalized;
  END IF;

  PERFORM companion.txn_stage(
    'txn-sql-2',
    'worker-a',
    1700000000,
    '[{"shard_id":10,"key_range":"[a,m)","required_acks":2,"replica_acks":1}]'::jsonb
  );
  waiting := companion.txn_finalize('txn-sql-2', 1700000010);
  IF waiting->>'decision' <> 'wait_for_replication_evidence'
      OR waiting->>'status' <> 'staging' THEN
    RAISE EXCEPTION 'T5 txn_finalize did not wait without evidence: %', waiting;
  END IF;
  aborted := companion.txn_finalize('txn-sql-2', 1700006001);
  IF aborted->>'decision' <> 'abort_stale_staging_record'
      OR aborted->>'status' <> 'aborted' THEN
    RAISE EXCEPTION 'T5 stale staging record did not abort: %', aborted;
  END IF;

  BEGIN
    PERFORM companion.txn_stage(
      'txn-sql-1',
      'worker-a',
      1700000000,
      '[{"shard_id":10,"key_range":"[a,m)","required_acks":2,"replica_acks":2}]'::jsonb
    );
    RAISE EXCEPTION 'T5 txn_stage accepted a duplicate txn_id';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM NOT LIKE 'txn_id already staged:%' THEN
        RAISE;
      END IF;
  END;

  BEGIN
    PERFORM companion.txn_stage('txn-sql-empty', 'worker-a', 1700000000, '[]'::jsonb);
    RAISE EXCEPTION 'T5 txn_stage accepted empty intents';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'intents must not be empty' THEN
        RAISE;
      END IF;
  END;
END $$;

CREATE ROLE ai_blaise_rls_smoke;
CREATE TABLE rls_smoke_orders (
  order_id integer NOT NULL,
  tenant_id text NOT NULL,
  amount integer NOT NULL
);
INSERT INTO rls_smoke_orders(order_id, tenant_id, amount)
VALUES
  (1, 'tenant-a', 100),
  (2, 'tenant-b', 200);
ALTER TABLE rls_smoke_orders ENABLE ROW LEVEL SECURITY;
ALTER TABLE rls_smoke_orders FORCE ROW LEVEL SECURITY;
CREATE POLICY rls_smoke_tenant_isolation ON rls_smoke_orders
USING (companion_tenant_id_matches(tenant_id))
WITH CHECK (companion_tenant_id_matches(tenant_id));
GRANT SELECT, INSERT ON rls_smoke_orders TO ai_blaise_rls_smoke;

SELECT companion_set_session_claims('user-123', 'authenticated', 'tenant-a', 'jti-123');
SET ROLE ai_blaise_rls_smoke;
DO $$
DECLARE
  visible_count integer;
BEGIN
  SELECT count(*) INTO visible_count FROM rls_smoke_orders;
  IF visible_count <> 1 THEN
    RAISE EXCEPTION 'Sec1 RLS tenant-a should see exactly one row, got %',
      visible_count;
  END IF;
  IF NOT companion_tenant_id_matches('tenant-a') THEN
    RAISE EXCEPTION 'companion_tenant_id_matches must accept the active tenant';
  END IF;
  IF companion_tenant_id_matches('tenant-b') THEN
    RAISE EXCEPTION 'companion_tenant_id_matches must reject another tenant';
  END IF;
  INSERT INTO rls_smoke_orders(order_id, tenant_id, amount)
  VALUES (3, 'tenant-a', 300);
  BEGIN
    INSERT INTO rls_smoke_orders(order_id, tenant_id, amount)
    VALUES (4, 'tenant-b', 400);
    RAISE EXCEPTION 'Sec1 RLS WITH CHECK allowed a cross-tenant insert';
  EXCEPTION
    WHEN insufficient_privilege THEN
      NULL;
  END;
END $$;
RESET ROLE;

SELECT companion_set_session_claims('user-456', 'authenticated', 'tenant-b', 'jti-456');
SET ROLE ai_blaise_rls_smoke;
DO $$
DECLARE
  visible_count integer;
BEGIN
  SELECT count(*) INTO visible_count FROM rls_smoke_orders;
  IF visible_count <> 1 THEN
    RAISE EXCEPTION 'Sec1 RLS tenant-b should see exactly one row, got %',
      visible_count;
  END IF;
END $$;
RESET ROLE;

SELECT set_config('ai_blaise.claim.tenant_id', '', false);
SET ROLE ai_blaise_rls_smoke;
DO $$
DECLARE
  visible_count integer;
BEGIN
  SELECT count(*) INTO visible_count FROM rls_smoke_orders;
  IF visible_count <> 0 THEN
    RAISE EXCEPTION 'Sec1 RLS without tenant claim should see zero rows, got %',
      visible_count;
  END IF;
  BEGIN
    PERFORM companion_require_tenant_id();
    RAISE EXCEPTION 'companion_require_tenant_id must fail without tenant claim';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'tenant_id claim must be set for RLS' THEN
        RAISE;
      END IF;
  END;
END $$;
RESET ROLE;
SQL

  docker exec -d "${container}" sh -c \
    "(printf 'BEGIN;\nSELECT pg_backend_pid();\n'; sleep 60; printf 'COMMIT;\n') | psql -U postgres -v ON_ERROR_STOP=1"

  local idle_seen=0
  local idle_count
  for _ in $(seq 1 20); do
    idle_count="$(
      docker exec "${container}" psql -U postgres -Atqv ON_ERROR_STOP=1 \
        -c "SELECT count(*) FROM companion_idle_transactions('100 milliseconds'::interval) WHERE state = 'idle in transaction';"
    )"
    if [[ "${idle_count}" =~ ^[1-9][0-9]*$ ]]; then
      idle_seen=1
      break
    fi
    sleep 1
  done

  if [[ "${idle_seen}" != "1" ]]; then
    docker exec "${container}" psql -U postgres -v ON_ERROR_STOP=1 \
      -c "SELECT pid, state, xact_start, query FROM pg_stat_activity WHERE datname = current_database() ORDER BY pid;" >&2 || true
    echo "companion_idle_transactions did not detect a real idle transaction (PG${pg_major})" >&2
    exit 1
  fi

  docker rm -f "${container}" >/dev/null 2>&1 || true
  active_container=""
  echo "ai_blaise_citus SQL extension smoke passed with ${postgres_image}"
}


run_bundle1_source_build_smoke() {
  # Bundle1 production-ready smoke: light target ships a complete PGDG +
  # source-build bundle. shared_preload_libraries matches the canonical
  # shared-preload-libraries.conf so initdb exercises the full required
  # cohabitation set.
  local target="bundle1-final-light"
  local preload_libraries="citus,timescaledb,pgaudit,pgauditlogtofile,pgsodium,pg_cron,age,pg_failover_slots,pgnodemx"
  local -a expected_extensions=(
    ai_blaise_citus
    citus
    timescaledb
    vector
    pg_cron
    pg_partman
    pgaudit
    pgauditlogtofile
    pgsodium
    hll
    topn
    tdigest
    pgnodemx
    postgis
    pg_graphql
    pg_jsonschema
    age
    pg_uuidv7
    pg_repack
    pg_warm
    pgcrypto
    pg_trgm
    citext
    rum
    pg_prewarm
  )
  if [[ "${bundle1_build_heavy}" == "1" ]]; then
    target="bundle1-final-full"
    expected_extensions+=(pg_search plv8)
  fi

  echo "=== bundle1 source-build smoke (${target}) ==="
  local source_git_sha
  local source_tree_state
  source_git_sha="$(git rev-parse HEAD)"
  source_tree_state="clean"
  if [[ -n "$(git status --porcelain)" ]]; then
    source_tree_state="dirty"
  fi
  # Pre-pull the build base image with bounded retry so the FROM step
  # in docker build does not flake on registry-1.docker.io transients.
  for attempt in 1 2 3; do
    if docker pull postgres:17-bookworm >/dev/null; then break; fi
    if [ "${attempt}" = "3" ]; then
      echo "docker pull postgres:17-bookworm failed after 3 attempts" >&2; exit 1
    fi
    sleep 5
  done
  docker build \
    -f images/citus-pg-overlay/Dockerfile \
    --target "${target}" \
    --build-arg PG_MAJOR=17 \
    --build-arg BASE_IMAGE=postgres:17-bookworm \
    --build-arg AI_BLAISE_SOURCE_GIT_SHA="${source_git_sha}" \
    --build-arg AI_BLAISE_SOURCE_TREE_STATE="${source_tree_state}" \
    -t "${bundle1_image}" \
    .


  local observed_source_git_sha
  observed_source_git_sha="$(docker image inspect -f '{{ index .Config.Labels "ai-blaise.citus.source-git-sha" }}' "${bundle1_image}")"
  if [[ "${observed_source_git_sha}" != "${source_git_sha}" ]]; then
    echo "bundle1 image source-git-sha label mismatch: expected ${source_git_sha}, observed ${observed_source_git_sha}" >&2
    exit 1
  fi
  local observed_source_tree_state
  observed_source_tree_state="$(docker image inspect -f '{{ index .Config.Labels "ai-blaise.citus.source-tree-state" }}' "${bundle1_image}")"
  if [[ "${observed_source_tree_state}" != "${source_tree_state}" ]]; then
    echo "bundle1 image source-tree-state label mismatch: expected ${source_tree_state}, observed ${observed_source_tree_state}" >&2
    exit 1
  fi
  local evidence_scope
  evidence_scope="$(docker image inspect -f '{{ index .Config.Labels "ai-blaise.citus.bundle1.evidence-scope" }}' "${bundle1_image}")"
  if [[ "${evidence_scope}" != "full-bundle-required-minus-plrust" ]]; then
    echo "bundle1 image evidence-scope label mismatch: ${evidence_scope} (expected full-bundle-required-minus-plrust)" >&2
    exit 1
  fi
  local full_initdb_path
  full_initdb_path="$(docker image inspect -f '{{ index .Config.Labels "ai-blaise.citus.bundle1.full-initdb-path" }}' "${bundle1_image}")"
  if [[ "${full_initdb_path}" != "true" ]]; then
    echo "bundle1 image must claim complete initdb evidence (observed ${full_initdb_path}, expected true)" >&2
    exit 1
  fi
  docker run --rm --entrypoint /bin/sh "${bundle1_image}" -c '
    set -eu
    test -s /usr/local/share/ai-blaise/citus/bundle1-source-build.lock.tsv
    grep -Fq "pgsodium" /usr/local/share/ai-blaise/citus/bundle1-source-build.lock.tsv
  ' >/dev/null

  local getkey_error_file
  getkey_error_file="$(mktemp)"
  if docker run --rm --entrypoint /usr/share/postgresql/17/extension/pgsodium_getkey "${bundle1_image}" >"${getkey_error_file}" 2>&1; then
    cat "${getkey_error_file}" >&2
    rm -f "${getkey_error_file}"
    echo "pgsodium_getkey succeeded without a configured key" >&2
    exit 1
  fi
  if ! grep -Fq "pgsodium key unavailable" "${getkey_error_file}"; then
    cat "${getkey_error_file}" >&2
    rm -f "${getkey_error_file}"
    echo "pgsodium_getkey did not fail closed with the expected error" >&2
    exit 1
  fi
  rm -f "${getkey_error_file}"
  local observed_key
  observed_key="$(docker run --rm \
    -e PGSODIUM_KEY=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef \
    --entrypoint /usr/share/postgresql/17/extension/pgsodium_getkey \
    "${bundle1_image}")"
  if [[ "${observed_key}" != "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef" ]]; then
    echo "pgsodium_getkey did not emit the configured deterministic test key" >&2
    exit 1
  fi

  local container="ai-blaise-bundle1-source-smoke-${RANDOM}-$$"
  active_container="${container}"
  docker run \
    --name "${container}" \
    -e POSTGRES_PASSWORD=postgres \
    -e PGSODIUM_KEY=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef \
    -d "${bundle1_image}" \
    -c "shared_preload_libraries=${preload_libraries}" >/dev/null

  # Wait for docker-entrypoint to finish initdb + run init scripts + restart
  # postgres. The temporary postgres during init responds to SELECT 1 too,
  # so a bare ready check races the shutdown/restart transition.
  local init_complete=0
  local _
  for _ in $(seq 1 240); do
    if docker logs "${container}" 2>&1 | grep -q "PostgreSQL init process complete"; then
      init_complete=1
      break
    fi
    if ! docker inspect -f '{{.State.Running}}' "${container}" 2>/dev/null | grep -q true; then
      docker logs "${container}" >&2 || true
      echo "bundle1 source-build container exited during init" >&2
      exit 1
    fi
    sleep 1
  done
  if [[ "${init_complete}" != "1" ]]; then
    docker logs "${container}" >&2 || true
    echo "bundle1 source-build container did not finish init scripts" >&2
    exit 1
  fi

  local ready=0
  for _ in $(seq 1 120); do
    if docker exec "${container}" psql -U postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
      ready=1
      break
    fi
    sleep 1
  done
  if [[ "${ready}" != "1" ]]; then
    docker logs "${container}" >&2 || true
    echo "bundle1 source-build container did not become ready" >&2
    exit 1
  fi

  local extension
  for extension in "${expected_extensions[@]}"; do
    docker exec "${container}" test -s "/usr/share/postgresql/17/extension/${extension}.control"
  done

  # Initdb already ran /docker-entrypoint-initdb.d/00-ai-blaise-extensions.sql
  # during container startup; verify pg_extension catalog records every
  # required production extension as installed.
  docker exec -i "${container}" psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
-- Verify the critical bundle1 production-ready extensions exist after initdb.
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_warm') THEN
    RAISE EXCEPTION 'Bundle1 initdb path did not create pg_warm extension';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'ai_blaise_citus') THEN
    RAISE EXCEPTION 'Bundle1 initdb path did not create ai_blaise_citus extension';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'citus') THEN
    RAISE EXCEPTION 'Bundle1 initdb path did not create citus extension';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'timescaledb') THEN
    RAISE EXCEPTION 'Bundle1 initdb path did not create timescaledb extension';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'vector') THEN
    RAISE EXCEPTION 'Bundle1 initdb path did not create vector extension';
  END IF;
END;
$$;
CREATE TABLE bundle1_warm_smoke(id integer PRIMARY KEY);
INSERT INTO bundle1_warm_smoke VALUES (1);
SELECT pg_warm('bundle1_warm_smoke'::regclass);
SELECT companion_internal.seed_extension_catalog();
SQL

  # Verify pg_extension catalog records every expected extension
  local missing_exts=""
  local ext
  for ext in "${expected_extensions[@]}"; do
    if [[ "${ext}" == "pg_prewarm" ]]; then continue; fi
    local present
    present="$(docker exec "${container}" psql -U postgres -Atqc "SELECT 1 FROM pg_extension WHERE extname='${ext}'")"
    if [[ "${present}" != "1" ]]; then
      missing_exts="${missing_exts} ${ext}"
    fi
  done
  if [[ -n "${missing_exts}" ]]; then
    docker logs "${container}" >&2 || true
    echo "bundle1 initdb path did not install extensions:${missing_exts}" >&2
    exit 1
  fi

  if [[ "${bundle1_build_heavy}" == "1" ]]; then
    docker exec -i "${container}" psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
CREATE EXTENSION pg_search;
CREATE EXTENSION plv8;
SELECT plv8_version();
SQL
  fi

  local image_id
  image_id="$(docker image inspect -f '{{.Id}}' "${bundle1_image}")"
  if [[ -n "${bundle1_evidence_file}" ]]; then
    if [[ ! -f "${bundle1_evidence_file}" ]]; then
      printf 'observed_at\tgit_sha\ttarget\timage_id\textensions\n' >"${bundle1_evidence_file}"
    fi
    printf '%s\t%s\t%s\t%s\t%s\n' \
      "$(date -Is)" \
      "${source_git_sha}" \
      "${target}" \
      "${image_id}" \
      "${expected_extensions[*]}" >>"${bundle1_evidence_file}"
  fi

  docker rm -f "${container}" >/dev/null 2>&1 || true
  active_container=""
  echo "bundle1 source-build smoke passed for ${target}: ${expected_extensions[*]}"
}

for pg_major in "${pg_majors[@]}"; do
  if ! [[ "${pg_major}" =~ ^[0-9]+$ ]]; then
    echo "invalid PG_MAJOR value: '${pg_major}' (must be numeric)" >&2
    exit 1
  fi
  run_smoke_for_pg_major "${pg_major}"
done

echo "ai_blaise_citus SQL extension smoke passed across PG majors: ${pg_majors[*]}"

if [[ "${bundle1_build_image}" == "1" ]]; then
  run_bundle1_source_build_smoke
fi
