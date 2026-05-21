#!/usr/bin/env bash
set -euo pipefail

# FEATURE: D13

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

cluster="${KIND_CLUSTER:-ai-blaise-citus-prod-smoke}"
namespace="${NAMESPACE:-ai-blaise-prod-live}"
release="${HELM_RELEASE:-ai-blaise-prod-live}"
prod_values_namespace="${PROD_VALUES_NAMESPACE:-${namespace}-values}"
prod_values_release="${PROD_VALUES_HELM_RELEASE:-${release}-values}"
chart_name="${CHART_NAME:-ai-blaise-citus}"
registry="${IMAGE_REGISTRY:-ai-blaise-local}"
tag="${TAG:-prod-smoke}"
postgres_image="${POSTGRES_IMAGE:-postgres:17}"
keep_kind="${KEEP_KIND:-0}"
build_images="${BUILD_IMAGES:-1}"
smoke_request_cpu="${SMOKE_REQUEST_CPU:-10m}"
smoke_request_memory="${SMOKE_REQUEST_MEMORY:-32Mi}"
smoke_limit_cpu="${SMOKE_LIMIT_CPU:-250m}"
smoke_limit_memory="${SMOKE_LIMIT_MEMORY:-256Mi}"

required_commands=(curl docker helm kind kubectl)
for command_name in "${required_commands[@]}"; do
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "${command_name} is required for the Kubernetes production smoke" >&2
    exit 1
  fi
done

images=(
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

cleanup() {
  if [[ "${keep_kind}" != "1" ]]; then
    kind delete cluster --name "${cluster}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

dump_k8s_diagnostics() {
  kubectl -n "${namespace}" get deployment,pod,job,svc -o wide >&2 || true
  kubectl -n "${namespace}" get events --sort-by=.lastTimestamp >&2 || true
  kubectl -n "${namespace}" describe pod >&2 || true
  kubectl -n "${namespace}" logs --all-containers --tail=80 -l "app.kubernetes.io/name=ai-blaise-citus" >&2 || true
}

apply_monitoring_crds() {
  kubectl apply -f - <<'YAML'
apiVersion: apiextensions.k8s.io/v1
kind: CustomResourceDefinition
metadata:
  name: servicemonitors.monitoring.coreos.com
spec:
  group: monitoring.coreos.com
  names:
    kind: ServiceMonitor
    listKind: ServiceMonitorList
    plural: servicemonitors
    singular: servicemonitor
  scope: Namespaced
  versions:
    - name: v1
      served: true
      storage: true
      schema:
        openAPIV3Schema:
          type: object
          x-kubernetes-preserve-unknown-fields: true
---
apiVersion: apiextensions.k8s.io/v1
kind: CustomResourceDefinition
metadata:
  name: prometheusrules.monitoring.coreos.com
spec:
  group: monitoring.coreos.com
  names:
    kind: PrometheusRule
    listKind: PrometheusRuleList
    plural: prometheusrules
    singular: prometheusrule
  scope: Namespaced
  versions:
    - name: v1
      served: true
      storage: true
      schema:
        openAPIV3Schema:
          type: object
          x-kubernetes-preserve-unknown-fields: true
YAML
  kubectl wait --for=condition=Established crd/servicemonitors.monitoring.coreos.com --timeout=60s
  kubectl wait --for=condition=Established crd/prometheusrules.monitoring.coreos.com --timeout=60s
}

wait_for_deployments() {
  if ! kubectl -n "${namespace}" wait --for=condition=available deployment --all --timeout=300s; then
    dump_k8s_diagnostics
    exit 1
  fi
}

wait_for_job() {
  local job_name="$1"
  if ! kubectl -n "${namespace}" wait --for=condition=complete "job/${job_name}" --timeout=120s; then
    kubectl -n "${namespace}" logs "job/${job_name}" --all-containers=true >&2 || true
    dump_k8s_diagnostics
    exit 1
  fi
}

wait_for_http() {
  local url="$1"
  local label="$2"
  local attempt

  for attempt in $(seq 1 30); do
    if curl -fsS "${url}" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done

  echo "timed out waiting for ${label} at ${url}" >&2
  return 1
}

probe_deployment_http() {
  local deployment="$1"
  local component="$2"
  local remote_port="${3:-8080}"
  local local_port="$4"
  local log_file
  local port_forward_pid

  log_file="$(mktemp)"
  kubectl -n "${namespace}" port-forward \
    --address 127.0.0.1 \
    "deployment/${deployment}" \
    "${local_port}:${remote_port}" >"${log_file}" 2>&1 &
  port_forward_pid="$!"

  cleanup_port_forward() {
    kill "${port_forward_pid}" >/dev/null 2>&1 || true
    wait "${port_forward_pid}" >/dev/null 2>&1 || true
    rm -f "${log_file}"
  }

  if ! wait_for_http "http://127.0.0.1:${local_port}/healthz" "${component} healthz"; then
    cat "${log_file}" >&2 || true
    cleanup_port_forward
    return 1
  fi

  if ! curl -fsS "http://127.0.0.1:${local_port}/healthz" |
    grep -F "\"component\":\"${component}\"" >/dev/null; then
    echo "${component} /healthz did not report the expected component name" >&2
    cat "${log_file}" >&2 || true
    cleanup_port_forward
    return 1
  fi

  if ! curl -fsS "http://127.0.0.1:${local_port}/readyz" |
    grep -F '"ready":true' >/dev/null; then
    echo "${component} /readyz did not report ready=true" >&2
    cat "${log_file}" >&2 || true
    cleanup_port_forward
    return 1
  fi

  if ! curl -fsS "http://127.0.0.1:${local_port}/metrics" |
    grep -F "ai_blaise_sidecar_ready{component=\"${component}\"} 1" >/dev/null; then
    echo "${component} /metrics did not report ai_blaise_sidecar_ready=1" >&2
    cat "${log_file}" >&2 || true
    cleanup_port_forward
    return 1
  fi

  cleanup_port_forward
}

expected_probe_component() {
  local deployment_component="$1"

  case "${deployment_component}" in
    mcp)
      printf '%s\n' "mcp-sidecar"
      ;;
    *)
      printf '%s\n' "${deployment_component}"
      ;;
  esac
}

