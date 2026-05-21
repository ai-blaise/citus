#!/usr/bin/env bash
set -euo pipefail

require_helm="${REQUIRE_HELM:-0}"
chart_dir="deploy/k8s/helm/citus-overlay"
argo_app="deploy/k8s/argo/app.yaml"
kind_smoke="ci/ai-blaise/kind-production-smoke.sh"
deploy_workflow=".github/workflows/ci-deploy.yml"
pool_workflow=".github/workflows/ci-pool.yml"
operator_workflow=".github/workflows/ci-operator.yml"
sidecar_workflow=".github/workflows/ci-sidecar.yml"
makefile="Makefile.ai-blaise"
custom_workflows=(.github/workflows/ci-*.yml)
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
  "${chart_dir}/values-exhaustive.yaml"
  "${chart_dir}/values-prod.yaml"
  "${chart_dir}/templates/operator-deployment.yaml"
  "${chart_dir}/templates/operator-rbac.yaml"
  "${chart_dir}/templates/operator-service.yaml"
  "${chart_dir}/templates/operator-servicemonitor.yaml"
  "${chart_dir}/templates/observability-dashboards.yaml"
  "${chart_dir}/templates/observability-prometheusrules.yaml"
  "${chart_dir}/templates/pool-deployment.yaml"
  "${chart_dir}/templates/pool-networkpolicy.yaml"
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
grep -q 'requireImageDigest:' "${chart_dir}/values.yaml"
grep -q '^operator:$' "${chart_dir}/values.yaml"
grep -q '^pool:$' "${chart_dir}/values.yaml"
grep -q '^postgres:$' "${chart_dir}/values.yaml"
grep -q '^observability:$' "${chart_dir}/values.yaml"
grep -q '^security:$' "${chart_dir}/values.yaml"
grep -q '^sidecarDefaults:$' "${chart_dir}/values.yaml"
grep -q '^sidecars:$' "${chart_dir}/values.yaml"
grep -q 'requireImageDigest: true' "${chart_dir}/values.yaml"
grep -q 'protocolPipeline:' "${chart_dir}/values.yaml"
grep -q 'adminPort:' "${chart_dir}/values.yaml"
grep -q 'upstream:' "${chart_dir}/values.yaml"
grep -q 'cidrAllowlist:' "${chart_dir}/values.yaml"
grep -q 'externalSecrets:' "${chart_dir}/values.yaml"
grep -q 'releaseAttestation:' "${chart_dir}/values.yaml"
grep -q 'requireImageDigest: false' "${chart_dir}/values-exhaustive.yaml"
grep -q 'ioMethod: io_uring' "${chart_dir}/values-exhaustive.yaml"
grep -q 'protocolPipeline:' "${chart_dir}/values-exhaustive.yaml"
grep -q 'externalSecrets:' "${chart_dir}/values-exhaustive.yaml"
grep -q 'releaseAttestation:' "${chart_dir}/values-exhaustive.yaml"
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
grep -q 'operator.image.digest' "${chart_dir}/templates/operator-deployment.yaml"
grep -q 'readinessProbe:' "${chart_dir}/templates/operator-deployment.yaml"
grep -q 'livenessProbe:' "${chart_dir}/templates/operator-deployment.yaml"
grep -q 'readOnlyRootFilesystem: true' "${chart_dir}/templates/operator-deployment.yaml"
grep -q 'args:' "${chart_dir}/templates/pool-deployment.yaml"
grep -q 'AI_BLAISE_LISTEN_ADDR' "${chart_dir}/templates/pool-deployment.yaml"
grep -q 'AI_BLAISE_POOL_ADMIN_ADDR' "${chart_dir}/templates/pool-deployment.yaml"
grep -q 'AI_BLAISE_POOL_LISTEN_ADDR' "${chart_dir}/templates/pool-deployment.yaml"
grep -q 'AI_BLAISE_POOL_UPSTREAM_ADDR' "${chart_dir}/templates/pool-deployment.yaml"
grep -q 'AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST' "${chart_dir}/templates/pool-deployment.yaml"
grep -q 'pool.image.digest' "${chart_dir}/templates/pool-deployment.yaml"
grep -q 'name: admin' "${chart_dir}/templates/pool-deployment.yaml"
grep -q 'readinessProbe:' "${chart_dir}/templates/pool-deployment.yaml"
grep -q 'livenessProbe:' "${chart_dir}/templates/pool-deployment.yaml"
grep -q 'readOnlyRootFilesystem: true' "${chart_dir}/templates/pool-deployment.yaml"
grep -A4 'readinessProbe:' "${chart_dir}/templates/pool-deployment.yaml" | grep -q 'port: admin'
grep -A4 'livenessProbe:' "${chart_dir}/templates/pool-deployment.yaml" | grep -q 'port: admin'
grep -q 'FEATURE: Sec13' "${chart_dir}/templates/pool-networkpolicy.yaml"
grep -q 'kind: NetworkPolicy' "${chart_dir}/templates/pool-networkpolicy.yaml"
grep -q 'cidrAllowlist' "${chart_dir}/templates/pool-networkpolicy.yaml"
grep -q 'ipBlock:' "${chart_dir}/templates/pool-networkpolicy.yaml"
grep -q 'targetPort: admin' "${chart_dir}/templates/pool-service.yaml"
grep -q 'args:' "${chart_dir}/templates/sidecar-deployments.yaml"
grep -q 'AI_BLAISE_LISTEN_ADDR' "${chart_dir}/templates/sidecar-deployments.yaml"
grep -q 'sidecarDefaults.digest' "${chart_dir}/templates/sidecar-deployments.yaml"
grep -q 'readinessProbe:' "${chart_dir}/templates/sidecar-deployments.yaml"
grep -q 'livenessProbe:' "${chart_dir}/templates/sidecar-deployments.yaml"
grep -q 'readOnlyRootFilesystem: true' "${chart_dir}/templates/sidecar-deployments.yaml"
if grep -q 'resources: \["\*"\]' "${chart_dir}/templates/operator-rbac.yaml"; then
  echo "operator RBAC must enumerate ai-blaise resources explicitly" >&2
  exit 1
