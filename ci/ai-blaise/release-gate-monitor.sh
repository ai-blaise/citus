#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
usage: ci/ai-blaise/release-gate-monitor.sh [--local-only] [--pr <number-or-url>] [--watch] [--interval <seconds>]

Runs the ai-blaise release/integration gate monitor. Local checks are bounded
and deterministic: they audit production-readiness wording, evidence fields,
stale V2 command counts, workflow coverage, benchmark formatting hooks, and
Docker/Postgres readiness guardrails. With --pr, the script also summarizes
GitHub check runs so broad matrix monitoring can happen in parallel with other
work; add --watch to wait for completion.
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

run_static_audit() {
  python3 <<'PY_AUDIT'
import pathlib
import re
import sys

ROOT = pathlib.Path(".")
DOCS = ROOT / "docs/ai-blaise/NEW_FEATURES.md"
AUDIT = ROOT / "docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md"
RELEASING = ROOT / "docs/ai-blaise/RELEASING.md"
MONITOR_DOC = ROOT / "docs/ai-blaise/RELEASE_GATE_MONITOR.md"
IMAGE_CHECK = ROOT / "ci/ai-blaise/image-check.sh"
BENCH_WORKFLOW = ROOT / ".github/workflows/ci-bench-smoke.yml"
MONITOR_WORKFLOW = ROOT / ".github/workflows/ci-release-gate-monitor.yml"
PROD_WORKFLOW = ROOT / ".github/workflows/ci-production-readiness.yml"
V2_CLOSURE = ROOT / "ci/ai-blaise/v2-closure-check.sh"
DOMAIN_CONTRACTS = ROOT / "companion/src/domain_contracts.rs"
TIMESCALE_INGEST = ROOT / "benchmarks/timescale-ingest/ingest.py"
MAKEFILE = ROOT / "Makefile.ai-blaise"

SOURCE_ROOTS = [
    "companion",
    "sidecar",
    "pool",
    "operator",
    "e2e",
    "tools",
    "patches",
    "images",
    "scripts",
]
PRODUCTION_STATUSES = {"ga", "stable", "production", "production-ready", "production ready"}
EVIDENCE_MARKERS = (
    "ci/ai-blaise/",
    "cargo test",
    "cargo run",
    "GitHub Actions",
    "VM proof",
    "Docker",
    "SQL runtime",
    "REQUIRE_DOCKER=1",
    "PR #",
    "local and VM verification",
)
GLOBAL_OVERCLAIMS = (
    "full plan is production-ready",
    "entire plan is production-ready",
    "all custom features are production-ready",
    "production certified by v2-acceptance",
    "v2 acceptance proves production",
)
ALPHA_OVERCLAIMS = (
    "production release eligible",
    "stable production workload",
    "fully production-ready",
    "GA-ready",
    "production certified",
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


def source_text() -> str:
    chunks = []
    for root_name in SOURCE_ROOTS:
        root = ROOT / root_name
        if not root.exists():
            continue
        for path in root.rglob("*"):
            if not path.is_file():
                continue
            if ".git" in path.parts or "target" in path.parts:
                continue
            chunks.append(path.read_text(encoding="utf-8", errors="ignore"))
    return "\n".join(chunks)


def feature_sections(docs: str):
    heading_re = re.compile(r"^###\s+([A-Za-z][A-Za-z0-9]*):\s+(.+)$", re.M)
    status_re = re.compile(r"^\*\*Status\*\*:\s*(.+)$", re.M)
    headings = list(heading_re.finditer(docs))
    for index, heading in enumerate(headings):
        end = headings[index + 1].start() if index + 1 < len(headings) else len(docs)
        body = docs[heading.start():end]
        status_match = status_re.search(body)
        yield {
            "id": heading.group(1),
            "title": heading.group(2).strip(),
            "status": status_match.group(1).strip() if status_match else "",
            "body": body,
        }


docs = read(DOCS)
audit = read(AUDIT)
releasing = read(RELEASING)
monitor_doc = read(MONITOR_DOC)
image_check = read(IMAGE_CHECK)
bench_workflow = read(BENCH_WORKFLOW)
monitor_workflow = read(MONITOR_WORKFLOW)
prod_workflow = read(PROD_WORKFLOW)
v2_closure = read(V2_CLOSURE)
domain_contracts = read(DOMAIN_CONTRACTS)
ingest = read(TIMESCALE_INGEST)
makefile = read(MAKEFILE)
source = source_text()

source_ids = set(re.findall(r"FEATURE:\s+([A-Za-z][A-Za-z0-9]*)", source))
doc_refs = set(re.findall(r"FEATURE:\s+([A-Za-z][A-Za-z0-9]*)", docs))
sections = list(feature_sections(docs))
section_ids = [section["id"] for section in sections]

if not source_ids:
    fail("no source FEATURE markers found")
if source_ids - doc_refs:
    fail("source FEATURE markers missing docs references: " + ", ".join(sorted(source_ids - doc_refs)))
if doc_refs - source_ids:
    fail("NEW_FEATURES.md references unknown source FEATURE ids: " + ", ".join(sorted(doc_refs - source_ids)))
if len(section_ids) != len(set(section_ids)):
    duplicates = sorted({feature_id for feature_id in section_ids if section_ids.count(feature_id) > 1})
    fail("duplicate feature headings: " + ", ".join(duplicates))

production_sections = [s for s in sections if s["status"].lower() in PRODUCTION_STATUSES]
alpha_sections = [s for s in sections if s["status"].lower() == "alpha"]
missing_status = [s["id"] for s in sections if not s["status"]]
if missing_status:
    fail("feature headings missing Status: " + ", ".join(sorted(missing_status)))

missing_production_evidence = [s["id"] for s in production_sections if "Production evidence:" not in s["body"]]
if missing_production_evidence:
    fail("production-ready headings missing Production evidence: " + ", ".join(sorted(missing_production_evidence)))

missing_executable_evidence = [
    s["id"]
    for s in production_sections
    if not any(marker in s["body"] for marker in EVIDENCE_MARKERS)
]
if missing_executable_evidence:
    fail("production-ready headings missing executable/CI/VM evidence markers: " + ", ".join(sorted(missing_executable_evidence)))

alpha_with_evidence = [s["id"] for s in alpha_sections if "Production evidence:" in s["body"]]
if alpha_with_evidence:
    fail("alpha headings must not carry Production evidence fields: " + ", ".join(sorted(alpha_with_evidence)))

for section in alpha_sections:
    body = compact(section["body"])
    for phrase in ALPHA_OVERCLAIMS:
        if compact(phrase) in body:
            fail(f"alpha heading {section['id']} contains production overclaim phrase: {phrase}")

for path, text in (
    (DOCS, docs),
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
    ("TO5,TS13", v2_closure, V2_CLOSURE),
    ("\\t22\\t11\\t51", v2_closure, V2_CLOSURE),
    ("assert_eq!(report.command_count, 51)", domain_contracts, DOMAIN_CONTRACTS),
    ("black --check benchmarks/timescale-ingest/ingest.py", bench_workflow, BENCH_WORKFLOW),
    ("custom_http_probe_paths", image_check, IMAGE_CHECK),
    ("PostgreSQL init process complete", image_check, IMAGE_CHECK),
    ("docker exec -i", image_check, IMAGE_CHECK),
    ("ci/ai-blaise/release-gate-monitor.sh", monitor_workflow, MONITOR_WORKFLOW),
    ("release-gate-monitor", prod_workflow + makefile, pathlib.Path("ci workflow/Makefile")),
):
    if needle not in text:
        fail(f"{path} missing release monitor baseline: {needle}")

for phrase in ("toy implementation", "toy runtime", "toy-only", "placeholder-only"):
    if phrase in compact(docs):
        fail(f"NEW_FEATURES.md still contains toy/placeholder overclaim wording: {phrase}")

print(
    "release_gate_monitor_static\t"
    f"source_feature_ids={len(source_ids)}\t"
    f"feature_headings={len(sections)}\t"
    f"production_ready={len(production_sections)}\t"
    f"alpha={len(alpha_sections)}\t"
    "v2_domain_commands=51\t"
    "production_release_overclaim_guard=true"
)
PY_AUDIT
}

run_runtime_baselines() {
  local domain_expected=$'38\tA1,API4,Auth2,G2,G3,Geo2,Geo3,IA3,JS2,L9,M1,M11,M13,M2,M7,PM3,PM4,S13,S14,S6,Search2,Search3,Search9,Sec1,Sec2,Sec5,Sec6,T8,TO3,TO4,TO5,TS13,TS14,TS15,TS16,TS17,TS9,WH2\t22\t11\t51'
  local domain_output
  domain_output="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical)"
  if ! printf '%s\n' "${domain_output}" | grep -Fqx "${domain_expected}"; then
    echo "expected domain contract row: ${domain_expected}" >&2
    echo "actual domain contract output:" >&2
    printf '%s\n' "${domain_output}" >&2
    fail "stale V2 domain command count"
  fi

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
  printf 'release_gate_monitor_runtime\tv2_domain_commands=51\tbenchmark_py_compile=ok\tshell_syntax=ok\n'
}

monitor_pr_checks_once() {
  local pr="$1"
  if ! command -v gh >/dev/null 2>&1; then
    fail "gh is required for --pr monitoring"
  fi

  local checks_payload
  if checks_payload="$(gh pr checks "${pr}" --json name,state,conclusion,workflow,detailsUrl 2>/dev/null)"; then
    CHECKS_FORMAT=json python3 - "${checks_payload}" <<'PY_CHECKS'
import json
import os
import sys

failures = []
pending = []
passes = []

if os.environ.get("CHECKS_FORMAT") == "json":
    checks = json.loads(sys.argv[1])
    for check in checks:
        name = check.get("name") or "unknown"
        state = (check.get("state") or "").upper()
        conclusion = (check.get("conclusion") or "").upper()
        workflow = check.get("workflow") or ""
        url = check.get("detailsUrl") or ""
        label = f"{workflow}/{name}" if workflow else name
        if conclusion in {"FAILURE", "CANCELLED", "TIMED_OUT", "ACTION_REQUIRED"} or state == "FAILURE":
            failures.append((label, conclusion or state, url))
        elif state in {"PENDING", "QUEUED", "IN_PROGRESS", "REQUESTED", "WAITING"} or not conclusion:
            pending.append((label, state or conclusion or "UNKNOWN", url))
        else:
            passes.append((label, conclusion or state, url))

print(f"release_gate_monitor_pr_checks\tpass={len(passes)}\tpending={len(pending)}\tfail={len(failures)}")
for label, status, url in failures[:30]:
    print(f"FAIL\t{status}\t{label}\t{url}")
for label, status, url in pending[:30]:
    print(f"PENDING\t{status}\t{label}\t{url}")
if failures:
    sys.exit(2)
if pending:
    sys.exit(3)
PY_CHECKS
  else
    checks_payload="$(gh pr checks "${pr}" 2>/dev/null || true)"
    if [[ -z "${checks_payload}" ]]; then
      fail "unable to read PR checks for ${pr}"
    fi
    CHECKS_FORMAT=table python3 - "${checks_payload}" <<'PY_CHECKS'
import sys

failures = []
pending = []
passes = []
for raw in sys.argv[1].splitlines():
    if not raw.strip():
        continue
    parts = raw.split("\t")
    if len(parts) < 2:
        parts = raw.split(None, 3)
    name = parts[0]
    status = parts[1].lower() if len(parts) > 1 else "unknown"
    url = parts[3] if len(parts) > 3 else ""
    if status in {"fail", "failure", "cancelled", "timed_out", "action_required"}:
        failures.append((name, status, url))
    elif status in {"pending", "queued", "in_progress", "waiting", "requested"}:
        pending.append((name, status, url))
    else:
        passes.append((name, status, url))

print(f"release_gate_monitor_pr_checks\tpass={len(passes)}\tpending={len(pending)}\tfail={len(failures)}")
for label, status, url in failures[:30]:
    print(f"FAIL\t{status}\t{label}\t{url}")
for label, status, url in pending[:30]:
    print(f"PENDING\t{status}\t{label}\t{url}")
if failures:
    sys.exit(2)
if pending:
    sys.exit(3)
PY_CHECKS
  fi

}

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
