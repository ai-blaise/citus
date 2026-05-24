#!/usr/bin/env bash
set -euo pipefail

# FEATURE: O5
# Live sidecar-controller proof: an in-cluster operator with scoped RBAC applies
# a digest-pinned sidecar Deployment/Service, patches Sidecar status, and serves
# live probe traffic through the generated Service.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

cluster_name="${SIDECAR_CONTROLLER_KIND_CLUSTER:-ai-blaise-o5-sidecar-live}"
namespace="${SIDECAR_CONTROLLER_NAMESPACE:-ai-blaise-o5}"
registry_name="${SIDECAR_CONTROLLER_REGISTRY:-ai-blaise-o5-registry}"
kind_node_image="${SIDECAR_CONTROLLER_KIND_NODE_IMAGE:-kindest/node:v1.30.0}"
evidence_file="${SIDECAR_CONTROLLER_EVIDENCE:-artifacts/sidecar-controller-live-evidence.tsv}"
runtime_dockerfile="images/rust-runtime/Dockerfile"
keep_cluster="${KEEP_KIND_CLUSTER:-0}"
operator_ref=""
sidecar_ref=""
operator_deploy="ai-blaise-citus-operator"
sidecar_name="primary"
sidecar_deploy="ai-blaise-citus-sidecar-primary-realtime"
mutable_name="mutable"
mutable_deploy="ai-blaise-citus-sidecar-mutable-realtime"
operator_log="artifacts/sidecar-controller-operator.log"
port_forward_pid=""

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "$1 is required for O5 sidecar controller live smoke" >&2
    exit 1
  fi
}

free_port() {
  python3 - <<'PY_PORT'
import socket
with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY_PORT
}