fi
if grep -q '"secrets"' "${chart_dir}/templates/operator-rbac.yaml"; then
  echo "operator RBAC must not grant Secret access while secret binding is alpha" >&2
  exit 1
fi
grep -q 'citusclusters' "${chart_dir}/templates/operator-rbac.yaml"
grep -q 'scheduledrepacks' "${chart_dir}/templates/operator-rbac.yaml"

if [[ ! -x scripts/citus-scale/deploy.sh ]]; then
  echo "missing executable D8 deploy wrapper: scripts/citus-scale/deploy.sh" >&2
  exit 1
fi
if [[ ! -x "${kind_smoke}" ]]; then
  echo "missing executable Kubernetes production smoke: ${kind_smoke}" >&2
  exit 1
fi

grep -q 'FEATURE: D8' scripts/citus-scale/deploy.sh
grep -q 'deploy_profile="${DEPLOY_PROFILE:-prod}"' scripts/citus-scale/deploy.sh
grep -q 'values-prod.yaml' scripts/citus-scale/deploy.sh
grep -q 'values-exhaustive.yaml' scripts/citus-scale/deploy.sh
grep -q 'ALLOW_ALPHA_INSTALL' scripts/citus-scale/deploy.sh
grep -q 'OPERATOR_IMAGE_DIGEST' scripts/citus-scale/deploy.sh
grep -q 'POOL_IMAGE_DIGEST' scripts/citus-scale/deploy.sh
grep -q 'ALLOW_MUTABLE_IMAGE_TAGS' scripts/citus-scale/deploy.sh
grep -q 'refusing to install non-production values file' scripts/citus-scale/deploy.sh
grep -q 'FEATURE: D13' "${kind_smoke}"
grep -q 'kind create cluster' "${kind_smoke}"
grep -q 'scripts/citus-scale/build-app-images.sh' "${kind_smoke}"
grep -q 'helm upgrade --install' "${kind_smoke}"
grep -q 'values-exhaustive.yaml' "${kind_smoke}"
grep -q 'global.requireImageDigest=false' "${kind_smoke}"
grep -q 'apply_monitoring_crds' "${kind_smoke}"
grep -q 'assert_observability_resources' "${kind_smoke}"
grep -q 'configmap/${chart_name}-dashboards' "${kind_smoke}"
grep -q 'prometheusrules.monitoring.coreos.com/${chart_name}-alerts' "${kind_smoke}"
grep -q 'AiBlaiseCitusSidecarNotReady' "${kind_smoke}"
grep -q 'DEFAULT_VALUES_NAMESPACE' "${kind_smoke}"
grep -q 'PROD_VALUES_NAMESPACE' "${kind_smoke}"
grep -q 'DEPLOY_PROFILE=prod' "${kind_smoke}"
grep -q 'MODE=install' "${kind_smoke}"
grep -q 'ALLOW_MUTABLE_IMAGE_TAGS=1' "${kind_smoke}"
grep -q 'scripts/citus-scale/deploy.sh' "${kind_smoke}"
grep -q 'assert_no_alpha_workload_deployments' "${kind_smoke}"
grep -q 'exhaustive image-matrix smoke passed' "${kind_smoke}"
grep -q 'helm uninstall "${release}"' "${kind_smoke}"
grep -q 'ClusterRole cleanup' "${kind_smoke}"
grep -q 'values.yaml default production-safe profile smoke passed' "${kind_smoke}"
grep -q 'values-prod.yaml production profile smoke passed' "${kind_smoke}"
grep -q 'probe_deployment_http' "${kind_smoke}"
grep -q 'expected_probe_component' "${kind_smoke}"
grep -q 'probe_pool_admin_pods' "${kind_smoke}"
grep -q 'pool admin smoke did not observe ready upstream metrics' "${kind_smoke}"
grep -q 'run_pool_cidr_deny_smoke' "${kind_smoke}"
grep -q 'ai-blaise-pool-cidr-deny-smoke' "${kind_smoke}"
grep -q 'ai_blaise_citus_pool_rejected_connections_total' "${kind_smoke}"
grep -q 'run_citusctl_image_smoke' "${kind_smoke}"
grep -q 'ai-blaise-citusctl-image-smoke' "${kind_smoke}"
grep -q 'citusctl inspect destructive=false requires_plan_id=true steps=3' "${kind_smoke}"
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
grep -q 'targetRevision: main' "${argo_app}"
grep -q 'valueFiles:' "${argo_app}"
grep -q 'values-prod.yaml' "${argo_app}"
grep -q 'prune: true' "${argo_app}"
grep -q 'selfHeal: true' "${argo_app}"
grep -q 'CreateNamespace=true' "${argo_app}"
grep -q 'PruneLast=true' "${argo_app}"
grep -q 'Install Helm for rendered chart checks' "${deploy_workflow}"
grep -q 'kind-production-smoke:' "${deploy_workflow}"
grep -q 'Run live Kubernetes production smoke' "${deploy_workflow}"
grep -q 'bash ci/ai-blaise/kind-production-smoke.sh' "${deploy_workflow}"
grep -Eq '^gate-close: .*kind-production-smoke' "${makefile}"
for workflow in "${deploy_workflow}" "${pool_workflow}" "${operator_workflow}" "${sidecar_workflow}"; do
  grep -q -- '- main' "${workflow}"
  grep -q -- '- ai-blaise/dev' "${workflow}"
