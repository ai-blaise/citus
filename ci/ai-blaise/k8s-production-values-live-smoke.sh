#!/usr/bin/env bash
set -euo pipefail

# FEATURE: D8
# FEATURE: D13
# Boundary: this generated Postgres chart is a fallback substrate smoke. It
# proves kind/Helm/Kubernetes readiness, immutable image guardrails, alpha-off
# production values, and SQL Service traffic. It does not deploy or certify the
# unpublished ai-blaise/Citus app, operator, or pool images. Use
# ci/ai-blaise/live-k8s-e2e.sh with CHART_DIR or COMMAND_CENTER_DIR plus
# digest-pinned app images for real command-center production deployment proof.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

PATH="${HOME}/.local/bin:${PATH}"

cluster_name="${KIND_CLUSTER_NAME:-ai-blaise-prod-values}"
release_name="${RELEASE_NAME:-ai-blaise-prod-values}"
namespace="${NAMESPACE:-ai-blaise-prod-values-$(date -u +%Y%m%d%H%M%S)}"
rollout_timeout="${ROLLOUT_TIMEOUT:-300s}"
teardown="${TEARDOWN:-1}"
keep_namespace_on_failure="${KEEP_NAMESPACE_ON_FAILURE:-0}"
artifact_dir="${ARTIFACT_DIR:-artifacts/k8s-production-values-live/$(date -u +%Y%m%dT%H%M%SZ)}"
postgres_image="${K8S_LIVE_POSTGRES_IMAGE:-docker.io/library/postgres:16-alpine@sha256:16bc17c64a573ef34162af9298258d1aec548232985b33ed7b1eac33ba35c229}"
postgres_password="${POSTGRES_PASSWORD:-ai-blaise-live-k8s}"
created_kind_cluster=0
created_namespace=0
ran_install=0

chart_dir="${artifact_dir}/chart"
values_file="${artifact_dir}/values-production.yaml"
rendered_manifest="${artifact_dir}/rendered.yaml"
render_guardrail_report="${artifact_dir}/production-values-guardrails.txt"
traffic_log="${artifact_dir}/sql-traffic-job.log"
summary_file="${artifact_dir}/summary.tsv"
boundary_file="${artifact_dir}/claim-boundary.txt"

log() {
  printf '[k8s-production-values-live] %s\n' "$*" >&2
}

die() {
  log "ERROR: $*"
  exit 1
}

need_cmd() {
  local command_name="$1"
  command -v "${command_name}" >/dev/null 2>&1 || die "missing required command: ${command_name}"
}

is_immutable_image_ref() {
  local image_ref="$1"
  [[ "${image_ref}" =~ @sha256:[a-f0-9]{64}$ ]]
}

reject_mutable_image() {
  local image_ref="$1"
  if [[ "${image_ref}" == *":latest" || "${image_ref}" == *":latest@"* ]]; then
    die "production-values smoke refuses latest image refs: ${image_ref}"
  fi
  if ! is_immutable_image_ref "${image_ref}"; then
    die "production-values smoke requires immutable @sha256 image refs: ${image_ref}"
  fi
}

choose_kind_context() {
  need_cmd kind
  need_cmd docker
  need_cmd kubectl

  if ! kind get clusters | grep -Fxq "${cluster_name}"; then
    log "creating kind cluster ${cluster_name}"
    kind create cluster --name "${cluster_name}"
    created_kind_cluster=1
  fi

  export KUBECONFIG="${KUBECONFIG:-${HOME}/.kube/config}"
  kubectl config use-context "kind-${cluster_name}" >/dev/null
  kubectl version --request-timeout=10s >/dev/null
}

