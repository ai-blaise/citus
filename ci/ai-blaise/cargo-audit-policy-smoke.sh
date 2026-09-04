#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

policy=".cargo/audit.toml"
expected_ignore='ignore = ["RUSTSEC-2021-0127"]'

if [[ ! -f "${policy}" ]]; then
  echo "cargo-audit policy is missing: ${policy}" >&2
  exit 1
fi

if [[ "$(grep -Ec '^ignore = ' "${policy}")" -ne 1 ]] ||
   ! grep -Fqx "${expected_ignore}" "${policy}"; then
  echo "cargo-audit policy must contain only the reviewed serde_cbor exception" >&2
  exit 1
fi

cargo audit --deny warnings

operator_tree="$(cargo tree -p ai_blaise_citus_operator -e all)"
if grep -Eq '(^|[[:space:]])serde_cbor v0\.11\.2([[:space:]]|$)' <<<"${operator_tree}"; then
  echo "the approved serde_cbor exception became reachable from the operator" >&2
  exit 1
fi

companion_tree="$(cargo tree -p ai_blaise_citus_companion --all-features -i serde_cbor -e all)"
for required in \
  'serde_cbor v0.11.2' \
  'pgrx v0.18.0' \
  'ai_blaise_citus_companion v0.1.0'; do
  if ! grep -Fq "${required}" <<<"${companion_tree}"; then
    echo "serde_cbor exception reachability changed; missing ${required}" >&2
    exit 1
  fi
done

printf 'cargo-audit-policy-smoke ok\n'
