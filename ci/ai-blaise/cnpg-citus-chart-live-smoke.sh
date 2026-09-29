#!/usr/bin/env bash
set -Eeuo pipefail

# FEATURE: S4
# B2 diagnostic: install the Command Center chart into an isolated kind cluster,
# let the Rust CitusCluster controller create one CNPG coordinator group and two
# logical worker groups, and execute the chart-owned read-only Helm SQL test.
#
# This script deliberately accepts an immutable archive of the Command Center
# chart instead of carrying a second copy of the chart in this repository. All
# container inputs must already exist in the local Docker daemon and must match
# caller-supplied image IDs. The run is useful native evidence, but a reused
# development host and task-local registry are not release-promotion evidence.

umask 077

run_id="${B2_RUN_ID:-b2-$(date -u +%Y%m%d%H%M%S)-$$}"
cluster_name="${B2_KIND_CLUSTER:-ai-blaise-${run_id}}"
namespace="${B2_NAMESPACE:-ai-blaise-b2}"
release_name="${B2_RELEASE_NAME:-citus-b2}"
registry_name="${B2_REGISTRY_NAME:-ai-blaise-${run_id}-registry}"
artifact_dir="${B2_ARTIFACT_DIR:-artifacts/cnpg-citus-chart-live/${run_id}}"
chart_subdir="${B2_CC_CHART_SUBDIR:-citus-cluster}"
keep_cluster="${B2_KEEP_KIND_CLUSTER:-0}"
ready_timeout_seconds="${B2_READY_TIMEOUT_SECONDS:-2400}"

runtime_dir=""
kubeconfig=""
registry_port=""
created_cluster=0
created_registry=0
diagnostics_collected=0

log() {
  printf '[cnpg-citus-chart-live] %s\n' "$*" >&2
}

die() {
  log "ERROR: $*"
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
}

need_env() {
  local name="$1"
  [[ -n "${!name:-}" ]] || die "${name} must be set"
}

require_sha256() {
  local label="$1"
  local value="$2"
  [[ "${value}" =~ ^[0-9a-f]{64}$ ]] || die "${label} must be 64 lowercase hexadecimal characters"
}

require_image_id() {
  local label="$1"
  local value="$2"
  [[ "${value}" =~ ^sha256:[0-9a-f]{64}$ ]] || die "${label} must be an exact sha256 image ID"
}

require_dns_label() {
  local label="$1"
  local value="$2"
  [[ ${#value} -le 63 && "${value}" =~ ^[a-z0-9]([-a-z0-9]*[a-z0-9])?$ ]] ||
    die "${label} must be a lowercase DNS label no longer than 63 characters"
}

file_sha256() {
  sha256sum "$1" | awk '{print $1}'
}

chart_tree_sha256() {
  python3 - "$1" <<'PY'
import hashlib
import pathlib
import sys

root = pathlib.Path(sys.argv[1]).resolve()
if not root.is_dir():
    raise SystemExit(f"chart tree is not a directory: {root}")
digest = hashlib.sha256()
for path in sorted(root.rglob("*"), key=lambda item: item.relative_to(root).as_posix()):
    if path.is_symlink():
        raise SystemExit(f"chart tree contains a symbolic link: {path.relative_to(root)}")
    if not path.is_file():
        continue
    relative = path.relative_to(root).as_posix().encode()
    payload = path.read_bytes()
    digest.update(len(relative).to_bytes(8, "big"))
    digest.update(relative)
    digest.update(len(payload).to_bytes(8, "big"))
    digest.update(payload)
print(digest.hexdigest())
PY
}

free_port() {
  python3 - <<'PY'
import socket
with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
}

inspect_image() {
  local label="$1"
  local reference="$2"
  local expected_id="$3"
  local actual_id
  local platform
  actual_id="$(docker image inspect "${reference}" --format '{{.Id}}' 2>/dev/null)" ||
    die "${label} image is not present locally: ${reference}"
  [[ "${actual_id}" == "${expected_id}" ]] ||
    die "${label} image ID mismatch: expected ${expected_id}, got ${actual_id}"
  platform="$(docker image inspect "${expected_id}" --format '{{.Os}}/{{.Architecture}}')"
  [[ "${platform}" == "linux/amd64" ]] ||
    die "${label} image must be linux/amd64, got ${platform}"
}

image_label() {
  local image_id="$1"
  local key="$2"
  docker image inspect "${image_id}" --format "{{ index .Config.Labels \"${key}\" }}"
}

publish_image() {
  local image_id="$1"
  local repository="$2"
  local local_ref="127.0.0.1:${registry_port}/${repository}:${run_id}"
  local manifest_url="http://127.0.0.1:${registry_port}/v2/${repository}/manifests/${run_id}"
  local push_log="${artifact_dir}/push-${repository//\//-}.log"
  local push_digest
  local registry_digest

  docker tag "${image_id}" "${local_ref}"
  docker push "${local_ref}" >"${push_log}" 2>&1
  push_digest="$(awk '/digest: sha256:/ { for (i = 1; i <= NF; i++) if ($i ~ /^sha256:/) { print $i; exit } }' "${push_log}")"
  require_image_id "${repository} registry manifest digest" "${push_digest}"
  registry_digest="$(
    curl -fsSI \
      -H 'Accept: application/vnd.oci.image.manifest.v1+json, application/vnd.docker.distribution.manifest.v2+json' \
      "${manifest_url}" |
      awk -F': ' 'tolower($1) == "docker-content-digest" { sub("\r$", "", $2); print $2; exit }'
  )"
  [[ "${registry_digest}" == "${push_digest}" ]] ||
    die "registry digest mismatch for ${repository}: push=${push_digest}, head=${registry_digest}"
  printf 'localhost:%s/%s@%s\n' "${registry_port}" "${repository}" "${push_digest}"
}

secure_remove_runtime() {
  [[ -n "${runtime_dir}" ]] || return 0
  case "${runtime_dir}" in
    /tmp/ai-blaise-cnpg-b2.*)
      if command -v shred >/dev/null 2>&1; then
        find "${runtime_dir}" -type f -exec shred -u -- {} + >/dev/null 2>&1 || true
      fi
      rm -rf -- "${runtime_dir}"
      ;;
    *)
      log "refusing to remove unexpected runtime directory: ${runtime_dir}"
      ;;
  esac
}

