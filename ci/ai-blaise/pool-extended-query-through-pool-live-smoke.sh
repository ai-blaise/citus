#!/usr/bin/env bash
set -euo pipefail

# FEATURE: T7
# Live extended-query pipeline smoke THROUGH the pool. Boots postgres:17,
# starts the pool pointing at it, runs the pool/wire `pipeline_live_smoke`
# example against the POOL data port (not postgres), and verifies that the
# pool's `/metrics` endpoint reports non-zero Parse/Bind/Execute/Sync frame
# counters - i.e. the pool/wire codec really ran on the production data path.
#
# This complements `pool-extended-query-pipeline-live-smoke.sh` (which goes
# direct to postgres) by proving the pool itself observes extended-query
# traffic when client and backend are bridged through `handle_proxy_connection`.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

artifacts_dir="${repo_root}/artifacts"
mkdir -p "${artifacts_dir}"
evidence_tsv="${artifacts_dir}/pool-extended-query-through-pool-evidence.tsv"

require_docker="${REQUIRE_DOCKER:-0}"
postgres_image="${POOL_EXT_QUERY_SMOKE_IMAGE:-postgres:17}"

log() { printf '%s %s\n' "[pool-ext-through-pool]" "$*" >&2; }
fail() { log "FAIL: $*"; exit 1; }

if ! command -v cargo >/dev/null 2>&1; then
  if [ -x "${HOME}/.cargo/bin/cargo" ]; then
    export PATH="${HOME}/.cargo/bin:${PATH}"
  else
    fail "cargo not on PATH"
  fi
fi

if [ "${require_docker}" != "1" ]; then
  log "REQUIRE_DOCKER=1 not set; only running source-presence checks"
  for pattern in \
    "forward_client_to_upstream" \
    "ExtQueryCounters" \
    "ai_blaise_citus_pool_ext_query_frames_total" \
    "record_extended_frame"; do
    if ! grep -q "${pattern}" pool/src/proxy.rs; then
      fail "pool/src/proxy.rs missing required marker: ${pattern}"
    fi
  done
  log "source-presence checks passed (forward_client_to_upstream + ExtQueryCounters + metrics + record_extended_frame in pool/src/proxy.rs)"
  exit 0
fi

if ! command -v docker >/dev/null 2>&1; then
  fail "REQUIRE_DOCKER=1 but docker not on PATH"
fi
if ! command -v python3 >/dev/null 2>&1; then
  fail "python3 required for port picker"
fi

choose_port() {
  python3 - <<'PY'
import socket
with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
}

choose_distinct_port() {
  local candidate
  local used
  while true; do
    candidate="$(choose_port)"
    for used in "$@"; do
      if [ "${candidate}" = "${used}" ]; then
        continue 2
      fi
    done
    printf '%s\n' "${candidate}"
    return 0
  done
}

postgres_port="$(choose_port)"
pool_port="$(choose_distinct_port "${postgres_port}")"
admin_port="$(choose_distinct_port "${postgres_port}" "${pool_port}")"
container="ai-blaise-pool-ext-through-pool-${RANDOM}-$$"
pool_log="$(mktemp -t ai-blaise-pool-ext-through.XXXXXX.log)"
pool_pid=""

cleanup() {
  if [ -n "${pool_pid}" ] && kill -0 "${pool_pid}" >/dev/null 2>&1; then
    kill "${pool_pid}" >/dev/null 2>&1 || true
    wait "${pool_pid}" >/dev/null 2>&1 || true
  fi
  docker rm -f "${container}" >/dev/null 2>&1 || true
  rm -f "${pool_log}"
}
trap cleanup EXIT

log "booting ${postgres_image} on 127.0.0.1:${postgres_port}"
docker run --name "${container}" \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_HOST_AUTH_METHOD=trust \
  -p "127.0.0.1:${postgres_port}:5432" \
  -d "${postgres_image}" >/dev/null

postgres_ready=0
for _ in $(seq 1 90); do
  if docker exec "${container}" psql -U postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
    postgres_ready=1
    break
  fi
  sleep 1
done
[ "${postgres_ready}" = "1" ] || { docker logs "${container}" >&2; fail "${postgres_image} did not become ready"; }

log "starting pool listen=127.0.0.1:${pool_port} admin=127.0.0.1:${admin_port} upstream=127.0.0.1:${postgres_port}"
AI_BLAISE_POOL_LISTEN_ADDR="127.0.0.1:${pool_port}" \
  AI_BLAISE_POOL_ADMIN_ADDR="127.0.0.1:${admin_port}" \
  AI_BLAISE_POOL_UPSTREAM_ADDR="127.0.0.1:${postgres_port}" \
  AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST="127.0.0.0/8" \
  cargo run --quiet -p ai_blaise_citus_pool -- serve >"${pool_log}" 2>&1 &
