#!/usr/bin/env bash
# FEATURE: A10 A11
set -euo pipefail

# Build/cache a test-only HTTP wrapper around the immutable real-Citus PG17
# fixture. A bare local sha256 image ID is not a portable BuildKit FROM
# reference, while a locally built tag has no registry manifest digest. The
# builder therefore verifies the content-derived local tag immediately before
# and after use, then proves that the child rootfs extends the immutable parent
# rootfs. It never falls back to an unverified or externally supplied tag.

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

verify_parent_tag() {
  local resolved_parent_id
  if ! resolved_parent_id="$(docker image inspect --format '{{.Id}}' "${fixture_tag}" 2>/dev/null)" ||
     [[ "${resolved_parent_id}" != "${fixture_image_id}" ]]; then
    echo "base real-Citus fixture tag does not resolve to the expected image ID" >&2
    exit 1
  fi
}

verify_parent_tag
fixture_parent="${fixture_tag}"

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

verify_parent_ancestry() {
  local parent_layers child_layers
  if ! parent_layers="$(docker image inspect --format '{{json .RootFS.Layers}}' "${fixture_image_id}")" ||
     ! child_layers="$(docker image inspect --format '{{json .RootFS.Layers}}' "${image_id}")"; then
    echo "could not inspect real-Citus HTTP fixture rootfs ancestry" >&2
    exit 1
  fi
  if ! python3 - "${parent_layers}" "${child_layers}" <<'PY'
import json
import re
import sys


def parse_layers(value: str) -> list[str]:
    layers = json.loads(value)
    if not isinstance(layers, list) or not layers:
        raise ValueError("rootfs layer inventory must be a nonempty list")
    if any(
        not isinstance(layer, str)
        or re.fullmatch(r"sha256:[0-9a-f]{64}", layer) is None
        for layer in layers
    ):
        raise ValueError("rootfs layer inventory contains a malformed digest")
    return layers


try:
    parent = parse_layers(sys.argv[1])
    child = parse_layers(sys.argv[2])
except (IndexError, json.JSONDecodeError, ValueError) as error:
    print(f"invalid real-Citus HTTP fixture rootfs evidence: {error}", file=sys.stderr)
    raise SystemExit(1) from error

if len(child) <= len(parent) or child[: len(parent)] != parent:
    print(
        "real-Citus HTTP fixture does not extend the verified parent rootfs",
        file=sys.stderr,
    )
    raise SystemExit(1)
PY
  then
    echo "real-Citus HTTP fixture parent ancestry verification failed" >&2
    exit 1
  fi
}

verify_http_fixture() {
  image_id="$(docker image inspect --format '{{.Id}}' "${image}")"
  [[ "${image_id}" =~ ^sha256:[0-9a-f]{64}$ ]] || {
    echo "real-Citus HTTP fixture has a nonimmutable image ID" >&2
    exit 1
  }
  verify_label "ai-blaise.citus.test-fixture.http" "true"
  verify_label "ai-blaise.citus.test-fixture.release-target" "false"
  verify_label "ai-blaise.citus.test-fixture.pg-major" "${pg_major}"
  verify_label "ai-blaise.citus.test-fixture.id" "${fixture_id}"
  verify_label "ai-blaise.citus.test-fixture.http-package" "${package_name}"
  verify_label "ai-blaise.citus.test-fixture.http-package-version" "${package_version}"
  verify_label "ai-blaise.citus.test-fixture.http-parent-image-id" "${fixture_image_id}"
  verify_label "ai-blaise.citus.test-fixture.http-parent-fixture-id" "${fixture_id}"
  verify_label "ai-blaise.citus.test-fixture.http-id" "${http_fixture_id}"
  verify_parent_tag
  verify_parent_ancestry
}

if docker image inspect "${image}" >/dev/null 2>&1; then
  verify_http_fixture
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

verify_http_fixture
printf '%s\n' "${image_id}"
