#!/usr/bin/env bash
# FEATURE: A10 A11
# Build or verify a source-bound real-Citus image for companion integration tests.
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
base_lock="${repo_root}/images/citus-test-fixture/base-images.lock.tsv"
contract_check="${repo_root}/ci/ai-blaise/real-citus-test-fixture-contract.py"
context_builder="${repo_root}/ci/ai-blaise/materialize-real-citus-test-fixture.py"
pg_major="${CITUS_TEST_FIXTURE_PG_MAJOR:-17}"
image="${CITUS_TEST_FIXTURE_IMAGE:-}"
make_jobs="${CITUS_TEST_FIXTURE_MAKE_JOBS:-2}"
contract_only=0
fixture_context=""
fixture_tmp_parent="${TMPDIR:-/tmp}"
fixture_tmp_parent="${fixture_tmp_parent%/}"

fail() {
  echo "real-Citus test fixture: $*" >&2
  exit 1
}

usage() {
  cat <<'EOF'
usage: build-real-citus-test-fixture.sh [--pg-major MAJOR] [--image REF] [--contract-only]

Snapshots the exact selected checkout inputs into the locked PostgreSQL base.
The resulting image is test-only and is not a Bundle1 release operand.
EOF
}

cleanup() {
  if [[ -n "${fixture_context}" && -d "${fixture_context}" ]]; then
    case "${fixture_context}" in
      "${fixture_tmp_parent}"/ai-blaise-citus-fixture.*)
        rm -rf -- "${fixture_context}"
        ;;
      *)
        echo "real-Citus test fixture: refusing unexpected temporary path cleanup" >&2
        ;;
    esac
  fi
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

while [[ $# -gt 0 ]]; do
  case "$1" in
    --pg-major)
      [[ $# -ge 2 ]] || fail "--pg-major requires a value"
      pg_major="$2"
      shift 2
      ;;
    --image)
      [[ $# -ge 2 ]] || fail "--image requires a value"
      image="$2"
      shift 2
      ;;
    --contract-only)
      contract_only=1
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      fail "unknown argument: $1"
      ;;
  esac
done

if [[ "${contract_only}" == "1" ]]; then
  python3 "${contract_check}"
  exit 0
fi
python3 "${contract_check}" >&2

command -v docker >/dev/null 2>&1 || fail "docker is required"
[[ "${pg_major}" =~ ^(16|17|18)$ ]] || fail "PostgreSQL major must be 16, 17, or 18"
[[ "${make_jobs}" =~ ^[1-9][0-9]*$ ]] || fail "CITUS_TEST_FIXTURE_MAKE_JOBS must be a positive integer"
if ((make_jobs > 32)); then
  fail "CITUS_TEST_FIXTURE_MAKE_JOBS must not exceed 32"
fi

base_image="$({
  awk -F '\t' -v pg_major="${pg_major}" '
    NR > 1 && $1 == pg_major { count += 1; image = $2 }
    END { if (count == 1) print image; else exit 1 }
  ' "${base_lock}"
} || true)"
[[ -n "${base_image}" ]] || fail "PostgreSQL ${pg_major} has no unique locked fixture base image"

source_git_sha="$(git -C "${repo_root}" rev-parse --verify 'HEAD^{commit}')"
source_git_tree="$(git -C "${repo_root}" rev-parse --verify 'HEAD^{tree}')"
[[ "${source_git_sha}" =~ ^[0-9a-f]{40}$ ]] || fail "Git commit identity is not canonical SHA-1"
[[ "${source_git_tree}" =~ ^[0-9a-f]{40}$ ]] || fail "Git tree identity is not canonical SHA-1"
source_tree_state="clean"
if ! git -C "${repo_root}" diff --quiet --no-ext-diff HEAD -- ||
  [[ -n "$(git -C "${repo_root}" ls-files --others --exclude-standard)" ]]; then
  source_tree_state="dirty"
fi

fixture_context="$(mktemp -d "${fixture_tmp_parent}/ai-blaise-citus-fixture.XXXXXX")"
source_content_sha256="$(
  python3 "${context_builder}" \
    --source "${repo_root}" \
    --destination "${fixture_context}"
)"
[[ "${source_content_sha256}" =~ ^[0-9a-f]{64}$ ]] ||
  fail "fixture source content identity is not canonical SHA-256"
dockerfile="${fixture_context}/images/citus-test-fixture/Dockerfile"
[[ -s "${dockerfile}" ]] || fail "materialized fixture Dockerfile is missing"

