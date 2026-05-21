#!/usr/bin/env bash
set -euo pipefail

audit_file="docs/ai-blaise/LICENSE_AUDIT.md"

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

# GPL-2.0 / GPL-3.0 transitive Rust deps would virally contaminate the
# AGPL-3.0 fork's distribution. The workspace itself is AGPL-3.0; the
# transitive set must stay permissive (MIT, Apache-2.0, BSD, MPL,
# Unlicense, ISC, Zlib, BSL-1.0) or weak-copyleft (LGPL). Strong
# copyleft (GPL-2.0-only, GPL-3.0-only) is rejected.
if [[ -s "Cargo.lock" ]]; then
  # Pinned-by-name blocklist. Append crate names here when crates.io
  # advisories or `cargo-deny` flag GPL-licensed deps we must block by
  # name from the Rust dependency tree. Empty by default; the SPDX
  # scan below catches the general case.
  gpl_crate_names=()
  for crate in "${gpl_crate_names[@]}"; do
    if grep -Eq "^name = \"${crate}\"" Cargo.lock; then
      echo "GPL-licensed crate forbidden in Cargo.lock: ${crate}" >&2
      exit 1
    fi
  done

  # SPDX scan: parse `cargo metadata` and flag any package whose
  # license expression contains GPL-2.0 or GPL-3.0 without also
  # offering an AGPL or LGPL fallback (which are compatible).
  if command -v cargo >/dev/null 2>&1 && command -v jq >/dev/null 2>&1; then
    metadata_json="$(cargo metadata --format-version 1 2>/dev/null || true)"
    if [[ -n "${metadata_json}" ]]; then
      gpl_hits="$(printf '%s\n' "${metadata_json}" \
        | jq -r '.packages[]
            | select(
                (.license // "") as $lic
                | ($lic | test("(^|[^A-Za-z])GPL-[23]\\.0([^A-Za-z]|$)"))
                  and ($lic | test("AGPL") | not)
                  and ($lic | test("LGPL") | not)
              )
            | "\(.name) \(.version) \(.license)"' 2>/dev/null || true)"
      if [[ -n "${gpl_hits}" ]]; then
        echo "GPL-2.0 / GPL-3.0 Rust dependency forbidden:" >&2
        printf '  %s\n' "${gpl_hits}" >&2
        exit 1
      fi
    fi
  fi
fi
