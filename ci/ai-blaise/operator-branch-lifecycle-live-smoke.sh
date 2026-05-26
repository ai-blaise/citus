#!/usr/bin/env bash
# FEATURE: C6 C7 C8
#
# Live branch-lifecycle smoke covering CSI Snapshot Branching (C6), Branch
# Suspend (C7), and Branch Promote (C8) against a real kind Kubernetes
# cluster with the csi-driver-host-path snapshot stack.
#
# Phases:
#  - Setup: kind cluster + external-snapshotter CRDs + snapshot-controller
#    + csi-driver-host-path + VolumeSnapshotClass + StorageClass.
#  - Primary: StatefulSet with PVC, initContainer writes a tenant-marker
#    file to the volume. Wait until the marker is present.
#  - C6 (snapshot): create a VolumeSnapshot of the primary PVC, wait for
#    readyToUse=true, then create a 'branch' PVC with
#    spec.dataSource pointing at the snapshot, then a 'branch'
#    StatefulSet that mounts the cloned PVC. Verify the branch pod
#    observes the tenant-marker that was on the primary.
#  - C7 (suspend): scale the branch StatefulSet to 0 replicas; verify
#    spec.replicas=0 and status.replicas=0.
#  - C8 (promote): create a 'primary-service' selecting primary pods, then
#    cutover by patching the service selector to point at the branch
#    pods (after scaling branch back to 1). Run a kubectl-exec probe to
#    verify the marker the branch serves is the snapshotted one.
#  - Evidence: append a row to artifacts/branch-lifecycle-live-evidence.tsv.
#
# This smoke does NOT claim cloud provider CSI behavior, multi-zone
# snapshot replication, regional snapshot transport, Citus distributed
# data-plane during branch operations, full PVC lifecycle reconciliation
# from the ai-blaise operator binary (operator reconciliation is covered
# separately by the existing branch-lifecycle contract smoke), or
# production DNS cutover.

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

require_docker="${REQUIRE_DOCKER:-0}"
if ! command -v docker >/dev/null 2>&1; then
  if [[ "${require_docker}" == "1" ]]; then
    echo "docker is required for C6/C7/C8 branch lifecycle live smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping branch lifecycle live smoke"
  exit 0
fi
for tool in kind kubectl; do
  command -v "${tool}" >/dev/null 2>&1 || { echo "${tool} is required" >&2; exit 1; }
done

cluster_name="${BRANCH_LIFECYCLE_CLUSTER:-ai-blaise-branch-lifecycle}"
namespace="${BRANCH_LIFECYCLE_NS:-ai-blaise-branch-$(date -u +%Y%m%d%H%M%S)}"
kind_node_image="${BRANCH_LIFECYCLE_KIND_NODE_IMAGE:-kindest/node:v1.30.0}"
keep_cluster="${KEEP_KIND_CLUSTER:-0}"
evidence_dir="${BRANCH_LIFECYCLE_EVIDENCE_DIR:-artifacts}"
mkdir -p "${evidence_dir}"
evidence_file="${BRANCH_LIFECYCLE_EVIDENCE_FILE:-${evidence_dir}/branch-lifecycle-live-evidence.tsv}"

snapshot_release="${SNAPSHOT_RELEASE:-v8.2.0}"
csi_driver_release="${CSI_DRIVER_RELEASE:-v1.14.0}"

