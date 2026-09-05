#!/usr/bin/env bash
# FEATURE: A10 A11
set -euo pipefail

# Build/cache a test-only HTTP wrapper around the immutable real-Citus PG17
# fixture. A bare local sha256 image ID is not a portable FROM reference, so
# the builder requires the expected local fixture tag pinned by that digest,
# verifies the compound reference, and fails closed on engines that cannot
# resolve it. It never falls back to a floating parent tag.

repo_root="$(git rev-parse --show-toplevel)"
base_builder="${repo_root}/ci/ai-blaise/build-real-citus-test-fixture.sh"
contract_check="${repo_root}/ci/ai-blaise/real-citus-test-fixture-contract.py"
dockerfile="${repo_root}/images/citus-test-fixture/Dockerfile.http"
package_lock="${repo_root}/images/citus-test-fixture/http-packages.lock.tsv"
pg_major=17
contract_only=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --pg-major)
      [[ $# -ge 2 ]] || { echo "--pg-major requires a value" >&2; exit 2; }
      pg_major="$2"
      shift 2
      ;;
    --contract-only)
      contract_only=1
      shift
      ;;
    *)
      echo "unknown real-Citus HTTP fixture builder argument: $1" >&2
      exit 2
      ;;
  esac
done

python3 "${contract_check}" >&2
if [[ "${contract_only}" == "1" ]]; then
  printf 'real-citus-http-test-fixture-contract passed\n'
  exit 0
fi

if [[ "${pg_major}" != "17" ]]; then
  echo "real-Citus HTTP fixture currently supports only PG17" >&2
  exit 1
fi
if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required to build the real-Citus HTTP test fixture" >&2
  exit 1
fi

package_row="$(python3 - "${package_lock}" "${pg_major}" <<'PY'
import csv
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
major = sys.argv[2]
with path.open(encoding="utf-8", newline="") as handle:
    reader = csv.DictReader(handle, delimiter="\t")
    if reader.fieldnames != ["pg_major", "package", "version"]:
        raise SystemExit(20)
    rows = list(reader)
matches = [row for row in rows if row["pg_major"] == major]
if len(matches) != 1 or any(not value for value in matches[0].values()):
    raise SystemExit(21)
print(f"{matches[0]['package']}\t{matches[0]['version']}")
PY
)" || {
  echo "real-Citus HTTP package lock is malformed or missing PG${pg_major}" >&2
  exit 1
}
IFS=$'\t' read -r package_name package_version extra <<<"${package_row}"
if [[ -z "${package_name}" || -z "${package_version}" || -n "${extra:-}" ]]; then
  echo "real-Citus HTTP package lock row is incomplete" >&2
  exit 1
fi

build_root="$(mktemp -d "${TMPDIR:-/tmp}/real-citus-http-fixture.XXXXXX")"
cleanup() {
  rm -rf "${build_root}"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
install -m 0644 "${dockerfile}" "${build_root}/Dockerfile"

fixture_image_id="$("${base_builder}" --pg-major "${pg_major}")"
if [[ ! "${fixture_image_id}" =~ ^sha256:[0-9a-f]{64}$ ]]; then
  echo "base real-Citus fixture builder did not return an immutable image ID" >&2
  exit 1
fi
if [[ "$(docker image inspect --format '{{.Id}}' "${fixture_image_id}")" != "${fixture_image_id}" ]]; then
  echo "base real-Citus fixture image ID could not be verified" >&2
  exit 1
fi
fixture_id="$(docker image inspect --format '{{ index .Config.Labels "ai-blaise.citus.test-fixture.id" }}' "${fixture_image_id}")"
if [[ ! "${fixture_id}" =~ ^[0-9a-f]{64}$ ]]; then
  echo "base real-Citus fixture identity label is invalid" >&2
  exit 1
fi
fixture_tag="ai-blaise-citus-test-fixture:pg${pg_major}-${fixture_id}"
if [[ "$(docker image inspect --format '{{.Id}}' "${fixture_tag}")" != "${fixture_image_id}" ]]; then
  echo "base real-Citus fixture tag does not resolve to the expected image ID" >&2
  exit 1
fi
fixture_parent="docker.io/library/${fixture_tag}@${fixture_image_id}"
if ! resolved_parent_id="$(docker image inspect --format '{{.Id}}' "${fixture_parent}" 2>/dev/null)" ||
   [[ "${resolved_parent_id}" != "${fixture_image_id}" ]]; then
  echo "Docker engine cannot resolve the locally cached digest-pinned fixture parent" >&2
  exit 1
fi

dockerfile_sha256="$(python3 - "${build_root}/Dockerfile" <<'PY'
import hashlib
import pathlib
import sys

print(hashlib.sha256(pathlib.Path(sys.argv[1]).read_bytes()).hexdigest())
PY
)"
http_fixture_id="$(python3 - \
  "${pg_major}" \
  "${fixture_image_id}" \
  "${fixture_id}" \
  "${package_name}" \
  "${package_version}" \
  "${dockerfile_sha256}" <<'PY'
