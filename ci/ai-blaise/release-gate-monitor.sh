#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
usage: ci/ai-blaise/release-gate-monitor.sh [--local-only] [--pr <number-or-url>] [--watch] [--interval <seconds>]

Runs the ai-blaise release/integration gate monitor. Local checks are bounded
and deterministic: they validate the structural feature inventory, confirm
that release qualification remains fail-closed, audit release wording and
workflow coverage, and check benchmark and Docker/Postgres guardrails. With
--pr, the script also summarizes GitHub check runs so broad matrix monitoring
can happen in parallel with other work; add --watch to wait for completion.
USAGE
}

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

pr_ref=""
watch_checks=0
interval=60
local_only=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --local-only)
      local_only=1
      shift
      ;;
    --pr)
      pr_ref="${2:-}"
      if [[ -z "${pr_ref}" ]]; then
        echo "--pr requires a PR number or URL" >&2
        exit 2
      fi
      shift 2
      ;;
    --watch)
      watch_checks=1
      shift
      ;;
    --interval)
      interval="${2:-}"
      if ! [[ "${interval}" =~ ^[0-9]+$ ]] || [[ "${interval}" -lt 5 ]]; then
        echo "--interval must be an integer >= 5" >&2
        exit 2
      fi
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

fail() {
  echo "release-gate-monitor: $*" >&2
  exit 1
}

run_feature_register_audit() {
  local check_output
  if ! check_output="$(cargo run --locked --quiet -p ai_blaise_feature_register -- check)"; then
    fail "feature register structural validation failed"
  fi
  if ! grep -Fqx $'feature_register_check\tpassed' <<<"${check_output}"; then
    fail "feature register check did not emit its structural success marker"
  fi

  local summary_output
  if ! summary_output="$(cargo run --locked --quiet -p ai_blaise_feature_register -- summary)"; then
    fail "feature register structural summary failed"
  fi
  if ! grep -Fqx $'feature_register_summary\tvalid' <<<"${summary_output}"; then
    fail "feature register summary did not emit its structural validity marker"
  fi

  local register_rows
  register_rows="$(awk -F $'\t' '$1 == "rows" && NF == 2 { print $2 }' <<<"${summary_output}")"
  if ! [[ "${register_rows}" =~ ^[1-9][0-9]*$ ]]; then
    fail "feature register summary must contain exactly one positive row count"
  fi

  local release_output release_status
  set +e
  release_output="$(cargo run --locked --quiet -p ai_blaise_feature_register -- release-gaps)"
  release_status=$?
  set -e
  if [[ "${release_status}" != "1" ]]; then
    fail "release-gaps must return unqualified exit 1, got ${release_status}"
  fi

  local expected_columns
  expected_columns=$'feature_release_gap_columns\tid\ttitle\tkind\tdisposition\tcanonical_ids\tmaturity\thistorical_claimed_status\tscope\tblockers'
  if [[ "$(sed -n '1p' <<<"${release_output}")" != $'feature_release_gaps\tblocked' ]] \
    || [[ "$(sed -n '2p' <<<"${release_output}")" != $'release_evidence_verifier\tunimplemented' ]] \
    || [[ "$(sed -n '3p' <<<"${release_output}")" != "${expected_columns}" ]]; then
    fail "release-gaps lost its fail-closed report contract"
  fi

  local release_rows invalid_release_rows
  release_rows="$(awk -F $'\t' '$1 == "feature_release_gap" { count++ } END { print count + 0 }' <<<"${release_output}")"
  invalid_release_rows="$(awk -F $'\t' 'NR > 3 && ($1 != "feature_release_gap" || NF != 10) { count++ } END { print count + 0 }' <<<"${release_output}")"
  if [[ "${release_rows}" != "${register_rows}" ]] || [[ "${invalid_release_rows}" != "0" ]]; then
    fail "release-gaps must preserve every validated feature as one ten-field blocked record"
  fi

  printf 'release_gate_monitor_inventory\trows=%s\tvalidation=passed\trelease_qualification=blocked\trelease_evidence_verifier=unimplemented\n' "${register_rows}"
}