pool_pid="$!"

pool_ready=0
for _ in $(seq 1 180); do
  if ! kill -0 "${pool_pid}" >/dev/null 2>&1; then
    cat "${pool_log}" >&2
    fail "pool proxy exited before readiness"
  fi
  if curl -fsS "http://127.0.0.1:${admin_port}/readyz" 2>/dev/null \
      | grep -Fq '"upstream_ready":true'; then
    pool_ready=1
    break
  fi
  sleep 1
done
[ "${pool_ready}" = "1" ] || { cat "${pool_log}" >&2; fail "pool did not become ready"; }

log "running pipeline_live_smoke through pool 127.0.0.1:${pool_port}"
smoke_output="$(cargo run --quiet --example pipeline_live_smoke -p ai_blaise_citus_pool_wire -- \
  --host 127.0.0.1 --port "${pool_port}" --user postgres --database postgres 2>&1)"
printf '%s\n' "${smoke_output}"

for expected in good_sum=42 bad_error_observed=true ready_after_recovery=I reuse_text_value=21 reuse_binary_value=35 reuse_ready_idle_count=2; do
  if ! printf '%s' "${smoke_output}" | grep -q "${expected}"; then
    fail "pipeline_live_smoke (through pool) missing required field: ${expected}"
  fi
done

log "scraping pool metrics for extended-query frame counters"
metrics="$(curl -fsS "http://127.0.0.1:${admin_port}/metrics")"

declare -A frame_counts=()
for frame in Parse Bind Describe Execute Sync Flush Close Terminate; do
  value="$(printf '%s' "${metrics}" \
    | awk -v frame="${frame}" '
        /^ai_blaise_citus_pool_ext_query_frames_total\{frame="/ {
          if (index($0, "frame=\"" frame "\"")) {
            split($0, parts, " ");
            print parts[length(parts)];
            exit;
          }
        }')"
  if [ -z "${value}" ]; then
    printf '%s\n' "${metrics}" >&2
    fail "pool metrics missing ai_blaise_citus_pool_ext_query_frames_total{frame=\"${frame}\"}"
  fi
  frame_counts["${frame}"]="${value}"
done

# Three pipelines run through the pool:
#   good: Parse + Bind + Describe + Execute + Sync
#   bad:  Parse + Bind + Execute + Flush + Sync
#   reuse: Parse + Bind + Execute + Sync + Bind + Execute + Sync
# Totals: Parse=3, Bind=4, Describe=1, Execute=4, Sync=4, Flush=1.
declare -A min_counts=(
  [Parse]=3 [Bind]=4 [Describe]=1 [Execute]=4 [Sync]=4 [Flush]=1
)
for frame in Parse Bind Describe Execute Sync Flush; do
  value="${frame_counts[${frame}]}"
  min="${min_counts[${frame}]}"
  if [ "${value}" -lt "${min}" ]; then
    printf '%s\n' "${metrics}" >&2
    fail "${frame} counter expected >= ${min} (three pipelines), got ${value}"
  fi
done

log "pool ext-query counters: Parse=${frame_counts[Parse]} Bind=${frame_counts[Bind]} Describe=${frame_counts[Describe]} Execute=${frame_counts[Execute]} Sync=${frame_counts[Sync]} Flush=${frame_counts[Flush]}"

decode_errors="$(printf '%s' "${metrics}" \
  | awk '/^ai_blaise_citus_pool_ext_query_decode_errors_total / { print $NF; exit; }')"
if [ -z "${decode_errors}" ] || [ "${decode_errors}" != "0" ]; then
  printf '%s\n' "${metrics}" >&2
  fail "expected pool ext_query_decode_errors_total=0, got '${decode_errors}'"
fi

ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
{
  printf 'timestamp\tpool_port\tadmin_port\tpostgres_port\tparse\tbind\tdescribe\texecute\tsync\tflush\tdecode_errors\tgood_sum\tbad_error_observed\tready_after_recovery\tevidence_boundary\n'
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t42\ttrue\tI\tt7-extended-query-through-pool\n' \
    "${ts}" "${pool_port}" "${admin_port}" "${postgres_port}" \
    "${frame_counts[Parse]}" "${frame_counts[Bind]}" "${frame_counts[Describe]}" \
    "${frame_counts[Execute]}" "${frame_counts[Sync]}" "${frame_counts[Flush]}" \
    "${decode_errors}"
} > "${evidence_tsv}"

log "evidence row written to ${evidence_tsv}"
log "pool-extended-query-through-pool-live-smoke passed"