write_chart() {
  mkdir -p "${chart_dir}/templates" "${artifact_dir}"

  cat >"${chart_dir}/Chart.yaml" <<'YAML'
apiVersion: v2
name: ai-blaise-production-values-live
version: 0.1.0
appVersion: "postgres-16"
description: VM live Kubernetes production-values evidence harness for ai-blaise/citus.
YAML

  cat >"${values_file}" <<YAML
productionValues:
  strict: true
  alphaSidecarsEnabled: false
  mutableImagesAllowed: false
  latestImagesAllowed: false
postgres:
  image: "${postgres_image}"
  database: postgres
  username: postgres
  password: "${postgres_password}"
  replicas: 1
  resources:
    requests:
      cpu: 100m
      memory: 256Mi
    limits:
      memory: 512Mi
traffic:
  enabled: true
  serviceDns: "${release_name}-postgres.${namespace}.svc.cluster.local"
YAML

  cat >"${chart_dir}/templates/_helpers.tpl" <<'YAML'
{{- define "prod-values.name" -}}
{{- .Chart.Name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- define "prod-values.fullname" -}}
{{- printf "%s-%s" .Release.Name "postgres" | trunc 63 | trimSuffix "-" -}}
{{- end -}}
YAML

  cat >"${chart_dir}/templates/serviceaccount.yaml" <<'YAML'
apiVersion: v1
kind: ServiceAccount
metadata:
  name: {{ include "prod-values.fullname" . }}
  labels:
    app.kubernetes.io/name: {{ include "prod-values.name" . }}
    app.kubernetes.io/instance: {{ .Release.Name }}
automountServiceAccountToken: false
---
apiVersion: v1
kind: ServiceAccount
metadata:
  name: {{ include "prod-values.fullname" . }}-client
  labels:
    app.kubernetes.io/name: {{ include "prod-values.name" . }}
    app.kubernetes.io/instance: {{ .Release.Name }}
automountServiceAccountToken: false
YAML

  cat >"${chart_dir}/templates/secret.yaml" <<'YAML'
apiVersion: v1
kind: Secret
metadata:
  name: {{ include "prod-values.fullname" . }}
  labels:
    app.kubernetes.io/name: {{ include "prod-values.name" . }}
    app.kubernetes.io/instance: {{ .Release.Name }}
type: Opaque
stringData:
  postgres-password: {{ .Values.postgres.password | quote }}
YAML

  cat >"${chart_dir}/templates/configmap.yaml" <<'YAML'
apiVersion: v1
kind: ConfigMap
metadata:
  name: {{ include "prod-values.fullname" . }}-sql
  labels:
    app.kubernetes.io/name: {{ include "prod-values.name" . }}
    app.kubernetes.io/instance: {{ .Release.Name }}
data:
  traffic.sql: |
    SELECT version();
    CREATE TABLE IF NOT EXISTS ai_blaise_live_k8s_evidence (
      id integer PRIMARY KEY,
      note text NOT NULL,
      created_at timestamptz NOT NULL DEFAULT now()
    );
    INSERT INTO ai_blaise_live_k8s_evidence (id, note)
    VALUES (1, 'k8s-production-values-live')
    ON CONFLICT (id) DO UPDATE SET note = EXCLUDED.note;
    SELECT 'traffic-result=' || note FROM ai_blaise_live_k8s_evidence WHERE id = 1;
YAML

  cat >"${chart_dir}/templates/service.yaml" <<'YAML'
apiVersion: v1
kind: Service
metadata:
  name: {{ include "prod-values.fullname" . }}
  labels:
    app.kubernetes.io/name: {{ include "prod-values.name" . }}
    app.kubernetes.io/instance: {{ .Release.Name }}
    ai-blaise.io/production-values: "true"
    ai-blaise.io/alpha-sidecars-enabled: "false"
spec:
  type: ClusterIP
  selector:
    app.kubernetes.io/name: {{ include "prod-values.name" . }}
    app.kubernetes.io/instance: {{ .Release.Name }}
    app.kubernetes.io/component: postgres
  ports:
    - name: postgres
      port: 5432
      targetPort: postgres
YAML

  cat >"${chart_dir}/templates/statefulset.yaml" <<'YAML'
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: {{ include "prod-values.fullname" . }}
  labels:
    app.kubernetes.io/name: {{ include "prod-values.name" . }}
    app.kubernetes.io/instance: {{ .Release.Name }}
    app.kubernetes.io/component: postgres
    ai-blaise.io/production-values: "true"
    ai-blaise.io/alpha-sidecars-enabled: "false"
spec:
  serviceName: {{ include "prod-values.fullname" . }}
  replicas: {{ .Values.postgres.replicas }}
  selector:
    matchLabels:
      app.kubernetes.io/name: {{ include "prod-values.name" . }}
      app.kubernetes.io/instance: {{ .Release.Name }}
      app.kubernetes.io/component: postgres
  template:
    metadata:
      labels:
        app.kubernetes.io/name: {{ include "prod-values.name" . }}
        app.kubernetes.io/instance: {{ .Release.Name }}
        app.kubernetes.io/component: postgres
        ai-blaise.io/production-values: "true"
        ai-blaise.io/alpha-sidecars-enabled: "false"
    spec:
      serviceAccountName: {{ include "prod-values.fullname" . }}
      automountServiceAccountToken: false
      terminationGracePeriodSeconds: 30
      securityContext:
        fsGroup: 70
      containers:
        - name: postgres
          image: {{ .Values.postgres.image | quote }}
          imagePullPolicy: IfNotPresent
          ports:
            - name: postgres
              containerPort: 5432
          env:
            - name: POSTGRES_DB
              value: {{ .Values.postgres.database | quote }}
            - name: POSTGRES_USER
              value: {{ .Values.postgres.username | quote }}
            - name: POSTGRES_PASSWORD
              valueFrom:
                secretKeyRef:
                  name: {{ include "prod-values.fullname" . }}
                  key: postgres-password
            - name: PGDATA
              value: /var/lib/postgresql/data/pgdata
          readinessProbe:
            exec:
              command: ["pg_isready", "-U", {{ .Values.postgres.username | quote }}, "-d", {{ .Values.postgres.database | quote }}]
            initialDelaySeconds: 5
            periodSeconds: 5
            timeoutSeconds: 3
            failureThreshold: 18
          livenessProbe:
            exec:
              command: ["pg_isready", "-U", {{ .Values.postgres.username | quote }}, "-d", {{ .Values.postgres.database | quote }}]
            initialDelaySeconds: 20
            periodSeconds: 10
            timeoutSeconds: 3
            failureThreshold: 6
          resources:
{{ toYaml .Values.postgres.resources | indent 12 }}
          volumeMounts:
            - name: pgdata
              mountPath: /var/lib/postgresql/data
      volumes:
        - name: pgdata
          emptyDir: {}
YAML

  cat >"${chart_dir}/templates/pdb.yaml" <<'YAML'
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: {{ include "prod-values.fullname" . }}
  labels:
    app.kubernetes.io/name: {{ include "prod-values.name" . }}
    app.kubernetes.io/instance: {{ .Release.Name }}
spec:
  minAvailable: 1
  selector:
    matchLabels:
      app.kubernetes.io/name: {{ include "prod-values.name" . }}
      app.kubernetes.io/instance: {{ .Release.Name }}
      app.kubernetes.io/component: postgres
YAML

  cat >"${chart_dir}/templates/networkpolicy.yaml" <<'YAML'
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: {{ include "prod-values.fullname" . }}
  labels:
    app.kubernetes.io/name: {{ include "prod-values.name" . }}
    app.kubernetes.io/instance: {{ .Release.Name }}
spec:
  podSelector:
    matchLabels:
      app.kubernetes.io/name: {{ include "prod-values.name" . }}
      app.kubernetes.io/instance: {{ .Release.Name }}
      app.kubernetes.io/component: postgres
  policyTypes: ["Ingress"]
  ingress:
    - from:
        - podSelector:
            matchLabels:
              app.kubernetes.io/name: {{ include "prod-values.name" . }}
              app.kubernetes.io/instance: {{ .Release.Name }}
              app.kubernetes.io/component: sql-client
      ports:
        - protocol: TCP
          port: 5432
YAML

  cat >"${chart_dir}/templates/job.yaml" <<'YAML'
apiVersion: batch/v1
kind: Job
metadata:
  name: {{ include "prod-values.fullname" . }}-sql-client
  labels:
    app.kubernetes.io/name: {{ include "prod-values.name" . }}
    app.kubernetes.io/instance: {{ .Release.Name }}
    app.kubernetes.io/component: sql-client
    ai-blaise.io/production-values: "true"
    ai-blaise.io/alpha-sidecars-enabled: "false"
spec:
  backoffLimit: 1
  ttlSecondsAfterFinished: 300
  template:
    metadata:
      labels:
        app.kubernetes.io/name: {{ include "prod-values.name" . }}
        app.kubernetes.io/instance: {{ .Release.Name }}
        app.kubernetes.io/component: sql-client
        ai-blaise.io/production-values: "true"
        ai-blaise.io/alpha-sidecars-enabled: "false"
    spec:
      restartPolicy: Never
      serviceAccountName: {{ include "prod-values.fullname" . }}-client
      automountServiceAccountToken: false
      containers:
        - name: sql-client
          image: {{ .Values.postgres.image | quote }}
          imagePullPolicy: IfNotPresent
          env:
            - name: PGPASSWORD
              valueFrom:
                secretKeyRef:
                  name: {{ include "prod-values.fullname" . }}
                  key: postgres-password
          command:
            - sh
            - -ceu
            - |
              for attempt in $(seq 1 90); do
                if pg_isready -h {{ include "prod-values.fullname" . }} -p 5432 -U {{ .Values.postgres.username | quote }} -d {{ .Values.postgres.database | quote }}; then
                  break
                fi
                sleep 2
              done
              psql -v ON_ERROR_STOP=1 \
                -h {{ include "prod-values.fullname" . }} \
                -p 5432 \
                -U {{ .Values.postgres.username | quote }} \
                -d {{ .Values.postgres.database | quote }} \
                -f /evidence/traffic.sql
              echo "traffic=sql-service status=ok service={{ include "prod-values.fullname" . }}.{{ .Release.Namespace }}.svc.cluster.local port=5432"
          volumeMounts:
            - name: evidence-sql
              mountPath: /evidence
              readOnly: true
          resources:
            requests:
              cpu: 50m
              memory: 128Mi
            limits:
              memory: 256Mi
      volumes:
        - name: evidence-sql
          configMap:
            name: {{ include "prod-values.fullname" . }}-sql
YAML
}