collect_diagnostics() {
  [[ "${diagnostics_collected}" == "0" ]] || return 0
  diagnostics_collected=1
  mkdir -p "${artifact_dir}"
  {
    printf 'claim_scope\tdiagnostic-non-promotion\n'
    printf 'secret_policy\tSecret objects and local key material are excluded from retained logs\n'
    printf 'run_id\t%s\n' "${run_id}"
  } >"${artifact_dir}/claim-boundary.tsv"

  if [[ "${created_cluster}" == "1" ]] && kubectl version --request-timeout=5s >/dev/null 2>&1; then
    kubectl get nodes -o wide >"${artifact_dir}/nodes.txt" 2>&1 || true
    kubectl get crd citusclusters.citus.ai-blaise.io clusters.postgresql.cnpg.io imagecatalogs.postgresql.cnpg.io \
      -o yaml >"${artifact_dir}/crds-selected.yaml" 2>&1 || true
    kubectl -n cnpg-system get deployment,pod,service,event -o wide \
      >"${artifact_dir}/cnpg-inventory.txt" 2>&1 || true
    kubectl -n "${namespace}" get deployment,pod,job,configmap,service,serviceaccount,role,rolebinding,cituscluster,cluster.postgresql.cnpg.io,imagecatalog.postgresql.cnpg.io,event \
      -o wide >"${artifact_dir}/workload-inventory.txt" 2>&1 || true
    kubectl -n "${namespace}" get cituscluster,cluster.postgresql.cnpg.io,imagecatalog.postgresql.cnpg.io,job,configmap \
      -o yaml >"${artifact_dir}/workload-safe-state.yaml" 2>&1 || true

    for scope in "cnpg-system" "${namespace}"; do
      while IFS= read -r pod; do
        [[ -n "${pod}" ]] || continue
        while IFS= read -r container; do
          [[ -n "${container}" ]] || continue
          kubectl -n "${scope}" logs "${pod}" -c "${container}" --tail=200 \
            >"${artifact_dir}/log-${scope}-${pod}-${container}.txt" 2>&1 || true
        done < <(kubectl -n "${scope}" get pod "${pod}" -o jsonpath='{range .spec.containers[*]}{.name}{"\n"}{end}' 2>/dev/null || true)
      done < <(kubectl -n "${scope}" get pods -o name 2>/dev/null | sed 's#^pod/##' || true)
    done

    helm -n "${namespace}" status "${release_name}" >"${artifact_dir}/helm-status.txt" 2>&1 || true
    helm -n "${namespace}" get manifest "${release_name}" >"${artifact_dir}/helm-manifest.yaml" 2>&1 || true
    helm -n "${namespace}" get values "${release_name}" >"${artifact_dir}/helm-values.yaml" 2>&1 || true
  fi
}

cleanup() {
  if [[ "${keep_cluster}" == "1" ]]; then
    log "B2_KEEP_KIND_CLUSTER=1; retaining isolated cluster ${cluster_name} and registry ${registry_name}"
  else
    if [[ "${created_registry}" == "1" ]] && docker network inspect kind >/dev/null 2>&1; then
      docker network disconnect kind "${registry_name}" >/dev/null 2>&1 || true
    fi
    if [[ "${created_cluster}" == "1" ]]; then
      kind delete cluster --name "${cluster_name}" >/dev/null 2>&1 || true
      created_cluster=0
    fi
    if [[ "${created_registry}" == "1" ]]; then
      docker rm -fv "${registry_name}" >/dev/null 2>&1 || true
      created_registry=0
    fi
    if [[ -n "${registry_port}" ]]; then
      docker image rm \
        "127.0.0.1:${registry_port}/b2/cloudnative-pg:${run_id}" \
        "127.0.0.1:${registry_port}/b2/citus-operator:${run_id}" \
        "127.0.0.1:${registry_port}/b2/citus:${run_id}" >/dev/null 2>&1 || true
    fi
  fi
  secure_remove_runtime
}