citus_extension_version="$({
  awk -F "'" '
    /^default_version = / { count += 1; version = $2 }
    END { if (count == 1) print version; else exit 1 }
  ' "${fixture_context}/src/backend/distributed/citus.control"
} || true)"
[[ "${citus_extension_version}" =~ ^[0-9]+\.[0-9]+-[0-9]+$ ]] ||
  fail "Citus control default_version is not canonical"

fixture_identity="$(
  python3 -c \
    'import hashlib, sys; print(hashlib.sha256("\0".join(sys.argv[1:]).encode()).hexdigest())' \
    "${pg_major}" "${base_image}" "${citus_extension_version}" "${source_content_sha256}"
)"
[[ "${fixture_identity}" =~ ^[0-9a-f]{64}$ ]] || fail "fixture identity is not canonical SHA-256"

if [[ -z "${image}" ]]; then
  image="ai-blaise-citus-test-fixture:pg${pg_major}-${fixture_identity}"
fi
[[ "${image}" =~ ^[A-Za-z0-9][A-Za-z0-9._/@:-]*$ ]] || fail "fixture image reference is malformed"

read_label() {
  local label="$1"
  docker image inspect --format "{{ index .Config.Labels \"${label}\" }}" "${image_id}"
}

verify_label() {
  local label="$1"
  local expected="$2"
  local observed
  observed="$(read_label "${label}")"
  if [[ "${observed}" != "${expected}" ]]; then
    fail "fixture image label ${label} does not match the requested source"
  fi
}

if ! docker image inspect "${image}" >/dev/null 2>&1; then
  [[ "${image}" != sha256:* ]] || fail "requested immutable fixture image ID does not exist"
  docker build \
    --file "${dockerfile}" \
    --target companion-test-fixture \
    --build-arg "BASE_IMAGE=${base_image}" \
    --build-arg "CITUS_EXTENSION_VERSION=${citus_extension_version}" \
    --build-arg "PG_MAJOR=${pg_major}" \
    --build-arg "MAKE_JOBS=${make_jobs}" \
    --build-arg "AI_BLAISE_FIXTURE_ID=${fixture_identity}" \
    --build-arg "AI_BLAISE_SOURCE_CONTENT_SHA256=${source_content_sha256}" \
    --build-arg "AI_BLAISE_SOURCE_GIT_SHA=${source_git_sha}" \
    --build-arg "AI_BLAISE_SOURCE_GIT_TREE=${source_git_tree}" \
    --build-arg "AI_BLAISE_SOURCE_TREE_STATE=${source_tree_state}" \
    --tag "${image}" \
    "${fixture_context}" >&2
fi

image_id="$(docker image inspect --format '{{.Id}}' "${image}")"
[[ "${image_id}" =~ ^sha256:[0-9a-f]{64}$ ]] || fail "fixture image has no immutable Docker image ID"

verify_label "ai-blaise.citus.test-fixture" "true"
verify_label "ai-blaise.citus.test-fixture.scope" "source-built-companion-test-only"
verify_label "ai-blaise.citus.test-fixture.release-target" "false"
verify_label "ai-blaise.citus.test-fixture.pg-major" "${pg_major}"
verify_label "ai-blaise.citus.test-fixture.base-image" "${base_image}"
verify_label "ai-blaise.citus.test-fixture.citus-extension-version" "${citus_extension_version}"
verify_label "ai-blaise.citus.test-fixture.id" "${fixture_identity}"
verify_label "ai-blaise.citus.source-content-sha256" "${source_content_sha256}"

provenance_git_sha="$(read_label "ai-blaise.citus.source-git-sha")"
provenance_git_tree="$(read_label "ai-blaise.citus.source-git-tree")"
provenance_tree_state="$(read_label "ai-blaise.citus.source-tree-state")"
provenance_revision="$(read_label "org.opencontainers.image.revision")"
[[ "${provenance_git_sha}" =~ ^[0-9a-f]{40}$ ]] || fail "fixture image Git provenance SHA is malformed"
[[ "${provenance_git_tree}" =~ ^[0-9a-f]{40}$ ]] || fail "fixture image Git provenance tree is malformed"
[[ "${provenance_tree_state}" =~ ^(clean|dirty)$ ]] || fail "fixture image tree-state provenance is malformed"
[[ "${provenance_revision}" == "${provenance_git_sha}" ]] || fail "fixture image OCI revision disagrees with Git provenance"

printf '%s\n' "${image_id}"