install_postgres_fixture() {
  local target_namespace="$1"

  kubectl create namespace "${target_namespace}" --dry-run=client -o yaml | kubectl apply -f -

  cat <<YAML | kubectl apply -n "${target_namespace}" -f -
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ai-blaise-citus-postgres
  labels:
    app.kubernetes.io/name: ai-blaise-citus-postgres
spec:
  replicas: 1
  selector:
    matchLabels:
      app.kubernetes.io/name: ai-blaise-citus-postgres
  template:
    metadata:
      labels:
        app.kubernetes.io/name: ai-blaise-citus-postgres
    spec:
      containers:
        - name: postgres
          image: ${postgres_image}
          env:
            - name: POSTGRES_PASSWORD
              value: postgres
          ports:
            - name: postgres
              containerPort: 5432
---
apiVersion: v1
kind: Service
metadata:
  name: ai-blaise-citus-postgres-rw
spec:
  selector:
    app.kubernetes.io/name: ai-blaise-citus-postgres
  ports:
    - name: postgres
      port: 5432
      targetPort: postgres
YAML

  kubectl -n "${target_namespace}" rollout status deployment/ai-blaise-citus-postgres --timeout=240s
}

probe_pool_admin_pods() {
  local pods
  local pod
  local local_port=18120
  local total_requests=0

  pods="$(
    kubectl -n "${namespace}" get pods \
      -l "app.kubernetes.io/name=${chart_name},app.kubernetes.io/component=pool" \
      -o jsonpath='{range .items[*]}{.metadata.name}{"\n"}{end}'
  )"

  if [[ -z "${pods}" ]]; then
    echo "no pool pods found for admin metrics probe" >&2
    return 1
  fi

  while IFS= read -r pod; do
    [[ -n "${pod}" ]] || continue

    local log_file
    local port_forward_pid
    local metrics
    local requests

    log_file="$(mktemp)"
    kubectl -n "${namespace}" port-forward \
      --address 127.0.0.1 \
      "pod/${pod}" \
      "${local_port}:8080" >"${log_file}" 2>&1 &
    port_forward_pid="$!"

    cleanup_pool_port_forward() {
      kill "${port_forward_pid}" >/dev/null 2>&1 || true
      wait "${port_forward_pid}" >/dev/null 2>&1 || true
      rm -f "${log_file}"
    }

    if ! wait_for_http "http://127.0.0.1:${local_port}/readyz" "pool ${pod} readyz"; then
      cat "${log_file}" >&2 || true
      cleanup_pool_port_forward
      return 1
    fi

    if ! curl -fsS "http://127.0.0.1:${local_port}/readyz" |
      grep -F '"upstream_ready":true' >/dev/null; then
      echo "pool pod ${pod} /readyz did not report upstream_ready=true" >&2
      cat "${log_file}" >&2 || true
      cleanup_pool_port_forward
      return 1
    fi

    metrics="$(curl -fsS "http://127.0.0.1:${local_port}/metrics")"
    if ! printf '%s\n' "${metrics}" |
      awk '/^ai_blaise_citus_pool_upstream_ready/ && $2 == 1 { ready = 1 }
           END { exit ready ? 0 : 1 }'; then
      echo "pool pod ${pod} /metrics did not report upstream readiness" >&2
      cat "${log_file}" >&2 || true
      cleanup_pool_port_forward
      return 1
    fi

    requests="$(
      printf '%s\n' "${metrics}" |
        awk '/^ai_blaise_citus_pool_requests_total / { print int($2) }'
    )"
    requests="${requests:-0}"
    total_requests="$((total_requests + requests))"

    cleanup_pool_port_forward
    local_port="$((local_port + 1))"
  done <<<"${pods}"

  if [[ "${total_requests}" -lt 1 ]]; then
    echo "pool pod metrics did not record the SQL smoke connection" >&2
    return 1
  fi
}

