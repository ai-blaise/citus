#!/usr/bin/env bash
set -euo pipefail

# FEATURE: D10
# Fail-closed release-hardening runbook smoke. This proves the release runbook
# and companion contract produce an auditable release record and block
# production promotion while alpha features remain in release scope. It does not
# claim that a release candidate has received human owner signoff.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

report="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-release-hardening-canonical)"

REPORT="${report}" python3 - <<'PY'
import os

raw = os.environ["REPORT"].strip().splitlines()
if len(raw) != 2:
    raise SystemExit(f"expected two-line release hardening report, got {len(raw)}")
headers = raw[0].split("\t")
values = raw[1].split("\t")
row = dict(zip(headers, values))
expected = {
    "feature_id": "D10",
    "required_gates": "19",
    "release_record_fields": "10",
    "production_release_block_required": "true",
    "owner_signoff_required": "true",
    "rollback_evidence_required": "true",
    "production_gap_audit_required": "true",
    "runbook_command_check_required": "true",
}
missing = [key for key in expected if key not in row]
if missing:
    raise SystemExit("release hardening report missing columns: " + ",".join(missing))
for key, value in expected.items():
    if row[key] != value:
        raise SystemExit(f"release hardening report mismatch for {key}: expected {value}, got {row[key]}")
PY

bash ci/ai-blaise/runbook-command-check.sh
bash ci/ai-blaise/docs-evidence-boundary-check.sh
bash ci/ai-blaise/production-gap-audit.sh >/dev/null

release_check="$(mktemp)"
release_record="$(mktemp)"
trap 'rm -f "${release_check}" "${release_record}"' EXIT

if ci/ai-blaise/production-readiness-check.sh production-release >"${release_check}" 2>&1; then
  echo "production release mode unexpectedly passed while alpha features remain in release scope" >&2
  exit 1
fi
grep -Fq "production release blocked: non-production feature statuses remain" "${release_check}"
if grep -Eq '(^|[,[:space:]])D10([,[:space:]]|$)' "${release_check}"; then
  echo "D10 must not be listed as a production-release blocker after this promotion" >&2
  cat "${release_check}" >&2
  exit 1
fi

source_revision="$(git rev-parse --verify HEAD)"
{
  printf 'source_revision=%s\n' "${source_revision}"
  printf 'image_digest_manifest=required\n'
  printf 'production_readiness_audit=production-release-blocked\n'
  printf 'production_gap_audit=passed\n'
  printf 'docs_evidence_boundary_audit=passed\n'
  printf 'runbook_command_check=passed\n'
  printf 'release_block_status=blocked-while-alpha-remains\n'
  printf 'alpha_feature_scope=explicit\n'
  printf 'rollback_checkpoint=required-before-promotion\n'
  printf 'owner_signoff=required-before-promotion\n'
} >"${release_record}"

for field in \
  source_revision \
  image_digest_manifest \
  production_readiness_audit \
  production_gap_audit \
  docs_evidence_boundary_audit \
  runbook_command_check \
  release_block_status \
  alpha_feature_scope \
  rollback_checkpoint \
  owner_signoff
do
  grep -Eq "^${field}=" "${release_record}"
done

printf 'release_hardening_runbook=passed\n'
printf 'required_gates=19\n'
printf 'release_record_fields=10\n'
printf 'production_release_blocked=true\n'
printf 'owner_signoff_required=true\n'
printf 'rollback_evidence_required=true\n'
printf 'production_gap_audit_required=true\n'
printf 'runbook_command_check_required=true\n'
printf 'release_record_source_revision=%s\n' "${source_revision}"