run_static_audit() {
  python3 <<'PY_AUDIT'
import pathlib
import re
import sys

ROOT = pathlib.Path(".")
FEATURE_REGISTER = ROOT / "docs/features.tsv"
AUDIT = ROOT / "docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md"
RELEASING = ROOT / "docs/ai-blaise/RELEASING.md"
MONITOR_DOC = ROOT / "docs/ai-blaise/RELEASE_GATE_MONITOR.md"
IMAGE_CHECK = ROOT / "ci/ai-blaise/image-check.sh"
BENCH_WORKFLOW = ROOT / ".github/workflows/ci-bench-smoke.yml"
MONITOR_WORKFLOW = ROOT / ".github/workflows/ci-release-gate-monitor.yml"
PROD_WORKFLOW = ROOT / ".github/workflows/ci-production-readiness.yml"
MAKEFILE = ROOT / "Makefile.ai-blaise"

GLOBAL_OVERCLAIMS = (
    "full plan is production-ready",
    "entire plan is production-ready",
    "all custom features are production-ready",
    "production certified by v2-acceptance",
    "v2 acceptance proves production",
)
def fail(message: str) -> None:
    print(message, file=sys.stderr)
    sys.exit(1)


def read(path: pathlib.Path) -> str:
    if not path.exists() or not path.is_file():
        fail(f"missing release gate monitor input: {path}")
    return path.read_text(encoding="utf-8", errors="ignore")


def compact(text: str) -> str:
    return " ".join(text.split()).lower()


feature_register = read(FEATURE_REGISTER)
audit = read(AUDIT)
releasing = read(RELEASING)
monitor_doc = read(MONITOR_DOC)
image_check = read(IMAGE_CHECK)
bench_workflow = read(BENCH_WORKFLOW)
monitor_workflow = read(MONITOR_WORKFLOW)
prod_workflow = read(PROD_WORKFLOW)
makefile = read(MAKEFILE)

for path, text in (
    (FEATURE_REGISTER, feature_register),
    (AUDIT, audit),
    (RELEASING, releasing),
    (MONITOR_DOC, monitor_doc),
):
    body = compact(text)
    for phrase in GLOBAL_OVERCLAIMS:
        if compact(phrase) in body:
            fail(f"{path} contains release overclaim wording: {phrase}")

for phrase in (
    "not production-ready as a whole",
    "modeled release gates",
    "v2 acceptance model must not be cited as production evidence",
    "parallel matrix monitoring",
):
    if compact(phrase) not in compact(audit + "\n" + releasing + "\n" + monitor_doc):
        fail(f"release docs must preserve guardrail phrase: {phrase}")

for needle, text, path in (
    ("black --check benchmarks/timescale-ingest/ingest.py", bench_workflow, BENCH_WORKFLOW),
    ("custom_http_probe_paths", image_check, IMAGE_CHECK),
    ("PostgreSQL init process complete", image_check, IMAGE_CHECK),
    ("docker exec -i", image_check, IMAGE_CHECK),
    ("ci/ai-blaise/release-gate-monitor.sh", monitor_workflow, MONITOR_WORKFLOW),
    ("release-gate-monitor_test.py", monitor_workflow, MONITOR_WORKFLOW),
    ("release-gate-monitor", prod_workflow + makefile, pathlib.Path("ci workflow/Makefile")),
):
    if needle not in text:
        fail(f"{path} missing release monitor baseline: {needle}")

print(
    "release_gate_monitor_static\t"
    "feature_inventory=structurally_validated\t"
    "release_qualification=blocked\t"
    "release_evidence_verifier=unimplemented\t"
    "production_release_overclaim_guard=true"
)
PY_AUDIT
}

run_runtime_baselines() {
  python3 -m py_compile benchmarks/timescale-ingest/ingest.py
  if command -v black >/dev/null 2>&1; then
    black --check benchmarks/timescale-ingest/ingest.py
  elif python3 -m black --version >/dev/null 2>&1; then
    python3 -m black --check benchmarks/timescale-ingest/ingest.py
  elif [[ "${REQUIRE_BLACK:-0}" == "1" ]]; then
    fail "black is required but not installed"
  else
    echo "release-gate-monitor: black not installed; CI workflow installs it and enforces benchmark formatting"
  fi

  bash -n ci/ai-blaise/image-check.sh
  bash -n ci/ai-blaise/production-readiness-check.sh
  bash -n ci/ai-blaise/production-gap-audit.sh
  bash -n ci/ai-blaise/v2-closure-check.sh
  bash -n ci/ai-blaise/v2-acceptance-check.sh
  printf 'release_gate_monitor_runtime\tbenchmark_py_compile=ok\tshell_syntax=ok\n'
}

