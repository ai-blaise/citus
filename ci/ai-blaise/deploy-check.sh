#!/usr/bin/env bash
set -euo pipefail

chart_dir="deploy/k8s/helm/citus-overlay"
kind_smoke="ci/ai-blaise/kind-production-smoke.sh"
required_files=(
  "${kind_smoke}"
  "${chart_dir}/Chart.yaml"
  "${chart_dir}/values.yaml"
  "${chart_dir}/values-dev.yaml"
  "${chart_dir}/values-prod.yaml"
  "${chart_dir}/templates/operator-deployment.yaml"
  "${chart_dir}/templates/operator-rbac.yaml"
  "${chart_dir}/templates/operator-service.yaml"
  "${chart_dir}/templates/operator-servicemonitor.yaml"
  "${chart_dir}/templates/observability-dashboards.yaml"
  "${chart_dir}/templates/observability-prometheusrules.yaml"
  "${chart_dir}/templates/pool-deployment.yaml"
  "${chart_dir}/templates/pool-service.yaml"
  "${chart_dir}/templates/sidecar-deployments.yaml"
  "${chart_dir}/templates/tools-deployment.yaml"
  "${chart_dir}/crds/ai-blaise-citus-crds.yaml"
)

for file in "${required_files[@]}"; do
  if [[ ! -s "${file}" ]]; then
    echo "missing required deploy artifact: ${file}" >&2
    exit 1
  fi
done

if grep -R "kube-prometheus-stack\\|external-secrets\\|velero\\|loki\\|falco" "${chart_dir}"; then
  echo "deploy chart must not bundle third-party platform charts" >&2
  exit 1
fi

grep -q '^apiVersion: v2$' "${chart_dir}/Chart.yaml"
grep -q '^name: ai-blaise-citus-overlay$' "${chart_dir}/Chart.yaml"
grep -q '^global:$' "${chart_dir}/values.yaml"
grep -q '^operator:$' "${chart_dir}/values.yaml"
grep -q '^pool:$' "${chart_dir}/values.yaml"
grep -q '^postgres:$' "${chart_dir}/values.yaml"
grep -q '^observability:$' "${chart_dir}/values.yaml"
grep -q '^security:$' "${chart_dir}/values.yaml"
grep -q '^sidecarDefaults:$' "${chart_dir}/values.yaml"
grep -q '^sidecars:$' "${chart_dir}/values.yaml"
grep -q 'ioMethod: io_uring' "${chart_dir}/values.yaml"
grep -q 'protocolPipeline:' "${chart_dir}/values.yaml"
grep -q 'adminPort:' "${chart_dir}/values.yaml"
grep -q 'upstream:' "${chart_dir}/values.yaml"
grep -q 'cidrAllowlist:' "${chart_dir}/values.yaml"
grep -q 'externalSecrets:' "${chart_dir}/values.yaml"
grep -q 'releaseAttestation:' "${chart_dir}/values.yaml"
grep -q 'FEATURE: O6' "${chart_dir}/templates/observability-dashboards.yaml"
grep -q 'FEATURE: O10' "${chart_dir}/templates/observability-prometheusrules.yaml"
grep -q 'kind: ConfigMap' "${chart_dir}/templates/observability-dashboards.yaml"
grep -q 'kind: PrometheusRule' "${chart_dir}/templates/observability-prometheusrules.yaml"
grep -q 'args:' "${chart_dir}/templates/operator-deployment.yaml"
grep -q 'AI_BLAISE_LISTEN_ADDR' "${chart_dir}/templates/operator-deployment.yaml"
grep -q 'readinessProbe:' "${chart_dir}/templates/operator-deployment.yaml"
grep -q 'livenessProbe:' "${chart_dir}/templates/operator-deployment.yaml"
grep -q 'readOnlyRootFilesystem: true' "${chart_dir}/templates/operator-deployment.yaml"
grep -q 'args:' "${chart_dir}/templates/pool-deployment.yaml"
grep -q 'AI_BLAISE_LISTEN_ADDR' "${chart_dir}/templates/pool-deployment.yaml"
grep -q 'AI_BLAISE_POOL_ADMIN_ADDR' "${chart_dir}/templates/pool-deployment.yaml"
grep -q 'AI_BLAISE_POOL_LISTEN_ADDR' "${chart_dir}/templates/pool-deployment.yaml"
grep -q 'AI_BLAISE_POOL_UPSTREAM_ADDR' "${chart_dir}/templates/pool-deployment.yaml"
grep -q 'name: admin' "${chart_dir}/templates/pool-deployment.yaml"
grep -q 'readinessProbe:' "${chart_dir}/templates/pool-deployment.yaml"
grep -q 'livenessProbe:' "${chart_dir}/templates/pool-deployment.yaml"
grep -q 'readOnlyRootFilesystem: true' "${chart_dir}/templates/pool-deployment.yaml"
grep -A4 'readinessProbe:' "${chart_dir}/templates/pool-deployment.yaml" | grep -q 'port: admin'
grep -A4 'livenessProbe:' "${chart_dir}/templates/pool-deployment.yaml" | grep -q 'port: admin'
grep -q 'targetPort: admin' "${chart_dir}/templates/pool-service.yaml"
grep -q 'args:' "${chart_dir}/templates/sidecar-deployments.yaml"
grep -q 'AI_BLAISE_LISTEN_ADDR' "${chart_dir}/templates/sidecar-deployments.yaml"
grep -q 'readinessProbe:' "${chart_dir}/templates/sidecar-deployments.yaml"
grep -q 'livenessProbe:' "${chart_dir}/templates/sidecar-deployments.yaml"
grep -q 'readOnlyRootFilesystem: true' "${chart_dir}/templates/sidecar-deployments.yaml"

