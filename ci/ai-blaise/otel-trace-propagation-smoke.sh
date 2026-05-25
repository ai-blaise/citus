#!/usr/bin/env bash
set -euo pipefail

# ci/ai-blaise/otel-trace-propagation-smoke.sh
#
# Verifies that a W3C traceparent embedded in libpq startup parameters survives
# the pool proxy, PostgreSQL, companion SQL, and shared sidecar HTTP ingress.
# Runs in two modes:
#
#   * Default mode (docker available, kind not required): starts a real
#     PostgreSQL container with ai_blaise_citus installed, runs the pool proxy
#     against it, connects with traceparent-bearing PGOPTIONS through psql,
#     asserts the pool's stderr log line and Prometheus counters reflect the
#     tap, verifies companion SQL projection, and checks sidecar `/tracez`.
#
#   * Optional KIND mode (REQUIRE_KIND=1 plus a kind binary on PATH): in
#     addition to the default mode, launches a 3-node kind cluster with a
#     Jaeger all-in-one deployment, verifies the in-cluster PostgreSQL trace
#     GUC path, sends a synthetic OTLP span keyed to the accepted trace_id, and
#     queries the Jaeger HTTP API to confirm that trace is retrievable. This is
#     a correlation-harness proof, not automatic pool/companion/sidecar span
#     export.
#
# Outside release mode the default smoke skips when Docker is unavailable so
# lightweight CI can include it unconditionally. Release mode and
# REQUIRE_DOCKER=1 fail closed.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

pg_major="${OTEL_SMOKE_PG_MAJOR:-17}"
postgres_image="${OTEL_SMOKE_POSTGRES_IMAGE:-postgres:${pg_major}}"
extension_dir="${repo_root}/images/citus-pg-overlay/extensions"
control_file="${extension_dir}/ai_blaise_citus.control"
sql_file="${extension_dir}/ai_blaise_citus--0.1.0.sql"
release_mode="${AI_BLAISE_RELEASE_MODE:-0}"
require_docker="${REQUIRE_DOCKER:-${release_mode}}"
require_kind="${REQUIRE_KIND:-0}"

trace_id="${OTEL_SMOKE_TRACE_ID:-4bf92f3577b34da6a3ce929d0e0e4736}"
parent_id="${OTEL_SMOKE_PARENT_ID:-00f067aa0ba902b7}"
traceparent="00-${trace_id}-${parent_id}-01"
tracestate="${OTEL_SMOKE_TRACESTATE:-vendor=ai-blaise}"
# Use libpq's options startup parameter so PostgreSQL accepts the dotted
# custom GUC (trace.parent) without truncating it through application_name.
libpq_options="-c trace.parent=${traceparent} -c trace.state=${tracestate}"
application_name="ai_blaise_otel_smoke"

if ! command -v docker >/dev/null 2>&1; then
  if [[ "${require_docker}" == "1" ]]; then
    echo "docker is required for otel-trace-propagation smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping otel-trace-propagation smoke"
  exit 0
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required for otel-trace-propagation smoke" >&2
  exit 1
fi

for file in "${control_file}" "${sql_file}"; do
  if [[ ! -s "${file}" ]]; then
    echo "missing otel trace propagation artifact: ${file}" >&2
    exit 1
  fi
done

choose_port() {
  python3 - <<'PY'
import socket

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
}

postgres_port="$(choose_port)"
pool_port="$(choose_port)"
admin_port="$(choose_port)"
shared_port="$(choose_port)"
container="ai-blaise-otel-smoke-${RANDOM}-$$"
pool_log="$(mktemp -t ai-blaise-otel-smoke.XXXXXX.log)"
shared_log="$(mktemp -t ai-blaise-otel-shared.XXXXXX.log)"
trace_sql_output="$(mktemp -t ai-blaise-otel-sql.XXXXXX.tsv)"
pool_pid=""
shared_pid=""

