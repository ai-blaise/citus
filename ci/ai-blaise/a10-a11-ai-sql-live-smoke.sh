#!/usr/bin/env bash
# FEATURE: A10 A11
#
# Live AI SQL execution smoke for A10 (Streaming Chat Completion UDF) and
# A11 (Semantic Catalog Text-To-SQL). Boots an OpenAI-compatible mock HTTP
# server, boots the immutable real-Citus PG17 HTTP test fixture, registers a provider
# binding, sets the companion.ai_endpoint_override GUC, and exercises both
# A10 streaming and A11 generated-SQL execution end to end.
#
# Mock server vs real LLM: the OpenAI-compatible mock at
# ci/ai-blaise/mock-llm/server.py exercises the same HTTP+JSON code path
# the SQL extension would use against any OpenAI-API-compatible provider
# (Ollama, vLLM, OpenAI, Azure OpenAI, Together, Anthropic-via-LiteLLM).
# Pointing the GUC at a real provider URL with credentials in the binding
# secret_ref is the production deployment path.
#
# Safety: A11 generated-SQL execution validates the LLM output against the
# deterministic template shape (SELECT-only, configured relation, tenant_id
# filter, LIMIT clause) before EXECUTE. Forbidden write/DDL keywords cause
# fail-closed; statement_timeout=2s bounds runaway queries.

set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
http_fixture_builder="${repo_root}/ci/ai-blaise/build-real-citus-http-test-fixture.sh"
fixture_contract="${repo_root}/ci/ai-blaise/real-citus-test-fixture-contract.py"
mock_server="${repo_root}/ci/ai-blaise/mock-llm/server.py"
mock_image="docker.io/library/python:3.12-slim@sha256:78387bc3881b8273120a12ebe6c1ab22b018ccc2c9adf565ae1ac9b536e184ea"
pg_major=17

for file in "${http_fixture_builder}" "${fixture_contract}" "${mock_server}"; do
  if [[ ! -s "${file}" ]]; then
    echo "missing A10/A11 real-Citus HTTP fixture artifact: ${file}" >&2
    exit 1
  fi
done
if [[ ! -x "${http_fixture_builder}" ]]; then
  echo "real-Citus HTTP test fixture builder is not executable" >&2
  exit 1
fi

python3 "${fixture_contract}"

if ! command -v docker >/dev/null 2>&1; then
  if [[ "${REQUIRE_DOCKER:-0}" == "1" ]]; then echo "docker required" >&2; exit 1; fi
  echo "docker unavailable; skipping A10/A11 smoke"; exit 0
fi

evidence_dir="${A10_A11_EVIDENCE_DIR:-artifacts}"
mkdir -p "${evidence_dir}"
evidence_file="${A10_A11_EVIDENCE_FILE:-${evidence_dir}/a10-a11-ai-sql-evidence.tsv}"
fixture_image="$("${http_fixture_builder}" --pg-major "${pg_major}")"

network="a10-a11-${RANDOM}-$$"
pg_container="a10-a11-pg-${RANDOM}-$$"
mock_container="a10-a11-mock-${RANDOM}-$$"

