#!/usr/bin/env bash
set -euo pipefail

mode="${1:-local}"
shift || true

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "${repo_root}"

failures=0
cleanup_paths=()

cleanup() {
  local path
  for path in "${cleanup_paths[@]}"; do
    rm -rf "${path}"
  done
}
trap cleanup EXIT

log() {
  printf '[env-preflight] %s\n' "$*"
}

record_failure() {
  printf '[env-preflight] error: %s\n' "$*" >&2
  failures=$((failures + 1))
}

require_command() {
  local tool="$1"
  local purpose="$2"

  if command -v "${tool}" >/dev/null 2>&1; then
    log "ok: ${tool} ($(command -v "${tool}"))"
    return 0
  fi

  record_failure "missing ${tool}: ${purpose}"
  return 1
}

optional_command() {
  local tool="$1"
  local purpose="$2"

  if command -v "${tool}" >/dev/null 2>&1; then
    log "ok: ${tool} ($(command -v "${tool}"))"
  else
    log "exploratory-only: ${tool} missing (${purpose})"
  fi
}

require_file() {
  local path="$1"
  local purpose="$2"

  if [[ -s "${path}" ]]; then
    log "ok: ${path}"
    return 0
  fi

  record_failure "missing ${path}: ${purpose}"
  return 1
}

require_executable() {
  local path="$1"
  local purpose="$2"

  if [[ -x "${path}" ]]; then
    log "ok: ${path}"
    return 0
  fi

  if [[ -e "${path}" ]]; then
    record_failure "${path} is not executable: ${purpose}"
  else
    record_failure "missing ${path}: ${purpose}"
  fi
  return 1
}

require_docker_daemon() {
  require_command docker "Docker-backed release smokes" || return 0
  if docker info >/dev/null 2>&1; then
    log "ok: docker daemon reachable"
  else
    record_failure "docker is installed but the daemon is not reachable"
  fi
}

check_client_versions() {
  if command -v helm >/dev/null 2>&1; then
    helm version --short >/dev/null 2>&1 || record_failure "helm is installed but 'helm version --short' failed"
  fi
  if command -v kubectl >/dev/null 2>&1; then
    kubectl version --client >/dev/null 2>&1 || record_failure "kubectl is installed but 'kubectl version --client' failed"
  fi
  if command -v cargo >/dev/null 2>&1; then
    cargo --version >/dev/null 2>&1 || record_failure "cargo is installed but 'cargo --version' failed"
  fi
  if command -v rustfmt >/dev/null 2>&1; then
    rustfmt --version >/dev/null 2>&1 || record_failure "rustfmt is installed but 'rustfmt --version' failed"
  fi
  if command -v psql >/dev/null 2>&1; then
    psql --version >/dev/null 2>&1 || record_failure "psql is installed but 'psql --version' failed"
  fi
  if command -v shellcheck >/dev/null 2>&1; then
    shellcheck --version >/dev/null 2>&1 || record_failure "shellcheck is installed but 'shellcheck --version' failed"
  fi
}

check_makefile_targets() {
  local makefile="Makefile.ai-blaise"
  local tmpdir
  tmpdir="$(mktemp -d)"
  cleanup_paths+=("${tmpdir}")

  awk '/^\.PHONY:/ { for (i = 2; i <= NF; i++) print $i }' "${makefile}" | sort -u >"${tmpdir}/phony"
  awk '
    /^[^#[:space:].][^:]*:/ {
      target = $1
      sub(/:.*/, "", target)
      if (target !~ /[$()]/) print target
    }
  ' "${makefile}" | sort -u >"${tmpdir}/rules"

  local missing
  missing="$(comm -23 "${tmpdir}/phony" "${tmpdir}/rules" | tr '\n' ' ' | sed 's/[[:space:]]*$//')"
  if [[ -n "${missing}" ]]; then
    record_failure "${makefile} declares .PHONY targets without recipes: ${missing}"
  else
    log "ok: ${makefile} .PHONY targets have recipes"
  fi
}

require_release_toolchain() {
  require_command python3 "release checks and benchmark result validation" || true
  require_command make "release gate orchestration" || true
  require_command git "patch and upstream sync checks" || true
  require_command cargo "Rust workspace checks and canonical runners" || true
  require_command rustfmt "cargo fmt parity with GitHub workflows" || true
  require_command docker "Docker-backed release smokes and image builds" || true
  require_command helm "rendered Helm release checks" || true
  require_command kubectl "Kubernetes dry-run and kind production smoke" || true
  require_command kind "local Kubernetes production smoke" || true
  require_command jq "license metadata and JSON contract checks" || true
  require_command psql "SQL, benchmark, and extension smokes" || true
  require_command black "Python style checks" || true
  require_command shellcheck "shell script linting" || true
  require_docker_daemon
  check_client_versions
}