cleanup() {
  if [[ -n "${shared_pid}" ]] && kill -0 "${shared_pid}" >/dev/null 2>&1; then
    kill "${shared_pid}" >/dev/null 2>&1 || true
    wait "${shared_pid}" >/dev/null 2>&1 || true
  fi
  if [[ -n "${pool_pid}" ]] && kill -0 "${pool_pid}" >/dev/null 2>&1; then
    kill "${pool_pid}" >/dev/null 2>&1 || true
    wait "${pool_pid}" >/dev/null 2>&1 || true
  fi
  docker rm -f "${container}" >/dev/null 2>&1 || true
  rm -f "${pool_log}" "${shared_log}" "${trace_sql_output}"
}
trap cleanup EXIT

docker run \
  --name "${container}" \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_HOST_AUTH_METHOD=trust \
  -p "127.0.0.1:${postgres_port}:5432" \
  -v "${control_file}:/usr/share/postgresql/${pg_major}/extension/ai_blaise_citus.control:ro" \
  -v "${sql_file}:/usr/share/postgresql/${pg_major}/extension/ai_blaise_citus--0.1.0.sql:ro" \
  -d "${postgres_image}" >/dev/null

postgres_init_complete=0
for _ in $(seq 1 120); do
  if docker logs "${container}" 2>&1 | grep -q "PostgreSQL init process complete"; then
    postgres_init_complete=1
    break
  fi
  sleep 1
done

if [[ "${postgres_init_complete}" != "1" ]]; then
  docker logs "${container}" >&2 || true
  echo "postgres container did not finish init scripts" >&2
  exit 1
fi

postgres_ready=0
for _ in $(seq 1 60); do
  if docker exec "${container}" psql -U postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
    postgres_ready=1
    break
  fi
  sleep 1
done

if [[ "${postgres_ready}" != "1" ]]; then
  docker logs "${container}" >&2 || true
  echo "postgres container did not become ready" >&2
  exit 1
fi

docker exec -i "${container}" psql -U postgres -v ON_ERROR_STOP=1 <<'SQL' >/dev/null
CREATE EXTENSION ai_blaise_citus;
SQL

AI_BLAISE_POOL_LISTEN_ADDR="127.0.0.1:${pool_port}" \
  AI_BLAISE_POOL_ADMIN_ADDR="127.0.0.1:${admin_port}" \
  AI_BLAISE_POOL_UPSTREAM_ADDR="127.0.0.1:${postgres_port}" \
  AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST="127.0.0.0/8" \
  cargo run -q -p ai_blaise_citus_pool -- serve >"${pool_log}" 2>&1 &
pool_pid="$!"

pool_ready=0
for _ in $(seq 1 120); do
  if ! kill -0 "${pool_pid}" >/dev/null 2>&1; then
    cat "${pool_log}" >&2
    echo "pool proxy exited before readiness" >&2
    exit 1
  fi
  if curl -fsS "http://127.0.0.1:${admin_port}/readyz" 2>/dev/null |
    grep -Fq '"upstream_ready":true'; then
    pool_ready=1
    break
  fi
  sleep 1
done

if [[ "${pool_ready}" != "1" ]]; then
  cat "${pool_log}" >&2
  echo "pool proxy did not report upstream-ready readiness" >&2
  exit 1
fi

if ! docker run --rm \
  -i \
  --network host \
  -e PGPASSWORD=postgres \
  -e PGAPPNAME="${application_name}" \
  -e PGOPTIONS="${libpq_options}" \
  -e PGSSLMODE=disable \
  "${postgres_image}" \
  psql -h 127.0.0.1 -p "${pool_port}" -U postgres -d postgres -Atqv ON_ERROR_STOP=1 >"${trace_sql_output}" <<SQL
SELECT 'postgres_traceparent' || E'\t' || current_setting('trace.parent', true);
SELECT 'postgres_tracestate' || E'\t' || current_setting('trace.state', true);
SELECT 'postgres_application_name' || E'\t' || current_setting('application_name');
SELECT 'companion_current_traceparent' || E'\t' || companion.current_traceparent();
SELECT 'companion_current_tracestate' || E'\t' || companion.current_tracestate();
BEGIN;
SELECT 'companion_projected' || E'\t' || (companion.project_traceparent_from_application_name('application=companion-sql;traceparent=${traceparent};tracestate=vendor=companion')->>'projected');
SELECT 'companion_projected_traceparent' || E'\t' || companion.current_traceparent();
SELECT 'companion_projected_tracestate' || E'\t' || companion.current_tracestate();
COMMIT;
BEGIN;
SELECT 'companion_invalid_projected' || E'\t' || (companion.project_traceparent_from_application_name('application=companion-sql;traceparent=not-a-traceparent')->>'projected');
COMMIT;
SQL
then
  cat "${pool_log}" >&2
  echo "psql traceparent-bearing application_name connection failed" >&2
  exit 1
