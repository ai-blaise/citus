#!/usr/bin/env bash
set -euo pipefail

# Historical compatibility entrypoint. The Rust feature-register tool owns TSV
# validation and identity coverage. This wrapper is not a V2 runtime-closure or
# release gate.

usage() {
  cat <<'EOF'
usage: v2-closure-check.sh [--repo PATH] [--feature-register-bin PATH]

With no arguments, validate this checkout using the workspace
ai_blaise_feature_register crate. The overrides are intended for tests and for
validating an explicitly selected checkout with the real validator binary.
EOF
}

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
source_repo="$(cd -- "${script_dir}/../.." && pwd -P)"
register_repo="${source_repo}"
feature_register_bin=""

while (($# > 0)); do
  case "$1" in
    --repo)
      (($# >= 2)) || {
        echo "--repo requires a path" >&2
        usage >&2
        exit 64
      }
      register_repo="$2"
      shift 2
      ;;
    --feature-register-bin)
      (($# >= 2)) || {
        echo "--feature-register-bin requires a path" >&2
        usage >&2
        exit 64
      }
      feature_register_bin="$2"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 64
      ;;
  esac
done

if [[ ! -d "${register_repo}" ]]; then
  echo "feature-register repository is not a directory: ${register_repo}" >&2
  exit 64
fi
register_repo="$(cd -- "${register_repo}" && pwd -P)"

if [[ -n "${feature_register_bin}" ]]; then
  if [[ ! -x "${feature_register_bin}" ]]; then
    echo "feature-register binary is not executable: ${feature_register_bin}" >&2
    exit 64
  fi
  feature_register_bin="$(cd -- "$(dirname -- "${feature_register_bin}")" && pwd -P)/$(basename -- "${feature_register_bin}")"
fi

run_feature_register() {
  local command="$1"
  if [[ -n "${feature_register_bin}" ]]; then
    "${feature_register_bin}" "${command}" --repo "${register_repo}"
  else
    (
      cd -- "${source_repo}"
      cargo run --locked --quiet -p ai_blaise_feature_register -- \
        "${command}" --repo "${register_repo}"
    )
  fi
}

extract_positive_count() {
  local label="$1"
  local report="$2"
  local value
  value="$(awk -F '\t' -v label="${label}" '$1 == label && NF == 2 { print $2 }' <<<"${report}")"
  if ! [[ "${value}" =~ ^[1-9][0-9]*$ ]]; then
    echo "feature-register report lost its positive ${label} count" >&2
    exit 1
  fi
  printf '%s' "${value}"
}

# Each successful command performs the authoritative Rust validation before it
# renders. Do not duplicate that validator in this shell compatibility layer.
check_output="$(run_feature_register check)"
coverage_output="$(run_feature_register check-source-coverage)"
run_feature_register summary >/dev/null

inventory_rows="$(extract_positive_count rows "${check_output}")"
source_marker_ids="$(extract_positive_count source_marker_ids "${coverage_output}")"

# This one semantic label is retained so a future tool change cannot silently
# turn source-marker enumeration into implementation or release evidence.
if ! grep -Fqx $'claim_boundary\tidentity-coverage-only' <<<"${coverage_output}"; then
  echo "feature-register source coverage lost its identity-only claim boundary" >&2
  exit 1
fi

set +e
release_output="$(run_feature_register release-gaps)"
release_status=$?
set -e
if [[ "${release_status}" -ne 1 ]]; then
  echo "feature-register release-gaps exited ${release_status}; expected the intentional blocked status 1" >&2
  if [[ "${release_status}" -eq 0 ]]; then
    exit 1
  fi
  exit "${release_status}"
fi

# Exit 1 is only accepted when the trusted tool identifies it as its deliberate
# release-rejection path, rather than an arbitrary command failure.
if [[ "$(sed -n '1p' <<<"${release_output}")" != $'feature_release_gaps\tblocked' ]] \
  || [[ "$(sed -n '2p' <<<"${release_output}")" != $'release_evidence_verifier\tunimplemented' ]]; then
  echo "feature-register release-gaps lost its blocked/unimplemented boundary" >&2
  exit 1
fi

printf '%s\t%s\n' \
  v2_inventory_check passed \
  authority inventory-only \
  inventory_rows "${inventory_rows}" \
  source_marker_ids "${source_marker_ids}" \
  runtime_closure unverified \
  release_qualification blocked \
  release_evidence_verifier unimplemented
