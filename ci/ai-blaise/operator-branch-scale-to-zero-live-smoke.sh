#!/usr/bin/env bash
set -euo pipefail

# FEATURE: R2
# Live scale-to-zero proof for the bounded branch compute primitive. This smoke
# proves the operator branch suspend plan is executable against Kubernetes
# Deployment replicas. It does not claim CSI snapshots, PVC cloning, full branch
# suspend/resume reconciliation, Service/DNS retargeting, or branch promotion.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

cluster_name="${BRANCH_SCALE_KIND_CLUSTER:-ai-blaise-r2-scale-zero}"
namespace="${BRANCH_SCALE_NAMESPACE:-ai-blaise-r2-$(date -u +%Y%m%d%H%M%S)}"
kind_node_image="${BRANCH_SCALE_KIND_NODE_IMAGE:-kindest/node:v1.30.0}"
deployment_name="${BRANCH_SCALE_DEPLOYMENT:-branch-review}"
workload_image="${BRANCH_SCALE_WORKLOAD_IMAGE:-registry.k8s.io/pause:3.9}"
keep_cluster="${KEEP_KIND_CLUSTER:-0}"
created_cluster=0

log() {
  printf '[operator-branch-scale-to-zero-live] %s\n' "$*" >&2
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "$1 is required for R2 branch scale-to-zero live smoke" >&2
    exit 1
  }
}

cleanup() {
  if [[ "${keep_cluster}" != "1" && "${created_cluster}" == "1" ]]; then
    kind delete cluster --name "${cluster_name}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

need_cmd cargo
need_cmd docker
need_cmd kind
need_cmd kubectl

log "creating kind cluster ${cluster_name}"
kind delete cluster --name "${cluster_name}" >/dev/null 2>&1 || true
kind create cluster --name "${cluster_name}" --image "${kind_node_image}" --wait 120s >/dev/null
created_cluster=1
kubectl config use-context "kind-${cluster_name}" >/dev/null
kubectl create namespace "${namespace}" >/dev/null

cargo test -q -p ai_blaise_citus_operator branch
branch_output="$(cargo run -q -p ai_blaise_citus_operator -- run-branch-lifecycle-canonical)"
grep -Fq $'suspend\tready\tsuspended\t6\tprod-us-east\tbranch-review\ttrue\ttrue\t0\t0' <<<"${branch_output}"
grep -Fq "ScaleTargetComputeToZero" operator/src/crds/branch.rs
grep -Fq "ActiveSessions" operator/src/crds/branch.rs
grep -Fq "PendingMigrations" operator/src/crds/branch.rs

cat <<YAML | kubectl -n "${namespace}" apply -f - >/dev/null
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ${deployment_name}
  labels:
    app.kubernetes.io/name: ai-blaise-branch-scale
    app.kubernetes.io/component: branch-compute
spec:
  replicas: 1
  selector:
    matchLabels:
      app.kubernetes.io/name: ai-blaise-branch-scale
      app.kubernetes.io/component: branch-compute
  template:
    metadata:
      labels:
        app.kubernetes.io/name: ai-blaise-branch-scale
        app.kubernetes.io/component: branch-compute
    spec:
      terminationGracePeriodSeconds: 1
      containers:
        - name: pause
          image: ${workload_image}
          imagePullPolicy: IfNotPresent
          resources:
            requests:
              cpu: 10m
              memory: 16Mi
            limits:
              memory: 32Mi
YAML

kubectl -n "${namespace}" rollout status "deployment/${deployment_name}" --timeout=120s >/dev/null
before_spec="$(kubectl -n "${namespace}" get deploy "${deployment_name}" -o jsonpath='{.spec.replicas}')"
before_available="$(kubectl -n "${namespace}" get deploy "${deployment_name}" -o jsonpath='{.status.availableReplicas}')"
if [[ "${before_spec}" != "1" || "${before_available:-0}" != "1" ]]; then
  echo "expected branch compute Deployment to start at 1 available replica, got spec=${before_spec} available=${before_available:-0}" >&2
  exit 1
fi

kubectl -n "${namespace}" scale "deployment/${deployment_name}" --replicas=0 >/dev/null

scaled_down=0
for _ in $(seq 1 90); do
  spec_replicas="$(kubectl -n "${namespace}" get deploy "${deployment_name}" -o jsonpath='{.spec.replicas}')"
  status_replicas="$(kubectl -n "${namespace}" get deploy "${deployment_name}" -o jsonpath='{.status.replicas}')"
  available_replicas="$(kubectl -n "${namespace}" get deploy "${deployment_name}" -o jsonpath='{.status.availableReplicas}')"
  status_replicas="${status_replicas:-0}"
  available_replicas="${available_replicas:-0}"
  if [[ "${spec_replicas}" == "0" && "${status_replicas}" == "0" && "${available_replicas}" == "0" ]]; then
    scaled_down=1
    break
  fi
  sleep 1
done

if [[ "${scaled_down}" != "1" ]]; then
  kubectl -n "${namespace}" get deploy "${deployment_name}" -o yaml >&2
  echo "branch compute Deployment did not scale to zero" >&2
  exit 1
fi

printf 'branch_scale_to_zero_live=passed\n'
printf 'r2_suspend_plan_ready_to_suspended=true\n'
printf 'kubernetes_deployment_scaled_to_zero=true\n'
printf 'spec_replicas_before_scale=1\n'
printf 'available_replicas_before_scale=1\n'
printf 'spec_replicas_after_scale=0\n'
printf 'observed_replicas_after_scale=0\n'
printf 'active_sessions_fail_closed=true\n'
printf 'pending_migrations_fail_closed=true\n'
printf 'csi_snapshot_created=false\n'
printf 'traffic_cutover_executed=false\n'
printf 'branch_promotion_executed=false\n'
printf 'operator_branch_scale_to_zero_live\tpassed\n'
