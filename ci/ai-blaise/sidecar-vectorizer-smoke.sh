#!/usr/bin/env bash
set -euo pipefail

# FEATURE: A2
# FEATURE: A3
# FEATURE: A4
# FEATURE: A5
# FEATURE: A6

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

postgres_image="${VECTORIZER_SMOKE_POSTGRES_IMAGE:-postgres:17}"
require_docker="${REQUIRE_DOCKER:-0}"

if ! command -v docker >/dev/null 2>&1; then
  if [[ "${require_docker}" == "1" ]]; then
    echo "docker is required for the sidecar vectorizer smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping sidecar vectorizer smoke"
  exit 0
fi

container="ai-blaise-vectorizer-smoke-${RANDOM}-$$"
binary_pid=""
binary_log="$(mktemp)"

cleanup() {
  if [[ -n "${binary_pid}" ]]; then
    kill "${binary_pid}" >/dev/null 2>&1 || true
    wait "${binary_pid}" 2>/dev/null || true
  fi
  docker rm -f "${container}" >/dev/null 2>&1 || true
  rm -f "${binary_log}"
}
trap cleanup EXIT

# Build the binary up-front so the runtime startup is deterministic.
cargo build -q -p ai_blaise_citus_sidecar_vectorizer

# Start Postgres on a published port we can read back via docker port.
docker run \
  --name "${container}" \
  -e POSTGRES_HOST_AUTH_METHOD=trust \
  -p 127.0.0.1::5432 \
  -d "${postgres_image}" >/dev/null

ready=0
for _ in $(seq 1 120); do
  if docker exec "${container}" psql -U postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 1
done
if [[ "${ready}" != "1" ]]; then
  echo "postgres container did not become ready in time" >&2
  docker logs "${container}" >&2 || true
  exit 1
fi

postgres_port="$(docker port "${container}" 5432/tcp | head -n 1 | awk -F: '{print $NF}')"
if [[ -z "${postgres_port}" ]]; then
  echo "failed to determine postgres host port" >&2
  exit 1
fi
postgres_url="postgres://postgres@127.0.0.1:${postgres_port}/postgres"
listen_port="$(python3 - <<'ENDPORT'
import socket
with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
ENDPORT
)"

# Launch the sidecar binary against the container.
AI_BLAISE_LISTEN_ADDR="127.0.0.1:${listen_port}" \
AI_BLAISE_VECTORIZER_DATABASE_URL="${postgres_url}" \
AI_BLAISE_VECTORIZER_PROVIDER_MODE=mock \
AI_BLAISE_VECTORIZER_BATCH_SIZE=16 \
AI_BLAISE_VECTORIZER_POLL_INTERVAL_MS=50 \
AI_BLAISE_VECTORIZER_MOCK_DIMENSIONS=8 \
RUST_LOG=info \
target/debug/ai_blaise_citus_sidecar_vectorizer serve \
  >"${binary_log}" 2>&1 &
binary_pid=$!

# Wait for the HTTP probe surface to come up before enqueuing rows.
ready=0
for _ in $(seq 1 60); do
  if curl -sf "http://127.0.0.1:${listen_port}/readyz" >/dev/null 2>&1; then
    ready=1
    break
  fi
  if ! kill -0 "${binary_pid}" 2>/dev/null; then
    echo "sidecar binary exited before becoming ready" >&2
    cat "${binary_log}" >&2
    exit 1
  fi
  sleep 0.5
done
if [[ "${ready}" != "1" ]]; then
  echo "sidecar did not become ready in time" >&2
  cat "${binary_log}" >&2
  exit 1
fi

# Seed a budget for the smoke tenant and enqueue 100 rows.
psql_args=(-h 127.0.0.1 -p "${postgres_port}" -U postgres -d postgres -v ON_ERROR_STOP=1 -Atq)
PGPASSWORD="" psql "${psql_args[@]}" -c "INSERT INTO ai.tenant_budget(tenant_id, remaining_tokens) VALUES ('smoke-tenant', 100000) ON CONFLICT (tenant_id) DO UPDATE SET remaining_tokens = EXCLUDED.remaining_tokens;"

for index in $(seq 1 100); do
  PGPASSWORD="" psql "${psql_args[@]}" -c "INSERT INTO ai.vectorizer_queue(tenant_id, provider, model, source_table, source_pk, source_text) VALUES ('smoke-tenant', 'mock', 'embed-v1', 'public.documents', 'doc-${index}', 'document number ${index} with a moderate amount of text body to embed');" >/dev/null
done

# Wait up to 30 seconds for every row to land in succeeded state.
deadline=$((SECONDS + 30))
final_completed=0
while [[ ${SECONDS} -lt ${deadline} ]]; do
  completed="$(PGPASSWORD="" psql "${psql_args[@]}" -c "SELECT count(*) FROM ai.vectorizer_queue WHERE status = 'succeeded'")"
  if [[ "${completed}" -ge 100 ]]; then
    final_completed="${completed}"
    break
  fi
  sleep 0.5
done