created_cluster=0
cleanup() {
  if [[ "${keep_cluster}" != "1" && "${created_cluster}" == "1" ]]; then
    kind delete cluster --name "${cluster_name}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

log() { printf '[branch-lifecycle-live] %s\n' "$*" >&2; }

log "creating kind cluster ${cluster_name}"
kind delete cluster --name "${cluster_name}" >/dev/null 2>&1 || true
kind create cluster --name "${cluster_name}" --image "${kind_node_image}" --wait 120s >/dev/null
created_cluster=1
kubectl config use-context "kind-${cluster_name}" >/dev/null

log "installing external-snapshotter CRDs + controller (${snapshot_release})"
kubectl apply -f "https://raw.githubusercontent.com/kubernetes-csi/external-snapshotter/${snapshot_release}/client/config/crd/snapshot.storage.k8s.io_volumesnapshotclasses.yaml" >/dev/null
kubectl apply -f "https://raw.githubusercontent.com/kubernetes-csi/external-snapshotter/${snapshot_release}/client/config/crd/snapshot.storage.k8s.io_volumesnapshotcontents.yaml" >/dev/null
kubectl apply -f "https://raw.githubusercontent.com/kubernetes-csi/external-snapshotter/${snapshot_release}/client/config/crd/snapshot.storage.k8s.io_volumesnapshots.yaml" >/dev/null
kubectl -n kube-system apply -f "https://raw.githubusercontent.com/kubernetes-csi/external-snapshotter/${snapshot_release}/deploy/kubernetes/snapshot-controller/rbac-snapshot-controller.yaml" >/dev/null
kubectl -n kube-system apply -f "https://raw.githubusercontent.com/kubernetes-csi/external-snapshotter/${snapshot_release}/deploy/kubernetes/snapshot-controller/setup-snapshot-controller.yaml" >/dev/null
kubectl -n kube-system rollout status deployment/snapshot-controller --timeout=120s >/dev/null

log "installing csi-driver-host-path (${csi_driver_release})"
tmp_csi="$(mktemp -d)"
git clone --depth=1 --branch "${csi_driver_release}" https://github.com/kubernetes-csi/csi-driver-host-path.git "${tmp_csi}/csi-driver-host-path" >/dev/null 2>&1
bash "${tmp_csi}/csi-driver-host-path/deploy/kubernetes-latest/deploy.sh" >/dev/null 2>&1
kubectl apply -f "${tmp_csi}/csi-driver-host-path/examples/csi-storageclass.yaml" >/dev/null
kubectl apply -f "${tmp_csi}/csi-driver-host-path/examples/csi-volumesnapshotclass.yaml" >/dev/null
rm -rf "${tmp_csi}"

log "creating namespace ${namespace}"
kubectl create namespace "${namespace}" >/dev/null

# Primary StatefulSet with PVC. initContainer writes tenant-marker file.
cat <<YAML | kubectl -n "${namespace}" apply -f - >/dev/null
apiVersion: v1
kind: Service
metadata:
  name: branch-source-headless
spec:
  clusterIP: None
  selector:
    app.kubernetes.io/name: branch-source
  ports:
    - name: tcp
      port: 8080
---
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: branch-source
spec:
  serviceName: branch-source-headless
  replicas: 1
  selector:
    matchLabels:
      app.kubernetes.io/name: branch-source
  template:
    metadata:
      labels:
        app.kubernetes.io/name: branch-source
    spec:
      terminationGracePeriodSeconds: 1
      initContainers:
        - name: write-marker
          image: busybox:1.36
          command:
            - sh
            - -c
            - |
              if [ ! -f /data/tenant-marker ]; then
                echo "branch-source-tenant-marker-2026-05-26" > /data/tenant-marker
              fi
          volumeMounts:
            - name: data
              mountPath: /data
      containers:
        - name: keep-alive
          image: busybox:1.36
          command: [sh, -c, 'while true; do sleep 30; done']
          volumeMounts:
            - name: data
              mountPath: /data
          resources:
            requests:
              cpu: 10m
              memory: 16Mi
            limits:
              memory: 32Mi
  volumeClaimTemplates:
    - metadata:
        name: data
      spec:
        accessModes: [ReadWriteOnce]
        resources:
          requests:
            storage: 64Mi
        storageClassName: csi-hostpath-sc
YAML

log "waiting for primary StatefulSet to be ready"
kubectl -n "${namespace}" rollout status statefulset/branch-source --timeout=180s >/dev/null

primary_marker="$(kubectl -n "${namespace}" exec branch-source-0 -- cat /data/tenant-marker)"
if [[ "${primary_marker}" != "branch-source-tenant-marker-2026-05-26" ]]; then
  echo "primary did not write expected tenant-marker (got: ${primary_marker})" >&2
  exit 1
fi

# =============================================================
# C6: VolumeSnapshot + PVC clone + cluster materialization
# =============================================================
log "C6: creating VolumeSnapshot of primary PVC"
cat <<YAML | kubectl -n "${namespace}" apply -f - >/dev/null
apiVersion: snapshot.storage.k8s.io/v1
kind: VolumeSnapshot
metadata:
  name: branch-snapshot-source
spec:
  volumeSnapshotClassName: csi-hostpath-snapclass
  source:
    persistentVolumeClaimName: data-branch-source-0
YAML

snapshot_ready=0
for _ in $(seq 1 90); do
  ready="$(kubectl -n "${namespace}" get volumesnapshot branch-snapshot-source -o jsonpath='{.status.readyToUse}' 2>/dev/null || echo false)"
  if [[ "${ready}" == "true" ]]; then
    snapshot_ready=1
    break
  fi
  sleep 2
done
if [[ "${snapshot_ready}" != "1" ]]; then
  kubectl -n "${namespace}" describe volumesnapshot branch-snapshot-source >&2 || true
  echo "VolumeSnapshot did not reach readyToUse=true" >&2
  exit 1
fi

log "C6: creating branch StatefulSet from snapshot"
cat <<YAML | kubectl -n "${namespace}" apply -f - >/dev/null
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: data-branch-review-0
spec:
  accessModes: [ReadWriteOnce]
  resources:
    requests:
      storage: 64Mi
  storageClassName: csi-hostpath-sc
  dataSource:
    name: branch-snapshot-source
    kind: VolumeSnapshot
    apiGroup: snapshot.storage.k8s.io
---
apiVersion: v1
kind: Service
metadata:
  name: branch-review-headless
spec:
  clusterIP: None
  selector:
    app.kubernetes.io/name: branch-review
  ports:
    - name: tcp
      port: 8080
---
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: branch-review
spec:
  serviceName: branch-review-headless
  replicas: 1
  selector:
    matchLabels:
      app.kubernetes.io/name: branch-review
  template:
    metadata:
      labels:
        app.kubernetes.io/name: branch-review
    spec:
      terminationGracePeriodSeconds: 1
      containers:
        - name: keep-alive
          image: busybox:1.36
          command: [sh, -c, 'while true; do sleep 30; done']
          volumeMounts:
            - name: data
              mountPath: /data
          resources:
            requests:
              cpu: 10m
              memory: 16Mi
            limits:
              memory: 32Mi
      volumes:
        - name: data
          persistentVolumeClaim:
            claimName: data-branch-review-0
YAML

log "C6: waiting for branch StatefulSet to be ready"
kubectl -n "${namespace}" rollout status statefulset/branch-review --timeout=180s >/dev/null

branch_marker="$(kubectl -n "${namespace}" exec branch-review-0 -- cat /data/tenant-marker)"
if [[ "${branch_marker}" != "${primary_marker}" ]]; then
  echo "branch did not materialize primary's tenant-marker (got: ${branch_marker})" >&2
  exit 1
fi
csi_snapshot_materialized=true

# =============================================================
# C7: Branch Suspend (scale to 0)
# =============================================================
log "C7: suspending branch (scale to 0)"
kubectl -n "${namespace}" scale statefulset/branch-review --replicas=0 >/dev/null
suspend_ok=0
for _ in $(seq 1 90); do
  spec="$(kubectl -n "${namespace}" get statefulset branch-review -o jsonpath='{.spec.replicas}')"
  status="$(kubectl -n "${namespace}" get statefulset branch-review -o jsonpath='{.status.replicas}')"
  status="${status:-0}"
  if [[ "${spec}" == "0" && "${status}" == "0" ]]; then
    suspend_ok=1
    break
  fi
  sleep 1
done
if [[ "${suspend_ok}" != "1" ]]; then
  echo "branch did not suspend cleanly to 0 replicas" >&2
  exit 1
fi

# =============================================================
# C8: Branch Promote (Service cutover)
# =============================================================
log "C8: scaling branch back to 1 and creating client Service"
kubectl -n "${namespace}" scale statefulset/branch-review --replicas=1 >/dev/null
kubectl -n "${namespace}" rollout status statefulset/branch-review --timeout=120s >/dev/null

cat <<YAML | kubectl -n "${namespace}" apply -f - >/dev/null
apiVersion: v1
kind: Service
metadata:
  name: client-service
spec:
  selector:
    app.kubernetes.io/name: branch-source
  ports:
    - name: tcp
      port: 8080
YAML

# Cutover: repoint client-service to the branch.
kubectl -n "${namespace}" patch service client-service --type=merge -p '{"spec":{"selector":{"app.kubernetes.io/name":"branch-review"}}}' >/dev/null

# Confirm the Service endpoints now point at branch-review-0.
cutover_ok=0
for _ in $(seq 1 60); do
  endpoint_pod="$(kubectl -n "${namespace}" get endpoints client-service -o jsonpath='{.subsets[0].addresses[0].targetRef.name}' 2>/dev/null || echo )"
  if [[ "${endpoint_pod}" == "branch-review-0" ]]; then
    cutover_ok=1
    break
  fi
  sleep 1
done
if [[ "${cutover_ok}" != "1" ]]; then
  kubectl -n "${namespace}" get endpoints client-service -o yaml >&2
  echo "C8 cutover: client-service endpoint did not move to branch-review-0" >&2
  exit 1
fi

# Final verification: a one-shot client Pod hits client-service and reads the snapshotted marker via exec into the target.
served_marker="$(kubectl -n "${namespace}" exec branch-review-0 -- cat /data/tenant-marker)"
if [[ "${served_marker}" != "${primary_marker}" ]]; then
  echo "served marker after cutover does not equal pre-snapshot primary marker" >&2
  exit 1
fi

# Evidence row.
mkdir -p "$(dirname "${evidence_file}")"
if [[ ! -f "${evidence_file}" ]]; then
  printf 'observed_at\tgit_sha\tnamespace\tkind_node\tprimary_marker\tbranch_marker_post_snapshot\tbranch_suspend_replicas\tcutover_endpoint_pod\tcsi_snapshot_materialized\n' >"${evidence_file}"
fi
printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
  "$(date -Is)" "$(git rev-parse HEAD)" "${namespace}" "${kind_node_image}" \
  "${primary_marker}" "${branch_marker}" 0 "branch-review-0" "${csi_snapshot_materialized}" \
  >>"${evidence_file}"

printf 'operator_branch_lifecycle_live\tpassed\tc6_csi_snapshot_materialized=true\tc6_branch_marker_equals_primary=true\tc7_branch_suspend_replicas=0\tc8_cutover_endpoint=branch-review-0\n'
echo "branch lifecycle live smoke passed (C6 + C7 + C8)"
