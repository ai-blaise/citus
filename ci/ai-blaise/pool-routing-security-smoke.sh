#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if ! command -v cargo >/dev/null 2>&1 && [ -x "${HOME}/.cargo/bin/cargo" ]; then
  export PATH="${HOME}/.cargo/bin:${PATH}"
fi

report="$(cargo run -q -p ai_blaise_citus_pool -- run-canonical)"

REPORT="${report}" python3 - <<PY
import os

raw = os.environ["REPORT"].strip().splitlines()
if len(raw) != 2:
    raise SystemExit(f"expected two TSV lines from pool canonical report, got {len(raw)}")
headers = raw[0].split("\t")
values = raw[1].split("\t")
if len(headers) != len(values):
    raise SystemExit(f"header/value mismatch: {len(headers)} != {len(values)}")
row = dict(zip(headers, values))

expected = {
    "mirror_rule_count": "1",
    "mirror_decision_bucket": "42",
    "mirrored_canary_routes": "1",
    "htap_analytical_routes": "1",
    "htap_fail_closed_rejections": "1",
    "tls_rotation_due": "true",
    "tls_previous_key_valid": "true",
    "tls_previous_key_present": "true",
    "tls_key_fingerprint_len": "16",
    "geo_replica_regions": "2",
    "geo_fallback_routes": "1",
    "geo_invalid_cidr_rejections": "1",
}
missing = [key for key in expected if key not in row]
if missing:
    raise SystemExit("pool routing/security report missing columns: " + ",".join(missing))
for key, value in expected.items():
    if row[key] != value:
        raise SystemExit(f"pool routing/security report mismatch for {key}: expected {value}, got {row[key]}")
print("pool-routing-security-smoke ok")
PY
