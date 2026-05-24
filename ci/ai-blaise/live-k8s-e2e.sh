#!/usr/bin/env bash
set -euo pipefail

# FEATURE: D13
# FEATURE: D8

PATH="${HOME}/.local/bin:${PATH}"

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

mode="${LIVE_K8S_MODE:-dry-run}"
release_name="${RELEASE_NAME:-ai-blaise-citus-e2e}"
namespace="${NAMESPACE:-ai-blaise-citus-e2e}"
rollout_timeout="${ROLLOUT_TIMEOUT:-300s}"
teardown="${TEARDOWN:-1}"
keep_namespace_on_failure="${KEEP_NAMESPACE_ON_FAILURE:-0}"
require_chart="${REQUIRE_CHART:-0}"
require_http="${REQUIRE_HTTP:-0}"
require_sql="${REQUIRE_SQL:-0}"
check_image_published="${CHECK_IMAGE_PUBLISHED:-1}"
allow_unpublished_images="${ALLOW_UNPUBLISHED_IMAGES:-0}"
production_values_strict="${PRODUCTION_VALUES_STRICT:-0}"
artifact_dir="${ARTIFACT_DIR:-artifacts/live-k8s-e2e/$(date -u +%Y%m%dT%H%M%SZ)}"

rendered_manifest="${artifact_dir}/rendered.yaml"
rendered_images="${artifact_dir}/images.txt"
services_json="${artifact_dir}/services.json"
helm_lint_log="${artifact_dir}/helm-lint.log"

kubectl_cmd=(kubectl)
helm_cmd=(helm)
helm_value_args=()
helm_extra_args=()
pf_pids=()
ran_install=0
created_namespace=0
created_kind_cluster=0
chart_dir=""

usage() {
  cat <<'USAGE'
usage: ci/ai-blaise/live-k8s-e2e.sh

Modes:
  LIVE_K8S_MODE=dry-run  Render/lint when a chart is supplied. No live traffic.
  LIVE_K8S_MODE=real     Install into the current kube context and run traffic.
  LIVE_K8S_MODE=kind     Create/use a kind cluster, install, and run traffic.

Required for real/kind:
  CHART_DIR=/path/to/command-center/helm/charts/citus-cluster
    or COMMAND_CENTER_DIR=/path/to/command-center

Image handling:
  AI_BLAISE_STACK_IMAGE_REF=repo/name:tag       Override simple charts that use image.repository/tag.
  HELM_SET_ARGS='--set-string key=value ...'    Chart-specific image/value overrides.
  LOCAL_IMAGE_REFS='repo/name:tag ...'          Images to load into kind before install.
  ALLOW_UNPUBLISHED_IMAGES=1                    Skip docker manifest/local image preflight.
  PRODUCTION_VALUES_STRICT=1                    Reject mutable/latest image refs, placeholder/local production images, alpha sidecar leaks, and imagePullPolicy Always before install.

Traffic handling:
  REQUIRE_HTTP=1 requires at least one HTTP probe target.
  REQUIRE_SQL=1 requires a PostgreSQL service target and successful psql traffic.
  HTTP_SERVICE/HTTP_PORT and SQL_SERVICE/SQL_PORT can pin service selection.
  HTTP_PATHS defaults to '/healthz /readyz /metrics'.
  SQL_TEST_FILE or SQL_TEST_SQL can override the default write/read SQL.

Dry-run mode is a contract smoke only. It never sends live HTTP or SQL traffic
and must not be cited as runtime evidence.
USAGE
}

log() {
  printf '[live-k8s-e2e] %s\n' "$*" >&2
}

die() {
  log "ERROR: $*"
  exit 1
}

need_cmd() {
  local command_name="$1"
  command -v "${command_name}" >/dev/null 2>&1 || die "missing required command: ${command_name}"
}

normalize_words() {
  local raw="$1"
  raw="${raw//,/ }"
  printf '%s\n' "${raw}"
}

split_words() {
  local raw="$1"
  local -n out_ref="$2"
  out_ref=()
  raw="$(normalize_words "${raw}")"
  if [[ -n "${raw// }" ]]; then
    # shellcheck disable=SC2034 # out_ref is a nameref assigned for the caller.
    read -r -a out_ref <<<"${raw}"
  fi
}

