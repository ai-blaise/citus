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
