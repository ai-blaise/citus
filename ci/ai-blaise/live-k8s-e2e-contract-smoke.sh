#!/usr/bin/env bash
set -euo pipefail

# FEATURE: D13

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT
mkdir -p "${tmp_dir}/bin" "${tmp_dir}/chart"
printf 'apiVersion: v2\nname: fake-citus-cluster\nversion: 0.1.0\n' >"${tmp_dir}/chart/Chart.yaml"

cat >"${tmp_dir}/bin/helm" <<'SH_HELM'
#!/usr/bin/env bash
set -euo pipefail
printf 'helm\t%s\n' "$*" >>"${FAKE_LOG}"
case "${1:-}" in
  lint)
    exit 0
    ;;
  template)
    cat <<'YAML'
apiVersion: apps/v1
kind: Deployment
metadata:
  name: fake-citus
spec:
  template:
    spec:
      containers:
        - name: fake-citus
          image: example.invalid/ai-blaise/fake-citus:contract
YAML
    ;;
  upgrade|uninstall|get)
    exit 0
    ;;
  *)
    echo "unexpected fake helm invocation: $*" >&2
    exit 2
    ;;
esac
SH_HELM
chmod +x "${tmp_dir}/bin/helm"

cat >"${tmp_dir}/bin/kubectl" <<'SH_KUBECTL'
#!/usr/bin/env bash
set -euo pipefail
namespace=""
if [[ "${1:-}" == "-n" ]]; then
  namespace="$2"
  shift 2
fi
printf 'kubectl\t%s\t%s\n' "${namespace}" "$*" >>"${FAKE_LOG}"
case "${1:-}" in
  version)
    exit 0
    ;;
  get)
    case "${2:-}" in
      namespace)
        [[ "${FAKE_NAMESPACE_EXISTS:-0}" == "1" ]]
        ;;
      deploy,statefulset,daemonset)
        echo 'deployment.apps/fake-citus'
        ;;
      pods)
        exit 0
        ;;
      svc)
        echo '{"items":[]}'
        ;;
      all|events)
        echo 'fake diagnostics'
        ;;
      *)
        echo '{}'
        ;;
    esac
    ;;
  rollout|wait|delete|describe|logs)
    exit 0
    ;;
  *)
    echo "unexpected fake kubectl invocation: $*" >&2
    exit 2
    ;;
esac
SH_KUBECTL
chmod +x "${tmp_dir}/bin/kubectl"

run_case() {
  local name="$1"
  local namespace_exists="$2"
  local expect_namespace_delete="$3"
  local log_file="${tmp_dir}/${name}.log"
  local output_file="${tmp_dir}/${name}.out"
  : >"${log_file}"

  FAKE_LOG="${log_file}" \
  FAKE_NAMESPACE_EXISTS="${namespace_exists}" \
  PATH="${tmp_dir}/bin:${PATH}" \
  LIVE_K8S_MODE=real \
  CHART_DIR="${tmp_dir}/chart" \
  CHECK_IMAGE_PUBLISHED=0 \
  REQUIRE_HTTP=0 \
  REQUIRE_SQL=0 \
  RELEASE_NAME="ai-blaise-${name}" \
  NAMESPACE="ai-blaise-${name}" \
  ARTIFACT_DIR="${tmp_dir}/artifacts-${name}" \
    bash ci/ai-blaise/live-k8s-e2e.sh >"${output_file}"

  if [[ "${expect_namespace_delete}" == "1" ]]; then
    grep -Fq $'kubectl\t\tdelete namespace' "${log_file}" || {
      echo "expected namespace deletion for ${name}" >&2
      cat "${log_file}" >&2
      exit 1
    }
  elif grep -Fq $'kubectl\t\tdelete namespace' "${log_file}"; then
    echo "pre-existing namespace was deleted for ${name}" >&2
    cat "${log_file}" >&2
    exit 1
  fi
}

run_case preexisting 1 0
run_case created 0 1

grep -Fq 'need_cmd jq' ci/ai-blaise/live-k8s-e2e.sh
printf 'live_k8s_e2e_contract_smoke\tstatus=ok\n'
