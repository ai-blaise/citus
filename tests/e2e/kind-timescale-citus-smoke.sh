#!/usr/bin/env bash
set -euo pipefail

cluster_name="${KIND_CLUSTER_NAME:-ai-blaise-citus-smoke}"
namespace="${SMOKE_NAMESPACE:-ai-blaise-citus-smoke}"
chart_dir="${CHART_DIR:-deploy/k8s/helm/citus-overlay}"
db_image="${SMOKE_DB_IMAGE:-}"
run_live="${RUN_KIND_SMOKE:-0}"
reuse_cluster="${KIND_REUSE_CLUSTER:-0}"

require_file() {
  local file="$1"
  if [[ ! -s "${file}" ]]; then
    echo "missing smoke artifact: ${file}" >&2
    exit 1
  fi
}

require_command() {
  local command_name="$1"
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "missing required command for live smoke: ${command_name}" >&2
    exit 1
  fi
}

require_file "${chart_dir}/Chart.yaml"
require_file "${chart_dir}/values.yaml"
require_file "${chart_dir}/crds/ai-blaise-citus-crds.yaml"

if [[ "${run_live}" != "1" ]]; then
  echo "kind smoke contract-only check verified; set RUN_KIND_SMOKE=1 and SMOKE_DB_IMAGE to run live cohabitation evidence"
  exit 0
fi

for command_name in docker kind kubectl helm psql; do
  require_command "${command_name}"
done

if [[ -z "${db_image}" ]]; then
  echo "SMOKE_DB_IMAGE must point at an image containing Postgres, Citus, TimescaleDB, and ai_blaise_citus" >&2
  exit 1
fi

created_cluster=0
if ! kind get clusters | grep -qx "${cluster_name}"; then
  kind create cluster --name "${cluster_name}"
  created_cluster=1
fi

cleanup() {
  if [[ "${created_cluster}" == "1" && "${reuse_cluster}" != "1" ]]; then
    kind delete cluster --name "${cluster_name}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

kubectl config use-context "kind-${cluster_name}" >/dev/null
kubectl create namespace "${namespace}" --dry-run=client -o yaml | kubectl apply -f -

rendered="$(mktemp)"
trap 'rm -f "${rendered}"; cleanup' EXIT
helm template ai-blaise-citus "${chart_dir}" --namespace "${namespace}" >"${rendered}"
kubectl apply --dry-run=client -f "${chart_dir}/crds/ai-blaise-citus-crds.yaml" >/dev/null
kubectl apply --dry-run=client -n "${namespace}" -f "${rendered}" >/dev/null

kind load docker-image --name "${cluster_name}" "${db_image}"

cat <<YAML | kubectl apply -n "${namespace}" -f -
apiVersion: v1
kind: Pod
metadata:
  name: smoke-postgres
  labels:
    app.kubernetes.io/name: ai-blaise-citus-smoke
spec:
  restartPolicy: Never
  containers:
    - name: postgres
      image: ${db_image}
      imagePullPolicy: IfNotPresent
      env:
        - name: POSTGRES_PASSWORD
          value: postgres
      args:
        - postgres
        - -c
        - shared_preload_libraries=citus,timescaledb
        - -c
        - citus.cohabit_extensions=timescaledb
YAML

kubectl wait -n "${namespace}" --for=condition=Ready pod/smoke-postgres --timeout=240s

kubectl exec -n "${namespace}" smoke-postgres -- \
  psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
SHOW shared_preload_libraries;
SELECT current_setting('citus.cohabit_extensions', true) AS cohabit_extensions;
CREATE EXTENSION IF NOT EXISTS citus;
CREATE EXTENSION IF NOT EXISTS timescaledb;
SELECT extname
FROM pg_extension
WHERE extname IN ('citus', 'timescaledb')
ORDER BY extname;
CREATE TABLE IF NOT EXISTS citus_smoke_metrics (
  tenant_id integer NOT NULL,
  metric_time timestamptz NOT NULL,
  value double precision NOT NULL
);
SELECT create_distributed_table('citus_smoke_metrics', 'tenant_id');
CREATE TABLE IF NOT EXISTS timescale_smoke_metrics (
  metric_time timestamptz NOT NULL,
  value double precision NOT NULL
);
SELECT create_hypertable('timescale_smoke_metrics', 'metric_time', if_not_exists => true);
DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_available_extensions WHERE name = 'ai_blaise_citus') THEN
    CREATE EXTENSION IF NOT EXISTS ai_blaise_citus;
    IF EXISTS (SELECT 1 FROM companion_feature_status() WHERE status = 'planned') THEN
      RAISE EXCEPTION 'companion_feature_status must not report planned features';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM companion_feature_status() WHERE feature_id = 'TS1') THEN
      RAISE EXCEPTION 'companion_feature_status must report TS1';
    END IF;
    PERFORM distribute_hypertable('timescale_smoke_metrics', 'metric_time', '1 day', 4);
    PERFORM time_range_shard_pruner('timescale_smoke_metrics', 'metric_time');
  END IF;
END $$;
SQL

echo "kind Timescale-on-Citus smoke passed for ${db_image}"