on_exit() {
  local status=$?
  trap - EXIT
  set +e
  collect_diagnostics
  cleanup
  if [[ "${status}" -eq 0 ]]; then
    log "PASS; safe evidence retained in ${artifact_dir}"
  else
    log "FAIL (${status}); safe diagnostics retained in ${artifact_dir}"
  fi
  exit "${status}"
}
trap on_exit EXIT

for command_name in awk cp curl date docker find grep helm kind kubectl mkdir openssl python3 rm sed sha256sum tar tr wc; do
  need_cmd "${command_name}"
done

for variable in \
  B2_CC_CHART_ARCHIVE \
  B2_CC_CHART_ARCHIVE_SHA256 \
  B2_CC_CHART_TREE_SHA256 \
  B2_CC_REVIEWED_CHART_FINGERPRINT \
  B2_EXPECTED_CRD_BUNDLE_SHA256 \
  B2_EXPECTED_CHART_VERSION \
  B2_CNPG_MANIFEST \
  B2_CNPG_MANIFEST_SHA256 \
  B2_CNPG_MANIFEST_IMAGE \
  B2_CNPG_IMAGE \
  B2_CNPG_IMAGE_ID \
  B2_OPERATOR_IMAGE \
  B2_OPERATOR_IMAGE_ID \
  B2_OPERATOR_SOURCE_ARCHIVE_SHA256 \
  B2_OPERATOR_SOURCE_REVISION \
  B2_OPERATOR_TREE_STATE \
  B2_OPERAND_IMAGE \
  B2_OPERAND_IMAGE_ID \
  B2_OPERAND_SOURCE_REVISION \
  B2_OPERAND_TREE_STATE \
  B2_REGISTRY_IMAGE \
  B2_REGISTRY_IMAGE_ID \
  B2_KIND_BINARY_SHA256 \
  B2_KIND_NODE_IMAGE \
  B2_KIND_NODE_IMAGE_ID \
  B2_EXPECTED_KUBERNETES_VERSION \
  B2_EXPECTED_CITUS_VERSION \
  B2_EXPECTED_COMPANION_VERSION; do
  need_env "${variable}"
done