import hashlib
import struct
import sys

digest = hashlib.sha256(b"ai-blaise/real-citus-http-test-fixture/v1\0")
for value in sys.argv[1:]:
    encoded = value.encode("utf-8")
    digest.update(struct.pack(">Q", len(encoded)))
    digest.update(encoded)
print(digest.hexdigest())
PY
)"
image="ai-blaise-citus-http-test-fixture:pg${pg_major}-${http_fixture_id}"

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
    echo "cached real-Citus HTTP fixture label mismatch: ${label}" >&2
    exit 1
  fi
}

if docker image inspect "${image}" >/dev/null 2>&1; then
  image_id="$(docker image inspect --format '{{.Id}}' "${image}")"
  [[ "${image_id}" =~ ^sha256:[0-9a-f]{64}$ ]] || {
    echo "cached real-Citus HTTP fixture has a nonimmutable image ID" >&2
    exit 1
  }
  verify_label "ai-blaise.citus.test-fixture.http" "true"
  verify_label "ai-blaise.citus.test-fixture.release-target" "false"
  verify_label "ai-blaise.citus.test-fixture.pg-major" "${pg_major}"
  verify_label "ai-blaise.citus.test-fixture.http-package" "${package_name}"
  verify_label "ai-blaise.citus.test-fixture.http-package-version" "${package_version}"
  verify_label "ai-blaise.citus.test-fixture.http-parent-image-id" "${fixture_image_id}"
  verify_label "ai-blaise.citus.test-fixture.http-parent-fixture-id" "${fixture_id}"
  verify_label "ai-blaise.citus.test-fixture.http-id" "${http_fixture_id}"
  printf '%s\n' "${image_id}"
  exit 0
fi

docker build \
  --pull=false \
  --target companion-http-test-fixture \
  --build-arg "REAL_CITUS_FIXTURE_PARENT=${fixture_parent}" \
  --build-arg "PG_MAJOR=${pg_major}" \
  --build-arg "PG_HTTP_PACKAGE=${package_name}" \
  --build-arg "PG_HTTP_PACKAGE_VERSION=${package_version}" \
  --build-arg "AI_BLAISE_HTTP_FIXTURE_ID=${http_fixture_id}" \
  --build-arg "REAL_CITUS_FIXTURE_IMAGE_ID=${fixture_image_id}" \
  --build-arg "REAL_CITUS_FIXTURE_ID=${fixture_id}" \
  -f "${build_root}/Dockerfile" \
  -t "${image}" \
  "${build_root}" >&2

image_id="$(docker image inspect --format '{{.Id}}' "${image}")"
[[ "${image_id}" =~ ^sha256:[0-9a-f]{64}$ ]] || {
  echo "built real-Citus HTTP fixture has a nonimmutable image ID" >&2
  exit 1
}
verify_label "ai-blaise.citus.test-fixture.http" "true"
verify_label "ai-blaise.citus.test-fixture.release-target" "false"
verify_label "ai-blaise.citus.test-fixture.pg-major" "${pg_major}"
verify_label "ai-blaise.citus.test-fixture.http-package" "${package_name}"
verify_label "ai-blaise.citus.test-fixture.http-package-version" "${package_version}"
verify_label "ai-blaise.citus.test-fixture.http-parent-image-id" "${fixture_image_id}"
verify_label "ai-blaise.citus.test-fixture.http-parent-fixture-id" "${fixture_id}"
verify_label "ai-blaise.citus.test-fixture.http-id" "${http_fixture_id}"
printf '%s\n' "${image_id}"