monitor_pr_checks_once() {
  local pr="$1"
  if ! command -v gh >/dev/null 2>&1; then
    fail "gh is required for --pr monitoring"
  fi

  local checks_payload checks_status
  if checks_payload="$(gh pr checks --json name,state,bucket,workflow,link -- "${pr}" 2>/dev/null)"; then
    checks_status=0
  else
    checks_status=$?
  fi
  case "${checks_status}" in
    0|1|8)
      ;;
    *)
      fail "unable to read PR checks for ${pr}"
      ;;
  esac

  python3 - "${checks_status}" 3<<<"${checks_payload}" <<'PY_CHECKS'
import json
import os
import sys

FIELDS = {"name", "state", "bucket", "workflow", "link"}
STATE_BUCKETS = {
    "SUCCESS": "pass",
    "SKIPPED": "skipping",
    "NEUTRAL": "skipping",
    "ERROR": "fail",
    "FAILURE": "fail",
    "TIMED_OUT": "fail",
    "ACTION_REQUIRED": "fail",
    "CANCELLED": "cancel",
    "EXPECTED": "pending",
    "REQUESTED": "pending",
    "WAITING": "pending",
    "QUEUED": "pending",
    "PENDING": "pending",
    "IN_PROGRESS": "pending",
    "STALE": "pending",
}


def reject(message: str) -> None:
    print(f"release-gate-monitor: invalid PR checks response: {message}", file=sys.stderr)
    sys.exit(4)


try:
    with os.fdopen(3, encoding="utf-8") as checks_stream:
        checks = json.load(checks_stream)
except (json.JSONDecodeError, OSError, TypeError, UnicodeError):
    reject("malformed JSON")

if not isinstance(checks, list) or not checks:
    reject("expected a nonempty check list")

checks_status = int(sys.argv[1])

grouped = {
    bucket: [] for bucket in ("pass", "pending", "skipping", "fail", "cancel")
}
for index, check in enumerate(checks, start=1):
    if not isinstance(check, dict):
        reject(f"check {index} is not an object")
    if set(check) != FIELDS:
        reject(f"check {index} has an unexpected field set")
    if any(not isinstance(check[field], str) for field in FIELDS):
        reject(f"check {index} contains a non-string field")
    if any(
        any(ord(character) < 32 or ord(character) == 127 for character in check[field])
        for field in FIELDS
    ):
        reject(f"check {index} contains a control character")

    name = check["name"]
    state = check["state"]
    bucket = check["bucket"]
    workflow = check["workflow"]
    link = check["link"]
    if not name or not state or not bucket:
        reject(f"check {index} is missing its name, state, or bucket")
    expected_bucket = STATE_BUCKETS.get(state)
    if expected_bucket is None:
        reject(f"check {index} has unknown state {state!r}")
    if bucket != expected_bucket:
        reject(f"check {index} has inconsistent state and bucket")

    label = f"{workflow}/{name}" if workflow else name
    grouped[bucket].append((label, state, link))

for records in grouped.values():
    records.sort()

if checks_status == 1 and not (grouped["fail"] or grouped["cancel"]):
    reject("CLI exit 1 did not include a failed or cancelled check")
if checks_status == 8 and not (
    grouped["pending"] or grouped["fail"] or grouped["cancel"]
):
    reject("CLI exit 8 did not include a pending or terminal check")

print(
    "release_gate_monitor_pr_checks"
    f"\tpass={len(grouped['pass'])}"
    f"\tpending={len(grouped['pending'])}"
    f"\tskipping={len(grouped['skipping'])}"
    f"\tfail={len(grouped['fail'])}"
    f"\tcancel={len(grouped['cancel'])}"
)
for bucket, record_type in (
    ("fail", "FAIL"),
    ("cancel", "CANCEL"),
    ("pending", "PENDING"),
    ("skipping", "SKIPPING"),
):
    for label, state, link in grouped[bucket][:30]:
        print(f"{record_type}\t{state}\t{label}\t{link}")

if grouped["fail"] or grouped["cancel"]:
    sys.exit(2)
if grouped["pending"]:
    sys.exit(3)
PY_CHECKS
}

run_feature_register_audit
run_static_audit
if [[ "${RELEASE_GATE_MONITOR_STATIC:-0}" != "1" ]]; then
  run_runtime_baselines
fi

if [[ -n "${pr_ref}" && "${local_only}" -eq 0 ]]; then
  while true; do
    set +e
    monitor_pr_checks_once "${pr_ref}"
    rc=$?
    set -e
    case "${rc}" in
      0)
        break
        ;;
      2)
        exit 1
        ;;
      3)
        if [[ "${watch_checks}" -eq 1 ]]; then
          sleep "${interval}"
          continue
        fi
        exit 0
        ;;
      *)
        exit "${rc}"
        ;;
    esac
  done
fi
