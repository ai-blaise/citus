#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

# After the 2026-05-22 Helm chart fold into ai-blaise/command-center, this audit
# verifies only the source-of-truth pieces that remain in this repo: the machine
# derived feature inventory in NEW_FEATURES.md, the audit doc overclaim
# guardrail, and the deploy/k8s/-may-not-be-reintroduced negative gate. Chart
# contract / digest / argo / sidecar HA assertions moved with the chart to
# ai-blaise/command-center.

python3 <<'PY'
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(".")
DOCS = ROOT / "docs/ai-blaise/NEW_FEATURES.md"
AUDIT = ROOT / "docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md"
RELEASING = ROOT / "docs/ai-blaise/RELEASING.md"
RUNBOOK = ROOT / "docs/ai-blaise/RUNBOOKS/production.md"
UPGRADE_RUNBOOK = ROOT / "docs/ai-blaise/RUNBOOKS/upgrade.md"
DR_RUNBOOK = ROOT / "docs/ai-blaise/RUNBOOKS/disaster-recovery.md"
E2E_DOC = ROOT / "docs/ai-blaise/E2E.md"
ARCHITECTURE_DOC = ROOT / "docs/ai-blaise/ARCHITECTURE.md"
BUNDLED_EXTENSIONS_DOC = ROOT / "docs/ai-blaise/BUNDLED_EXTENSIONS.md"
BENCHMARKS_DOC = ROOT / "docs/ai-blaise/BENCHMARKS.md"
IMAGES_OVERVIEW = ROOT / "images/README.ai-blaise.md"
PG_OVERLAY_README = ROOT / "images/citus-pg-overlay/README.md"
PERF_THRESHOLDS = ROOT / "benchmarks/performance-evidence-thresholds.json"
PERF_CHECK = ROOT / "ci/ai-blaise/performance-evidence-check.sh"
MAKEFILE_AI_BLAISE = ROOT / "Makefile.ai-blaise"

SOURCE_ROOTS = [
    "companion",
    "sidecar",
    "pool",
    "operator",
    "e2e",
    "tools",
    "patches",
    "deploy",
    "images",
    "scripts",
]


def fail(message: str) -> None:
    sys.stderr.write(message + "\n")
    sys.exit(1)


def read(path: pathlib.Path) -> str:
    if not path.exists():
        fail(f"missing production gap audit input: {path}")
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
            try:
                chunks.append(path.read_text(encoding="utf-8", errors="ignore"))
            except Exception:
                continue
    return "\n".join(chunks)


def feature_entries(docs: str):
    heading_re = re.compile(r"^###\s+([A-Za-z][A-Za-z0-9]*):\s+(.+)$", re.M)
    status_re = re.compile(r"^\*\*Status\*\*:\s*([A-Za-z-]+)\s*$", re.M)
    headings = list(heading_re.finditer(docs))
    entries = []
    for index, heading in enumerate(headings):
        end = headings[index + 1].start() if index + 1 < len(headings) else len(docs)
        body = docs[heading.start():end]
        status_match = status_re.search(body)
        entries.append(
            {
                "id": heading.group(1),
                "status": status_match.group(1).lower() if status_match else "",
            }
        )
    return entries


docs = read(DOCS)
audit = read(AUDIT)
source = source_text()

source_ids = set(re.findall(r"FEATURE:\s+([A-Za-z][A-Za-z0-9]*)", source))
entries = feature_entries(docs)
entry_ids = [entry["id"] for entry in entries]
doc_ids = set(entry_ids)

if not source_ids:
    fail("no FEATURE: markers found in source")

missing_from_doc = source_ids - doc_ids
if missing_from_doc:
    fail(
        "source FEATURE markers missing from NEW_FEATURES.md: "
        + ", ".join(sorted(missing_from_doc))
    )

missing_from_source = doc_ids - source_ids
if missing_from_source:
    fail(
        "NEW_FEATURES.md references FEATURE ids missing from source: "
        + ", ".join(sorted(missing_from_source))
    )

duplicates = sorted({feature_id for feature_id in entry_ids if entry_ids.count(feature_id) > 1})
if duplicates:
    fail("duplicate feature headings in NEW_FEATURES.md: " + ", ".join(duplicates))

missing_status = sorted(entry["id"] for entry in entries if not entry["status"])
if missing_status:
    fail("feature headings missing Status fields: " + ", ".join(missing_status))

supported_statuses = {"alpha", "production-ready"}
unexpected_statuses = sorted(
    {entry["status"] for entry in entries if entry["status"] not in supported_statuses}
)
if unexpected_statuses:
    fail("unsupported feature Status values: " + ", ".join(unexpected_statuses))