render_chart() {
  need_cmd helm
  helm lint "${chart_dir}" --values "${values_file}" >"${artifact_dir}/helm-lint.log" 2>&1 || {
    sed -n '1,220p' "${artifact_dir}/helm-lint.log" >&2
    die "helm lint failed"
  }
  helm template "${release_name}" "${chart_dir}" \
    --namespace "${namespace}" \
    --values "${values_file}" \
    >"${rendered_manifest}"
}

validate_rendered_manifest() {
  need_cmd python3
  need_cmd kubectl
  python3 - "${rendered_manifest}" "${render_guardrail_report}" <<'PYSCRIPT'
import re
import sys
from pathlib import Path
manifest = Path(sys.argv[1])
report = Path(sys.argv[2])
text = manifest.read_text()
errors = []
images = re.findall(r'^\s*image:\s*["\']?([^"\'\s]+)', text, flags=re.MULTILINE)
if not images:
    errors.append('rendered manifest has no images')
for image in images:
    if ':latest' in image or image.endswith(':latest'):
        errors.append(f'mutable latest image rejected: {image}')
    if not re.search(r'@sha256:[a-f0-9]{64}$', image):
        errors.append(f'image is not immutable by digest: {image}')
    if image.startswith('example.invalid/') or image.startswith('localhost/'):
        errors.append(f'placeholder/local image rejected for production-values evidence: {image}')
alpha_true = re.findall(r'(?im)^\s*[^#\n]*(?:alpha|experimental|preview)[^:\n]*:\s*["\']?(?:true|enabled)["\']?\s*$', text)
if alpha_true:
    errors.extend(f'alpha/experimental enablement rejected: {line.strip()}' for line in alpha_true)
if re.search(r'(?im)^\s*imagePullPolicy:\s*Always\s*$', text):
    errors.append('imagePullPolicy Always rejected for immutable production-values smoke')
if re.search(r'(?i)(sidecar-(analytical|auth|backup|cdc|coldtier|edge-functions|graphql|hlc|mcp|postgrest|raft|realtime|repack|schema-job|storage|txn-status|vectorizer))', text):
    errors.append('alpha sidecar workload leaked into production-values live smoke')
required = ['kind: StatefulSet', 'kind: Service', 'kind: Job', 'kind: Secret', 'kind: NetworkPolicy', 'kind: PodDisruptionBudget']
for phrase in required:
    if phrase not in text:
        errors.append(f'missing rendered Kubernetes resource: {phrase}')
if 'traffic=sql-service' not in text:
    errors.append('SQL service traffic job marker missing')
report.parent.mkdir(parents=True, exist_ok=True)
if errors:
    report.write_text('status=failed\n' + '\n'.join(errors) + '\n')
    for error in errors:
        print(error, file=sys.stderr)
    sys.exit(1)
report.write_text('status=ok\nimages=' + ','.join(sorted(set(images))) + '\nalpha_sidecars=false\nmutable_images=false\ntraffic=sql-service\nclaim_boundary=postgres_substrate_only\nno_citus_app_images=true\n')
PYSCRIPT
  kubectl apply --dry-run=client --validate=false -f "${rendered_manifest}" >"${artifact_dir}/kubectl-client-dry-run.txt"
}

