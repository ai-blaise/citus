#!/usr/bin/env bash
set -euo pipefail

chart_dir="deploy/k8s/helm/citus-overlay"
argo_app="deploy/k8s/argo/app.yaml"
kind_smoke="ci/ai-blaise/kind-production-smoke.sh"
deploy_workflow=".github/workflows/ci-deploy.yml"
pool_workflow=".github/workflows/ci-pool.yml"
operator_workflow=".github/workflows/ci-operator.yml"
sidecar_workflow=".github/workflows/ci-sidecar.yml"
makefile="Makefile.ai-blaise"
required_files=(
  "${argo_app}"
  "${kind_smoke}"
  "${deploy_workflow}"
  "${pool_workflow}"
  "${operator_workflow}"
  "${sidecar_workflow}"
  "${makefile}"
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
grep -q 'ai_blaise_sidecar_ready' "${chart_dir}/templates/observability-dashboards.yaml"
grep -q 'ai_blaise_sidecar_ready' "${chart_dir}/templates/observability-prometheusrules.yaml"
if grep -R "ai_blaise_citus_sidecar_ready" "${chart_dir}/templates"; then
  echo "observability templates must query emitted ai_blaise_sidecar_ready metric" >&2
  exit 1
fi
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
grep -q 'apply_monitoring_crds' "${kind_smoke}"
grep -q 'PROD_VALUES_NAMESPACE' "${kind_smoke}"
grep -q -- '-f deploy/k8s/helm/citus-overlay/values-prod.yaml' "${kind_smoke}"
grep -q 'assert_no_alpha_workload_deployments' "${kind_smoke}"
grep -q 'exhaustive image-matrix smoke passed' "${kind_smoke}"
grep -q 'helm uninstall "${release}"' "${kind_smoke}"
grep -q 'ClusterRole cleanup' "${kind_smoke}"
grep -q 'values-prod.yaml production profile smoke passed' "${kind_smoke}"
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
grep -q 'helm:' "${argo_app}"
grep -q 'valueFiles:' "${argo_app}"
grep -q 'values-prod.yaml' "${argo_app}"
grep -q 'Install Helm for rendered chart checks' "${deploy_workflow}"
grep -q 'kind-production-smoke:' "${deploy_workflow}"
grep -q 'Run live Kubernetes production smoke' "${deploy_workflow}"
grep -q 'bash ci/ai-blaise/kind-production-smoke.sh' "${deploy_workflow}"
grep -Eq '^gate-close: .*kind-production-smoke' "${makefile}"
for workflow in "${pool_workflow}" "${operator_workflow}" "${sidecar_workflow}"; do
  grep -q 'ai-blaise/bootstrap-v2' "${workflow}"
done

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

prod_enabled_sidecars="$(
  awk '
    /^sidecars:$/ { in_sidecars = 1; next }
    in_sidecars && /^[^[:space:]-]/ { in_sidecars = 0 }
    in_sidecars && /^[[:space:]]*- name:/ { name = $3 }
    in_sidecars && /^[[:space:]]+enabled:[[:space:]]+true$/ { print name }
  ' "${chart_dir}/values-prod.yaml"
)"

if [[ -n "${prod_enabled_sidecars}" ]]; then
  echo "values-prod.yaml must not enable alpha sidecars by default:" >&2
  echo "${prod_enabled_sidecars}" >&2
  exit 1
fi

prod_enabled_alpha_intent="$(
  python3 - "${chart_dir}/values-prod.yaml" <<'PY'
import pathlib
import re
import sys

values = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")

def section(name: str) -> str:
    match = re.search(rf"^{name}:\n(?P<body>(?:  .*\n|  \n)*)", values, re.M)
    return match.group("body") if match else ""

pool = section("pool")
postgres = section("postgres")
security = section("security")
findings = []

if re.search(r"protocolPipeline:\n(?:    .*\n)*    enabled:\s+true\b", pool):
    findings.append("T7 pool.protocolPipeline.enabled")

if re.search(r"networkPolicy:\n(?:    .*\n)*    cidrAllowlist:\n(?:      - .+\n)+", pool):
    findings.append("Sec13 pool.networkPolicy.cidrAllowlist")

if re.search(r"ioMethod:\s+io_uring\b", postgres):
    findings.append("T6 postgres.ioMethod")

if re.search(r"externalSecrets:\n(?:    .*\n)*    enabled:\s+true\b", security):
    findings.append("Sec7 security.externalSecrets.enabled")

if re.search(r"tls:\n(?:    .*\n)*    (clients|postgres|sidecars):\s+true\b", security):
    findings.append("Sec8 security.tls")

if re.search(r"releaseAttestation:\n(?:    .*\n)*    (sbom|cosign):\s+true\b", security):
    findings.append("Sec9 security.releaseAttestation")

print("\n".join(findings))
PY
)"

if [[ -n "${prod_enabled_alpha_intent}" ]]; then
  echo "values-prod.yaml must not enable alpha runtime/security intent controls by default:" >&2
  echo "${prod_enabled_alpha_intent}" >&2
  exit 1
fi

if grep -R "{{" "${chart_dir}/crds"; then
  echo "crds/ files must be static Kubernetes YAML, not Helm templates" >&2
  exit 1
fi

crd_count="$(grep -c '^kind: CustomResourceDefinition$' "${chart_dir}/crds/ai-blaise-citus-crds.yaml")"
if [[ "${crd_count}" -ne 17 ]]; then
  echo "expected 17 CRDs, found ${crd_count}" >&2
  exit 1
fi

if command -v helm >/dev/null 2>&1; then
  render_dir="$(mktemp -d)"
  cleanup_render() {
    rm -rf "${render_dir}"
  }
  trap cleanup_render EXIT

  helm template default-check "${chart_dir}" >"${render_dir}/default.yaml"
  helm template dev-check "${chart_dir}" \
    -f "${chart_dir}/values-dev.yaml" >"${render_dir}/dev.yaml"
  helm template prod-check "${chart_dir}" \
    -f "${chart_dir}/values-prod.yaml" >"${render_dir}/prod.yaml"

  grep -q 'kind: Deployment' "${render_dir}/default.yaml"
  grep -q 'app.kubernetes.io/component: sidecar-analytical' "${render_dir}/default.yaml"
  grep -q 'kind: Deployment' "${render_dir}/dev.yaml"
  grep -q 'app.kubernetes.io/component: sidecar-mcp' "${render_dir}/dev.yaml"
  grep -q 'app.kubernetes.io/component: tools' "${render_dir}/dev.yaml"

  grep -q 'name: ai-blaise-citus-operator' "${render_dir}/prod.yaml"
  grep -q 'name: ai-blaise-citus-pool' "${render_dir}/prod.yaml"
  grep -q 'kind: ServiceMonitor' "${render_dir}/prod.yaml"
  grep -q 'kind: PrometheusRule' "${render_dir}/prod.yaml"
  grep -q 'ai_blaise_sidecar_ready' "${render_dir}/prod.yaml"
  if grep -q 'app.kubernetes.io/component: sidecar-' "${render_dir}/prod.yaml"; then
    echo "values-prod.yaml render must not include alpha sidecar deployments" >&2
    exit 1
  fi
  if grep -q 'app.kubernetes.io/component: tools' "${render_dir}/prod.yaml"; then
    echo "values-prod.yaml render must not include alpha tools deployment" >&2
    exit 1
  fi
else
  echo "helm unavailable; skipping rendered chart profile checks"
fi
