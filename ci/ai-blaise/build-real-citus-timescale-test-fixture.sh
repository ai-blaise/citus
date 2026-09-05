#!/usr/bin/env bash
# FEATURE: TS6 TS18
# Build or verify a source-bound Citus + vendor TimescaleDB test fixture.
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
contract_check="${repo_root}/ci/ai-blaise/real-citus-timescale-test-fixture-contract.py"
context_builder="${repo_root}/ci/ai-blaise/materialize-real-citus-timescale-test-fixture.py"
image="${CITUS_TIMESCALE_TEST_FIXTURE_IMAGE:-}"
make_jobs="${CITUS_TIMESCALE_TEST_FIXTURE_MAKE_JOBS:-2}"
timescaledb_minor="${CITUS_TIMESCALE_TEST_FIXTURE_MINOR:-2.27}"
contract_only=0
fixture_context=""
fixture_tmp_parent="${TMPDIR:-/tmp}"
fixture_tmp_parent="${fixture_tmp_parent%/}"

fail() {
  echo "real-Citus Timescale test fixture: $*" >&2
  exit 1
}

cleanup() {
  if [[ -n "${fixture_context}" && -d "${fixture_context}" ]]; then
    case "${fixture_context}" in
      "${fixture_tmp_parent}"/ai-blaise-citus-timescale-fixture.*)
        rm -rf -- "${fixture_context}"
        ;;
      *)
        echo "real-Citus Timescale test fixture: refusing unexpected temporary path cleanup" >&2
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
    --image)
      [[ $# -ge 2 ]] || fail "--image requires a value"
      image="$2"
      shift 2
      ;;
    --timescaledb-minor)
      [[ $# -ge 2 ]] || fail "--timescaledb-minor requires a value"
      timescaledb_minor="$2"
      shift 2
      ;;
    --contract-only)
      contract_only=1
      shift
      ;;
    --help|-h)
      echo "usage: build-real-citus-timescale-test-fixture.sh [--timescaledb-minor 2.27|2.28] [--image REF] [--contract-only]"
      exit 0
      ;;
    *)
      fail "unknown argument: $1"
      ;;
  esac
done

python3 "${contract_check}" >&2
if [[ "${contract_only}" == "1" ]]; then
  echo "real-citus-timescale-test-fixture-contract passed"
  exit 0
fi

command -v docker >/dev/null 2>&1 || fail "docker is required"
[[ "${timescaledb_minor}" =~ ^2\.(27|28)$ ]] || fail "TimescaleDB minor must be 2.27 or 2.28"
[[ "${make_jobs}" =~ ^[1-9][0-9]*$ ]] || fail "CITUS_TIMESCALE_TEST_FIXTURE_MAKE_JOBS must be a positive integer"
if ((make_jobs > 32)); then
  fail "CITUS_TIMESCALE_TEST_FIXTURE_MAKE_JOBS must not exceed 32"
fi

source_git_sha="$(git -C "${repo_root}" rev-parse --verify 'HEAD^{commit}')"
source_git_tree="$(git -C "${repo_root}" rev-parse --verify 'HEAD^{tree}')"
[[ "${source_git_sha}" =~ ^[0-9a-f]{40}$ ]] || fail "Git commit identity is not canonical SHA-1"
[[ "${source_git_tree}" =~ ^[0-9a-f]{40}$ ]] || fail "Git tree identity is not canonical SHA-1"
source_tree_state="clean"
if ! git -C "${repo_root}" diff --quiet --no-ext-diff HEAD -- ||
  [[ -n "$(git -C "${repo_root}" ls-files --others --exclude-standard)" ]]; then
  source_tree_state="dirty"
fi

fixture_context="$(mktemp -d "${fixture_tmp_parent}/ai-blaise-citus-timescale-fixture.XXXXXX")"
source_content_sha256="$(
  python3 "${context_builder}" \
    --source "${repo_root}" \
    --destination "${fixture_context}"
)"
[[ "${source_content_sha256}" =~ ^[0-9a-f]{64}$ ]] || fail "fixture source identity is not canonical SHA-256"

dockerfile="${fixture_context}/images/citus-timescale-cohabitation/Dockerfile"
staged_lock="${fixture_context}/images/citus-timescale-cohabitation/base-image.lock.tsv"
[[ -s "${dockerfile}" && -s "${staged_lock}" ]] || fail "materialized fixture inputs are incomplete"

lock_row="$(python3 - "${staged_lock}" "${timescaledb_minor}" <<'PY'
import csv
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
selected_minor = sys.argv[2]
with path.open(encoding="utf-8", newline="") as handle:
    reader = csv.DictReader(handle, delimiter="\t")
    if reader.fieldnames != ["pg_major", "timescaledb_minor", "base_image"]:
        raise SystemExit(20)
    rows = list(reader)
if len(rows) != 2 or {row["timescaledb_minor"] for row in rows} != {"2.27", "2.28"}:
    raise SystemExit(21)
if any(not value for row in rows for value in row.values()):
    raise SystemExit(22)
matches = [row for row in rows if row["timescaledb_minor"] == selected_minor]
if len(matches) != 1:
    raise SystemExit(23)
print("\t".join(matches[0][key] for key in reader.fieldnames))
PY
)" || fail "Timescale fixture base lock is malformed"
IFS=$'\t' read -r pg_major timescaledb_minor base_image extra <<<"${lock_row}"
[[ "${pg_major}" == "17" && "${timescaledb_minor}" =~ ^2\.(27|28)$ && -z "${extra:-}" ]] || fail "Timescale fixture base lock row is unsupported"
[[ "${base_image}" =~ ^docker\.io/timescale/timescaledb-ha:pg17-ts2\.(27|28)@sha256:[0-9a-f]{64}$ ]] || fail "Timescale fixture base is not immutable"

