#!/usr/bin/env bash
set -euo pipefail

audit_file="docs/ai-blaise/LICENSE_AUDIT.md"
release_mode="${AI_BLAISE_RELEASE_MODE:-0}"

if [[ ! -s "${audit_file}" ]]; then
  echo "missing ${audit_file}" >&2
  exit 1
fi

required_components=(
  "Citus"
  "TimescaleDB Apache parts"
  "TimescaleDB TSL parts"
  "pgcat"
  "pgrx"
  "kube-rs"
  "pg_repack"
  "pgvector"
  "pg_search"
  "PostgREST"
  "Deno"
  "Bun"
  "DataFusion / Arrow"
  "Iceberg Rust"
)

for component in "${required_components[@]}"; do
  if ! grep -Fq "| ${component} |" "${audit_file}"; then
    echo "license audit missing component: ${component}" >&2
    exit 1
  fi
done

if grep -RIn "unknown license\\|TODO license\\|proprietary dependency" \
  docs/ai-blaise companion sidecar pool operator tools deploy patches; then
  echo "license audit contains unresolved license language" >&2
  exit 1
fi

if grep -RIn "timescaledb.*/tsl\\|/tsl/" patches companion sidecar pool operator tools deploy; then
  echo "TSL source must not be patched or vendored" >&2
  exit 1
fi

# Per-language attribution files exist at repo root and are referenced
# from the audit doc.
required_attribution_files=(
  "ATTRIBUTIONS-Rust.md"
  "ATTRIBUTIONS-Go.md"
  "ATTRIBUTIONS-TypeScript.md"
)
for f in "${required_attribution_files[@]}"; do
  if [[ ! -s "${f}" ]]; then
    echo "license audit missing attribution file: ${f}" >&2
    exit 1
  fi
  if ! grep -Fq "${f}" "${audit_file}"; then
    echo "license audit must reference ${f} from ${audit_file}" >&2
    exit 1
  fi
done

# The repository dependency policy rejects a Rust package whose declared
# license expression contains GPL-2.0 or GPL-3.0 unless that expression also
# contains AGPL or LGPL. This check enforces that recorded policy; it does not
# make a legal compatibility determination.
if [[ ! -s "Cargo.lock" ]]; then
  echo "license metadata scan requires a nonempty Cargo.lock" >&2
  exit 1
fi

cargo_available=1
jq_available=1
if ! command -v cargo >/dev/null 2>&1; then
  cargo_available=0
fi
if ! command -v jq >/dev/null 2>&1; then
  jq_available=0
fi

if [[ "${cargo_available}" == "0" || "${jq_available}" == "0" ]]; then
  if [[ "${release_mode}" == "1" ]]; then
    if [[ "${cargo_available}" == "0" ]]; then
      echo "license metadata scan requires cargo in release mode" >&2
    fi
    if [[ "${jq_available}" == "0" ]]; then
      echo "license metadata scan requires jq in release mode" >&2
    fi
    exit 1
  fi

  echo "license-check: exploratory-only metadata scan skipped; cargo and jq are required for release evidence" >&2
  exit 0
fi

metadata_file="$(mktemp "${TMPDIR:-/tmp}/ai-blaise-license-metadata.XXXXXX")"
cleanup() {
  rm -f -- "${metadata_file}"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

if ! cargo metadata --locked --format-version 1 >"${metadata_file}"; then
  echo "license metadata scan failed: cargo metadata --locked did not complete" >&2
  exit 1
fi
if [[ ! -s "${metadata_file}" ]]; then
  echo "license metadata scan failed: cargo metadata --locked returned empty output" >&2
  exit 1
fi

# Before applying the dependency policy, prove that jq can parse a real,
# nonempty Cargo workspace graph and that every workspace member is represented
# by a package in the metadata document. A jq execution or validation failure is
# fatal in exploratory and release modes alike once the scan has started.
if ! jq -s -e '
    (length == 1)
    and (
      .[0] as $metadata
      | ($metadata | type == "object")
      and ($metadata.packages | type == "array" and length > 0)
      and ($metadata.workspace_members | type == "array" and length > 0)
      and ($metadata.workspace_root | type == "string" and length > 0)
      and ($metadata.resolve | type == "object")
      and ($metadata.resolve.nodes | type == "array" and length > 0)
      and (all($metadata.packages[];
        type == "object"
        and (.id | type == "string" and length > 0)
        and (.name | type == "string" and length > 0)
        and (.version | type == "string" and length > 0)))
      and (all($metadata.workspace_members[];
        type == "string" and length > 0))
      and (all($metadata.workspace_members[];
        . as $member | any($metadata.packages[]; .id == $member)))
    )
  ' "${metadata_file}" >/dev/null; then
  echo "license metadata scan failed: jq rejected malformed or incomplete cargo metadata" >&2
  exit 1
fi

if ! gpl_hits="$(jq -r '
    .packages[]
    | select(
        (.license // "") as $lic
        | ($lic | test("(^|[^A-Za-z])GPL-[23]\\.0([^A-Za-z]|$)"))
          and ($lic | test("AGPL") | not)
          and ($lic | test("LGPL") | not)
      )
    | "\(.name) \(.version) \(.license)"
  ' "${metadata_file}")"; then
  echo "license metadata scan failed: jq could not evaluate repository dependency policy" >&2
  exit 1
fi

if [[ -n "${gpl_hits}" ]]; then
  echo "GPL-2.0 / GPL-3.0 Rust dependency forbidden by repository dependency policy:" >&2
  printf '  %s\n' "${gpl_hits}" >&2
  exit 1
fi

echo "license-check: locked Cargo metadata scan passed"
