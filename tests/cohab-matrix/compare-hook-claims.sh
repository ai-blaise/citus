#!/usr/bin/env bash
set -euo pipefail

# Validate a running cohabiting PostgreSQL container's extension/Citus-admission
# state and the per-TS-version static hook inventory at
# `tests/cohab-matrix/<TS_VERSION>/expected-hook-claims.tsv`.
#
# This script does not directly probe symbol-level C hook claims; PostgreSQL
# does not expose `planner_hook` / `ExecutorStart_hook` etc. through SQL.
# Instead it uses `pg_extension` introspection plus the Citus cohabit GUCs to
# prove that TimescaleDB and Citus loaded together and that the exact runtime
# admits `timescaledb`. The expected-claim TSV is a separate static inventory;
# each row documents whether it is source-measured or a carry-forward
# expectation. This script validates only its schema/status vocabulary and
# rejects unresolved rows; it cannot observe or compare live C hook pointers.
#
# Usage:
#   tests/cohab-matrix/compare-hook-claims.sh <TS_VERSION> <CONTAINER_NAME>
#
# Exits 0 when runtime admission and static inventory structure pass, non-zero
# on runtime admission or inventory-structure drift.

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <TS_VERSION> <CONTAINER_NAME>" >&2
  exit 2
fi

ts_version="$1"
container="$2"

repo_root="$(git rev-parse --show-toplevel)"
expected_tsv="${repo_root}/tests/cohab-matrix/${ts_version}/expected-hook-claims.tsv"

if [[ ! -s "${expected_tsv}" ]]; then
  echo "missing expected-hook-claims for TS ${ts_version}: ${expected_tsv}" >&2
  exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "docker required to compare hook claims" >&2
  exit 1
fi

# Verify both extensions are installed in the running container and that the
# Citus cohabit GUC admits `timescaledb`. These facts do not measure hook
# pointer ownership; the source-measured TSV remains a static input.
ext_rows="$(docker exec "${container}" psql -U postgres -At -F $'\t' -c "
  SELECT extname, extversion
  FROM pg_extension
  WHERE extname IN ('citus', 'timescaledb')
  ORDER BY extname
" 2>/dev/null || true)"

if ! grep -q '^citus' <<<"${ext_rows}"; then
  echo "matrix comparator: citus extension is not installed in container ${container}" >&2
  exit 1
fi

if ! grep -q '^timescaledb' <<<"${ext_rows}"; then
  echo "matrix comparator: timescaledb extension is not installed in container ${container}" >&2
  exit 1
fi

ts_actual_version="$(awk -F '\t' '$1 == "timescaledb" { print $2 }' <<<"${ext_rows}")"
if [[ -z "${ts_actual_version}" ]]; then
  echo "matrix comparator: could not read timescaledb extversion" >&2
  exit 1
fi

# Check that the observed TS major.minor matches what the caller asked for.
# `extversion` is the full semver; the matrix is indexed by major.minor.
ts_actual_minor="$(printf '%s\n' "${ts_actual_version}" | awk -F. '{ printf "%s.%s", $1, $2 }')"
if [[ "${ts_actual_minor}" != "${ts_version}" ]]; then
  echo "matrix comparator: container reports TimescaleDB ${ts_actual_version} but matrix asked for ${ts_version}" >&2
  exit 1
fi

cohabit="$(docker exec "${container}" psql -U postgres -Atqc "SHOW citus.cohabit_extensions" 2>/dev/null || true)"
if [[ "${cohabit}" != *timescaledb* ]]; then
  echo "matrix comparator: citus.cohabit_extensions does not include timescaledb (got: ${cohabit})" >&2
  exit 1
fi

# Verify every row in the expected TSV has a recognized claim status.
unknown_rows=0
total_rows=0
while IFS=$'\t' read -r hook_symbol claim_status notes; do
  if [[ "${hook_symbol}" == "hook_symbol" ]]; then
    continue
  fi
  if [[ -z "${hook_symbol}" ]]; then
    continue
  fi
  if [[ -z "${notes}" ]]; then
    echo "matrix validator: TS ${ts_version} row for ${hook_symbol} has empty source-measurement notes" >&2
    exit 1
  fi
  total_rows=$((total_rows + 1))
  case "${claim_status}" in
    claimed|not_claimed)
      ;;
    unknown)
      unknown_rows=$((unknown_rows + 1))
      ;;
    *)
      echo "matrix comparator: TS ${ts_version} row for ${hook_symbol} has unrecognized claim_status '${claim_status}'" >&2
      exit 1
      ;;
  esac
done <"${expected_tsv}"

if [[ "${total_rows}" -lt 1 ]]; then
  echo "matrix comparator: TS ${ts_version} expected-hook-claims.tsv has no rows" >&2
  exit 1
fi

if [[ "${unknown_rows}" -ne 0 && "${TS_VERSION_MATRIX_ALLOW_UNKNOWN:-0}" != "1" ]]; then
  echo "matrix comparator: TS ${ts_version} has ${unknown_rows} unknown hook claims; live images require measured claimed/not_claimed rows" >&2
  echo "matrix comparator: set TS_VERSION_MATRIX_ALLOW_UNKNOWN=1 only for exploratory local probes, not production gates" >&2
  exit 1
fi

echo "matrix validator: TS ${ts_version} runtime admission passed; static hook inventory is structurally closed (${total_rows} rows, ${unknown_rows} unknown; hook_runtime_comparison=unavailable)"