run_pool_sql_smoke() {
  kubectl -n "${namespace}" delete job ai-blaise-pool-sql-smoke >/dev/null 2>&1 || true
  cat <<'YAML' | kubectl apply -n "${namespace}" -f -
apiVersion: batch/v1
kind: Job
metadata:
  name: ai-blaise-pool-sql-smoke
spec:
  backoffLimit: 0
  template:
    spec:
      restartPolicy: Never
      containers:
        - name: psql
          image: postgres:17
          env:
            - name: PGPASSWORD
              value: postgres
          command:
            - sh
            - -lc
            - |
              set -eu
              psql -h ai-blaise-citus-pool -p 5432 -U postgres -d postgres -Atqv ON_ERROR_STOP=1 <<'SQL'
              SELECT 42::int;
              CREATE TEMP TABLE pool_proxy_smoke(value integer);
              INSERT INTO pool_proxy_smoke VALUES (7), (35);
              SELECT sum(value)::int FROM pool_proxy_smoke;
              SQL
YAML
  wait_for_job ai-blaise-pool-sql-smoke
  sql_output="$(kubectl -n "${namespace}" logs job/ai-blaise-pool-sql-smoke)"
  if [[ "${sql_output}" != $'42\n42' ]]; then
    echo "unexpected Kubernetes SQL smoke output:" >&2
    printf '%s\n' "${sql_output}" >&2
    exit 1
  fi

  probe_pool_admin_pods

  kubectl -n "${namespace}" delete job ai-blaise-pool-admin-smoke >/dev/null 2>&1 || true
  cat <<'YAML' | kubectl apply -n "${namespace}" -f -
apiVersion: batch/v1
kind: Job
metadata:
  name: ai-blaise-pool-admin-smoke
spec:
  backoffLimit: 0
  template:
    spec:
      restartPolicy: Never
      containers:
        - name: admin-http
          image: postgres:17
          command:
            - bash
            - -lc
            - |
              set -euo pipefail
              http_get() {
                local target_path="$1"
                exec 3<>/dev/tcp/ai-blaise-citus-pool/8080
                printf 'GET %s HTTP/1.1\r\nHost: ai-blaise-citus-pool\r\nConnection: close\r\n\r\n' "${target_path}" >&3
                cat <&3 || true
                exec 3<&-
                exec 3>&-
              }
              for attempt in $(seq 1 30); do
                if http_get /readyz | grep -F '"upstream_ready":true' &&
                   http_get /metrics |
                     awk '/^ai_blaise_citus_pool_upstream_ready/ && $2 == 1 { ready = 1 }
                          END { exit ready ? 0 : 1 }'; then
                  exit 0
                fi
                sleep 1
              done
              echo "pool admin smoke did not observe ready upstream metrics" >&2
              exit 1
YAML
  wait_for_job ai-blaise-pool-admin-smoke
}

