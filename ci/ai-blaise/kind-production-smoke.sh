#!/usr/bin/env bash
set -euo pipefail

# FEATURE: D13

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

cluster="${KIND_CLUSTER:-ai-blaise-citus-prod-smoke}"
namespace="${NAMESPACE:-ai-blaise-prod-live}"
release="${HELM_RELEASE:-ai-blaise-prod-live}"
registry="${IMAGE_REGISTRY:-ai-blaise-local}"
tag="${TAG:-prod-smoke}"
postgres_image="${POSTGRES_IMAGE:-postgres:17}"
keep_kind="${KEEP_KIND:-0}"
build_images="${BUILD_IMAGES:-1}"
smoke_request_cpu="${SMOKE_REQUEST_CPU:-10m}"
smoke_request_memory="${SMOKE_REQUEST_MEMORY:-32Mi}"
smoke_limit_cpu="${SMOKE_LIMIT_CPU:-250m}"
smoke_limit_memory="${SMOKE_LIMIT_MEMORY:-256Mi}"

required_commands=(docker helm kind kubectl)
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

if ! kind get clusters | grep -Fxq "${cluster}"; then
  kind create cluster --name "${cluster}"
fi

kubectl config use-context "kind-${cluster}" >/dev/null

if [[ "${build_images}" == "1" ]]; then
  IMAGE_REGISTRY="${registry}" TAG="${tag}" PUSH=false \
    scripts/citus-scale/build-app-images.sh
fi

for image in "${images[@]}"; do
  kind load docker-image "${registry}/${image}:${tag}" --name "${cluster}"
done

kubectl create namespace "${namespace}" --dry-run=client -o yaml | kubectl apply -f -

cat <<YAML | kubectl apply -n "${namespace}" -f -
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

kubectl -n "${namespace}" rollout status deployment/ai-blaise-citus-postgres --timeout=240s

helm upgrade --install "${release}" deploy/k8s/helm/citus-overlay \
  --namespace "${namespace}" \
  --create-namespace \
  --set "global.imageRegistry=${registry}" \
  --set "global.imagePullPolicy=IfNotPresent" \
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
                cat <&3
                exec 3<&-
                exec 3>&-
              }
              http_get /readyz | grep -F '"upstream_ready":true'
              http_get /metrics |
                awk '/^ai_blaise_citus_pool_upstream_ready/ && $2 == 1 { ready = 1 }
                     /^ai_blaise_citus_pool_requests_total / && $2 >= 1 { requests = 1 }
                     END { exit ready && requests ? 0 : 1 }'
YAML
wait_for_job ai-blaise-pool-admin-smoke

kubectl -n "${namespace}" get deployment,pod,svc
echo "ai_blaise_citus Kubernetes production smoke passed in kind/${cluster}"