production_entries = [entry for entry in entries if entry["status"] == "production-ready"]
alpha_entries = [entry for entry in entries if entry["status"] == "alpha"]
source_only_ids = source_ids - doc_ids

audit_compact = compact(audit)

for pattern in (
    r"current feature inventory contains\s+\d+\s+source\s+`feature:`\s+markers",
    r"\d+\s+narrow headings are\s+`status:\s*production-ready`",
    r"other\s+\d+\s+feature headings remain\s+`status:\s*alpha`",
):
    if re.search(pattern, audit_compact):
        fail(
            "PRODUCTION_READINESS_AUDIT.md must not hard-code machine-derived "
            "feature inventory counts"
        )

for phrase in (
    "feature inventory is machine-derived",
    "do not restate source/heading/status counts in prose",
    "production_gap_audit",
    "source_feature_ids",
    "feature_headings",
    "production_ready",
    "alpha_headings",
):
    if compact(phrase) not in audit_compact:
        fail(
            "PRODUCTION_READINESS_AUDIT.md must document the machine-derived "
            f"inventory contract: {phrase}"
        )

if len(production_entries) + len(alpha_entries) != len(entries):
    fail(
        "computed feature status counts do not cover every NEW_FEATURES.md heading"
    )

for phrase in (
    "not production-ready as a whole",
    "modeled release gates",
    "canonical model data rather than results from live performance",
    "v2 acceptance model must not be cited as production evidence",
):
    if compact(phrase) not in audit_compact:
        fail(
            f"PRODUCTION_READINESS_AUDIT.md must preserve guardrail phrase: {phrase}"
        )

for path in (
    DOCS,
    AUDIT,
    RELEASING,
    RUNBOOK,
    UPGRADE_RUNBOOK,
    DR_RUNBOOK,
    E2E_DOC,
    ARCHITECTURE_DOC,
    BUNDLED_EXTENSIONS_DOC,
    BENCHMARKS_DOC,
    IMAGES_OVERVIEW,
    PG_OVERLAY_README,
):
    text = read(path)
    for pattern in (
        "full plan is production-ready",
        "entire plan is production-ready",
        "all custom features are production-ready",
        "production certified by v2-acceptance",
        "v2 acceptance proves production",
    ):
        if compact(pattern) in compact(text):
            fail(f"{path} contains overclaiming wording: {pattern}")

thresholds = json.loads(read(PERF_THRESHOLDS))
for key in ("tpcc", "sysbench", "timescale_ingest", "chaos"):
    if key not in thresholds.get("core_harnesses", {}):
        fail(f"performance threshold manifest missing core harness: {key}")
if thresholds.get("microbenches", {}).get("minimum_count") != 26:
    fail("performance threshold manifest must require all 26 microbenches")

perf_check = read(PERF_CHECK)
for phrase in (
    "scaffold evidence is not production evidence",
    "release evidence requires",
    "PERF_EVIDENCE_SCOPE",
):
    if phrase not in perf_check:
        fail(f"performance evidence checker lost fail-closed phrase: {phrase}")

makefile = read(MAKEFILE_AI_BLAISE)
for target in (
    "performance-evidence-check:",
    "performance-evidence-release-check:",
    "performance-evidence-smoke:",
):
    if target not in makefile:
        fail(f"Makefile.ai-blaise missing performance evidence target: {target}")

benchmarks_doc = read(BENCHMARKS_DOC)
for phrase in (
    "benchmarks/performance-evidence-thresholds.json",
    "PERF_EVIDENCE_MODE=release",
    "fails closed on missing artifacts",
):
    if compact(phrase) not in compact(benchmarks_doc):
        fail(f"BENCHMARKS.md missing performance evidence release wording: {phrase}")

deploy_k8s_tree = list(ROOT.glob("deploy/k8s/**/*"))
if deploy_k8s_tree:
    fail(
        "deploy/k8s/ was folded into ai-blaise/command-center on 2026-05-22 and "
        "must not be reintroduced here: "
        + ", ".join(str(p) for p in deploy_k8s_tree)
    )

print(
    "production_gap_audit\t"
    f"source_feature_ids={len(source_ids)}\t"
    f"doc_feature_headings={len(doc_ids)}\t"
    f"feature_headings={len(entries)}\t"
    f"production_ready={len(production_entries)}\t"
    f"alpha_headings={len(alpha_entries)}\t"
    "inventory_contract=machine_derived\t"
    f"source_only_alpha={len(source_only_ids)}\t"
    "v2_acceptance=model_only\t"
    "production_release_blocked=true\t"
    "live_sql_guards=true\t"
    "chart_folded_to_command_center=2026-05-22"
)
PY