install_release() {
  if kubectl get namespace "${namespace}" >/dev/null 2>&1; then
    created_namespace=0
  else
    created_namespace=1
  fi
  ran_install=1
  log "installing release=${release_name} namespace=${namespace}"
  helm upgrade --install "${release_name}" "${chart_dir}" \
    --namespace "${namespace}" \
    --create-namespace \
    --values "${values_file}" \
    --wait \
    --timeout "${rollout_timeout}"
}

wait_for_live_evidence() {
  local statefulset="statefulset/${release_name}-postgres"
  local job="job/${release_name}-postgres-sql-client"
  log "waiting for ${statefulset}"
  kubectl -n "${namespace}" rollout status "${statefulset}" --timeout "${rollout_timeout}"
  kubectl -n "${namespace}" wait pod \
    -l "app.kubernetes.io/instance=${release_name},app.kubernetes.io/component=postgres" \
    --for=condition=Ready \
    --timeout "${rollout_timeout}"
  log "waiting for SQL traffic job"
  if ! kubectl -n "${namespace}" wait "${job}" --for=condition=complete --timeout "${rollout_timeout}"; then
    kubectl -n "${namespace}" get pods -o wide >"${artifact_dir}/pods-on-job-failure.txt" 2>&1 || true
    kubectl -n "${namespace}" logs "${job}" >"${traffic_log}" 2>&1 || true
    sed -n '1,220p' "${traffic_log}" >&2 || true
    die "SQL traffic job did not complete"
  fi
  kubectl -n "${namespace}" logs "${job}" >"${traffic_log}"
  grep -Fq 'traffic=sql-service status=ok' "${traffic_log}" || die "SQL traffic job log missing success marker"
  grep -Fq 'traffic-result=k8s-production-values-live' "${traffic_log}" || die "SQL traffic job did not read back inserted row"
}

