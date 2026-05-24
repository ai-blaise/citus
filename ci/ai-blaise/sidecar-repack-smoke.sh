#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if ! command -v cargo >/dev/null 2>&1 && [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck source=/dev/null
  source "${HOME}/.cargo/env"
fi

output="$(cargo run -q -p ai_blaise_citus_sidecar_repack -- run-canonical)"
header="$(printf '%s\n' "${output}" | sed -n '1p')"
row="$(printf '%s\n' "${output}" | sed -n '2p')"

expected_header=$'target\tstrategy\tschedule\tmax_concurrency\tlock_timeout_ms\tshard_count\tfirst_shard_id\tfirst_worker\tfirst_table\tpg_major\tpg_repack_available\tpg19_repack_concurrently_available\tdry_run\texecuted\tevidence_boundary\texecutable\targs'
if [[ "${header}" != "${expected_header}" ]]; then
  echo "unexpected repack canonical header" >&2
  printf '%s\n' "${header}" >&2
  exit 1
fi

IFS=$'\t' read -r target strategy schedule max_concurrency lock_timeout_ms shard_count first_shard_id first_worker first_table pg_major pg_repack_available pg19_repack_concurrently_available dry_run executed evidence_boundary executable args <<<"${row}"

[[ "${target}" == "public.orders" ]]
[[ "${strategy}" == "pg_repack" ]]
[[ "${schedule}" == "0 3 * * 0" ]]
[[ "${max_concurrency}" == "2" ]]
[[ "${lock_timeout_ms}" == "500" ]]
[[ "${shard_count}" == "2" ]]
[[ "${first_shard_id}" == "102008" ]]
[[ "${first_worker}" == "worker-a" ]]
[[ "${first_table}" == "public.orders_102008" ]]
[[ "${pg_major}" == "18" ]]
[[ "${pg_repack_available}" == "true" ]]
[[ "${pg19_repack_concurrently_available}" == "false" ]]
[[ "${dry_run}" == "true" ]]
[[ "${executed}" == "false" ]]
[[ "${evidence_boundary}" == "dry-run-plan-only" ]]
[[ "${executable}" == "pg_repack" ]]
[[ "${args}" == "--table public.orders --jobs 2" ]]

printf 'sidecar_repack_smoke	strategy=%s	dry_run=%s	executed=%s	evidence_boundary=%s
' "${strategy}" "${dry_run}" "${executed}" "${evidence_boundary}"


if [[ "${REQUIRE_DOCKER:-0}" != "1" ]]; then
  exit 0
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required when REQUIRE_DOCKER=1" >&2
  exit 1
fi

cargo build -q -p ai_blaise_citus_sidecar_repack

stamp="$(date +%Y%m%d%H%M%S)"
tmpdir="$(mktemp -d)"
image="ai-blaise-repack-sidecar-live-smoke:${stamp}"
container="ai-blaise-repack-smoke-${stamp}-$$"

cleanup() {
  docker rm -f "${container}" >/dev/null 2>&1 || true
  docker rmi "${image}" >/dev/null 2>&1 || true
  rm -rf "${tmpdir}"
}
trap cleanup EXIT

cat >"${tmpdir}/Dockerfile" <<'DOCKERFILE'
FROM postgres:17-bookworm
RUN apt-get update \
  && apt-get install -y --no-install-recommends postgresql-17-repack \
  && rm -rf /var/lib/apt/lists/*
COPY ai_blaise_citus_sidecar_repack /usr/local/bin/ai_blaise_citus_sidecar_repack
RUN chmod 0755 /usr/local/bin/ai_blaise_citus_sidecar_repack
DOCKERFILE

cp target/debug/ai_blaise_citus_sidecar_repack "${tmpdir}/ai_blaise_citus_sidecar_repack"
docker build -q -t "${image}" -f "${tmpdir}/Dockerfile" "${tmpdir}" >/dev/null
docker run -d --name "${container}" -e POSTGRES_HOST_AUTH_METHOD=trust "${image}" >/dev/null

ready=0
for _ in $(seq 1 60); do
  if docker exec -u postgres "${container}" pg_isready -d postgres >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 1
done
if [[ "${ready}" != "1" ]]; then
  docker logs "${container}" >&2 || true
  echo "postgres did not become ready for repack live smoke" >&2
  exit 1
fi

docker exec -i -u postgres "${container}" psql -v ON_ERROR_STOP=1 -d postgres <<'SQL' >/dev/null
SET client_min_messages TO warning;
CREATE EXTENSION IF NOT EXISTS pg_repack;
DROP TABLE IF EXISTS public.orders;
CREATE TABLE public.orders (
  id bigserial PRIMARY KEY,
  tenant_id integer NOT NULL,
  payload text NOT NULL
);
INSERT INTO public.orders (tenant_id, payload)
SELECT i % 8, repeat('x', 200)
FROM generate_series(1, 2000) AS i;
DELETE FROM public.orders WHERE id % 4 = 0;
CREATE INDEX orders_tenant_id_idx ON public.orders (tenant_id);
ANALYZE public.orders;
SQL

live_output="$(docker exec -u postgres \
  -e AI_BLAISE_REPACK_DATABASE_URL=postgres \
  -e AI_BLAISE_REPACK_TARGET=public.orders \
  -e AI_BLAISE_REPACK_JOBS=2 \
  -e AI_BLAISE_REPACK_WAIT_TIMEOUT_SECS=5 \
  -e AI_BLAISE_REPACK_PG_MAJOR=17 \
  "${container}" ai_blaise_citus_sidecar_repack run-live-pg-repack)"

live_header="$(printf '%s\n' "${live_output}" | sed -n '1p')"
live_row="$(printf '%s\n' "${live_output}" | sed -n '2p')"
expected_live_header=$'target\tstrategy\tdry_run\texecuted\texit_code\tevidence_boundary\texecutable\targs\tstdout_bytes\tstderr_bytes'
if [[ "${live_header}" != "${expected_live_header}" ]]; then
  echo "unexpected repack live header" >&2
  printf '%s\n' "${live_header}" >&2
  exit 1
fi

IFS=$'\t' read -r live_target live_strategy live_dry_run live_executed live_exit_code live_evidence_boundary live_executable live_args live_stdout_bytes live_stderr_bytes <<<"${live_row}"
[[ "${live_target}" == "public.orders" ]]
[[ "${live_strategy}" == "pg_repack" ]]
[[ "${live_dry_run}" == "false" ]]
[[ "${live_executed}" == "true" ]]
[[ "${live_exit_code}" == "0" ]]
[[ "${live_evidence_boundary}" == "live-pg-repack-execution" ]]
[[ "${live_executable}" == "pg_repack" ]]
[[ "${live_args}" == *"--dbname postgres"* ]]
[[ "${live_args}" == *"--table public.orders"* ]]
[[ "${live_args}" == *"--wait-timeout 5"* ]]

verify="$(docker exec -u postgres "${container}" psql -At -d postgres -c "SELECT count(*)::text || E'\t' || (EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_repack'))::text FROM public.orders;")"
IFS=$'\t' read -r rows_after extension_present <<<"${verify}"
[[ "${rows_after}" == "1500" ]]
[[ "${extension_present}" == "true" ]]

printf 'sidecar_repack_live_pg_repack\ttarget=%s\tdry_run=%s\texecuted=%s\tevidence_boundary=%s\trows_after=%s\textension=pg_repack\n' \
  "${live_target}" "${live_dry_run}" "${live_executed}" "${live_evidence_boundary}" "${rows_after}"