fi

grep -Fq $'postgres_traceparent\t'"${traceparent}" "${trace_sql_output}"
grep -Fq $'postgres_tracestate\t'"${tracestate}" "${trace_sql_output}"
grep -Fq $'companion_current_traceparent\t'"${traceparent}" "${trace_sql_output}"
grep -Fq $'companion_current_tracestate\t'"${tracestate}" "${trace_sql_output}"
grep -Fq $'companion_projected\ttrue' "${trace_sql_output}"
grep -Fq $'companion_projected_traceparent\t'"${traceparent}" "${trace_sql_output}"
grep -Fq $'companion_projected_tracestate\tvendor=companion' "${trace_sql_output}"
grep -Fq $'companion_invalid_projected\tfalse' "${trace_sql_output}"

# 1) The pool's stderr log must record the traceparent that was tapped.
if ! grep -Fq "trace_tap=present" "${pool_log}"; then
  cat "${pool_log}" >&2
  echo "pool proxy did not record a traceparent tap" >&2
  exit 1
fi
if ! grep -Fq "traceparent=${traceparent}" "${pool_log}"; then
  cat "${pool_log}" >&2
  echo "pool proxy did not log the exact traceparent embedded in application_name" >&2
  exit 1
fi

# 2) The pool's Prometheus exposition must show traceparent_tapped_total >= 1
#    and rejected_connections_total == 0.
metrics="$(curl -fsS "http://127.0.0.1:${admin_port}/metrics")"
if ! printf '%s\n' "${metrics}" | awk '
  /^ai_blaise_citus_pool_traceparent_tapped_total / && $2 >= 1 { tapped = 1 }
  /^ai_blaise_citus_pool_rejected_connections_total / && $2 == 0 { rejected = 1 }
  END { exit (tapped && rejected) ? 0 : 1 }
'; then
  cat "${pool_log}" >&2
  echo "pool metrics did not show traceparent_tapped_total >= 1 with rejected_connections_total == 0" >&2
  printf '%s\n' "${metrics}" >&2
  exit 1
fi

# 3) A second connection without a traceparent must increment the
#    traceparent_absent_total counter, proving the tap distinguishes both
#    cases. We use psql with the default libpq application_name and assert
#    the counter rises.
absent_before="$(printf '%s\n' "${metrics}" | awk '
  /^ai_blaise_citus_pool_traceparent_absent_total / { print $2 }
')"
if ! docker run --rm \
  -i \
  --network host \
  -e PGPASSWORD=postgres \
  -e PGSSLMODE=disable \
  "${postgres_image}" \
  psql -h 127.0.0.1 -p "${pool_port}" -U postgres -d postgres -Atqv ON_ERROR_STOP=1 >"${trace_sql_output}" <<SQL
SELECT 1;
SQL
then
  cat "${pool_log}" >&2
  echo "follow-up psql without traceparent failed" >&2
  exit 1
fi

metrics_after="$(curl -fsS "http://127.0.0.1:${admin_port}/metrics")"
absent_after="$(printf '%s\n' "${metrics_after}" | awk '
  /^ai_blaise_citus_pool_traceparent_absent_total / { print $2 }
')"
if [[ -z "${absent_before:-}" || -z "${absent_after:-}" || "${absent_after}" -le "${absent_before}" ]]; then
  cat "${pool_log}" >&2
  echo "pool metrics did not increment traceparent_absent_total for the no-traceparent connection" >&2
  printf 'before=%s after=%s\n' "${absent_before}" "${absent_after}" >&2
  exit 1
fi


AI_BLAISE_LISTEN_ADDR="127.0.0.1:${shared_port}" \
  cargo run -q -p ai_blaise_citus_sidecar_shared -- serve >"${shared_log}" 2>&1 &