probe_pool_rejected_metrics() {
  local pods
  local pod
  local local_port=18220
  local total_rejected=0

  pods="$(
    kubectl -n "${namespace}" get pods \
      -l "app.kubernetes.io/name=${chart_name},app.kubernetes.io/component=pool" \
      -o jsonpath='{range .items[*]}{.metadata.name}{"\n"}{end}'
  )"

  if [[ -z "${pods}" ]]; then
    echo "no pool pods found for CIDR rejection metrics probe" >&2
    return 1
  fi

  while IFS= read -r pod; do
    [[ -n "${pod}" ]] || continue

    local log_file
    local port_forward_pid
    local metrics
    local rejected

    log_file="$(mktemp)"
    kubectl -n "${namespace}" port-forward \
      --address 127.0.0.1 \
      "pod/${pod}" \
      "${local_port}:8080" >"${log_file}" 2>&1 &
    port_forward_pid="$!"

    cleanup_pool_rejected_port_forward() {
      kill "${port_forward_pid}" >/dev/null 2>&1 || true
      wait "${port_forward_pid}" >/dev/null 2>&1 || true
      rm -f "${log_file}"
    }

    if ! wait_for_http "http://127.0.0.1:${local_port}/metrics" "pool ${pod} rejected metrics"; then
      cat "${log_file}" >&2 || true
      cleanup_pool_rejected_port_forward
      return 1
    fi

    metrics="$(curl -fsS "http://127.0.0.1:${local_port}/metrics")"
    rejected="$(
      printf '%s\n' "${metrics}" |
        awk '/^ai_blaise_citus_pool_rejected_connections_total / { print int($2) }'
    )"
    rejected="${rejected:-0}"
    total_rejected="$((total_rejected + rejected))"

    cleanup_pool_rejected_port_forward
    local_port="$((local_port + 1))"
  done <<<"${pods}"

  if [[ "${total_rejected}" -lt 1 ]]; then
    echo "pool pod metrics did not record the CIDR-denied SQL connection" >&2
    return 1
  fi
}

run_pool_cidr_deny_smoke() {
  helm upgrade "${release}" deploy/k8s/helm/citus-overlay \
    --namespace "${namespace}" \
    --reuse-values \
    --set-string "pool.networkPolicy.cidrAllowlist[0]=192.0.2.0/24"

  kubectl -n "${namespace}" rollout status "deployment/${chart_name}-pool" --timeout=240s

  kubectl -n "${namespace}" delete job ai-blaise-pool-cidr-deny-smoke >/dev/null 2>&1 || true
  cat <<'YAML' | kubectl apply -n "${namespace}" -f -
apiVersion: batch/v1
kind: Job
metadata:
  name: ai-blaise-pool-cidr-deny-smoke
spec:
  backoffLimit: 0
  template:
    spec:
      restartPolicy: Never
      containers:
        - name: psql
          image: postgres:17
          env:
            - name: PGPASSWORD
              value: postgres
            - name: PGCONNECT_TIMEOUT
              value: "5"
          command:
            - sh
            - -lc
            - |
              set -eu
              if psql -h ai-blaise-citus-pool -p 5432 -U postgres -d postgres -Atqc 'SELECT 1'; then
                echo "pool CIDR deny smoke unexpectedly allowed SQL traffic" >&2
                exit 1
              fi
YAML
  wait_for_job ai-blaise-pool-cidr-deny-smoke
  probe_pool_rejected_metrics
  echo "ai_blaise_citus pool CIDR deny smoke passed in kind/${cluster}/${namespace}"
}

run_citusctl_image_smoke() {
  local citusctl_output

  kubectl -n "${namespace}" delete job ai-blaise-citusctl-image-smoke >/dev/null 2>&1 || true
  cat <<YAML | kubectl apply -n "${namespace}" -f -
apiVersion: batch/v1
kind: Job
metadata:
  name: ai-blaise-citusctl-image-smoke
spec:
  backoffLimit: 0
  template:
    spec:
      restartPolicy: Never
      containers:
        - name: citusctl
          image: ${registry}/citusctl:${tag}
          imagePullPolicy: IfNotPresent
YAML
  wait_for_job ai-blaise-citusctl-image-smoke
  citusctl_output="$(kubectl -n "${namespace}" logs job/ai-blaise-citusctl-image-smoke)"
  if [[ "${citusctl_output}" != "citusctl inspect destructive=false requires_plan_id=true steps=3" ]]; then
    echo "unexpected citusctl image smoke output:" >&2
    printf '%s\n' "${citusctl_output}" >&2
    exit 1
  fi
}

assert_deployment_replicas() {
  local deployment="$1"
  local expected="$2"
  local actual

  actual="$(kubectl -n "${namespace}" get "deployment/${deployment}" -o jsonpath='{.spec.replicas}')"
  if [[ "${actual}" != "${expected}" ]]; then
    echo "deployment/${deployment} expected ${expected} replicas, found ${actual}" >&2
    dump_k8s_diagnostics
    exit 1
  fi
}

assert_no_alpha_workload_deployments() {
  local forbidden

  forbidden="$(
    kubectl -n "${namespace}" get deployments -o jsonpath='{range .items[*]}{.metadata.name}{"\n"}{end}' |
      grep -E 'sidecar-|tools' || true
  )"

  if [[ -n "${forbidden}" ]]; then
    echo "values-prod.yaml rendered alpha workload deployments:" >&2
    printf '%s\n' "${forbidden}" >&2
    dump_k8s_diagnostics
    exit 1
  fi
}