completed="$(PGPASSWORD="" psql "${psql_args[@]}" -c "SELECT count(*) FROM ai.vectorizer_queue WHERE status = 'succeeded'")"
usage_rows="$(PGPASSWORD="" psql "${psql_args[@]}" -c "SELECT count(*) FROM ai.usage_log WHERE tenant_id = 'smoke-tenant'")"
remaining_budget="$(PGPASSWORD="" psql "${psql_args[@]}" -c "SELECT remaining_tokens FROM ai.tenant_budget WHERE tenant_id = 'smoke-tenant'")"

if [[ "${completed}" -lt 100 ]]; then
  echo "sidecar vectorizer smoke failed: only ${completed}/100 rows embedded" >&2
  cat "${binary_log}" >&2
  exit 1
fi

if [[ "${usage_rows}" -lt 100 ]]; then
  echo "sidecar vectorizer smoke failed: only ${usage_rows} usage_log entries" >&2
  exit 1
fi

if [[ "${remaining_budget}" -ge 100000 ]]; then
  echo "sidecar vectorizer smoke failed: tenant budget was not decremented (${remaining_budget})" >&2
  exit 1
fi

# Exercise the /vectorize endpoint and the /queue/status endpoint.
vectorize_body='{"tenant_id":"smoke-tenant","provider":"mock","model":"embed-v1","source_table":"public.documents","source_pk":"manual-1","source_text":"manual smoke embedding"}'
vectorize_status="$(curl -s -o /tmp/vectorize-${$}.json -w '%{http_code}' \
  -H 'content-type: application/json' \
  -X POST "http://127.0.0.1:${listen_port}/vectorize" \
  -d "${vectorize_body}")"
if [[ "${vectorize_status}" != "200" ]]; then
  echo "sidecar /vectorize returned ${vectorize_status}" >&2
  cat "/tmp/vectorize-${$}.json" >&2 || true
  exit 1
fi
rm -f "/tmp/vectorize-${$}.json"

queue_status="$(curl -sf "http://127.0.0.1:${listen_port}/queue/status?tenant=smoke-tenant")"
echo "queue_status_payload=${queue_status}"

metrics_payload="$(curl -sf "http://127.0.0.1:${listen_port}/metrics")"
if ! printf '%s\n' "${metrics_payload}" | grep -Fq "ai_blaise_vectorizer_rows_embedded_total"; then
  echo "vectorizer metrics did not expose embedded row counter" >&2
  printf '%s\n' "${metrics_payload}" >&2
  exit 1
fi

# Confirm /vectorize rejects malformed/manual requests before spending budget.
invalid_body='{"tenant_id":"smoke-tenant","provider":"mock;drop","model":"embed-v1","source_table":"public.documents","source_pk":"bad-provider","source_text":"manual smoke embedding"}'
invalid_status="$(curl -s -o /tmp/invalid-vectorize-${$}.json -w '%{http_code}' \
  -H 'content-type: application/json' \
  -X POST "http://127.0.0.1:${listen_port}/vectorize" \
  -d "${invalid_body}")"
if [[ "${invalid_status}" != "400" ]]; then
  echo "expected /vectorize to return 400 for invalid provider name, got ${invalid_status}" >&2
  cat "/tmp/invalid-vectorize-${$}.json" >&2 || true
  exit 1
fi
rm -f "/tmp/invalid-vectorize-${$}.json"

empty_text_body='{"tenant_id":"smoke-tenant","provider":"mock","model":"embed-v1","source_table":"public.documents","source_pk":"empty-text","source_text":""}'
empty_text_status="$(curl -s -o /tmp/empty-text-${$}.json -w '%{http_code}' \
  -H 'content-type: application/json' \
  -X POST "http://127.0.0.1:${listen_port}/vectorize" \
  -d "${empty_text_body}")"
if [[ "${empty_text_status}" != "400" ]]; then
  echo "expected /vectorize to return 400 for empty source_text, got ${empty_text_status}" >&2
  cat "/tmp/empty-text-${$}.json" >&2 || true
  exit 1
fi
rm -f "/tmp/empty-text-${$}.json"

# Confirm /vectorize returns 402 when the tenant has no budget row.
no_budget_status="$(curl -s -o /tmp/no-budget-${$}.json -w '%{http_code}' \
  -H 'content-type: application/json' \
  -X POST "http://127.0.0.1:${listen_port}/vectorize" \
  -d '{"tenant_id":"smoke-tenant-unbudgeted","provider":"mock","model":"embed-v1","source_table":"public.documents","source_pk":"x","source_text":"this tenant has no budget"}')"
if [[ "${no_budget_status}" != "402" ]]; then
  echo "expected /vectorize to return 402 for an unprovisioned tenant, got ${no_budget_status}" >&2
  cat "/tmp/no-budget-${$}.json" >&2 || true
  exit 1
fi
rm -f "/tmp/no-budget-${$}.json"

echo "sidecar vectorizer smoke passed: 100/100 rows embedded in $((30 - (deadline - SECONDS)))s, usage_rows=${usage_rows}, remaining_budget=${remaining_budget}"
