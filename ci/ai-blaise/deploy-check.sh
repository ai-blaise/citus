#!/usr/bin/env bash
set -euo pipefail

chart_dir="deploy/k8s/helm/citus-overlay"
required_files=(
  "${chart_dir}/Chart.yaml"
  "${chart_dir}/values.yaml"
  "${chart_dir}/values-dev.yaml"
  "${chart_dir}/values-prod.yaml"
  "${chart_dir}/templates/operator-deployment.yaml"
  "${chart_dir}/templates/operator-rbac.yaml"
  "${chart_dir}/templates/operator-service.yaml"
  "${chart_dir}/templates/operator-servicemonitor.yaml"
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
grep -q '^sidecars:$' "${chart_dir}/values.yaml"

if grep -R "{{" "${chart_dir}/crds"; then
  echo "crds/ files must be static Kubernetes YAML, not Helm templates" >&2
  exit 1
fi

crd_count="$(grep -c '^kind: CustomResourceDefinition$' "${chart_dir}/crds/ai-blaise-citus-crds.yaml")"
if [[ "${crd_count}" -ne 17 ]]; then
  echo "expected 17 CRDs, found ${crd_count}" >&2
  exit 1
fi