find_chart_dir() {
  local candidate
  if [[ -n "${CHART_DIR:-}" ]]; then
    printf '%s\n' "${CHART_DIR}"
    return 0
  fi

  if [[ -n "${COMMAND_CENTER_DIR:-}" ]]; then
    candidate="${COMMAND_CENTER_DIR}/helm/charts/citus-cluster"
    if [[ -d "${candidate}" ]]; then
      printf '%s\n' "${candidate}"
      return 0
    fi
  fi

  for candidate in \
    "${repo_root}/../command-center/helm/charts/citus-cluster" \
    "${repo_root}/../../command-center/helm/charts/citus-cluster" \
    "${repo_root}/../cc/command-center/helm/charts/citus-cluster"; do
    if [[ -d "${candidate}" ]]; then
      printf '%s\n' "${candidate}"
      return 0
    fi
  done

  return 1
}

setup_kube_commands() {
  if [[ -n "${KUBECTL_CONTEXT:-}" ]]; then
    kubectl_cmd=(kubectl --context "${KUBECTL_CONTEXT}")
    helm_cmd=(helm --kube-context "${KUBECTL_CONTEXT}")
  fi
}

kube() {
  "${kubectl_cmd[@]}" "$@"
}

helm_run() {
  "${helm_cmd[@]}" "$@"
}