shared_pid="$!"

shared_ready=0
for _ in $(seq 1 120); do
  if ! kill -0 "${shared_pid}" >/dev/null 2>&1; then
    cat "${shared_log}" >&2
    echo "shared sidecar exited before readiness" >&2
    exit 1
  fi
  if curl -fsS "http://127.0.0.1:${shared_port}/readyz" >/dev/null 2>&1; then
    shared_ready=1
    break
  fi
  sleep 1
done
if [[ "${shared_ready}" != "1" ]]; then
  cat "${shared_log}" >&2
  echo "shared sidecar did not become ready" >&2
  exit 1
fi

sidecar_trace="$(curl -fsS \
  -H "traceparent: ${traceparent}" \
  -H "tracestate: ${tracestate}" \
  "http://127.0.0.1:${shared_port}/tracez")"
if ! grep -Fq '"valid":true' <<<"${sidecar_trace}" || \
   ! grep -Fq '"traceparent":"'"${traceparent}"'"' <<<"${sidecar_trace}" || \
   ! grep -Fq '"tracestate":"'"${tracestate}"'"' <<<"${sidecar_trace}"; then
  cat "${shared_log}" >&2
  echo "shared sidecar did not project trace headers through /tracez" >&2
  printf '%s\n' "${sidecar_trace}" >&2
  exit 1
fi

sidecar_absent="$(curl -fsS "http://127.0.0.1:${shared_port}/tracez")"
if ! grep -Fq '"valid":false' <<<"${sidecar_absent}" || \
   ! grep -Fq '"traceparent":null' <<<"${sidecar_absent}"; then
  cat "${shared_log}" >&2
  echo "shared sidecar did not report absent trace headers through /tracez" >&2
  printf '%s\n' "${sidecar_absent}" >&2
  exit 1
fi

echo "ai_blaise_citus pool proxy traceparent tap smoke passed (default mode)"

# Optional KIND mode for end-to-end pool->companion->sidecar tracing through
# a real cluster. We only attempt it when explicitly requested AND when kind,
# kubectl, and helm are on the PATH; otherwise we exit cleanly here.
if [[ "${require_kind}" != "1" ]]; then
  exit 0
fi

for tool in kind kubectl helm; do
  if ! command -v "${tool}" >/dev/null 2>&1; then
    echo "REQUIRE_KIND=1 but ${tool} is not available; failing" >&2
    exit 1
  fi
done

kind_cluster="ai-blaise-otel-smoke-${RANDOM}"
kind_config="$(mktemp -t kind-otel.XXXXXX.yaml)"

cat >"${kind_config}" <<KIND
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
nodes:
  - role: control-plane
  - role: worker
  - role: worker
KIND

otlp_payload=""

kind_cleanup() {
  cleanup
  rm -f "${kind_config}"
  if [[ -n "${otlp_payload}" ]]; then
    rm -f "${otlp_payload}"
  fi
  kind delete cluster --name "${kind_cluster}" >/dev/null 2>&1 || true
}
trap kind_cleanup EXIT

kind create cluster --name "${kind_cluster}" --config "${kind_config}" >/dev/null

kubectl --context "kind-${kind_cluster}" apply -f - <<'JAEGER' >/dev/null
apiVersion: apps/v1
kind: Deployment
metadata:
  name: jaeger
  namespace: default
spec:
  replicas: 1
  selector:
    matchLabels: { app: jaeger }
  template:
    metadata:
      labels: { app: jaeger }
    spec:
      containers:
        - name: jaeger
          image: jaegertracing/all-in-one:1.57
          ports:
            - { containerPort: 16686 }
            - { containerPort: 4318 }
---
apiVersion: v1
kind: Service
metadata:
  name: jaeger
  namespace: default
spec:
  selector: { app: jaeger }
  ports:
    - { name: ui, port: 16686, targetPort: 16686 }
    - { name: otlp, port: 4318, targetPort: 4318 }
JAEGER

kubectl --context "kind-${kind_cluster}" wait deployment/jaeger --for condition=Available --timeout=180s >/dev/null