assert_measured_results() {
  if [[ "$#" -eq 0 ]]; then
    record_failure "assert-measured-results requires at least one JSON path or directory"
    return
  fi
  require_command python3 "benchmark result JSON validation" || return
  python3 - "$@" <<'PY'
import json
import pathlib
import sys

inputs = [pathlib.Path(arg) for arg in sys.argv[1:]]
paths: list[pathlib.Path] = []
for item in inputs:
    if item.is_dir():
        paths.extend(sorted(item.glob("*.json")))
    else:
        paths.append(item)

if not paths:
    print("[env-preflight] error: no benchmark JSON files matched", file=sys.stderr)
    sys.exit(1)

scaffolds: list[str] = []
checked = 0


def visit(path: pathlib.Path, value, label: str) -> None:
    global checked
    if isinstance(value, dict):
        mode = str(value.get("mode", ""))
        note = str(value.get("note", ""))
        name = value.get("ext") or value.get("workload") or value.get("harness") or label
        if mode or note:
            checked += 1
        if mode == "scaffold" or "scaffold" in note.lower():
            scaffolds.append(f"{path}: {name}: mode={mode or '<unset>'} note={note or '<unset>'}")
        for key in ("results", "entries"):
            nested = value.get(key)
            if isinstance(nested, list):
                for index, item in enumerate(nested):
                    visit(path, item, f"{label}.{key}[{index}]")
    elif isinstance(value, list):
        for index, item in enumerate(value):
            visit(path, item, f"{label}[{index}]")


for path in paths:
    try:
        data = json.loads(path.read_text())
    except FileNotFoundError:
        print(f"[env-preflight] error: missing benchmark result {path}", file=sys.stderr)
        sys.exit(1)
    except json.JSONDecodeError as exc:
        print(f"[env-preflight] error: invalid JSON in {path}: {exc}", file=sys.stderr)
        sys.exit(1)
    visit(path, data, path.name)

if checked == 0:
    print("[env-preflight] error: benchmark results did not expose mode or note fields", file=sys.stderr)
    sys.exit(1)

if scaffolds:
    print("[env-preflight] error: release benchmark evidence contains scaffold results", file=sys.stderr)
    for item in scaffolds:
        print(f"  - {item}", file=sys.stderr)
    sys.exit(1)

print(f"[env-preflight] ok: benchmark results contain no scaffold evidence ({checked} records checked)")
PY
}

case "${mode}" in
  local)
    log "local exploratory preflight: missing release-only tools are reported but do not fail"
    optional_command cargo "Rust workspace targets will fail without it"
    optional_command docker "Docker-backed smokes require it"
    optional_command helm "rendered Helm checks require it"
    optional_command kubectl "Kubernetes smokes require it"
    optional_command jq "license metadata scan requires it"
    optional_command psql "SQL and benchmark smokes require it"
    optional_command rustfmt "cargo fmt parity requires it"
    optional_command black "Python style checks require it"
    optional_command shellcheck "shell lint checks require it"
    optional_command kind "kind production smoke requires it"
    ;;
  style)
    require_command black "Python style checks" || true
    require_command isort "Python import ordering checks" || true
    require_command flake8 "Python lint checks" || true
    optional_command citus_indent "Citus C indentation check"
    ;;
  release)
    log "release preflight: every release-mode dependency must be present before expensive jobs start"
    check_makefile_targets
    require_release_toolchain
    require_executable ci/ai-blaise/deploy-check.sh "Makefile deploy-check release evidence" || true
    require_executable ci/ai-blaise/kind-production-smoke.sh "Makefile kind-production-smoke release evidence" || true
    ;;
  deploy)
    log "deploy preflight: Helm deploy evidence must fail closed"
    require_command helm "rendering production Helm values" || true
    check_client_versions
    require_executable ci/ai-blaise/deploy-check.sh "deploy-check target" || true
    ;;
  kind-production)
    log "kind production preflight: live Kubernetes release evidence must fail closed"
    require_command docker "kind node runtime" || true
    require_command kind "kind cluster lifecycle" || true
    require_command kubectl "Kubernetes API interactions" || true
    require_command helm "Helm rendering and install" || true
    require_command psql "live SQL proof" || true
    require_docker_daemon
    check_client_versions
    require_executable ci/ai-blaise/kind-production-smoke.sh "kind-production-smoke target" || true
    ;;
  assert-measured-results)
    assert_measured_results "$@"
    ;;
  *)
    cat >&2 <<EOF
usage: $0 [local|style|release|deploy|kind-production|assert-measured-results <json-or-dir>...]
EOF
    exit 2
    ;;
esac

if [[ "${failures}" -ne 0 ]]; then
  log "failed (${failures} issue(s))"
  exit 1
fi

log "ok (${mode})"
