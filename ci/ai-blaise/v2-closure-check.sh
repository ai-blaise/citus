#!/usr/bin/env bash
set -euo pipefail

docs_file="docs/ai-blaise/NEW_FEATURES.md"
implementation_roots=(
  companion
  sidecar
  pool
  operator
  e2e
  tools
  patches
  deploy
  images
  scripts
)

required_v2_ids=(
  A7
  A9
  A10
  A11
  A12
  C11
  C12
  C13
  D7
  D8
  D9
  D10
  D11
  EF6
  Edge1
  Edge2
  F2
  F3
  F4
  F5
  G1
  Geo1
  IA1
  IA2
  JS1
  L7
  L10
  L11
  M4
  M6
  M10
  M12
  MR3
  MR6
  MR7
  MR9
  O7
  O8
  O9
  O11
  O12
  PM1
  PM2
  R3
  R6
  R8
  R11
  R12
  RT5
  S1
  S3
  S7
  S8
  S12
  Search1
  Search4
  Search5
  Search6
  Sec3
  Sec4
  Sec7
  Sec8
  Sec9
  Sec10
  Sec11
  Sec13
  Sec14
  Sec15
  Sto2
  T4
  T6
  T7
  T10
  T11
  T13
  T14
  TS10
  TS11
  WF1
)

contains_feature_id() {
  local id="$1"
  shift

  if command -v rg >/dev/null 2>&1; then
    rg -q "FEATURE: ${id}([^A-Za-z0-9]|$)" "$@"
  else
    grep -R -Eq "FEATURE: ${id}([^A-Za-z0-9]|$)" "$@"
  fi
}

for id in "${required_v2_ids[@]}"; do
  if ! contains_feature_id "${id}" "${implementation_roots[@]}"; then
    echo "V2 closure id missing from implementation markers: ${id}" >&2
    exit 1
  fi

  if ! contains_feature_id "${id}" "${docs_file}"; then
    echo "V2 closure id missing from ${docs_file}: ${id}" >&2
    exit 1
  fi
done

stale_pattern='future|initial Rust spec|not actual|later slice'
if grep -E -n "${stale_pattern}" docs/ai-blaise/NEW_FEATURES.md operator/CRDS.md; then
  echo "Stale V2 closure wording remains in docs." >&2
  exit 1
fi

for crate_dir in tools/* pool operator companion e2e sidecar/*; do
  [[ -f "${crate_dir}/Cargo.toml" ]] || continue

  has_bin=false
  if [[ -d "${crate_dir}/src/bin" ]] \
    && find "${crate_dir}/src/bin" -type f | grep -q .; then
    has_bin=true
  fi

  if [[ ! -f "${crate_dir}/src/main.rs" && "${has_bin}" == false ]]; then
    echo "Overlay crate missing executable target: ${crate_dir}" >&2
    exit 1
  fi
done

assert_row() {
  local label="$1"
  local expected="$2"
  shift 2

  local output
  if ! output="$("$@")"; then
    echo "${label} runner failed." >&2
    exit 1
  fi

  if ! printf '%s\n' "${output}" | grep -Fqx "${expected}"; then
    echo "${label} runner did not emit expected TSV row." >&2
    echo "Expected: ${expected}" >&2
    echo "Actual output:" >&2
    printf '%s\n' "${output}" >&2
    exit 1
  fi
}

assert_row \
  operator \
  $'17\t3\t3\t32\t5\t8\t13\t30\t3072\t2\t2\t2\t2' \
  cargo run -q -p ai_blaise_citus_operator -- run-canonical

assert_row \
  companion-extension-catalog \
  $'45\t38\t47\t18\t26\t1\t18' \
  cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical

assert_row \
  companion-advanced-planner \
  $'27\t1\t1\t4096\t2\t2\t256\t19\t40\t1\t2' \
  cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical

assert_row \
  companion-domain-contracts \


  $'38\tA1,API4,Auth2,G2,G3,Geo2,Geo3,IA3,JS2,L9,M1,M11,M13,M2,M7,PM3,PM4,S13,S14,S6,Search2,Search3,Search9,Sec1,Sec2,Sec5,Sec6,T8,TO3,TO4,TO5,TS13,TS14,TS15,TS16,TS17,TS9,WH2\t22\t11\t49' \


  cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical

assert_row \
  companion-operations \
  $'15\t1\t2\t3\t2\t6\t1' \
  cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical

assert_row \
  companion-plan-runtime \
  $'1\t1\t1\t8\t1\t1\t1\t1\t5' \
  cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-plan-runtime-canonical

assert_row \
  pool \
  $'1\t1000\t1\t1\t1\t5\t1\t2000\t1\t32\ttrue\t1\t3600\ttrue\ttrue\t1000\t1\t1\t1\t1\t10000\t1\t1\t2\t1\t8\t1\t1\t1\t1\t1\t1\t1\t2\t1' \
  cargo run -q -p ai_blaise_citus_pool -- run-canonical

assert_row \
  tools-mcp \
  $'3\t2\t3\t1' \
  cargo run -q -p ai_blaise_citus_mcp -- run-canonical

assert_row \
  tools-citusctl \
  $'5\t21\t2\t5\t1\t17' \
  cargo run -q -p ai_blaise_citusctl -- run-canonical

assert_row \
  tools-admin \
  $'8\t1' \
  cargo run -q -p ai_blaise_citus_admin -- run-canonical

assert_row \
  schema-designer \
  $'1\t0\t1\t5' \
  cargo run -q -p ai_blaise_citus_schema_designer -- run-canonical

assert_row \
  tui \
  $'9\t2\ttrue\t9' \
  cargo run -q -p ai_blaise_citus_tui -- run-canonical

assert_row \
  watch \
  $'3\t9\t9\t5' \
  cargo run -q -p ai_blaise_citus_watch -- run-canonical