# In KIND mode the pool's `trace_tap=present` log line, the in-cluster
# PostgreSQL trace GUC, and a Jaeger API lookup for the same trace_id are the
# assertion surfaces. The synthetic OTLP span keeps this bounded to a
# correlation-harness proof; automatic span export remains outside this smoke.
kubectl --context "kind-${kind_cluster}" run otel-smoke-postgres \
  --image="${postgres_image}" \
  --env=POSTGRES_PASSWORD=postgres \
  --env=POSTGRES_HOST_AUTH_METHOD=trust \
  --port=5432 \
  --restart=Never >/dev/null
kubectl --context "kind-${kind_cluster}" wait pod/otel-smoke-postgres \
  --for=condition=Ready --timeout=120s >/dev/null

kind_postgres_ready=0
for _ in $(seq 1 120); do
  if kubectl --context "kind-${kind_cluster}" exec otel-smoke-postgres -- \
    psql -U postgres -d postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
    kind_postgres_ready=1
    break
  fi
  sleep 1
done
if [[ "${kind_postgres_ready}" != "1" ]]; then
  kubectl --context "kind-${kind_cluster}" logs otel-smoke-postgres >&2 || true
  echo "kind postgres pod did not become SQL-ready" >&2
  exit 1
fi

kubectl --context "kind-${kind_cluster}" exec otel-smoke-postgres -- \
  psql -U postgres -d postgres -c "SET trace.parent TO '${traceparent}'; SELECT current_setting('trace.parent', true);" >/dev/null

otlp_payload="$(mktemp -t ai-blaise-otel-jaeger.XXXXXX.json)"
start_ns="$(date +%s%N)"
end_ns="$((start_ns + 1000000))"
cat >"${otlp_payload}" <<OTLP
{
  "resourceSpans": [
    {
      "resource": {
        "attributes": [
          {"key": "service.name", "value": {"stringValue": "ai-blaise-otel-smoke"}},
          {"key": "deployment.environment", "value": {"stringValue": "kind"}}
        ]
      },
      "scopeSpans": [
        {
          "scope": {"name": "ai-blaise-otel-trace-propagation-smoke"},
          "spans": [
            {
              "traceId": "${trace_id}",
              "spanId": "1111111111111111",
              "parentSpanId": "${parent_id}",
              "name": "pool.trace_tap",
              "kind": 2,
              "startTimeUnixNano": "${start_ns}",
              "endTimeUnixNano": "${end_ns}",
              "attributes": [
                {"key": "traceparent", "value": {"stringValue": "${traceparent}"}},
                {"key": "component", "value": {"stringValue": "pool"}},
                {"key": "evidence_scope", "value": {"stringValue": "synthetic-jaeger-correlation-harness"}}
              ]
            }
          ]
        }
      ]
    }
  ]
}
OTLP

kubectl --context "kind-${kind_cluster}" run "otel-smoke-otlp-${RANDOM}" \
  --rm -i --restart=Never --image=curlimages/curl:8.10.1 --command -- \
  curl -fsS -X POST "http://jaeger:4318/v1/traces" \
  -H "content-type: application/json" --data-binary @- \
  <"${otlp_payload}" >/dev/null

jaeger_trace_found=0
for attempt in $(seq 1 60); do
  query_output="$(kubectl --context "kind-${kind_cluster}" run "otel-smoke-jaeger-query-${attempt}-${RANDOM}" \
    --rm -i --restart=Never --image=curlimages/curl:8.10.1 --command -- \
    curl -fsS "http://jaeger:16686/api/traces/${trace_id}" 2>/dev/null || true)"
  if grep -Fq "pool.trace_tap" <<<"${query_output}" && \
     grep -Fq "${trace_id}" <<<"${query_output}" && \
     grep -Fq "synthetic-jaeger-correlation-harness" <<<"${query_output}"; then
    jaeger_trace_found=1
    break
  fi
  sleep 2
done

if [[ "${jaeger_trace_found}" != "1" ]]; then
  echo "Jaeger API did not return the synthetic trace_id ${trace_id}" >&2
  exit 1
fi

echo "ai_blaise_citus pool proxy traceparent tap smoke passed (kind mode with Jaeger API correlation)"
