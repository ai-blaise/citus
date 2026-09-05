#!/usr/bin/env bash
# FEATURE: A10 A11
#
# Focused SQL-visible contract smoke for the AI UDF surfaces. This proves
# deterministic, fail-closed request-intent behavior only; it does not call a live LLM provider or execute generated SQL.
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
extension_dir="${repo_root}/images/citus-pg-overlay/extensions"
fixture_builder="${repo_root}/ci/ai-blaise/build-real-citus-test-fixture.sh"
fixture_contract="${repo_root}/ci/ai-blaise/real-citus-test-fixture-contract.py"
control_file="${extension_dir}/ai_blaise_citus.control"
sql_file="${extension_dir}/ai_blaise_citus--0.1.0.sql"
upgrade_sql="${extension_dir}/ai_blaise_citus--0.1.0--0.1.1.sql"
downgrade_sql="${extension_dir}/ai_blaise_citus--0.1.1--0.1.0.sql"
security_sql="${extension_dir}/ai_blaise_citus--0.1.1--0.1.2.sql"
require_docker="${REQUIRE_DOCKER:-0}"
pg_major=17

for file in \
  "${fixture_builder}" \
  "${fixture_contract}" \
  "${control_file}" \
  "${sql_file}" \
  "${upgrade_sql}" \
  "${downgrade_sql}" \
  "${security_sql}"; do
  if [[ ! -s "${file}" ]]; then
    echo "missing AI SQL contract smoke artifact: ${file}" >&2
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
    echo "docker is required for AI SQL contract smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping AI SQL contract smoke"
  exit 0
fi

