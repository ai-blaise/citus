#!/usr/bin/env bash
set -euo pipefail

# FEATURE: D8

release="${RELEASE_NAME:-ai-blaise-citus}"
namespace="${NAMESPACE:-ai-blaise-citus}"
chart_dir="${CHART_DIR:-deploy/k8s/helm/citus-overlay}"
deploy_profile="${DEPLOY_PROFILE:-prod}"
mode="${MODE:-template}"
allow_alpha_install="${ALLOW_ALPHA_INSTALL:-0}"

if [[ -n "${VALUES_FILE:-}" ]]; then
  values_file="${VALUES_FILE}"
else
  case "${deploy_profile}" in
    prod|production)
      values_file="${chart_dir}/values-prod.yaml"
      ;;
    dev)
      values_file="${chart_dir}/values-dev.yaml"
      ;;
    exhaustive|default)
      values_file="${chart_dir}/values.yaml"
      ;;
    *)
      echo "DEPLOY_PROFILE must be prod, dev, exhaustive, or default" >&2
      exit 1
      ;;
  esac
fi

if [[ ! -s "${chart_dir}/Chart.yaml" ]]; then
  echo "missing chart: ${chart_dir}/Chart.yaml" >&2
  exit 1
fi

if [[ ! -s "${values_file}" ]]; then
  echo "missing values file: ${values_file}" >&2
  exit 1
fi

if [[ "${mode}" == "install" && "${allow_alpha_install}" != "1" ]]; then
  prod_values="${chart_dir}/values-prod.yaml"
  values_file_abs="$(cd "$(dirname "${values_file}")" && pwd -P)/$(basename "${values_file}")"
  prod_values_abs="$(cd "$(dirname "${prod_values}")" && pwd -P)/$(basename "${prod_values}")"
  if [[ "${values_file_abs}" != "${prod_values_abs}" ]]; then
    echo "refusing to install non-production values file ${values_file}; set ALLOW_ALPHA_INSTALL=1 for dev/exhaustive/custom installs" >&2
    exit 1
  fi
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
