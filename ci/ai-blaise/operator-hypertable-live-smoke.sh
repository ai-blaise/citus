#!/usr/bin/env bash
set -euo pipefail

# FEATURE: TS7
# Live Hypertable-controller proof: an in-cluster operator watches the generated
# Hypertable CRD, executes the production Timescale/Citus SQL apply path against
# a live Postgres pod, patches status, and survives an immediate re-reconcile
# without duplicating bridge-state rows.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

cluster_name="${HYPERTABLE_CONTROLLER_KIND_CLUSTER:-ai-blaise-ts7-hypertable-live}"
namespace="${HYPERTABLE_CONTROLLER_NAMESPACE:-ai-blaise-ts7}"
kind_node_image="${HYPERTABLE_CONTROLLER_KIND_NODE_IMAGE:-kindest/node:v1.30.0}"
operator_image="${HYPERTABLE_OPERATOR_IMAGE:-ai-blaise-citus-operator:ts7-live}"
cohab_image="${TIMESCALE_COHABITATION_IMAGE:-ai-blaise-citus-timescale-cohabitation:local}"
cohab_base="${TIMESCALE_COHABITATION_BASE_IMAGE:-timescale/timescaledb-ha:pg17-ts2.27}"
rebuild_cohab="${TIMESCALE_COHABITATION_REBUILD:-1}"
evidence_file="${HYPERTABLE_CONTROLLER_EVIDENCE:-artifacts/operator-hypertable-live-evidence.tsv}"
keep_cluster="${KEEP_KIND_CLUSTER:-0}"
operator_log="artifacts/operator-hypertable-live-operator.log"
postgres_deploy="timescale-postgres"
hypertable_name="operator-metrics"

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "$1 is required for TS7 Hypertable controller live smoke" >&2
    exit 1
  fi
}