[[ "${keep_cluster}" == "0" || "${keep_cluster}" == "1" ]] || die "B2_KEEP_KIND_CLUSTER must be 0 or 1"
[[ ${#run_id} -le 128 && "${run_id}" =~ ^[a-z0-9][a-z0-9._-]*$ ]] ||
  die "B2_RUN_ID must be a lowercase OCI tag component no longer than 128 characters"
[[ "${ready_timeout_seconds}" =~ ^[0-9]+$ && "${ready_timeout_seconds}" -ge 60 && "${ready_timeout_seconds}" -le 7200 ]] ||
  die "B2_READY_TIMEOUT_SECONDS must be an integer from 60 through 7200"
require_dns_label "B2_KIND_CLUSTER" "${cluster_name}"
require_dns_label "B2_NAMESPACE" "${namespace}"
require_dns_label "B2_RELEASE_NAME" "${release_name}"
require_dns_label "B2_REGISTRY_NAME" "${registry_name}"
require_sha256 "B2_CC_CHART_ARCHIVE_SHA256" "${B2_CC_CHART_ARCHIVE_SHA256}"
require_sha256 "B2_CC_CHART_TREE_SHA256" "${B2_CC_CHART_TREE_SHA256}"
require_sha256 "B2_CC_REVIEWED_CHART_FINGERPRINT" "${B2_CC_REVIEWED_CHART_FINGERPRINT}"
require_sha256 "B2_EXPECTED_CRD_BUNDLE_SHA256" "${B2_EXPECTED_CRD_BUNDLE_SHA256}"
require_sha256 "B2_CNPG_MANIFEST_SHA256" "${B2_CNPG_MANIFEST_SHA256}"
require_sha256 "B2_KIND_BINARY_SHA256" "${B2_KIND_BINARY_SHA256}"
require_sha256 "B2_OPERATOR_SOURCE_ARCHIVE_SHA256" "${B2_OPERATOR_SOURCE_ARCHIVE_SHA256}"
require_image_id "B2_CNPG_IMAGE_ID" "${B2_CNPG_IMAGE_ID}"
require_image_id "B2_OPERATOR_IMAGE_ID" "${B2_OPERATOR_IMAGE_ID}"
require_image_id "B2_OPERAND_IMAGE_ID" "${B2_OPERAND_IMAGE_ID}"
require_image_id "B2_REGISTRY_IMAGE_ID" "${B2_REGISTRY_IMAGE_ID}"
require_image_id "B2_KIND_NODE_IMAGE_ID" "${B2_KIND_NODE_IMAGE_ID}"
[[ "${B2_KIND_NODE_IMAGE}" != *:latest && "${B2_KIND_NODE_IMAGE}" != latest ]] ||
  die "B2_KIND_NODE_IMAGE must not use a latest tag"
[[ "${chart_subdir}" =~ ^[a-zA-Z0-9._/-]+$ && "${chart_subdir}" != /* && "${chart_subdir}" != *..* ]] ||
  die "B2_CC_CHART_SUBDIR must be a safe relative archive path"
[[ "${B2_EXPECTED_KUBERNETES_VERSION}" =~ ^v1\.[0-9]+\.[0-9]+$ ]] ||
  die "B2_EXPECTED_KUBERNETES_VERSION must be an exact v1.x.y version"

actual_kind_sha="$(file_sha256 "$(command -v kind)")"
[[ "${actual_kind_sha}" == "${B2_KIND_BINARY_SHA256}" ]] ||
  die "kind binary SHA-256 mismatch: expected ${B2_KIND_BINARY_SHA256}, got ${actual_kind_sha}"
[[ -f "${B2_CC_CHART_ARCHIVE}" ]] || die "missing Command Center chart archive: ${B2_CC_CHART_ARCHIVE}"
[[ -f "${B2_CNPG_MANIFEST}" ]] || die "missing CNPG manifest: ${B2_CNPG_MANIFEST}"
[[ "$(file_sha256 "${B2_CC_CHART_ARCHIVE}")" == "${B2_CC_CHART_ARCHIVE_SHA256}" ]] ||
  die "Command Center chart archive SHA-256 mismatch"
[[ "$(file_sha256 "${B2_CNPG_MANIFEST}")" == "${B2_CNPG_MANIFEST_SHA256}" ]] ||
  die "CNPG manifest SHA-256 mismatch"

inspect_image "CNPG" "${B2_CNPG_IMAGE}" "${B2_CNPG_IMAGE_ID}"
inspect_image "Rust operator" "${B2_OPERATOR_IMAGE}" "${B2_OPERATOR_IMAGE_ID}"
inspect_image "Citus operand" "${B2_OPERAND_IMAGE}" "${B2_OPERAND_IMAGE_ID}"
inspect_image "registry" "${B2_REGISTRY_IMAGE}" "${B2_REGISTRY_IMAGE_ID}"
inspect_image "kind node" "${B2_KIND_NODE_IMAGE}" "${B2_KIND_NODE_IMAGE_ID}"

actual_operator_revision="$(image_label "${B2_OPERATOR_IMAGE_ID}" org.opencontainers.image.revision)"
actual_operator_tree_state="$(image_label "${B2_OPERATOR_IMAGE_ID}" ai-blaise.citus.source-tree-state)"
actual_operator_source_archive_sha="$(image_label "${B2_OPERATOR_IMAGE_ID}" ai-blaise.citus.source-archive-sha256)"
actual_operand_revision="$(image_label "${B2_OPERAND_IMAGE_ID}" ai-blaise.citus.source-git-sha)"
actual_operand_tree_state="$(image_label "${B2_OPERAND_IMAGE_ID}" ai-blaise.citus.source-tree-state)"
actual_operand_target="$(image_label "${B2_OPERAND_IMAGE_ID}" ai-blaise.citus.bundle1.target)"
actual_operand_scope="$(image_label "${B2_OPERAND_IMAGE_ID}" ai-blaise.citus.bundle1.evidence-scope)"
actual_operand_release_target="$(image_label "${B2_OPERAND_IMAGE_ID}" ai-blaise.citus.bundle1.release-target)"
actual_operand_full_initdb="$(image_label "${B2_OPERAND_IMAGE_ID}" ai-blaise.citus.bundle1.full-initdb-path)"
[[ "${actual_operator_revision}" == "${B2_OPERATOR_SOURCE_REVISION}" ]] ||
  die "operator source revision label mismatch"
[[ "${actual_operator_tree_state}" == "${B2_OPERATOR_TREE_STATE}" ]] ||
  die "operator source tree-state label mismatch"
[[ "${actual_operator_source_archive_sha}" == "${B2_OPERATOR_SOURCE_ARCHIVE_SHA256}" ]] ||
  die "operator source archive label mismatch"
[[ "${actual_operand_revision}" == "${B2_OPERAND_SOURCE_REVISION}" ]] ||
  die "operand source revision label mismatch"
[[ "${actual_operand_tree_state}" == "${B2_OPERAND_TREE_STATE}" ]] ||
  die "operand source tree-state label mismatch"
[[ "${actual_operand_target}" == "bundle1-final-full" ]] ||
  die "B2 requires the full Bundle1 operand target, got ${actual_operand_target}"
[[ "${actual_operand_scope}" == "full-bundle-required-minus-plrust" ]] ||
  die "full Bundle1 operand evidence-scope label mismatch"
[[ "${actual_operand_release_target}" == "true" && "${actual_operand_full_initdb}" == "true" ]] ||
  die "full Bundle1 operand is missing its release-target or full-initdb label"

runtime_dir="$(mktemp -d /tmp/ai-blaise-cnpg-b2.XXXXXX)"
kubeconfig="${runtime_dir}/kubeconfig"
export KUBECONFIG="${kubeconfig}"
chart_extract="${runtime_dir}/chart-input"
chart_dir="${chart_extract}/${chart_subdir}"
patched_cnpg="${runtime_dir}/cnpg-pinned.yaml"
secret_dir="${runtime_dir}/secrets"
mkdir -p "${artifact_dir}" "${chart_extract}" "${secret_dir}"

python3 - "${B2_CC_CHART_ARCHIVE}" "${chart_extract}" <<'PY'
import pathlib
import shutil
import sys
import tarfile

archive = pathlib.Path(sys.argv[1])
destination = pathlib.Path(sys.argv[2])
with tarfile.open(archive, "r:*") as handle:
    members = handle.getmembers()
    if not members:
        raise SystemExit("Command Center chart archive is empty")
    for member in members:
        path = pathlib.PurePosixPath(member.name)
        if path.is_absolute() or ".." in path.parts:
            raise SystemExit(f"unsafe archive path: {member.name}")
        if member.issym() or member.islnk() or member.isdev():
            raise SystemExit(f"archive member type is not allowed: {member.name}")
        target = destination.joinpath(*path.parts)
        if member.isdir():
            target.mkdir(parents=True, exist_ok=True)
        elif member.isfile():
            target.parent.mkdir(parents=True, exist_ok=True)
            source = handle.extractfile(member)
            if source is None:
                raise SystemExit(f"could not read archive member: {member.name}")
            with source, target.open("wb") as output:
                shutil.copyfileobj(source, output)
        else:
            raise SystemExit(f"archive member type is not allowed: {member.name}")
PY

[[ "$(file_sha256 "${B2_CC_CHART_ARCHIVE}")" == "${B2_CC_CHART_ARCHIVE_SHA256}" ]] ||
  die "Command Center chart archive changed while it was being snapshotted"

[[ -f "${chart_dir}/Chart.yaml" && -f "${chart_dir}/values.schema.json" ]] ||
  die "chart archive must contain ${chart_subdir}/Chart.yaml and values.schema.json"
[[ -f "${chart_dir}/crds/ai-blaise-citus-crds.yaml" ]] ||
  die "chart archive is missing the reviewed Rust-exported CRD bundle"
actual_chart_tree_sha="$(chart_tree_sha256 "${chart_dir}")"
[[ "${actual_chart_tree_sha}" == "${B2_CC_CHART_TREE_SHA256}" ]] ||
  die "Command Center chart tree SHA-256 mismatch: expected ${B2_CC_CHART_TREE_SHA256}, got ${actual_chart_tree_sha}"
actual_crd_bundle_sha="$(file_sha256 "${chart_dir}/crds/ai-blaise-citus-crds.yaml")"
[[ "${actual_crd_bundle_sha}" == "${B2_EXPECTED_CRD_BUNDLE_SHA256}" ]] ||
  die "Rust-exported CRD bundle SHA-256 mismatch: expected ${B2_EXPECTED_CRD_BUNDLE_SHA256}, got ${actual_crd_bundle_sha}"
chart_version="$(awk '$1 == "version:" { print $2; exit }' "${chart_dir}/Chart.yaml" | tr -d '"')"
chart_app_version="$(awk '$1 == "appVersion:" { print $2; exit }' "${chart_dir}/Chart.yaml" | tr -d '"')"
[[ "${chart_version}" == "${B2_EXPECTED_CHART_VERSION}" ]] || die "chart version mismatch: ${chart_version}"
[[ "${chart_app_version}" == "${B2_EXPECTED_CHART_VERSION}" ]] || die "chart appVersion mismatch: ${chart_app_version}"

cp "${B2_CNPG_MANIFEST}" "${patched_cnpg}"
[[ "$(file_sha256 "${B2_CNPG_MANIFEST}")" == "${B2_CNPG_MANIFEST_SHA256}" ]] ||
  die "CNPG manifest changed while it was being snapshotted"

if kind get clusters 2>/dev/null | grep -Fx "${cluster_name}" >/dev/null; then
  die "refusing to reuse existing kind cluster ${cluster_name}"
fi
if docker container inspect "${registry_name}" >/dev/null 2>&1; then
  die "refusing to reuse existing registry container ${registry_name}"
fi

registry_port="$(free_port)"
docker run -d --restart=no -p "127.0.0.1:${registry_port}:5000" --name "${registry_name}" \
  "${B2_REGISTRY_IMAGE_ID}" >"${artifact_dir}/registry-container-id.txt"
created_registry=1

cnpg_ref="$(publish_image "${B2_CNPG_IMAGE_ID}" b2/cloudnative-pg)"
operator_ref="$(publish_image "${B2_OPERATOR_IMAGE_ID}" b2/citus-operator)"
operand_ref="$(publish_image "${B2_OPERAND_IMAGE_ID}" b2/citus)"

cluster_resource="b2"
ca_secret="${cluster_resource}-server-ca"
tls_secret="${cluster_resource}-server-tls"
superuser_secret="${cluster_resource}-superuser"
values_file="${runtime_dir}/b2-values.yaml"
cat >"${values_file}" <<YAML
global:
  imageRegistry: localhost:${registry_port}
  imagePullPolicy: IfNotPresent
  requireImageDigest: true
operator:
  replicas: 1
  executionMode: apply
  controllers: [citus_cluster]
  secretNames: []
  controllerRbac:
    enabled: true
  image:
    repository: b2/citus-operator
    tag: ${run_id}
    digest: ${operator_ref##*@}
  serviceMonitor:
    enabled: false
pool:
  enabled: false
postgres:
  postgresMajor: 17
  ioMethod: ""
cluster:
  enabled: true
  name: ${cluster_resource}
  image:
    repository: b2/citus
    digest: ${operand_ref##*@}
  workers: 2
  coordinators: 1
  workerReplicas: 1
  postgresUid: 999
  postgresGid: 999
  storageClass: standard
  storageSize: 1Gi
  clusterDomain: cluster.local
  databases: [app, events]
  extensionVersions:
    citus: ${B2_EXPECTED_CITUS_VERSION}
    companion: ${B2_EXPECTED_COMPANION_VERSION}
  nodeTls:
    serverCaSecret: ${ca_secret}
    serverTlsSecret: ${tls_secret}
    superuserSecret: ${superuser_secret}
    connectTimeoutSeconds: 5
  bootstrap:
    backoffLimit: 3
    activeDeadlineSeconds: 1800
sidecarPriorityClasses:
  enabled: false
sidecarDefaults:
  serviceMonitor:
    enabled: false
  prometheusRule:
    enabled: false
tools:
  enabled: false
observability:
  grafanaDashboards:
    enabled: false
  alertRules:
    enabled: false
YAML

# Validate the exact runtime values against the immutable chart schema before
# launching kind or creating any Kubernetes resource.
cp "${values_file}" "${artifact_dir}/generated-values.yaml"
helm lint "${chart_dir}" --strict --kube-version "${B2_EXPECTED_KUBERNETES_VERSION#v}" \
  --values "${values_file}" >"${artifact_dir}/helm-lint.log"

python3 - "${patched_cnpg}" "${B2_CNPG_MANIFEST_IMAGE}" "${cnpg_ref}" <<'PY'
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
source = sys.argv[2]
replacement = sys.argv[3]
text = path.read_text(encoding="utf-8")
count = text.count(source)
if count != 2:
    raise SystemExit(f"expected exactly two CNPG manifest image references, found {count}")
path.write_text(text.replace(source, replacement), encoding="utf-8")
PY

kind_config="${runtime_dir}/kind.yaml"
cat >"${kind_config}" <<YAML
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
containerdConfigPatches:
  - |-
    [plugins."io.containerd.grpc.v1.cri".registry]
      config_path = "/etc/containerd/certs.d"
nodes:
  - role: control-plane
YAML

created_cluster=1
kind create cluster --name "${cluster_name}" --image "${B2_KIND_NODE_IMAGE}" \
  --config "${kind_config}" --kubeconfig "${kubeconfig}" --wait 180s \
  >"${artifact_dir}/kind-create.log" 2>&1
docker network connect kind "${registry_name}" >/dev/null
kind_node_name="$(kind get nodes --name "${cluster_name}")"
[[ -n "${kind_node_name}" && "${kind_node_name}" != *$'\n'* ]] ||
  die "expected exactly one kind node, got: ${kind_node_name:-none}"
observed_kind_node_id="$(docker container inspect "${kind_node_name}" --format '{{.Image}}')"
[[ "${observed_kind_node_id}" == "${B2_KIND_NODE_IMAGE_ID}" ]] ||
  die "kind node container image mismatch: expected ${B2_KIND_NODE_IMAGE_ID}, got ${observed_kind_node_id}"
registry_dir="/etc/containerd/certs.d/localhost:${registry_port}"
while IFS= read -r node; do
  [[ -n "${node}" ]] || continue
  docker exec "${node}" mkdir -p "${registry_dir}"
  printf '[host."http://%s:5000"]\n' "${registry_name}" |
    docker exec -i "${node}" cp /dev/stdin "${registry_dir}/hosts.toml"
done < <(kind get nodes --name "${cluster_name}")

server_version="$(kubectl version -o json | python3 -c 'import json,sys; print(json.load(sys.stdin)["serverVersion"]["gitVersion"])')"
[[ "${server_version}" == "${B2_EXPECTED_KUBERNETES_VERSION}" ]] ||
  die "Kubernetes server version mismatch: expected ${B2_EXPECTED_KUBERNETES_VERSION}, got ${server_version}"

kubectl apply --server-side --force-conflicts -f "${patched_cnpg}" >"${artifact_dir}/cnpg-apply.log"
kubectl wait --for=condition=Established crd/clusters.postgresql.cnpg.io --timeout=180s >/dev/null
kubectl wait --for=condition=Established crd/imagecatalogs.postgresql.cnpg.io --timeout=180s >/dev/null
kubectl -n cnpg-system rollout status deployment/cnpg-controller-manager --timeout=300s \
  >"${artifact_dir}/cnpg-rollout.log"

observed_cnpg_image="$(kubectl -n cnpg-system get deployment cnpg-controller-manager -o jsonpath='{.spec.template.spec.containers[?(@.name=="manager")].image}')"
observed_cnpg_env_image="$(kubectl -n cnpg-system get deployment cnpg-controller-manager -o jsonpath='{.spec.template.spec.containers[?(@.name=="manager")].env[?(@.name=="OPERATOR_IMAGE_NAME")].value}')"
[[ "${observed_cnpg_image}" == "${cnpg_ref}" && "${observed_cnpg_env_image}" == "${cnpg_ref}" ]] ||
  die "CNPG deployment did not retain the exact local registry digest"

cat >"${secret_dir}/ca.cnf" <<EOF
[req]
prompt = no
distinguished_name = dn
x509_extensions = ca_extensions
[dn]
CN = ai-blaise B2 disposable CA
[ca_extensions]
basicConstraints = critical,CA:TRUE,pathlen:0
keyUsage = critical,keyCertSign,cRLSign
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid:always
EOF
openssl req -x509 -newkey rsa:2048 -nodes -days 2 -sha256 \
  -config "${secret_dir}/ca.cnf" -keyout "${secret_dir}/ca.key" -out "${secret_dir}/ca.crt" \
  >"${artifact_dir}/openssl-ca.log" 2>&1

cat >"${secret_dir}/server.cnf" <<EOF
[req]
prompt = no
distinguished_name = dn
req_extensions = server_extensions
[dn]
CN = ${cluster_resource}-coordinator-rw.${namespace}.svc.cluster.local
[server_extensions]
basicConstraints = critical,CA:FALSE
keyUsage = critical,digitalSignature,keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names
[alt_names]
DNS.1 = ${cluster_resource}-coordinator-rw
DNS.2 = ${cluster_resource}-coordinator-rw.${namespace}
DNS.3 = ${cluster_resource}-coordinator-rw.${namespace}.svc
DNS.4 = ${cluster_resource}-coordinator-rw.${namespace}.svc.cluster.local
DNS.5 = ${cluster_resource}-worker-0-rw
DNS.6 = ${cluster_resource}-worker-0-rw.${namespace}
DNS.7 = ${cluster_resource}-worker-0-rw.${namespace}.svc
DNS.8 = ${cluster_resource}-worker-0-rw.${namespace}.svc.cluster.local
DNS.9 = ${cluster_resource}-worker-1-rw
DNS.10 = ${cluster_resource}-worker-1-rw.${namespace}
DNS.11 = ${cluster_resource}-worker-1-rw.${namespace}.svc
DNS.12 = ${cluster_resource}-worker-1-rw.${namespace}.svc.cluster.local
EOF
openssl req -new -newkey rsa:2048 -nodes -sha256 -config "${secret_dir}/server.cnf" \
  -keyout "${secret_dir}/tls.key" -out "${secret_dir}/server.csr" \
  >"${artifact_dir}/openssl-server-request.log" 2>&1
openssl x509 -req -sha256 -days 2 -in "${secret_dir}/server.csr" \
  -CA "${secret_dir}/ca.crt" -CAkey "${secret_dir}/ca.key" -CAcreateserial \
  -extfile "${secret_dir}/server.cnf" -extensions server_extensions \
  -out "${secret_dir}/tls.crt" >"${artifact_dir}/openssl-server-sign.log" 2>&1
openssl verify -CAfile "${secret_dir}/ca.crt" "${secret_dir}/tls.crt" >"${artifact_dir}/openssl-verify.log"

printf 'postgres' >"${secret_dir}/username"
openssl rand -hex 32 | tr -d '\n' >"${secret_dir}/password"

kubectl create namespace "${namespace}" >/dev/null
kubectl -n "${namespace}" create secret generic "${ca_secret}" \
  --from-file=ca.crt="${secret_dir}/ca.crt" >/dev/null
kubectl -n "${namespace}" label secret "${ca_secret}" cnpg.io/reload='' >/dev/null
kubectl -n "${namespace}" create secret tls "${tls_secret}" \
  --cert="${secret_dir}/tls.crt" --key="${secret_dir}/tls.key" >/dev/null
kubectl -n "${namespace}" label secret "${tls_secret}" cnpg.io/reload='' >/dev/null
kubectl -n "${namespace}" create secret generic "${superuser_secret}" \
  --type=kubernetes.io/basic-auth \
  --from-file=username="${secret_dir}/username" \
  --from-file=password="${secret_dir}/password" >/dev/null

helm install "${release_name}" "${chart_dir}" -n "${namespace}" \
  --values "${values_file}" --wait --timeout 10m >"${artifact_dir}/helm-install.log"
kubectl -n "${namespace}" rollout status deployment/ai-blaise-citus-operator --timeout=300s \
  >"${artifact_dir}/operator-rollout.log"

deadline=$(( $(date +%s) + ready_timeout_seconds ))
while :; do
  phase="$(kubectl -n "${namespace}" get cituscluster "${cluster_resource}" -o jsonpath='{.status.phase}' 2>/dev/null || true)"
  ready="$(kubectl -n "${namespace}" get cituscluster "${cluster_resource}" -o jsonpath='{range .status.conditions[?(@.type=="Ready")]}{.status}{end}' 2>/dev/null || true)"
  if [[ "${phase}" == "Ready" && "${ready}" == "True" ]]; then
    break
  fi
  if [[ "${phase}" == "Invalid" || "${phase}" == "Failed" ]]; then
    last_error="$(kubectl -n "${namespace}" get cituscluster "${cluster_resource}" -o jsonpath='{.status.lastError}' 2>/dev/null || true)"
    die "CitusCluster reached terminal phase ${phase}: ${last_error}"
  fi
  (( $(date +%s) < deadline )) || die "CitusCluster did not become Ready within ${ready_timeout_seconds}s (phase=${phase:-unset})"
  sleep 5
done

cnpg_cluster_count="$(kubectl -n "${namespace}" get cluster.postgresql.cnpg.io \
  -l "citus.ai-blaise.io/cluster=${cluster_resource}" -o name | wc -l | tr -d ' ')"
[[ "${cnpg_cluster_count}" == "3" ]] || die "expected exactly three managed CNPG clusters, got ${cnpg_cluster_count}"
for cnpg_cluster_name in \
  "${cluster_resource}-coordinator" \
  "${cluster_resource}-worker-0" \
  "${cluster_resource}-worker-1"; do
  kubectl -n "${namespace}" wait "cluster.postgresql.cnpg.io/${cnpg_cluster_name}" \
    --for=condition=Ready --timeout=5s >/dev/null
  observed_operand_image="$(kubectl -n "${namespace}" get "cluster.postgresql.cnpg.io/${cnpg_cluster_name}" -o jsonpath='{.spec.imageName}')"
  [[ "${observed_operand_image}" == "${operand_ref}" ]] ||
    die "${cnpg_cluster_name} operand image differs from exact pushed digest"
done

observed_operator_image="$(kubectl -n "${namespace}" get deployment ai-blaise-citus-operator -o jsonpath='{.spec.template.spec.containers[?(@.name=="operator")].image}')"
[[ "${observed_operator_image}" == "${operator_ref}" ]] || die "chart operator deployment image differs from exact pushed digest"

helm test "${release_name}" -n "${namespace}" --timeout 6m --logs \
  >"${artifact_dir}/helm-test.log" 2>&1
grep -F 'verified coordinator and worker topology, metadata agreement, TLS, and exact extension versions' \
  "${artifact_dir}/helm-test.log" >/dev/null || die "Helm SQL test did not emit its success marker"

{
  printf '%s\t%s\n' result PASS
  printf '%s\t%s\n' claim_scope diagnostic-non-promotion
  printf '%s\t%s\n' run_id "${run_id}"
  printf '%s\t%s\n' command_center_chart_archive_sha256 "${B2_CC_CHART_ARCHIVE_SHA256}"
  printf '%s\t%s\n' command_center_chart_tree_sha256 "${actual_chart_tree_sha}"
  printf '%s\t%s\n' command_center_reviewed_chart_fingerprint "${B2_CC_REVIEWED_CHART_FINGERPRINT}"
  printf '%s\t%s\n' rust_exported_crd_bundle_sha256 "${actual_crd_bundle_sha}"
  printf '%s\t%s\n' chart_version "${chart_version}"
  printf '%s\t%s\n' cnpg_manifest_sha256 "${B2_CNPG_MANIFEST_SHA256}"
  printf '%s\t%s\n' cnpg_image_id "${B2_CNPG_IMAGE_ID}"
  printf '%s\t%s\n' cnpg_registry_ref "${cnpg_ref}"
  printf '%s\t%s\n' operator_image_id "${B2_OPERATOR_IMAGE_ID}"
  printf '%s\t%s\n' operator_registry_ref "${operator_ref}"
  printf '%s\t%s\n' operator_source_archive_sha256 "${actual_operator_source_archive_sha}"
  printf '%s\t%s\n' operator_source_revision "${actual_operator_revision}"
  printf '%s\t%s\n' operator_source_tree_state "${actual_operator_tree_state}"
  printf '%s\t%s\n' operand_image_id "${B2_OPERAND_IMAGE_ID}"
  printf '%s\t%s\n' operand_registry_ref "${operand_ref}"
  printf '%s\t%s\n' operand_source_revision "${actual_operand_revision}"
  printf '%s\t%s\n' operand_source_tree_state "${actual_operand_tree_state}"
  printf '%s\t%s\n' operand_bundle_target "${actual_operand_target}"
  printf '%s\t%s\n' operand_evidence_scope "${actual_operand_scope}"
  printf '%s\t%s\n' kind_binary_sha256 "${actual_kind_sha}"
  printf '%s\t%s\n' kind_node_image "${B2_KIND_NODE_IMAGE}"
  printf '%s\t%s\n' kind_node_image_id "${observed_kind_node_id}"
  printf '%s\t%s\n' kubernetes_version "${server_version}"
  printf '%s\t%s\n' topology 1-coordinator+2-logical-workers
  printf '%s\t%s\n' databases app,events
  printf '%s\t%s\n' citus_extension_version "${B2_EXPECTED_CITUS_VERSION}"
  printf '%s\t%s\n' companion_extension_version "${B2_EXPECTED_COMPANION_VERSION}"
  printf '%s\t%s\n' helm_sql_test PASS
} >"${artifact_dir}/summary.tsv"

python3 - "${artifact_dir}/summary.tsv" <<'PY'
import csv
import pathlib
import sys

receipt = pathlib.Path(sys.argv[1])
with receipt.open(encoding="utf-8", newline="") as handle:
    rows = list(csv.reader(handle, delimiter="\t", strict=True))
if not rows or any(len(row) != 2 for row in rows):
    raise SystemExit("summary receipt must contain exactly two tab-delimited fields per row")
if any("\\t" in field or "\n" in field or "\r" in field for row in rows for field in row):
    raise SystemExit("summary receipt contains an escaped or embedded record delimiter")
values = dict(rows)
if len(values) != len(rows) or values.get("result") != "PASS" or values.get("helm_sql_test") != "PASS":
    raise SystemExit("summary receipt is missing unique PASS assertions")
PY

collect_diagnostics
