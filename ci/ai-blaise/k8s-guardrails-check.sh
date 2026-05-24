#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

renderer="deploy/contracts/render_k8s_guardrails.py"
manifest="deploy/contracts/k8s-production-guardrails.yaml"
contract_dir="deploy/contracts"

for file in "${renderer}" "${manifest}" "${contract_dir}/kustomization.yaml"; do
  if [[ ! -s "${file}" ]]; then
    echo "missing Kubernetes guardrail contract artifact: ${file}" >&2
    exit 1
  fi
done

tmpdir="$(mktemp -d)"
trap 'rm -rf "${tmpdir}"' EXIT
rendered="${tmpdir}/k8s-production-guardrails.yaml"

python3 "${renderer}" --validate-only
python3 "${renderer}" --check-file "${manifest}" >"${rendered}"

schema_input="${rendered}"
if command -v kustomize >/dev/null 2>&1; then
  kustomize build "${contract_dir}" >"${tmpdir}/kustomize.yaml"
  for kind in HorizontalPodAutoscaler PodDisruptionBudget NetworkPolicy; do
    grep -Fq "kind: ${kind}" "${tmpdir}/kustomize.yaml"
  done
  schema_input="${tmpdir}/kustomize.yaml"
else
  echo "kustomize not found; renderer and semantic guardrail validation completed" >&2
fi

if command -v kubeconform >/dev/null 2>&1; then
  kubeconform -strict -summary -kubernetes-version 1.30.0 "${schema_input}"
else
  echo "kubeconform not found; skipping optional Kubernetes schema validation" >&2
fi