cleanup() {
  docker rm --force --volumes "${pg_container}" "${mock_container}" >/dev/null 2>&1 || true
  docker network rm "${network}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

log() { printf '[a10-a11] %s\n' "$*" >&2; }

log "creating network + booting mock LLM server"
docker network create "${network}" >/dev/null
docker run -d --name "${mock_container}" --network "${network}" --network-alias mock-llm \
  -v "${mock_server}":/server.py:ro \
  "${mock_image}" python3 /server.py 8765 >/dev/null

mock_ready=0
for _ in $(seq 1 60); do
  if docker exec "${mock_container}" python3 -c 'import urllib.request; urllib.request.urlopen("http://localhost:8765/healthz")' >/dev/null 2>&1; then
    mock_ready=1
    break
  fi
  sleep 1
done
if [[ "${mock_ready}" != "1" ]]; then
  docker logs "${mock_container}" >&2 || true
  echo "mock LLM server did not become ready" >&2
  exit 1
fi

log "booting immutable real-Citus PG17 HTTP fixture"
docker run -d --name "${pg_container}" --network "${network}" \
  -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=app \
  -d "${fixture_image}" >/dev/null

postgres_init_complete=0
for _ in $(seq 1 120); do
  if docker logs "${pg_container}" 2>&1 | grep -q "PostgreSQL init process complete"; then
    postgres_init_complete=1
    break
  fi
  sleep 1
done
if [[ "${postgres_init_complete}" != "1" ]]; then
  docker logs "${pg_container}" >&2 || true
  echo "real-Citus HTTP fixture did not complete PostgreSQL initialization" >&2
  exit 1
fi

postgres_ready=0
for _ in $(seq 1 90); do
  if docker exec "${pg_container}" psql -U postgres -d app -Atqc 'SELECT 1' >/dev/null 2>&1; then
    postgres_ready=1
    break
  fi
  sleep 1
done
if [[ "${postgres_ready}" != "1" ]]; then
  docker logs "${pg_container}" >&2 || true
  echo "real-Citus HTTP fixture did not become SQL-ready" >&2
  exit 1
fi

docker exec -i "${pg_container}" psql -U postgres -d app -v ON_ERROR_STOP=1 <<'SQL'
CREATE EXTENSION citus;
CREATE EXTENSION pgcrypto;
CREATE EXTENSION http;
CREATE EXTENSION ai_blaise_citus;
DO $$
BEGIN
  IF (SELECT extversion FROM pg_extension WHERE extname = 'ai_blaise_citus')
      IS DISTINCT FROM '0.1.2' THEN
    RAISE EXCEPTION 'expected shipped ai_blaise_citus version 0.1.2';
  END IF;
END $$;

SET companion.ai_endpoint_override = 'http://mock-llm:8765/v1/chat/completions';

SELECT companion_internal.register_ai_provider_binding(
  p_binding_name => 'mock-ollama',
  p_tenant_id => 'tenant-a',
  p_provider => 'ollama',
  p_model => 'mock-llm-1.0',
  p_secret_ref => 'secret://mock/ollama-key',
  p_max_tokens_per_request => 256
);

CREATE TABLE orders (
  order_id bigint PRIMARY KEY,
  tenant_id text NOT NULL,
  amount_cents bigint NOT NULL
);
INSERT INTO orders SELECT i, 'tenant-a', i*100 FROM generate_series(1, 5) AS s(i);

INSERT INTO companion_internal.semantic_catalog_objects(
  tenant_id, object_name, relation_name, allowed_columns, description
) VALUES (
  'tenant-a', 'orders', 'orders', ARRAY['amount_cents','tenant_id'], 'Tenant orders table for semantic catalog tests'
);
SQL

log "phase 1: A10 live streaming chat completion"
a10_output="$(docker exec "${pg_container}" psql -U postgres -d app -v ON_ERROR_STOP=1 -Atc "
SET companion.ai_endpoint_override = 'http://mock-llm:8765/v1/chat/completions';
SELECT count(*) || ':' || (
  SELECT string_agg(event, ',' ORDER BY chunk_index)
  FROM companion_ai_chat_stream(
    'tenant-a', 'mock-ollama',
    '[{\"role\": \"user\", \"content\": \"summarize last 5 orders please\"}]'::jsonb,
    256, 0, true
  ) src
)
FROM companion_ai_chat_stream(
  'tenant-a', 'mock-ollama',
  '[{\"role\": \"user\", \"content\": \"summarize last 5 orders please\"}]'::jsonb,
  256, 0, true
)")"
log "A10 output: ${a10_output}"
a10_chunk_count="$(echo "${a10_output}" | tail -1 | cut -d: -f1)"
if [[ "${a10_chunk_count}" -lt 2 ]]; then
  echo "A10 should return >= 2 chunks (got ${a10_chunk_count})" >&2; exit 1
fi

log "phase 2: A11 live text-to-SQL with safety validation + execution"
a11_output="$(docker exec "${pg_container}" psql -U postgres -d app -v ON_ERROR_STOP=1 -Atc "
SET companion.ai_endpoint_override = 'http://mock-llm:8765/v1/chat/completions';
SELECT (companion_semantic_text_to_sql_intent(
  'tenant-a',
  'list the amount_cents and tenant_id for the orders please as sql',
  ARRAY['orders'],
  'mock-ollama',
  true
))::text")"
log "A11 output: ${a11_output}"

a11_executed_rows="$(echo "${a11_output}" | python3 ci/ai-blaise/mock-llm/extract_a11.py executed_rows)"
a11_evidence_boundary="$(echo "${a11_output}" | python3 ci/ai-blaise/mock-llm/extract_a11.py evidence_boundary)"
if [[ "${a11_executed_rows}" != "5" ]]; then
  echo "A11 executed_rows expected 5 (5 inserted orders), got ${a11_executed_rows}" >&2; exit 1
fi
if [[ "${a11_evidence_boundary}" != "live-provider-execution-safety-validated" ]]; then
  echo "A11 evidence_boundary should be live-provider-execution-safety-validated" >&2; exit 1
fi

if ! observed_at="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"; then
  echo "could not capture the A10/A11 evidence timestamp" >&2
  exit 1
fi
if [[ ! "${observed_at}" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$ ]]; then
  echo "A10/A11 evidence timestamp is not canonical UTC RFC3339" >&2
  exit 1
fi
if ! git_sha="$(git rev-parse --verify 'HEAD^{commit}')"; then
  echo "could not capture the A10/A11 evidence Git commit" >&2
  exit 1
fi
if [[ ! "${git_sha}" =~ ^[0-9a-f]{40}$ ]]; then
  echo "A10/A11 evidence Git commit is malformed" >&2
  exit 1
fi

mkdir -p "$(dirname "${evidence_file}")"
if [[ ! -f "${evidence_file}" ]]; then
  printf 'observed_at\tgit_sha\ta10_chunk_count\ta11_executed_rows\ta11_evidence_boundary\n' >"${evidence_file}"
fi
printf '%s\t%s\t%s\t%s\t%s\n' \
  "${observed_at}" "${git_sha}" \
  "${a10_chunk_count}" "${a11_executed_rows}" "${a11_evidence_boundary}" >>"${evidence_file}"

printf 'a10_a11_ai_sql_live\tpassed\ta10_chunk_count=%s\ta11_executed_rows=%s\ta11_evidence_boundary=%s\n' \
  "${a10_chunk_count}" "${a11_executed_rows}" "${a11_evidence_boundary}"
echo "A10/A11 AI SQL live smoke passed"
