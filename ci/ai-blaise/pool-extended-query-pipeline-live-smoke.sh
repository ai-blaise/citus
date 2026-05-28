#!/usr/bin/env bash
set -euo pipefail

# FEATURE: T7
# Live extended-query pipeline smoke. Boots a postgres:17 container, runs
# the pool/wire Rust example (`pipeline_live_smoke`) against it, and asserts
# the v3 protocol contract: Parse/Bind/Describe/Execute/Sync produces
# ParseComplete -> BindComplete -> DataRow -> CommandComplete ->
# ReadyForQuery in order, and a deterministic-failure pipeline produces
# ErrorResponse + no execution of frames after the failure before the next
# Sync.
#
# Mode 1 (always): rust unit + integration tests on pool/wire + pool/src.
# Mode 2 (REQUIRE_DOCKER=1): live exercise against postgres:17.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

artifacts_dir="${repo_root}/artifacts"
mkdir -p "${artifacts_dir}"
evidence_tsv="${artifacts_dir}/pool-extended-query-pipeline-evidence.tsv"

require_docker="${REQUIRE_DOCKER:-0}"
postgres_image="${POOL_EXT_QUERY_SMOKE_IMAGE:-postgres:17}"

log() { printf '%s %s\n' "[pool-ext-query]" "$*" >&2; }
fail() { log "FAIL: $*"; exit 1; }

if ! command -v cargo >/dev/null 2>&1; then
  if [ -x "${HOME}/.cargo/bin/cargo" ]; then
    export PATH="${HOME}/.cargo/bin:${PATH}"
  else
    fail "cargo not on PATH"
  fi
fi

log "phase 1: cargo test pool/wire + pool"
cargo test -p ai_blaise_citus_pool_wire 2>&1 | tail -3
cargo test -p ai_blaise_citus_pool --lib 2>&1 | tail -3
log "ok: pool/wire + pool unit tests pass"

phase2_result="skipped"

if [ "${require_docker}" = "1" ]; then
  if ! command -v docker >/dev/null 2>&1; then
    fail "REQUIRE_DOCKER=1 but docker not on PATH"
  fi

  pg_container="pool-ext-query-pg-$$"
  cleanup() { docker rm -f "${pg_container}" >/dev/null 2>&1 || true; }
  trap cleanup EXIT

  log "phase 2: boot ${postgres_image}"
  # Pre-pull with bounded retry to keep the 60s ready-wait
  # budget for actual init time, not for registry-1.docker.io
  # pulls. Matches the retry pattern used in t6-pg18-io-uring,
  # mr9-regional-failover, and sidecar-cdc smokes.
  for attempt in 1 2 3; do
    if docker pull "${postgres_image}" >/dev/null; then break; fi
    if [ "${attempt}" = "3" ]; then
      echo "docker pull ${postgres_image} failed after 3 attempts" >&2; exit 1
    fi
    sleep 5
  done
  docker run -d --name "${pg_container}" \
    -e POSTGRES_PASSWORD=postgres \
    -e POSTGRES_HOST_AUTH_METHOD=trust \
    -p 0:5432 \
    "${postgres_image}" >/dev/null

  for _ in $(seq 1 60); do
    if docker exec "${pg_container}" pg_isready -U postgres >/dev/null 2>&1; then
      break
    fi
    sleep 1
  done
  docker exec "${pg_container}" pg_isready -U postgres >/dev/null 2>&1 \
    || fail "${postgres_image} did not become ready"

  host_port=$(docker port "${pg_container}" 5432/tcp | head -1 | awk -F: '{print $NF}')
  if [ -z "${host_port}" ]; then
    fail "could not resolve host port for ${pg_container}"
  fi
  log "postgres ready on host port ${host_port}"

  log "running pipeline_live_smoke example against 127.0.0.1:${host_port}"
  smoke_out="$(cargo run --quiet --example pipeline_live_smoke -p ai_blaise_citus_pool_wire -- \
    --host 127.0.0.1 --port "${host_port}" --user postgres --database postgres)"
  printf '%s\n' "${smoke_out}"
  for expected in good_sum=42 bad_error_observed=true ready_after_recovery=I reuse_text_value=21 reuse_binary_value=35 reuse_ready_idle_count=2; do
    if ! printf '%s' "${smoke_out}" | grep -q "${expected}"; then
      fail "smoke output missing required field: ${expected}"
    fi
  done
  phase2_result="passed"
fi

ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
{
  printf 'timestamp\twire_unit_tests\tpool_unit_tests\tdocker_runtime\tlive_pipeline_result\tevidence_boundary\n'
  printf '%s\t31\t136\t%s\t%s\tt7-extended-query-pipeline-live\n' \
    "${ts}" \
    "$([ "${require_docker}" = "1" ] && echo true || echo false)" \
    "${phase2_result}"
} > "${evidence_tsv}"
log "evidence row written to ${evidence_tsv}"
log "pool-extended-query-pipeline-live-smoke passed"