cleanup() {
  if [[ "${keep_cluster}" != "1" ]]; then
    kind delete cluster --name "${cluster_name}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

require_tool cargo
require_tool docker
require_tool kind
require_tool kubectl

mkdir -p artifacts "$(dirname "${evidence_file}")"
: >"${operator_log}"

if [[ "${rebuild_cohab}" == "1" ]] || ! docker image inspect "${cohab_image}" >/dev/null 2>&1; then
  docker build \
    --file images/citus-timescale-cohabitation/Dockerfile \
    --build-arg "BASE_IMAGE=${cohab_base}" \
    --tag "${cohab_image}" \
    "${repo_root}"
fi

DOCKER_BUILDKIT=1 docker build \
  --file images/rust-runtime/Dockerfile \
  --build-arg "PACKAGE=ai_blaise_citus_operator" \
  --build-arg "BIN=ai_blaise_citus_operator" \
  --build-arg "DEFAULT_ARGS=serve" \
  --tag "${operator_image}" \
  "${repo_root}" >/tmp/ts7-operator-image-build.log

kind delete cluster --name "${cluster_name}" >/dev/null 2>&1 || true
kind create cluster --name "${cluster_name}" --image "${kind_node_image}" --wait 120s >/dev/null
kubectl config use-context "kind-${cluster_name}" >/dev/null
kind load docker-image --name "${cluster_name}" "${operator_image}" >/dev/null
kind load docker-image --name "${cluster_name}" "${cohab_image}" >/dev/null

cargo run -q -p ai_blaise_citus_operator -- print-hypertable-crd | kubectl apply -f - >/dev/null
kubectl create namespace "${namespace}" >/dev/null

cat <<YAML | kubectl -n "${namespace}" apply -f - >/dev/null
apiVersion: v1
kind: Service
metadata:
  name: ${postgres_deploy}
spec:
  selector:
    app.kubernetes.io/name: ai-blaise-citus
    app.kubernetes.io/component: timescale-postgres
  ports:
    - name: postgres
      port: 5432
      targetPort: postgres
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ${postgres_deploy}
  labels:
    app.kubernetes.io/name: ai-blaise-citus
    app.kubernetes.io/component: timescale-postgres
spec:
  replicas: 1
  selector:
    matchLabels:
      app.kubernetes.io/name: ai-blaise-citus
      app.kubernetes.io/component: timescale-postgres
  template:
    metadata:
      labels:
        app.kubernetes.io/name: ai-blaise-citus
        app.kubernetes.io/component: timescale-postgres
    spec:
      containers:
        - name: postgres
          image: ${cohab_image}
          imagePullPolicy: IfNotPresent
          args:
            - postgres
            - -c
            - shared_preload_libraries=timescaledb,citus
            - -c
            - citus.cohabit_extensions=timescaledb
          env:
            - name: POSTGRES_PASSWORD
              value: postgres
          ports:
            - name: postgres
              containerPort: 5432
          readinessProbe:
            exec:
              command: ["pg_isready", "-U", "postgres"]
            initialDelaySeconds: 5
            periodSeconds: 2
            timeoutSeconds: 2
          resources:
            requests:
              cpu: 500m
              memory: 1Gi
            limits:
              cpu: "2"
              memory: 3Gi
YAML

kubectl -n "${namespace}" rollout status "deployment/${postgres_deploy}" --timeout=240s >/dev/null

cat <<YAML | kubectl -n "${namespace}" apply -f - >/dev/null
apiVersion: batch/v1
kind: Job
metadata:
  name: ts7-db-init
spec:
  template:
    spec:
      restartPolicy: Never
      containers:
        - name: psql
          image: ${cohab_image}
          imagePullPolicy: IfNotPresent
          command: ["bash", "-ec"]
          env:
            - name: PGPASSWORD
              value: postgres
          args:
            - |
              until pg_isready -h ${postgres_deploy} -U postgres; do sleep 1; done
              psql -h ${postgres_deploy} -U postgres -v ON_ERROR_STOP=1 <<'SQL'
              CREATE EXTENSION IF NOT EXISTS citus;
              CREATE EXTENSION IF NOT EXISTS timescaledb;
              CREATE EXTENSION IF NOT EXISTS ai_blaise_citus;
              DO \$\$
              BEGIN
                IF (SELECT extversion FROM pg_extension WHERE extname = 'ai_blaise_citus')
                    IS DISTINCT FROM '0.1.2' THEN
                  RAISE EXCEPTION 'expected shipped ai_blaise_citus version 0.1.2';
                END IF;
              END \$\$;
              CREATE TABLE IF NOT EXISTS operator_metrics (
                metric_time timestamptz NOT NULL,
                tenant_id integer NOT NULL,
                value double precision NOT NULL
              );
              CREATE INDEX IF NOT EXISTS operator_metrics_metric_time_idx
              ON operator_metrics(metric_time);
              SQL
  backoffLimit: 0
YAML
kubectl -n "${namespace}" wait --for=condition=complete job/ts7-db-init --timeout=240s >/dev/null

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
  - apiGroups: ["citus.ai-blaise.io"]
    resources: ["hypertables"]
    verbs: ["get", "list", "watch"]
  - apiGroups: ["citus.ai-blaise.io"]
    resources: ["hypertables/status"]
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
  name: ai-blaise-citus-operator
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
          image: ${operator_image}
          imagePullPolicy: IfNotPresent
          env:
            - name: AI_BLAISE_OPERATOR_EXECUTION_MODE
              value: apply
            - name: AI_BLAISE_OPERATOR_CONTROLLERS
              value: hypertable
            - name: AI_BLAISE_HYPERTABLE_DATABASE_URL
              value: postgres://postgres:postgres@${postgres_deploy}.${namespace}.svc.cluster.local:5432/postgres
            - name: AI_BLAISE_LISTEN_ADDR
              value: 0.0.0.0:8080
            - name: RUST_LOG
              value: info
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
kubectl -n "${namespace}" rollout status deployment/ai-blaise-citus-operator --timeout=180s >/dev/null

cat <<YAML | kubectl -n "${namespace}" apply -f - >/dev/null
apiVersion: citus.ai-blaise.io/v2
kind: Hypertable
metadata:
  name: ${hypertable_name}
spec:
  table: operator_metrics
  timeColumn: metric_time
  distributionColumn: tenant_id
  chunkTimeInterval: 1 day
  numShards: 2
  compression:
    olderThan: 7 days
    segmentBy: [tenant_id]
    orderBy: [metric_time DESC]
  retention:
    dropAfter: 90 days
  continuousAggregates:
    - name: operator_metrics_hourly
      query: SELECT time_bucket('1 hour', metric_time) AS bucket, avg(value) AS avg_value FROM operator_metrics GROUP BY 1
      refreshStart: 7 days
      refreshEnd: 1 hour
      schedule: 1 hour
YAML

wait_for_status_field() {
  local jsonpath="$1"
  local expected="$2"
  local actual=""
  for _ in $(seq 1 180); do
    actual="$(kubectl -n "${namespace}" get hypertable "${hypertable_name}" -o "jsonpath=${jsonpath}" 2>/dev/null || true)"
    if [[ "${actual}" == "${expected}" ]]; then
      return 0
    fi
    sleep 1
  done
  kubectl -n "${namespace}" get hypertable "${hypertable_name}" -o yaml >&2 || true
  kubectl -n "${namespace}" logs deployment/ai-blaise-citus-operator >&2 || true
  echo "Hypertable status ${jsonpath} did not become ${expected}; last=${actual}" >&2
  return 1
}

wait_for_status_field '{.status.phase}' 'Applied'
status_hash="$(kubectl -n "${namespace}" get hypertable "${hypertable_name}" -o jsonpath='{.status.lastAppliedSqlHash}')"
if [[ ! "${status_hash}" =~ ^fnv1a64:[0-9a-f]{16}$ ]]; then
  kubectl -n "${namespace}" get hypertable "${hypertable_name}" -o yaml >&2
  echo "Hypertable status did not contain a stable SQL hash" >&2
  exit 1
fi
observed_generation="$(kubectl -n "${namespace}" get hypertable "${hypertable_name}" -o jsonpath='{.status.observedGeneration}')"
if [[ ! "${observed_generation}" =~ ^[0-9]+$ ]] || [[ "${observed_generation}" -lt 1 ]]; then
  kubectl -n "${namespace}" get hypertable "${hypertable_name}" -o yaml >&2
  echo "Hypertable status did not record observedGeneration" >&2
  exit 1
fi

kubectl -n "${namespace}" annotate hypertable "${hypertable_name}" "ts7.ai-blaise.io/reconcile=$(date +%s)" --overwrite >/dev/null
for _ in $(seq 1 180); do
  skipped="$(kubectl -n "${namespace}" get hypertable "${hypertable_name}" -o jsonpath='{.status.skippedStepCount}' 2>/dev/null || true)"
  phase="$(kubectl -n "${namespace}" get hypertable "${hypertable_name}" -o jsonpath='{.status.phase}' 2>/dev/null || true)"
  if [[ "${phase}" == "Applied" && "${skipped:-0}" =~ ^[0-9]+$ && "${skipped}" -ge 5 ]]; then
    break
  fi
  sleep 1
done
if [[ "${phase:-}" != "Applied" || ! "${skipped:-0}" =~ ^[0-9]+$ || "${skipped}" -lt 5 ]]; then
  kubectl -n "${namespace}" get hypertable "${hypertable_name}" -o yaml >&2 || true
  kubectl -n "${namespace}" logs deployment/ai-blaise-citus-operator >&2 || true
  echo "Hypertable re-reconcile did not skip previously applied bridge-state steps" >&2
  exit 1
fi

kubectl -n "${namespace}" exec deploy/${postgres_deploy} -- psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
DO $$
DECLARE
  bridge_features integer;
  duplicate_bridge_rows integer;
BEGIN
  IF NOT EXISTS (
    SELECT 1
    FROM _timescaledb_catalog.hypertable
    WHERE table_name = 'operator_metrics'
  ) THEN
    RAISE EXCEPTION 'Hypertable controller did not create a real Timescale hypertable';
  END IF;
  IF NOT EXISTS (
    SELECT 1
    FROM pg_dist_partition
    WHERE logicalrelid = 'operator_metrics'::regclass
  ) THEN
    RAISE EXCEPTION 'Hypertable controller did not distribute the table through Citus';
  END IF;
  IF to_regclass('operator_metrics_hourly') IS NULL THEN
    RAISE EXCEPTION 'Hypertable controller did not create the continuous aggregate';
  END IF;
  SELECT count(DISTINCT feature_id)
  INTO bridge_features
  FROM companion_timescale_bridge_state
  WHERE feature_id IN ('TS1', 'TS2', 'TS3', 'TS4', 'TS5')
    AND object_name IN ('operator_metrics', 'operator_metrics_hourly');
  IF bridge_features <> 5 THEN
    RAISE EXCEPTION 'expected five TS7 bridge-state feature ids, got %', bridge_features;
  END IF;
  SELECT count(*)
  INTO duplicate_bridge_rows
  FROM (
    SELECT feature_id, object_name
    FROM companion_timescale_bridge_state
    WHERE feature_id IN ('TS1', 'TS2', 'TS3', 'TS4', 'TS5')
      AND object_name IN ('operator_metrics', 'operator_metrics_hourly')
    GROUP BY feature_id, object_name
    HAVING count(*) > 1
  ) duplicates;
  IF duplicate_bridge_rows <> 0 THEN
    RAISE EXCEPTION 'Hypertable re-reconcile duplicated bridge-state rows';
  END IF;
END $$;
SQL

kubectl -n "${namespace}" logs deployment/ai-blaise-citus-operator >"${operator_log}" || true
operator_image_id="$(docker image inspect --format '{{.Id}}' "${operator_image}")"
cohab_image_id="$(docker image inspect --format '{{.Id}}' "${cohab_image}")"
status_phase="$(kubectl -n "${namespace}" get hypertable "${hypertable_name}" -o jsonpath='{.status.phase}')"
applied="$(kubectl -n "${namespace}" get hypertable "${hypertable_name}" -o jsonpath='{.status.appliedStepCount}')"
skipped="$(kubectl -n "${namespace}" get hypertable "${hypertable_name}" -o jsonpath='{.status.skippedStepCount}')"
cat >"${evidence_file}" <<EOF_EVIDENCE
feature_id	cluster	namespace	status_phase	observed_generation	applied_steps	skipped_steps	sql_hash	operator_image_id	cohabitation_image_id
TS7	${cluster_name}	${namespace}	${status_phase}	${observed_generation}	${applied}	${skipped}	${status_hash}	${operator_image_id}	${cohab_image_id}
EOF_EVIDENCE

cat "${evidence_file}"
echo "operator-hypertable-live-smoke passed"