load_value_args() {
  local value_files=()
  local value_file
  split_words "${VALUES_FILES:-}" value_files

  if [[ ${#value_files[@]} -eq 0 && -f "${chart_dir}/values-prod.yaml" ]]; then
    value_files=("${chart_dir}/values-prod.yaml")
  fi

  for value_file in "${value_files[@]}"; do
    [[ -f "${value_file}" ]] || die "values file does not exist: ${value_file}"
    helm_value_args+=(--values "${value_file}")
  done
}

append_stack_image_override() {
  local image_ref="${AI_BLAISE_STACK_IMAGE_REF:-}"
  local repository tag

  if [[ -z "${image_ref}" ]]; then
    return 0
  fi

  if [[ "${image_ref}" == *@* ]]; then
    die "AI_BLAISE_STACK_IMAGE_REF currently requires a tag ref; use HELM_SET_ARGS for digest-valued charts: ${image_ref}"
  fi

  repository="${image_ref%:*}"
  tag="${image_ref##*:}"
  if [[ "${repository}" == "${image_ref}" || "${tag}" == */* || -z "${repository}" || -z "${tag}" ]]; then
    die "AI_BLAISE_STACK_IMAGE_REF must be repo/name:tag: ${image_ref}"
  fi

  helm_extra_args+=(
    --set-string "image.repository=${repository}"
    --set-string "image.tag=${tag}"
    --set-string "workload.image.repository=${repository}"
    --set-string "workload.image.tag=${tag}"
  )
}

append_set_args() {
  local args=()
  append_stack_image_override
  split_words "${HELM_SET_ARGS:-}" args
  if [[ ${#args[@]} -gt 0 ]]; then
    helm_extra_args+=("${args[@]}")
  fi
}

prepare_chart() {
  mkdir -p "${artifact_dir}"
  if ! chart_dir="$(find_chart_dir)"; then
    if [[ "${require_chart}" == "1" || "${mode}" != "dry-run" ]]; then
      die "command-center chart not found; set CHART_DIR or COMMAND_CENTER_DIR"
    fi
    chart_dir=""
    return 0
  fi

  [[ -f "${chart_dir}/Chart.yaml" ]] || die "CHART_DIR is missing Chart.yaml: ${chart_dir}"
  load_value_args
  append_set_args
}

render_chart() {
  [[ -n "${chart_dir}" ]] || return 0
  need_cmd helm

  if ! helm lint "${chart_dir}" "${helm_value_args[@]}" "${helm_extra_args[@]}" >"${helm_lint_log}" 2>&1; then
    sed -n '1,220p' "${helm_lint_log}" >&2
    die "helm lint failed for ${chart_dir}"
  fi

  helm template "${release_name}" "${chart_dir}" \
    --namespace "${namespace}" \
    "${helm_value_args[@]}" \
    "${helm_extra_args[@]}" \
    >"${rendered_manifest}"

  awk '
    $1 == "image:" {
      image = $2
      gsub(/^"|"$/, "", image)
      if (image != "") print image
    }
  ' "${rendered_manifest}" | sort -u >"${rendered_images}"

  if [[ "${KUBECTL_CLIENT_DRY_RUN:-0}" == "1" ]]; then
    need_cmd kubectl
    kubectl apply --dry-run=client --validate=false -f "${rendered_manifest}" >/dev/null
  fi
}


validate_production_values_render() {
  [[ "${production_values_strict}" == "1" ]] || return 0
  [[ -n "${chart_dir}" ]] || return 0
  need_cmd python3
  python3 - "${rendered_manifest}" "${artifact_dir}/production-values-guardrails.txt" <<'PYSCRIPT'
import re
import sys
from pathlib import Path
manifest = Path(sys.argv[1])
report = Path(sys.argv[2])
text = manifest.read_text()
errors = []
images = re.findall(r'^\s*image:\s*["\']?([^"\'\s]+)', text, flags=re.MULTILINE)
if not images:
    errors.append('rendered chart contains no images')
for image in images:
    if ':latest' in image or image.endswith(':latest'):
        errors.append(f'latest image ref is not allowed in production values: {image}')
    if not re.search(r'@sha256:[a-f0-9]{64}$', image):
        errors.append(f'image must be pinned by immutable sha256 digest in production values: {image}')
    if image.startswith('example.invalid/') or image.startswith('localhost/'):
        errors.append(f'placeholder/local image is not production-values evidence: {image}')
alpha_true = re.findall(r'(?im)^\s*[^#\n]*(?:alpha|experimental|preview)[^:\n]*:\s*["\']?(?:true|enabled)["\']?\s*$', text)
for line in alpha_true:
    errors.append(f'alpha/experimental toggle enabled in production values: {line.strip()}')
if re.search(r'(?im)^\s*imagePullPolicy:\s*Always\s*$', text):
    errors.append('imagePullPolicy Always is rejected for strict production-values evidence')
if re.search(r'(?i)(sidecar-(analytical|auth|backup|cdc|coldtier|edge-functions|graphql|hlc|mcp|postgrest|raft|realtime|repack|schema-job|storage|txn-status|vectorizer))', text):
    errors.append('alpha sidecar workload rendered in production values')
report.parent.mkdir(parents=True, exist_ok=True)
if errors:
    report.write_text('status=failed\n' + '\n'.join(errors) + '\n')
    for error in errors:
        print(error, file=sys.stderr)
    sys.exit(1)
report.write_text('status=ok\nimages=' + ','.join(sorted(set(images))) + '\nmutable_images=false\nalpha_sidecars=false\n')
PYSCRIPT
}

check_rendered_images() {
  local image

  [[ -n "${chart_dir}" ]] || return 0
  if [[ ! -s "${rendered_images}" ]]; then
    die "rendered chart contains no container images"
  fi

  log "rendered images:"
  sed 's/^/[live-k8s-e2e]   /' "${rendered_images}" >&2

  if [[ "${mode}" == "dry-run" ]]; then
    if [[ -z "${AI_BLAISE_STACK_IMAGE_REF:-}${HELM_SET_ARGS:-}" ]]; then
      log "dry-run used chart defaults; real mode should pass published release images or locally built image refs"
    fi
    return 0
  fi

  if [[ "${allow_unpublished_images}" == "1" || "${check_image_published}" != "1" ]]; then
    log "image publication preflight disabled by env; live rollout may still fail if the cluster cannot pull the images"
    return 0
  fi

  need_cmd docker
  while IFS= read -r image; do
    [[ -n "${image}" ]] || continue
    if docker image inspect "${image}" >/dev/null 2>&1; then
      continue
    fi
    if docker manifest inspect "${image}" >/dev/null 2>&1; then
      continue
    fi
    die "image is not available locally or as a published manifest: ${image}; pass AI_BLAISE_STACK_IMAGE_REF/HELM_SET_ARGS, LOCAL_IMAGE_REFS for kind, or ALLOW_UNPUBLISHED_IMAGES=1 after preloading images"
  done <"${rendered_images}"
}

ensure_kind_cluster() {
  local cluster_name="${KIND_CLUSTER_NAME:-ai-blaise-citus-e2e}"
  local local_images=()
  local image

  need_cmd kind
  need_cmd docker

  if ! kind get clusters | grep -Fxq "${cluster_name}"; then
    log "creating kind cluster ${cluster_name}"
    kind create cluster --name "${cluster_name}"
    created_kind_cluster=1
  fi

  KUBECTL_CONTEXT="kind-${cluster_name}"
  setup_kube_commands

  split_words "${LOCAL_IMAGE_REFS:-}" local_images
  for image in "${local_images[@]}"; do
    docker image inspect "${image}" >/dev/null 2>&1 || die "LOCAL_IMAGE_REFS image is not present locally: ${image}"
    log "loading ${image} into kind/${cluster_name}"
    kind load docker-image --name "${cluster_name}" "${image}"
  done
}

verify_cluster_access() {
  need_cmd kubectl
  kube version --request-timeout=10s >/dev/null
}

install_release() {
  need_cmd helm
  if kube get namespace "${namespace}" >/dev/null 2>&1; then
    created_namespace=0
  else
    created_namespace=1
  fi

  log "installing release=${release_name} namespace=${namespace} chart=${chart_dir}"
  ran_install=1
  helm_run upgrade --install "${release_name}" "${chart_dir}" \
    --namespace "${namespace}" \
    --create-namespace \
    --wait \
    --timeout "${rollout_timeout}" \
    "${helm_value_args[@]}" \
    "${helm_extra_args[@]}"
}

selector_resources() {
  local resources
  resources="$(kube -n "${namespace}" get deploy,statefulset,daemonset \
    -l "app.kubernetes.io/instance=${release_name}" \
    -o name 2>/dev/null || true)"
  if [[ -z "${resources}" ]]; then
    resources="$(kube -n "${namespace}" get deploy,statefulset,daemonset -o name 2>/dev/null || true)"
  fi
  printf '%s\n' "${resources}"
}

wait_for_rollout() {
  local resource
  local resources

  resources="$(selector_resources)"
  [[ -n "${resources}" ]] || die "no Deployment, StatefulSet, or DaemonSet resources were installed"

  while IFS= read -r resource; do
    [[ -n "${resource}" ]] || continue
    log "waiting for rollout: ${resource}"
    kube -n "${namespace}" rollout status "${resource}" --timeout "${rollout_timeout}"
  done <<<"${resources}"

  if kube -n "${namespace}" get pods -l "app.kubernetes.io/instance=${release_name}" >/dev/null 2>&1; then
    kube -n "${namespace}" wait pod \
      -l "app.kubernetes.io/instance=${release_name}" \
      --for=condition=Ready \
      --timeout "${rollout_timeout}"
  else
    kube -n "${namespace}" wait pod --all --for=condition=Ready --timeout "${rollout_timeout}"
  fi
}

write_services_json() {
  need_cmd jq
  if kube -n "${namespace}" get svc -l "app.kubernetes.io/instance=${release_name}" -o json >"${services_json}" 2>/dev/null; then
    if jq -e '.items | length > 0' "${services_json}" >/dev/null; then
      return 0
    fi
  fi
  kube -n "${namespace}" get svc -o json >"${services_json}"
}

choose_port() {
  local port
  for port in $(seq "${PORT_FORWARD_START:-18080}" 65000); do
    if ! (: >/dev/tcp/127.0.0.1/"${port}") >/dev/null 2>&1; then
      printf '%s\n' "${port}"
      return 0
    fi
  done
  die "could not find a free localhost port for port-forward"
}

stop_port_forwards() {
  local pid
  for pid in "${pf_pids[@]}"; do
    if kill -0 "${pid}" >/dev/null 2>&1; then
      kill "${pid}" >/dev/null 2>&1 || true
      wait "${pid}" >/dev/null 2>&1 || true
    fi
  done
  pf_pids=()
}

start_port_forward() {
  local resource="$1"
  local remote_port="$2"
  local local_port
  local pid
  local log_file
  local _attempt

  local_port="$(choose_port)"
  log_file="${artifact_dir}/port-forward-${resource//\//-}-${remote_port}.log"
  log "kubectl port-forward ${resource} ${local_port}:${remote_port}"
  kube -n "${namespace}" port-forward "${resource}" "127.0.0.1:${local_port}:${remote_port}" \
    >"${log_file}" 2>&1 &
  pid="$!"
  pf_pids+=("${pid}")

  for _attempt in $(seq 1 75); do
    if ! kill -0 "${pid}" >/dev/null 2>&1; then
      sed -n '1,220p' "${log_file}" >&2 || true
      die "port-forward process exited early for ${resource}:${remote_port}"
    fi
    if (: >/dev/tcp/127.0.0.1/"${local_port}") >/dev/null 2>&1; then
      printf '%s\n' "${local_port}"
      return 0
    fi
    sleep 0.2
  done

  sed -n '1,220p' "${log_file}" >&2 || true
  die "timed out waiting for port-forward ${resource}:${remote_port}"
}

http_targets() {
  if [[ -n "${HTTP_SERVICE:-}" ]]; then
    printf '%s\t%s\n' "${HTTP_SERVICE}" "${HTTP_PORT:-8080}"
    return 0
  fi

  jq -r '
    .items[] as $svc
    | $svc.spec.ports[]?
    | select((.port != 5432) and (((.name // "") | test("http|admin|metrics|web"; "i")) or (.port == 8080) or (.port == 9090)))
    | [$svc.metadata.name, (.port | tostring)]
    | @tsv
  ' "${services_json}" | sort -u
}

sql_targets() {
  if [[ -n "${SQL_SERVICE:-}" ]]; then
    printf '%s\t%s\n' "${SQL_SERVICE}" "${SQL_PORT:-5432}"
    return 0
  fi

  jq -r '
    .items[] as $svc
    | $svc.spec.ports[]?
    | select((.port == 5432) or (((.name // "") | test("postgres|pgsql|sql|pool|pg"; "i"))))
    | [$svc.metadata.name, (.port | tostring)]
    | @tsv
  ' "${services_json}" | sort -u
}

run_http_traffic() {
  local targets
  local service port local_port path
  local paths=()

  need_cmd curl
  split_words "${HTTP_PATHS:-/healthz /readyz /metrics}" paths
  targets="$(http_targets)"

  if [[ -z "${targets}" ]]; then
    [[ "${require_http}" == "1" ]] && die "no HTTP service target found; set HTTP_SERVICE/HTTP_PORT or expose http/admin/metrics service ports"
    log "no HTTP service target found; HTTP traffic not required in this mode"
    return 0
  fi

  while IFS=$'\t' read -r service port; do
    [[ -n "${service}" && -n "${port}" ]] || continue
    local_port="$(start_port_forward "svc/${service}" "${port}")"
    for path in "${paths[@]}"; do
      log "HTTP GET svc/${service}:${port}${path}"
      curl -fsS --max-time "${HTTP_TIMEOUT_SECS:-10}" "http://127.0.0.1:${local_port}${path}" \
        >"${artifact_dir}/http-${service}-${port}-${path//\//_}.out"
    done
  done <<<"${targets}"
}

write_default_sql() {
  local file="$1"
  cat >"${file}" <<'SQL'
SELECT version();
CREATE TABLE IF NOT EXISTS ai_blaise_live_k8s_e2e (
  id integer PRIMARY KEY,
  note text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);
INSERT INTO ai_blaise_live_k8s_e2e (id, note)
VALUES (1, 'live-k8s-e2e')
ON CONFLICT (id) DO UPDATE SET note = EXCLUDED.note;
SELECT note FROM ai_blaise_live_k8s_e2e WHERE id = 1;
SQL
}

run_sql_traffic() {
  local targets
  local service port local_port sql_file

  need_cmd psql
  targets="$(sql_targets)"
  if [[ -z "${targets}" ]]; then
    [[ "${require_sql}" == "1" ]] && die "no SQL service target found; set SQL_SERVICE/SQL_PORT or expose a PostgreSQL service port"
    log "no SQL service target found; SQL traffic not required in this mode"
    return 0
  fi

  sql_file="${SQL_TEST_FILE:-${artifact_dir}/sql-test.sql}"
  if [[ -n "${SQL_TEST_SQL:-}" ]]; then
    printf '%s\n' "${SQL_TEST_SQL}" >"${sql_file}"
  elif [[ -z "${SQL_TEST_FILE:-}" ]]; then
    write_default_sql "${sql_file}"
  fi
  [[ -s "${sql_file}" ]] || die "SQL test file is empty: ${sql_file}"

  while IFS=$'\t' read -r service port; do
    [[ -n "${service}" && -n "${port}" ]] || continue
    local_port="$(start_port_forward "svc/${service}" "${port}")"
    log "psql traffic through svc/${service}:${port}"
    PGPASSWORD="${PGPASSWORD:-postgres}" psql \
      -h 127.0.0.1 \
      -p "${local_port}" \
      -U "${PGUSER:-postgres}" \
      -d "${PGDATABASE:-postgres}" \
      -v ON_ERROR_STOP=1 \
      -f "${sql_file}" \
      >"${artifact_dir}/sql-${service}-${port}.out"
    return 0
  done <<<"${targets}"
}

collect_diagnostics() {
  [[ "${ran_install}" == "1" ]] || return 0
  log "collecting failure diagnostics under ${artifact_dir}"
  kube -n "${namespace}" get all -o wide >"${artifact_dir}/kubectl-get-all.txt" 2>&1 || true
  kube -n "${namespace}" get events --sort-by=.lastTimestamp >"${artifact_dir}/kubectl-events.txt" 2>&1 || true
  kube -n "${namespace}" describe deploy,statefulset,daemonset,svc,pod >"${artifact_dir}/kubectl-describe.txt" 2>&1 || true
  kube -n "${namespace}" get pods -o name >"${artifact_dir}/pods.txt" 2>&1 || true
  while IFS= read -r pod; do
    [[ -n "${pod}" ]] || continue
    kube -n "${namespace}" logs "${pod}" --all-containers --tail="${LOG_TAIL_LINES:-200}" \
      >"${artifact_dir}/logs-${pod//\//-}.txt" 2>&1 || true
  done <"${artifact_dir}/pods.txt"
  helm_run -n "${namespace}" get all "${release_name}" >"${artifact_dir}/helm-get-all.txt" 2>&1 || true
}

cleanup() {
  local status="$?"
  set +e
  stop_port_forwards
  if [[ "${status}" -ne 0 ]]; then
    collect_diagnostics
  fi
  if [[ "${ran_install}" == "1" && "${teardown}" == "1" ]]; then
    if [[ "${status}" -ne 0 && "${keep_namespace_on_failure}" == "1" ]]; then
      log "keeping namespace ${namespace} after failure because KEEP_NAMESPACE_ON_FAILURE=1"
    else
      log "tearing down release=${release_name} namespace=${namespace}"
      helm_run -n "${namespace}" uninstall "${release_name}" >/dev/null 2>&1 || true
      if [[ "${created_namespace}" == "1" ]]; then
        kube delete namespace "${namespace}" --ignore-not-found --timeout=120s >/dev/null 2>&1 || true
      fi
    fi
  fi
  if [[ "${created_kind_cluster}" == "1" && "${DELETE_KIND_CLUSTER:-0}" == "1" ]]; then
    kind delete cluster --name "${KIND_CLUSTER_NAME:-ai-blaise-citus-e2e}" >/dev/null 2>&1 || true
  fi
  exit "${status}"
}
trap cleanup EXIT

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  usage
  exit 0
fi

case "${mode}" in
  dry-run|real|kind) ;;
  *) die "LIVE_K8S_MODE must be dry-run, real, or kind; got ${mode}" ;;
esac

setup_kube_commands
prepare_chart

if [[ -z "${chart_dir}" ]]; then
  log "dry-run chart lookup skipped: set CHART_DIR or COMMAND_CENTER_DIR to render the external command-center chart"
  printf 'live_k8s_e2e\tmode=%s\tchart=missing\ttraffic=not-run\tartifacts=%s\n' "${mode}" "${artifact_dir}"
  exit 0
fi

render_chart
validate_production_values_render
check_rendered_images

if [[ "${mode}" == "dry-run" ]]; then
  log "dry-run does not send live HTTP or SQL traffic"
  printf 'live_k8s_e2e\tmode=dry-run\tchart=%s\ttraffic=not-run\timages=%s\tartifacts=%s\n' \
    "${chart_dir}" \
    "$(wc -l <"${rendered_images}")" \
    "${artifact_dir}"
  exit 0
fi

if [[ "${mode}" == "kind" ]]; then
  ensure_kind_cluster
fi

verify_cluster_access
install_release
wait_for_rollout
write_services_json
run_http_traffic
run_sql_traffic

printf 'live_k8s_e2e\tmode=%s\tchart=%s\tnamespace=%s\trelease=%s\thttp_required=%s\tsql_required=%s\tartifacts=%s\n' \
  "${mode}" \
  "${chart_dir}" \
  "${namespace}" \
  "${release_name}" \
  "${require_http}" \
  "${require_sql}" \
  "${artifact_dir}"
