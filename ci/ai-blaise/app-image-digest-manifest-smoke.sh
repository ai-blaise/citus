#!/usr/bin/env bash
set -euo pipefail

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "${tmp_dir}"
}
trap cleanup EXIT

fake_docker="${tmp_dir}/bin/docker"
mkdir -p "$(dirname "${fake_docker}")"

cat >"${fake_docker}" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

case "$1" in
  build)
    exit 0
    ;;
  push)
    if [[ "${FAKE_DOCKER_PUSH_DIGEST_MODE:-present}" != "missing" ]]; then
      image="${@: -1}"
      printf '%s: digest: sha256:%064d size: 1234\n' "${image}" 1
    fi
    exit 0
    ;;
  image)
    if [[ "${2:-}" == "inspect" ]]; then
      image="${@: -1}"
      if [[ "${FAKE_DOCKER_DIGEST_MODE:-missing}" == "missing" ]]; then
        exit 0
      fi
      repository="${image%:*}"
      printf '%s@sha256:%064d\n' "${repository}" 1
      exit 0
    fi
    ;;
esac

echo "unexpected fake docker invocation: $*" >&2
exit 1
SH
chmod +x "${fake_docker}"

matrix="$(LIST_IMAGES=true scripts/citus-scale/build-app-images.sh)"
if [[ "$(printf '%s\n' "${matrix}" | wc -l | tr -d ' ')" -ne 21 ]]; then
  echo "image matrix list must include header plus 20 image rows" >&2
  printf '%s\n' "${matrix}" >&2
  exit 1
fi

grep -Fq $'citusctl\tai_blaise_citusctl\tai_blaise_citusctl\tplan inspect cluster' <<< "${matrix}"

if PATH="${tmp_dir}/bin:${PATH}" \
  PUSH=true \
  TAG=release-candidate \
  SOURCE_REVISION=0123456789abcdef0123456789abcdef01234567 \
  DIGEST_FILE="${tmp_dir}/missing-registry.tsv" \
  scripts/citus-scale/build-app-images.sh >"${tmp_dir}/missing-registry.out" 2>"${tmp_dir}/missing-registry.err"; then
  echo "build-app-images.sh must require IMAGE_REGISTRY for pushes" >&2
  exit 1
fi

grep -q "PUSH=true requires IMAGE_REGISTRY" "${tmp_dir}/missing-registry.err"

if PATH="${tmp_dir}/bin:${PATH}" \
  IMAGE_REGISTRY=registry.example.com/ai-blaise \
  TAG=latest \
  SOURCE_REVISION=0123456789abcdef0123456789abcdef01234567 \
  PUSH=true \
  DIGEST_FILE="${tmp_dir}/latest.tsv" \
  scripts/citus-scale/build-app-images.sh >"${tmp_dir}/latest.out" 2>"${tmp_dir}/latest.err"; then
  echo "build-app-images.sh must reject mutable release image tags" >&2
  exit 1
fi

grep -q "release image tag must not be mutable" "${tmp_dir}/latest.err"

manifest="${tmp_dir}/ai-blaise-image-digests.tsv"
PATH="${tmp_dir}/bin:${PATH}" \
  IMAGE_REGISTRY=registry.example.com/ai-blaise \
  TAG=release-candidate \
  SOURCE_REVISION=0123456789abcdef0123456789abcdef01234567 \
  PUSH=true \
  DIGEST_FILE="${manifest}" \
  scripts/citus-scale/build-app-images.sh

if [[ "$(wc -l <"${manifest}" | tr -d ' ')" -ne 21 ]]; then
  echo "digest manifest must include header plus 20 image rows" >&2
  cat "${manifest}" >&2
  exit 1
fi

expected_header=$'source_revision\trepository\timage\ttag\tdigest\tpackage\tbinary\tpushed'
if [[ "$(head -n 1 "${manifest}")" != "${expected_header}" ]]; then
  echo "unexpected digest manifest header" >&2
  head -n 1 "${manifest}" >&2
  exit 1
fi

RELEASE_DIGEST_MANIFEST="${manifest}" \
  REQUIRE_PUBLISHED_DIGESTS=1 \
  ci/ai-blaise/release-publishability-check.sh

awk -F'\t' '
  function is_sha256_digest(value) {
    return value ~ /^sha256:[0-9a-f]+$/ && length(value) == 71
  }
  $1 == "0123456789abcdef0123456789abcdef01234567" &&
  $2 == "citus-operator" &&
  $4 == "release-candidate" &&
  is_sha256_digest($5) &&
  $6 == "ai_blaise_citus_operator" &&
  $7 == "ai_blaise_citus_operator" &&
  $8 == "true" { found_operator = 1 }
  $1 == "0123456789abcdef0123456789abcdef01234567" &&
  $2 == "citus-pool" &&
  is_sha256_digest($5) &&
  $6 == "ai_blaise_citus_pool" &&
  $7 == "ai_blaise_citus_pool" { found_pool = 1 }
  END { exit (found_operator && found_pool) ? 0 : 1 }
' "${manifest}" || {
  echo "digest manifest missing operator/pool release digest rows" >&2
  cat "${manifest}" >&2
  exit 1
}

if PATH="${tmp_dir}/bin:${PATH}" \
  FAKE_DOCKER_DIGEST_MODE=missing \
  FAKE_DOCKER_PUSH_DIGEST_MODE=missing \
  IMAGE_REGISTRY=registry.example.com/ai-blaise \
  TAG=release-candidate \
  SOURCE_REVISION=0123456789abcdef0123456789abcdef01234567 \
  PUSH=true \
  DIGEST_FILE="${tmp_dir}/missing-digest.tsv" \
  scripts/citus-scale/build-app-images.sh >"${tmp_dir}/missing.out" 2>"${tmp_dir}/missing.err"; then
  echo "build-app-images.sh must fail a pushed image without an immutable digest" >&2
  exit 1
fi

grep -q "did not report an immutable repo digest" "${tmp_dir}/missing.err"