if ! kind get clusters | grep -Fxq "${cluster}"; then
  kind create cluster --name "${cluster}"
fi

kubectl config use-context "kind-${cluster}" >/dev/null
apply_monitoring_crds

if [[ "${build_images}" == "1" ]]; then
  IMAGE_REGISTRY="${registry}" TAG="${tag}" PUSH=false \
    scripts/citus-scale/build-app-images.sh
fi

for image in "${images[@]}"; do
  kind load docker-image "${registry}/${image}:${tag}" --name "${cluster}"
done

install_postgres_fixture "${namespace}"

helm upgrade --install "${release}" deploy/k8s/helm/citus-overlay \
  --namespace "${namespace}" \
  --create-namespace \
  --set "global.imageRegistry=${registry}" \
  --set "global.imagePullPolicy=IfNotPresent" \
  --set "global.requireImageDigest=false" \
  --set "operator.image.tag=${tag}" \
  --set "pool.image.tag=${tag}" \
  --set "sidecarDefaults.tag=${tag}" \
  --set "operator.resources.requests.cpu=${smoke_request_cpu}" \
  --set "operator.resources.requests.memory=${smoke_request_memory}" \
  --set "operator.resources.limits.cpu=${smoke_limit_cpu}" \
  --set "operator.resources.limits.memory=${smoke_limit_memory}" \
  --set "pool.resources.requests.cpu=${smoke_request_cpu}" \
  --set "pool.resources.requests.memory=${smoke_request_memory}" \
  --set "pool.resources.limits.cpu=${smoke_limit_cpu}" \
  --set "pool.resources.limits.memory=${smoke_limit_memory}" \
  --set "sidecarDefaults.resources.requests.cpu=${smoke_request_cpu}" \
  --set "sidecarDefaults.resources.requests.memory=${smoke_request_memory}" \
  --set "sidecarDefaults.resources.limits.cpu=${smoke_limit_cpu}" \
  --set "sidecarDefaults.resources.limits.memory=${smoke_limit_memory}"

wait_for_deployments

probe_deployment_http "${chart_name}-operator" operator 8080 18080

sidecar_probe_port=18081
for sidecar in "${images[@]}"; do
  if [[ "${sidecar}" != citus-sidecar-* ]]; then
    continue
  fi

  component="${sidecar#citus-sidecar-}"
  probe_component="$(expected_probe_component "${component}")"
  probe_deployment_http \
    "${chart_name}-sidecar-${component}" \
    "${probe_component}" \
    8080 \
    "${sidecar_probe_port}"
  sidecar_probe_port="$((sidecar_probe_port + 1))"
done

run_pool_sql_smoke
run_pool_cidr_deny_smoke
run_citusctl_image_smoke

kubectl -n "${namespace}" get deployment,pod,svc
echo "ai_blaise_citus exhaustive image-matrix smoke passed in kind/${cluster}/${namespace}"

helm uninstall "${release}" --namespace "${namespace}"
for _ in $(seq 1 30); do
  if ! kubectl get clusterrole "${chart_name}-operator" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
if kubectl get clusterrole "${chart_name}-operator" >/dev/null 2>&1; then
  echo "timed out waiting for ${chart_name}-operator ClusterRole cleanup" >&2
  exit 1
fi

namespace="${prod_values_namespace}"
install_postgres_fixture "${namespace}"

CHART_DIR=deploy/k8s/helm/citus-overlay \
  RELEASE_NAME="${prod_values_release}" \
  NAMESPACE="${namespace}" \
  DEPLOY_PROFILE=prod \
  MODE=install \
  IMAGE_REGISTRY="${registry}" \
  ALLOW_MUTABLE_IMAGE_TAGS=1 \
  OPERATOR_IMAGE_TAG="${tag}" \
  POOL_IMAGE_TAG="${tag}" \
  scripts/citus-scale/deploy.sh

wait_for_deployments
assert_deployment_replicas "${chart_name}-operator" 2
assert_deployment_replicas "${chart_name}-pool" 3
assert_no_alpha_workload_deployments
probe_deployment_http "${chart_name}-operator" operator 8080 18180
run_pool_sql_smoke

kubectl -n "${namespace}" get deployment,pod,svc
echo "ai_blaise_citus values-prod.yaml production profile smoke passed in kind/${cluster}/${namespace}"