cleanup() {
  if [[ -n "${port_forward_pid}" ]]; then
    kill "${port_forward_pid}" >/dev/null 2>&1 || true
    wait "${port_forward_pid}" >/dev/null 2>&1 || true
  fi
  if [[ "${keep_cluster}" != "1" ]]; then
    kind delete cluster --name "${cluster_name}" >/dev/null 2>&1 || true
    docker rm -f "${registry_name}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

require_tool cargo
require_tool curl
require_tool docker
require_tool kind
require_tool kubectl

if [[ ! -s "${runtime_dockerfile}" ]]; then
  echo "missing Rust runtime Dockerfile: ${runtime_dockerfile}" >&2
  exit 1
fi

mkdir -p "$(dirname "${evidence_file}")"
mkdir -p artifacts
: >"${operator_log}"

registry_port="${SIDECAR_CONTROLLER_REGISTRY_PORT:-$(free_port)}"
docker rm -f "${registry_name}" >/dev/null 2>&1 || true
docker run -d --restart=always -p "127.0.0.1:${registry_port}:5000" --name "${registry_name}" registry:2 >/dev/null

kind delete cluster --name "${cluster_name}" >/dev/null 2>&1 || true
kind_config="$(mktemp)"
cat >"${kind_config}" <<KIND
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
containerdConfigPatches:
- |-
  [plugins."io.containerd.grpc.v1.cri".registry.mirrors."localhost:${registry_port}"]
    endpoint = ["http://${registry_name}:5000"]
KIND
kind create cluster --name "${cluster_name}" --image "${kind_node_image}" --config "${kind_config}" --wait 120s >/dev/null
rm -f "${kind_config}"
docker network connect kind "${registry_name}" >/dev/null 2>&1 || true
kubectl config use-context "kind-${cluster_name}" >/dev/null

build_and_push() {
  local package="$1"
  local binary="$2"
  local repo="$3"
  local tag="localhost:${registry_port}/${repo}:o5-live"
  DOCKER_BUILDKIT=1 docker build \
    --file "${runtime_dockerfile}" \
    --build-arg "PACKAGE=${package}" \
    --build-arg "BIN=${binary}" \
    --build-arg "DEFAULT_ARGS=serve" \
    --tag "${tag}" \
    "${repo_root}" >/tmp/o5-build-${binary}.log
  local push_output
  push_output="$(docker push "${tag}" 2>&1)"
  local digest
  digest="$(sed -n 's/.*digest: \(sha256:[0-9a-f]\{64\}\).*/\1/p' <<<"${push_output}" | tail -1)"
  if [[ -z "${digest}" ]]; then
    echo "could not parse digest for ${tag}" >&2
    echo "${push_output}" >&2
    exit 1
  fi
  printf '%s@%s\n' "localhost:${registry_port}/${repo}" "${digest}"
}

operator_ref="$(build_and_push ai_blaise_citus_operator ai_blaise_citus_operator ai-blaise/citus-operator)"
sidecar_ref="$(build_and_push ai_blaise_citus_sidecar_realtime ai_blaise_citus_sidecar_realtime ai-blaise/citus-sidecar-realtime)"

cargo run -q -p ai_blaise_citus_operator -- print-sidecar-crd | kubectl apply -f - >/dev/null
kubectl create namespace "${namespace}" >/dev/null

cat <<YAML | kubectl -n "${namespace}" apply -f - >/dev/null
apiVersion: v1
kind: ServiceAccount
metadata:
  name: ai-blaise-citus-operator
---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: ai-blaise-citus-operator
rules:
  - apiGroups: ["apps"]
    resources: ["deployments"]
    verbs: ["get", "list", "watch", "create", "patch", "update"]
  - apiGroups: [""]
    resources: ["services"]
    verbs: ["get", "list", "watch", "create", "patch", "update"]
  - apiGroups: ["citus.ai-blaise.io"]
    resources: ["sidecars"]
    verbs: ["get", "list", "watch"]
  - apiGroups: ["citus.ai-blaise.io"]
    resources: ["sidecars/status"]
    verbs: ["get", "patch", "update"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: ai-blaise-citus-operator
subjects:
  - kind: ServiceAccount
    name: ai-blaise-citus-operator
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: Role
  name: ai-blaise-citus-operator
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ${operator_deploy}
  labels:
    app.kubernetes.io/name: ai-blaise-citus
    app.kubernetes.io/component: operator
spec:
  replicas: 1
  selector:
    matchLabels:
      app.kubernetes.io/name: ai-blaise-citus
      app.kubernetes.io/component: operator
  template:
    metadata:
      labels:
        app.kubernetes.io/name: ai-blaise-citus
        app.kubernetes.io/component: operator
    spec:
      serviceAccountName: ai-blaise-citus-operator
      securityContext:
        runAsNonRoot: true
        seccompProfile:
          type: RuntimeDefault
      containers:
        - name: operator
          image: ${operator_ref}
          imagePullPolicy: IfNotPresent
          env:
            - name: AI_BLAISE_OPERATOR_EXECUTION_MODE
              value: apply
            - name: AI_BLAISE_LISTEN_ADDR
              value: 0.0.0.0:8080
            - name: RUST_LOG
              value: info
            - name: AI_BLAISE_OPERATOR_CONTROLLERS
              value: sidecar
          ports:
            - name: http
              containerPort: 8080
          readinessProbe:
            httpGet:
              path: /readyz
              port: http
            initialDelaySeconds: 2
            periodSeconds: 2
          livenessProbe:
            httpGet:
              path: /healthz
              port: http
            initialDelaySeconds: 2
            periodSeconds: 5
          securityContext:
            allowPrivilegeEscalation: false
            capabilities:
              drop: ["ALL"]
            readOnlyRootFilesystem: true
          resources:
            requests:
              cpu: 100m
              memory: 128Mi
            limits:
              cpu: 500m
              memory: 512Mi
YAML

kubectl -n "${namespace}" rollout status "deployment/${operator_deploy}" --timeout=180s >/dev/null

cat <<YAML | kubectl -n "${namespace}" apply -f - >/dev/null
apiVersion: citus.ai-blaise.io/v2
kind: Sidecar
metadata:
  name: ${sidecar_name}
spec:
  type: realtime
  replicas: 1
  cpuMillis: 100
  memoryMib: 128
  image: ${sidecar_ref}
  configYaml: |
    subscriptions:
      max_per_tenant: 1000
YAML

kubectl -n "${namespace}" rollout status "deployment/${sidecar_deploy}" --timeout=180s >/dev/null

observed_deploy=""
observed_service=""
for _ in $(seq 1 120); do
  observed_deploy="$(kubectl -n "${namespace}" get sidecar "${sidecar_name}" -o jsonpath='{.status.deploymentName}' 2>/dev/null || true)"
  observed_service="$(kubectl -n "${namespace}" get sidecar "${sidecar_name}" -o jsonpath='{.status.serviceName}' 2>/dev/null || true)"
  if [[ "${observed_deploy}" == "${sidecar_deploy}" && "${observed_service}" == "${sidecar_deploy}" ]]; then
    break
  fi
  sleep 1
done
if [[ "${observed_deploy}" != "${sidecar_deploy}" || "${observed_service}" != "${sidecar_deploy}" ]]; then
  kubectl -n "${namespace}" get sidecar "${sidecar_name}" -o yaml >&2 || true
  echo "Sidecar status was not patched by operator" >&2
  exit 1
fi

owner_kind="$(kubectl -n "${namespace}" get deployment "${sidecar_deploy}" -o jsonpath='{.metadata.ownerReferences[0].kind}')"
owner_name="$(kubectl -n "${namespace}" get service "${sidecar_deploy}" -o jsonpath='{.metadata.ownerReferences[0].name}')"
if [[ "${owner_kind}" != "Sidecar" || "${owner_name}" != "${sidecar_name}" ]]; then
  echo "sidecar Deployment/Service owner references were not set" >&2
  exit 1
fi

pf_port="$(free_port)"
kubectl -n "${namespace}" port-forward --address 127.0.0.1 "svc/${sidecar_deploy}" "${pf_port}:8080" >/tmp/o5-sidecar-port-forward.log 2>&1 &
port_forward_pid="$!"
probe_ok=0
for _ in $(seq 1 60); do
  if curl -fsS "http://127.0.0.1:${pf_port}/healthz" >/tmp/o5-healthz.json \
    && curl -fsS "http://127.0.0.1:${pf_port}/readyz" >/tmp/o5-readyz.json \
    && curl -fsS "http://127.0.0.1:${pf_port}/metrics" >/tmp/o5-metrics.prom; then
    probe_ok=1
    break
  fi
  sleep 1
done
if [[ "${probe_ok}" != "1" ]]; then
  cat /tmp/o5-sidecar-port-forward.log >&2 || true
  kubectl -n "${namespace}" logs "deployment/${sidecar_deploy}" >&2 || true
  echo "generated sidecar Service did not serve live probe traffic" >&2
  exit 1
fi

grep -Fq '"state":"ready"' /tmp/o5-readyz.json
grep -Fq 'ai_blaise_sidecar_ready' /tmp/o5-metrics.prom

cat <<YAML | kubectl -n "${namespace}" apply -f - >/dev/null
apiVersion: citus.ai-blaise.io/v2
kind: Sidecar
metadata:
  name: ${mutable_name}
spec:
  type: realtime
  replicas: 1
  cpuMillis: 100
  memoryMib: 128
  image: localhost:${registry_port}/ai-blaise/citus-sidecar-realtime:latest
YAML

mutable_blocked=0
for _ in $(seq 1 60); do
  kubectl -n "${namespace}" logs "deployment/${operator_deploy}" >"${operator_log}" 2>/dev/null || true
  if grep -Fq "requires an immutable sha256 digest image" "${operator_log}"; then
    if ! kubectl -n "${namespace}" get deployment "${mutable_deploy}" >/dev/null 2>&1; then
      mutable_blocked=1
      break
    fi
  fi
  sleep 1
done
if [[ "${mutable_blocked}" != "1" ]]; then
  cat "${operator_log}" >&2 || true
  kubectl -n "${namespace}" get deployment "${mutable_deploy}" -o yaml >&2 || true
  echo "mutable sidecar image was not blocked before Deployment creation" >&2
  exit 1
fi

operator_can_i="$(kubectl -n "${namespace}" auth can-i patch sidecars.citus.ai-blaise.io --subresource=status --as "system:serviceaccount:${namespace}:ai-blaise-citus-operator" || true)"
deploy_can_i="$(kubectl -n "${namespace}" auth can-i patch deployments.apps --as "system:serviceaccount:${namespace}:ai-blaise-citus-operator" || true)"
if [[ "${operator_can_i}" != "yes" || "${deploy_can_i}" != "yes" ]]; then
  kubectl -n "${namespace}" get role ai-blaise-citus-operator -o yaml >&2 || true
  echo "operator RBAC does not allow required patch verbs" >&2
  exit 1
fi

git_sha="$(git rev-parse --short=12 HEAD)"
{
  printf 'feature\tassertion\tstatus\tdetail\n'
  printf 'O5\toperator_image_digest\tpassed\t%s\n' "${operator_ref}"
  printf 'O5\tsidecar_image_digest\tpassed\t%s\n' "${sidecar_ref}"
  printf 'O5\tkind_node_image\tpassed\t%s\n' "${kind_node_image}"
  printf 'O5\tcrd_status_subresource\tpassed\tprint-sidecar-crd applied status subresource in kind\n'
  printf 'O5\tin_cluster_rbac_apply\tpassed\tnamespace=%s serviceaccount=ai-blaise-citus-operator can_patch_status=%s can_patch_deployments=%s\n' "${namespace}" "${operator_can_i}" "${deploy_can_i}"
  printf 'O5\tdeployment_service_status\tpassed\tdeployment=%s service=%s status=%s git=%s\n' "${sidecar_deploy}" "${observed_service}" "${observed_deploy}" "${git_sha}"
  printf 'O5\tlive_service_probe_traffic\tpassed\thealthz_readyz_metrics via generated Service\n'
  printf 'O5\tmutable_image_fail_closed\tpassed\tmutable Deployment %s absent and operator logged immutable digest rejection\n' "${mutable_deploy}"
} >"${evidence_file}"

cat "${evidence_file}"
echo "ai_blaise_citus O5 sidecar controller live smoke passed"
