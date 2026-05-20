#!/usr/bin/env bash
set -euo pipefail

# FEATURE: D8

release="${RELEASE_NAME:-ai-blaise-citus}"
namespace="${NAMESPACE:-ai-blaise-citus}"
chart_dir="${CHART_DIR:-deploy/k8s/helm/citus-overlay}"
values_file="${VALUES_FILE:-${chart_dir}/values.yaml}"
mode="${MODE:-template}"

if [[ ! -s "${chart_dir}/Chart.yaml" ]]; then
  echo "missing chart: ${chart_dir}/Chart.yaml" >&2
  exit 1
fi

if [[ ! -s "${values_file}" ]]; then
  echo "missing values file: ${values_file}" >&2
  exit 1
fi

case "${mode}" in
  template)
    helm template "${release}" "${chart_dir}" \
      --namespace "${namespace}" \
      --values "${values_file}"
    ;;
  install)
    helm upgrade --install "${release}" "${chart_dir}" \
      --namespace "${namespace}" \
      --create-namespace \
      --values "${values_file}"
    ;;
  *)
    echo "MODE must be template or install" >&2
    exit 1
    ;;
esac