done
for workflow in "${custom_workflows[@]}"; do
  grep -q -- '- main' "${workflow}"
  grep -q -- '- ai-blaise/dev' "${workflow}"
  if grep -q -- '- ai-blaise/bootstrap-v2' "${workflow}"; then
    echo "custom CI workflow must not target stale bootstrap branch: ${workflow}" >&2
    exit 1
  fi
done

required_sidecars=(
  analytical auth backup cdc coldtier edge-functions graphql hlc mcp
  postgrest raft realtime repack schema-job storage txn-status vectorizer
)

for values_file in \
  "${chart_dir}/values.yaml" \
  "${chart_dir}/values-dev.yaml" \
  "${chart_dir}/values-exhaustive.yaml" \
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

default_enabled_sidecars="$(
  awk '
    /^sidecars:$/ { in_sidecars = 1; next }
    in_sidecars && /^[^[:space:]-]/ { in_sidecars = 0 }
    in_sidecars && /^[[:space:]]*- name:/ { name = $3 }
    in_sidecars && /^[[:space:]]+enabled:[[:space:]]+true$/ { print name }
  ' "${chart_dir}/values.yaml"
)"

if [[ -n "${default_enabled_sidecars}" ]]; then
  echo "values.yaml must not enable alpha sidecars by default:" >&2
  echo "${default_enabled_sidecars}" >&2
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

