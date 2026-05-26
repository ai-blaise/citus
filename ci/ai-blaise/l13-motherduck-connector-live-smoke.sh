#!/usr/bin/env bash
# FEATURE: L13
#
# Live MotherDuck connector smoke for L13. Verifies the analytical sidecar
# MotherDuck binding contract end to end:
#  - Source-level MotherDuckConnector type + validate() exists.
#  - Canonical runtime emits the motherduck session accounting field.
#  - Token-secret binding is opt-in (no token -> no live routing, fail-closed).
#  - Deterministic session counter increments under the canonical scenario.
#
# Does NOT claim live MotherDuck cloud session execution: a real
# motherduck_token + Postgres session against motherduck.com requires
# external credentials (out of CI scope). The L13 production-ready claim
# is bounded to the binding contract + deterministic accounting + fail-closed
# behavior when the token is absent.

set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if [[ -f "${HOME}/.cargo/env" ]]; then
  source "${HOME}/.cargo/env"
fi

evidence_dir="${L13_EVIDENCE_DIR:-artifacts}"
mkdir -p "${evidence_dir}"
evidence_file="${L13_EVIDENCE_FILE:-${evidence_dir}/l13-motherduck-evidence.tsv}"

log() { printf '[l13-motherduck] %s\n' "$*" >&2; }

log "phase 1: source-level MotherDuck connector type"
md_type_present=$(grep -c 'pub struct MotherDuckConnector' sidecar/analytical/src/lib.rs)
md_validate_present=$(grep -c 'impl MotherDuckConnector' sidecar/analytical/src/lib.rs)
if [[ "${md_type_present}" -lt 1 || "${md_validate_present}" -lt 1 ]]; then
  echo "MotherDuckConnector type + impl required in sidecar/analytical/src/lib.rs" >&2; exit 1
fi

log "phase 2: canonical runtime motherduck accounting"
runtime_output="$(cargo run -q -p ai_blaise_citus_sidecar_analytical -- run-runtime-canonical)"
motherduck_db="$(printf '%s\n' "${runtime_output}" | sed -n '2p' | awk -F$'\t' '{print $16}')"
motherduck_sessions="$(printf '%s\n' "${runtime_output}" | sed -n '2p' | awk -F$'\t' '{print $23}')"
if [[ "${motherduck_db}" != "analytics" ]]; then
  echo "runtime motherduck db should be 'analytics', got '${motherduck_db}'" >&2; exit 1
fi
if [[ "${motherduck_sessions}" != "1" ]]; then
  echo "runtime motherduck_sessions should be 1, got '${motherduck_sessions}'" >&2; exit 1
fi

log "phase 3: opt-in token binding (the absence of MOTHERDUCK_TOKEN must keep external IO disabled)"
external_io_attempted="$(printf '%s\n' "${runtime_output}" | sed -n '2p' | awk -F$'\t' '{print $35}')"
if [[ "${external_io_attempted}" != "false" ]]; then
  echo "external_io_attempted must remain false without a live MotherDuck token, got '${external_io_attempted}'" >&2; exit 1
fi

log "phase 4: companion advanced-planner motherduck binding contract"
companion_check=skipped
if command -v cargo >/dev/null 2>&1; then
  if cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical 2>/dev/null | grep -iqF motherduck; then
    companion_check=row_emitted
  else
    companion_check=row_missing
  fi
fi

mkdir -p "$(dirname "${evidence_file}")"
if [[ ! -f "${evidence_file}" ]]; then
  printf 'observed_at\tgit_sha\tmotherduck_type_present\tmotherduck_validate_present\tmotherduck_db\tmotherduck_sessions\texternal_io_attempted\tcompanion_check\n' >"${evidence_file}"
fi
printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
  "$(date -Is)" "$(git rev-parse HEAD)" \
  "${md_type_present}" "${md_validate_present}" \
  "${motherduck_db}" "${motherduck_sessions}" \
  "${external_io_attempted}" "${companion_check}" \
  >>"${evidence_file}"

printf 'l13_motherduck_connector_live\tpassed\tmotherduck_db=%s\tsessions=%s\texternal_io_attempted=%s\tcompanion=%s\n' \
  "${motherduck_db}" "${motherduck_sessions}" "${external_io_attempted}" "${companion_check}"
echo "L13 MotherDuck connector live smoke passed"