citus_extension_version="$({
  awk -F "'" '/^default_version = / { count += 1; version = $2 } END { if (count == 1) print version; else exit 1 }' \
    "${fixture_context}/src/backend/distributed/citus.control"
} || true)"
companion_extension_version="$({
  awk -F "'" '/^default_version = / { count += 1; version = $2 } END { if (count == 1) print version; else exit 1 }' \
    "${fixture_context}/images/citus-pg-overlay/extensions/ai_blaise_citus.control"
} || true)"
[[ "${citus_extension_version}" =~ ^[0-9]+\.[0-9]+-[0-9]+$ ]] || fail "Citus control version is not canonical"
[[ "${companion_extension_version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "companion control version is not canonical"

fixture_identity="$(python3 - \
  "${pg_major}" "${timescaledb_minor}" "${base_image}" \
  "${citus_extension_version}" "${companion_extension_version}" \
  "${source_content_sha256}" <<'PY'
import hashlib
import struct
import sys

digest = hashlib.sha256(b"ai-blaise/real-citus-timescale-test-fixture/v1\0")
for value in sys.argv[1:]:
    encoded = value.encode("utf-8")
    digest.update(struct.pack(">Q", len(encoded)))
    digest.update(encoded)
print(digest.hexdigest())
PY
)"
[[ "${fixture_identity}" =~ ^[0-9a-f]{64}$ ]] || fail "fixture identity is not canonical SHA-256"

if [[ -z "${image}" ]]; then
  image="ai-blaise-citus-timescale-test-fixture:pg${pg_major}-ts${timescaledb_minor}-${fixture_identity}"
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

verify_fixture() {
  verify_label "ai-blaise.citus.test-fixture.timescale" "true"
  verify_label "ai-blaise.citus.test-fixture.scope" "source-built-citus-timescaledb-companion-test-only"
  verify_label "ai-blaise.citus.test-fixture.release-target" "false"
  verify_label "ai-blaise.citus.test-fixture.pg-major" "${pg_major}"
  verify_label "ai-blaise.citus.test-fixture.base-image" "${base_image}"
  verify_label "ai-blaise.citus.test-fixture.timescaledb-minor" "${timescaledb_minor}"
  verify_label "ai-blaise.citus.test-fixture.citus-extension-version" "${citus_extension_version}"
  verify_label "ai-blaise.citus.test-fixture.companion-extension-version" "${companion_extension_version}"
  verify_label "ai-blaise.citus.test-fixture.timescale-id" "${fixture_identity}"
  verify_label "ai-blaise.citus.source-content-sha256" "${source_content_sha256}"

  provenance_git_sha="$(read_label "ai-blaise.citus.source-git-sha")"
  provenance_git_tree="$(read_label "ai-blaise.citus.source-git-tree")"
  provenance_tree_state="$(read_label "ai-blaise.citus.source-tree-state")"
  provenance_revision="$(read_label "org.opencontainers.image.revision")"
  [[ "${provenance_git_sha}" =~ ^[0-9a-f]{40}$ ]] || fail "fixture Git provenance SHA is malformed"
  [[ "${provenance_git_tree}" =~ ^[0-9a-f]{40}$ ]] || fail "fixture Git provenance tree is malformed"
  [[ "${provenance_tree_state}" =~ ^(clean|dirty)$ ]] || fail "fixture tree-state provenance is malformed"
  [[ "${provenance_revision}" == "${provenance_git_sha}" ]] || fail "fixture OCI revision disagrees with Git provenance"
}

if docker image inspect "${image}" >/dev/null 2>&1; then
  image_id="$(docker image inspect --format '{{.Id}}' "${image}")"
  [[ "${image_id}" =~ ^sha256:[0-9a-f]{64}$ ]] || fail "cached fixture has no immutable image ID"
  verify_fixture
  printf '%s\n' "${image_id}"
  exit 0
fi

[[ "${image}" != sha256:* ]] || fail "requested immutable fixture image ID does not exist"
docker build \
  --pull=false \
  --file "${dockerfile}" \
  --build-arg "BASE_IMAGE=${base_image}" \
  --build-arg "PG_MAJOR=${pg_major}" \
  --build-arg "TIMESCALEDB_MINOR=${timescaledb_minor}" \
  --build-arg "CITUS_EXTENSION_VERSION=${citus_extension_version}" \
  --build-arg "COMPANION_EXTENSION_VERSION=${companion_extension_version}" \
  --build-arg "MAKE_JOBS=${make_jobs}" \
  --build-arg "WITH_LLVM=no" \
  --build-arg "AI_BLAISE_COHAB_FIXTURE_ID=${fixture_identity}" \
  --build-arg "AI_BLAISE_SOURCE_CONTENT_SHA256=${source_content_sha256}" \
  --build-arg "AI_BLAISE_SOURCE_GIT_SHA=${source_git_sha}" \
  --build-arg "AI_BLAISE_SOURCE_GIT_TREE=${source_git_tree}" \
  --build-arg "AI_BLAISE_SOURCE_TREE_STATE=${source_tree_state}" \
  --tag "${image}" \
  "${fixture_context}" >&2

image_id="$(docker image inspect --format '{{.Id}}' "${image}")"
[[ "${image_id}" =~ ^sha256:[0-9a-f]{64}$ ]] || fail "built fixture has no immutable image ID"
verify_fixture
printf '%s\n' "${image_id}"