default_enabled_alpha_intent="$(
  python3 - "${chart_dir}/values.yaml" <<'PY'
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

if [[ -n "${default_enabled_alpha_intent}" ]]; then
  echo "values.yaml must not enable alpha runtime/security intent controls by default:" >&2
  echo "${default_enabled_alpha_intent}" >&2
  exit 1
fi

grep -q 'requireImageDigest: true' "${chart_dir}/values.yaml"
grep -q 'requireImageDigest: true' "${chart_dir}/values-prod.yaml"

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

  if helm template default-check-missing-digest "${chart_dir}" >"${render_dir}/default-missing-digest.yaml" 2>"${render_dir}/default-missing-digest.err"; then
    echo "values.yaml default render must require immutable operator/pool image digests" >&2
    exit 1
  fi
  grep -q 'requires an immutable digest' "${render_dir}/default-missing-digest.err"

  helm template default-check "${chart_dir}" \
    --set operator.image.digest=sha256:1111111111111111111111111111111111111111111111111111111111111111 \
    --set pool.image.digest=sha256:2222222222222222222222222222222222222222222222222222222222222222 \
    >"${render_dir}/default.yaml"
  helm template dev-check "${chart_dir}" \
    -f "${chart_dir}/values-dev.yaml" >"${render_dir}/dev.yaml"
  helm template exhaustive-check "${chart_dir}" \
    -f "${chart_dir}/values-exhaustive.yaml" >"${render_dir}/exhaustive.yaml"
  if helm template prod-check-missing-digest "${chart_dir}" \
    -f "${chart_dir}/values-prod.yaml" >"${render_dir}/prod-missing-digest.yaml" 2>"${render_dir}/prod-missing-digest.err"; then
    echo "values-prod.yaml render must require immutable operator/pool image digests" >&2
    exit 1
  fi
  grep -q 'requires an immutable digest' "${render_dir}/prod-missing-digest.err"

  helm template prod-check "${chart_dir}" \
    -f "${chart_dir}/values-prod.yaml" \
    --set operator.image.digest=sha256:1111111111111111111111111111111111111111111111111111111111111111 \
    --set pool.image.digest=sha256:2222222222222222222222222222222222222222222222222222222222222222 \
    >"${render_dir}/prod.yaml"

  grep -q 'kind: Deployment' "${render_dir}/default.yaml"
  grep -q 'name: ai-blaise-citus-operator' "${render_dir}/default.yaml"
  grep -q 'name: ai-blaise-citus-pool' "${render_dir}/default.yaml"
  grep -q 'kind: ServiceMonitor' "${render_dir}/default.yaml"
  grep -q 'kind: ConfigMap' "${render_dir}/default.yaml"
  grep -q 'kind: PrometheusRule' "${render_dir}/default.yaml"
  grep -q 'ai-blaise-citus-overview.json' "${render_dir}/default.yaml"
  grep -q 'AiBlaiseCitusSidecarNotReady' "${render_dir}/default.yaml"
  if grep -q 'app.kubernetes.io/component: sidecar-' "${render_dir}/default.yaml"; then
    echo "values.yaml default render must not include alpha sidecar deployments" >&2
    exit 1
  fi
  if grep -q 'app.kubernetes.io/component: tools' "${render_dir}/default.yaml"; then
    echo "values.yaml default render must not include alpha tools deployment" >&2
    exit 1
  fi
  grep -q 'kind: NetworkPolicy' "${render_dir}/exhaustive.yaml"
  grep -q 'AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST' "${render_dir}/exhaustive.yaml"
  grep -q '10.0.0.0/8' "${render_dir}/exhaustive.yaml"
  grep -q 'app.kubernetes.io/component: sidecar-analytical' "${render_dir}/exhaustive.yaml"
  grep -q 'kind: PrometheusRule' "${render_dir}/exhaustive.yaml"
  grep -q 'AiBlaiseCitusSidecarNotReady' "${render_dir}/exhaustive.yaml"
  grep -q 'kind: Deployment' "${render_dir}/dev.yaml"
  grep -q 'app.kubernetes.io/component: sidecar-mcp' "${render_dir}/dev.yaml"
  grep -q 'app.kubernetes.io/component: tools' "${render_dir}/dev.yaml"

  grep -q 'name: ai-blaise-citus-operator' "${render_dir}/prod.yaml"
  grep -q 'name: ai-blaise-citus-pool' "${render_dir}/prod.yaml"
  grep -q 'kind: ServiceMonitor' "${render_dir}/prod.yaml"
  grep -q 'kind: ConfigMap' "${render_dir}/prod.yaml"
  grep -q 'kind: PrometheusRule' "${render_dir}/prod.yaml"
  grep -q 'ai-blaise-citus-overview.json' "${render_dir}/prod.yaml"
  grep -q 'AiBlaiseCitusSidecarNotReady' "${render_dir}/prod.yaml"
  grep -q 'ai_blaise_sidecar_ready' "${render_dir}/prod.yaml"
  if grep -q 'app.kubernetes.io/component: sidecar-' "${render_dir}/prod.yaml"; then
    echo "values-prod.yaml render must not include alpha sidecar deployments" >&2
    exit 1
  fi
  if grep -q 'app.kubernetes.io/component: tools' "${render_dir}/prod.yaml"; then
    echo "values-prod.yaml render must not include alpha tools deployment" >&2
    exit 1
  fi

  if MODE=template scripts/citus-scale/deploy.sh >"${render_dir}/deploy-wrapper-missing-digest.yaml" 2>"${render_dir}/deploy-wrapper-missing-digest.err"; then
    echo "deploy.sh default production render must require immutable operator/pool image digests" >&2
    exit 1
  fi
  grep -q 'requires an immutable digest' "${render_dir}/deploy-wrapper-missing-digest.err"

  OPERATOR_IMAGE_DIGEST=sha256:1111111111111111111111111111111111111111111111111111111111111111 \
    POOL_IMAGE_DIGEST=sha256:2222222222222222222222222222222222222222222222222222222222222222 \
    MODE=template scripts/citus-scale/deploy.sh >"${render_dir}/deploy-wrapper-default.yaml"
  DEPLOY_PROFILE=dev MODE=template scripts/citus-scale/deploy.sh >"${render_dir}/deploy-wrapper-dev.yaml"
  grep -q 'name: ai-blaise-citus-operator' "${render_dir}/deploy-wrapper-default.yaml"
  grep -q 'name: ai-blaise-citus-pool' "${render_dir}/deploy-wrapper-default.yaml"
  if grep -q 'app.kubernetes.io/component: sidecar-' "${render_dir}/deploy-wrapper-default.yaml"; then
    echo "deploy.sh default render must use production values without alpha sidecars" >&2
    exit 1
  fi
  grep -q 'app.kubernetes.io/component: sidecar-mcp' "${render_dir}/deploy-wrapper-dev.yaml"

  if DEPLOY_PROFILE=dev MODE=install scripts/citus-scale/deploy.sh >"${render_dir}/deploy-wrapper-install.out" 2>"${render_dir}/deploy-wrapper-install.err"; then
    echo "deploy.sh must refuse non-production installs unless ALLOW_ALPHA_INSTALL=1" >&2
    exit 1
  fi
  grep -q 'refusing to install non-production values file' "${render_dir}/deploy-wrapper-install.err"
else
  if [[ "${require_helm}" == "1" ]]; then
    echo "helm is required for rendered chart profile checks" >&2
    exit 1
  fi
  echo "helm unavailable; skipping rendered chart profile checks"
fi