if [[ ! -x scripts/citus-scale/deploy.sh ]]; then
  echo "missing executable D8 deploy wrapper: scripts/citus-scale/deploy.sh" >&2
  exit 1
fi
if [[ ! -x "${kind_smoke}" ]]; then
  echo "missing executable Kubernetes production smoke: ${kind_smoke}" >&2
  exit 1
fi

grep -q 'FEATURE: D8' scripts/citus-scale/deploy.sh
grep -q 'FEATURE: D13' "${kind_smoke}"
grep -q 'kind create cluster' "${kind_smoke}"
grep -q 'scripts/citus-scale/build-app-images.sh' "${kind_smoke}"
grep -q 'helm upgrade --install' "${kind_smoke}"
grep -q 'probe_deployment_http' "${kind_smoke}"
grep -q 'expected_probe_component' "${kind_smoke}"
grep -q 'probe_pool_admin_pods' "${kind_smoke}"
grep -q 'port-forward' "${kind_smoke}"
grep -q '/healthz' "${kind_smoke}"
grep -q '/readyz' "${kind_smoke}"
grep -q '/metrics' "${kind_smoke}"
grep -q 'ai_blaise_sidecar_ready' "${kind_smoke}"
grep -q 'ai_blaise_citus_pool_requests_total' "${kind_smoke}"
grep -q 'psql -h ai-blaise-citus-pool' "${kind_smoke}"
grep -q 'FEATURE: D9' docs/ai-blaise/RUNBOOKS/upgrade.md
grep -q 'FEATURE: D10' docs/ai-blaise/RUNBOOKS/production.md
grep -q 'FEATURE: MR9' docs/ai-blaise/RUNBOOKS/disaster-recovery.md

required_sidecars=(
  analytical auth backup cdc coldtier edge-functions graphql hlc mcp
  postgrest raft realtime repack schema-job storage txn-status vectorizer
)

for values_file in \
  "${chart_dir}/values.yaml" \
  "${chart_dir}/values-dev.yaml" \
  "${chart_dir}/values-prod.yaml"; do
  for sidecar in "${required_sidecars[@]}"; do
    if ! grep -Eq "^[[:space:]]*- name: ${sidecar}$" "${values_file}"; then
      echo "missing sidecar ${sidecar} in ${values_file}" >&2
      exit 1
    fi
  done
done

if grep -R "{{" "${chart_dir}/crds"; then
  echo "crds/ files must be static Kubernetes YAML, not Helm templates" >&2
  exit 1
fi

crd_count="$(grep -c '^kind: CustomResourceDefinition$' "${chart_dir}/crds/ai-blaise-citus-crds.yaml")"
if [[ "${crd_count}" -ne 17 ]]; then
  echo "expected 17 CRDs, found ${crd_count}" >&2
  exit 1
fi