fixture_image="$("${fixture_builder}" --pg-major "${pg_major}")"
container="ai-blaise-ai-sql-contract-smoke-${RANDOM}-$$"
cleanup() {
  docker rm --force --volumes "${container}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run --name "${container}" \
  --network none \
  -e POSTGRES_PASSWORD=postgres \
  -d "${fixture_image}" >/dev/null

init_complete=0
for _ in $(seq 1 120); do
  if docker logs "${container}" 2>&1 | grep -q "PostgreSQL init process complete"; then
    init_complete=1
    break
  fi
  sleep 1
done
if [[ "${init_complete}" != "1" ]]; then
  echo "real-Citus fixture did not complete PostgreSQL initialization" >&2
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
  echo "real-Citus fixture did not become SQL-ready" >&2
  exit 1
fi

docker exec -i "${container}" psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
CREATE EXTENSION citus;
CREATE EXTENSION pgcrypto;
CREATE EXTENSION ai_blaise_citus;
DO $$
BEGIN
  IF to_regclass('pg_catalog.pg_dist_node') IS NULL
     OR to_regprocedure(
          'pg_catalog.citus_add_node(text,integer,integer,noderole,name)'
        ) IS NULL THEN
    RAISE EXCEPTION 'real Citus catalog or function surface is missing';
  END IF;
  IF (SELECT extversion FROM pg_extension WHERE extname = 'ai_blaise_citus')
      IS DISTINCT FROM '0.1.2' THEN
    RAISE EXCEPTION 'expected shipped ai_blaise_citus version 0.1.2';
  END IF;
END $$;

CREATE TABLE ai_sql_contract_docs (
  doc_id text PRIMARY KEY,
  tenant_id text NOT NULL,
  body text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

DO $$
DECLARE
  binding_name text;
  chat_event record;
  text_to_sql jsonb;
BEGIN
  IF NOT EXISTS (
    SELECT 1
    FROM companion_feature_status()
    WHERE feature_id = 'A10'
      AND status = 'sql-intent-fail-closed'
  ) THEN
    RAISE EXCEPTION 'A10 SQL intent status row missing';
  END IF;
  IF NOT EXISTS (
    SELECT 1
    FROM companion_feature_status()
    WHERE feature_id = 'A11'
      AND status = 'sql-intent-fail-closed'
  ) THEN
    RAISE EXCEPTION 'A11 SQL intent status row missing';
  END IF;

  binding_name := companion_internal.register_ai_provider_binding(
    'tenant_a_openai_chat',
    'tenant-a',
    'openai',
    'gpt-4.1-mini',
    'secret://tenant-a/openai',
    4096
  );
  IF binding_name <> 'tenant_a_openai_chat' THEN
    RAISE EXCEPTION 'AI provider binding returned unexpected name: %', binding_name;
  END IF;
  IF EXISTS (
    SELECT 1
    FROM companion_ai_provider_bindings AS binding
    WHERE binding.binding_name = 'tenant_a_openai_chat'
      AND binding.has_secret_ref
      AND binding.secret_ref_fingerprint IS NOT NULL
      AND binding.provider = 'openai'
      AND binding.model = 'gpt-4.1-mini'
  ) IS NOT TRUE THEN
    RAISE EXCEPTION 'AI provider binding view did not expose redacted binding metadata';
  END IF;

  SELECT * INTO chat_event
  FROM companion_ai_chat_stream(
    'tenant-a',
    'tenant_a_openai_chat',
    '[{"role":"system","content":"Answer using catalog facts only."},{"role":"user","content":"Summarize open orders."}]'::jsonb,
    512,
    0.2
  );
  IF chat_event.event <> 'request_intent'
     OR chat_event.payload ->> 'feature_id' <> 'A10'
     OR chat_event.payload ->> 'evidence_boundary' <> 'sql-intent-fail-closed-only'
     OR (chat_event.payload ->> 'provider_runtime_available')::boolean IS DISTINCT FROM false
     OR (chat_event.payload ->> 'secret_bound')::boolean IS DISTINCT FROM true THEN
    RAISE EXCEPTION 'A10 chat stream intent payload was not fail-closed: %', chat_event.payload;
  END IF;

  BEGIN
    PERFORM *
    FROM companion_ai_chat_stream(
      'tenant-a',
      'tenant_a_openai_chat',
      '[{"role":"tool","content":"bad"}]'::jsonb,
      64
    );
    RAISE EXCEPTION 'A10 accepted unsupported chat role';
  EXCEPTION WHEN raise_exception THEN
    IF SQLERRM <> 'unsupported chat message role: tool' THEN
      RAISE;
    END IF;
  END;

  PERFORM companion_internal.register_semantic_catalog_object(
    'tenant-a',
    'orders',
    'ai_sql_contract_docs',
    ARRAY['doc_id', 'tenant_id', 'body'],
    'Tenant-scoped support documents for safe text-to-SQL intent tests.'
  );
  text_to_sql := companion_semantic_text_to_sql_intent(
    'tenant-a',
    'show support documents for this tenant',
    ARRAY['orders'],
    'tenant_a_openai_chat'
  );
  IF text_to_sql ->> 'feature_id' <> 'A11'
     OR text_to_sql ->> 'evidence_boundary' <> 'sql-intent-fail-closed-only'
     OR (text_to_sql ->> 'execution_allowed')::boolean IS DISTINCT FROM false
     OR text_to_sql ->> 'sql_template' <> 'SELECT body, doc_id, tenant_id FROM ai_sql_contract_docs WHERE tenant_id = $1 LIMIT 100' THEN
    RAISE EXCEPTION 'A11 text-to-SQL intent payload was not deterministic/fail-closed: %', text_to_sql;
  END IF;

  BEGIN
    PERFORM companion_semantic_text_to_sql_intent(
      'tenant-a',
      'drop table users;',
      ARRAY['orders']
    );
    RAISE EXCEPTION 'A11 accepted SQL-control text in question';
  EXCEPTION WHEN raise_exception THEN
    IF SQLERRM <> 'question contains unsupported SQL-control text' THEN
      RAISE;
    END IF;
  END;
  BEGIN
    PERFORM companion_semantic_text_to_sql_intent(
      'tenant-a',
      'show support documents',
      ARRAY['missing_object']
    );
    RAISE EXCEPTION 'A11 accepted unregistered semantic object';
  EXCEPTION WHEN raise_exception THEN
    IF SQLERRM <> 'all catalog_objects must be registered for tenant' THEN
      RAISE;
    END IF;
  END;
END;
$$;
SQL

echo "OK ai-sql-contract smoke: A10/A11 sql-intent-fail-closed-only"
