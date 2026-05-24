#!/usr/bin/env bash
set -euo pipefail

# FEATURE: A9 Sec7 Sec8 Sec9

export PATH="${HOME}/.cargo/bin:${PATH}"

expected=$'6	5	5	5	15	4	4	4	4	5'
output="$(cargo run -q -p ai_blaise_citus_operator -- run-security-supply-chain-canonical)"

if ! printf '%s
' "${output}" | grep -Fqx "${expected}"; then
  echo "operator security supply-chain runner did not emit expected TSV row" >&2
  echo "Expected: ${expected}" >&2
  echo "Actual output:" >&2
  printf '%s
' "${output}" >&2
  exit 1
fi

cargo test -q -p ai_blaise_citus_operator security_supply_chain

security_source="operator/src/reconcile/security.rs"
for required in   "external-secrets.io/v1beta1"   "ExternalSecret"   "MissingExternalSecretBinding"   "WeakTlsVersion"   "MutableImageReference"   "InvalidSbomPath"   "slsa.dev/provenance/v1"   ".spdx.json"   ".sigstore.json"; do
  if ! grep -Fq "${required}" "${security_source}"; then
    echo "security supply-chain contract lost required assertion: ${required}" >&2
    exit 1
  fi
done

printf 'security-supply-chain-smoke ok
'