collect_evidence() {
  [[ "${ran_install}" == "1" ]] || return 0
  kubectl -n "${namespace}" get all -o wide >"${artifact_dir}/kubectl-get-all.txt" 2>&1 || true
  kubectl -n "${namespace}" get events --sort-by=.lastTimestamp >"${artifact_dir}/kubectl-events.txt" 2>&1 || true
  kubectl -n "${namespace}" get pods -o json >"${artifact_dir}/pods.json" 2>&1 || true
  kubectl -n "${namespace}" get svc -o json >"${artifact_dir}/services.json" 2>&1 || true
  helm -n "${namespace}" get manifest "${release_name}" >"${artifact_dir}/helm-get-manifest.yaml" 2>&1 || true
  helm -n "${namespace}" get values "${release_name}" --all >"${artifact_dir}/helm-get-values.yaml" 2>&1 || true
  if command -v jq >/dev/null 2>&1 && [[ -s "${artifact_dir}/pods.json" ]]; then
    jq -r '.items[] | .metadata.name as $pod | .status.containerStatuses[]? | [$pod, .name, .image, (.imageID // "")] | @tsv' \
      "${artifact_dir}/pods.json" >"${artifact_dir}/pod-images.tsv" || true
  fi
}

cleanup() {
  local status="$?"
  set +e
  collect_evidence
  if [[ "${ran_install}" == "1" && "${teardown}" == "1" ]]; then
    if [[ "${status}" -ne 0 && "${keep_namespace_on_failure}" == "1" ]]; then
      log "keeping namespace ${namespace} after failure because KEEP_NAMESPACE_ON_FAILURE=1"
    else
      log "tearing down release=${release_name} namespace=${namespace}"
      helm -n "${namespace}" uninstall "${release_name}" >/dev/null 2>&1 || true
      if [[ "${created_namespace}" == "1" ]]; then
        kubectl delete namespace "${namespace}" --ignore-not-found --timeout=120s >/dev/null 2>&1 || true
      fi
    fi
  fi
  if [[ "${created_kind_cluster}" == "1" && "${DELETE_KIND_CLUSTER:-0}" == "1" ]]; then
    kind delete cluster --name "${cluster_name}" >/dev/null 2>&1 || true
  fi
  exit "${status}"
}
trap cleanup EXIT

reject_mutable_image "${postgres_image}"
need_cmd helm
need_cmd kubectl
need_cmd kind
need_cmd docker
need_cmd python3

write_chart
render_chart
choose_kind_context
validate_rendered_manifest
install_release
wait_for_live_evidence
collect_evidence
cat >"${boundary_file}" <<BOUNDARY
claim_boundary=postgres_substrate_only
real_kind_cluster=true
real_helm_install=true
real_kubernetes_readiness=true
real_sql_service_traffic=true
immutable_image_digest=true
alpha_sidecars_enabled=false
no_unpublished_citus_app_images=true
no_operator_pool_or_citus_data_plane_claim=true
preferred_real_production_path=ci/ai-blaise/live-k8s-e2e.sh_with_COMMAND_CENTER_DIR_or_CHART_DIR_and_digest_pinned_app_images
BOUNDARY

printf 'k8s_production_values_live_smoke\tstatus=ok\tcluster=%s\tnamespace=%s\trelease=%s\ttraffic=sql-service\tclaim_boundary=postgres-substrate-only\timage=%s\tartifacts=%s\n' \
  "${cluster_name}" \
  "${namespace}" \
  "${release_name}" \
  "${postgres_image}" \
  "${artifact_dir}" | tee "${summary_file}"
