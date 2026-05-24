#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

# After the 2026-05-22 Helm chart fold into ai-blaise/command-center, this
# audit verifies only the source-of-truth pieces that remain in this repo:
# the feature inventory in NEW_FEATURES.md vs PRODUCTION_READINESS_AUDIT.md,
# the doc overclaim guardrail, and the deploy/k8s/-may-not-be-reintroduced
# negative gate. Chart contract / digest / argo / sidecar HA assertions
# moved with the chart to ai-blaise/command-center.

python3 <<'PY'
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
PITR_RUNBOOK = ROOT / "docs/ai-blaise/RUNBOOKS/pitr-restore.md"
E2E_DOC = ROOT / "docs/ai-blaise/E2E.md"
ARCHITECTURE_DOC = ROOT / "docs/ai-blaise/ARCHITECTURE.md"
BUNDLED_EXTENSIONS_DOC = ROOT / "docs/ai-blaise/BUNDLED_EXTENSIONS.md"
IMAGES_OVERVIEW = ROOT / "images/README.ai-blaise.md"
PG_OVERLAY_README = ROOT / "images/citus-pg-overlay/README.md"
MAKEFILE = ROOT / "Makefile.ai-blaise"
DR_RESTORE_DEPTH_CHECK = ROOT / "ci/ai-blaise/dr-restore-depth-check.sh"

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
    return [m.group(1) for m in heading_re.finditer(docs)]


def number_forms(n: int):
    return (str(n),)


docs = read(DOCS)
audit = read(AUDIT)
makefile = read(MAKEFILE)
dr_runbook = read(DR_RUNBOOK)
pitr_runbook = read(PITR_RUNBOOK)
dr_restore_depth_check = read(DR_RESTORE_DEPTH_CHECK)
source = source_text()

source_ids = set(re.findall(r"FEATURE:\s+([A-Za-z][A-Za-z0-9]*)", source))
entries = feature_entries(docs)
doc_ids = set(entries)

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

statuses = [
    m.group(1).lower()
    for m in re.finditer(r"\*\*Status\*\*:\s*([a-zA-Z-]+)", docs)
]
production_entries = [s for s in statuses if s == "production-ready"]
alpha_entries = [s for s in statuses if s == "alpha"]
source_only_ids = source_ids - doc_ids

audit_compact = compact(audit)

expected_inventory = compact(
    f"contains {len(source_ids)} source `feature:` markers and {len(entries)} "
    "feature headings"
)
if expected_inventory not in audit_compact:
    fail(
        "PRODUCTION_READINESS_AUDIT.md must report the current computed feature inventory"
    )

expected_production_counts = [
    compact(f"{form} narrow headings are `status: production-ready`")
    for form in number_forms(len(production_entries))
]
if not any(phrase in audit_compact for phrase in expected_production_counts):
    fail(
        "PRODUCTION_READINESS_AUDIT.md must report the current production-ready heading count"
    )

expected_alpha_count = compact(
    f"other {len(alpha_entries)} feature headings remain `status: alpha`"
)
if expected_alpha_count not in audit_compact:
    fail(
        "PRODUCTION_READINESS_AUDIT.md must report the current alpha heading count"
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
    PITR_RUNBOOK,
    E2E_DOC,
    ARCHITECTURE_DOC,
    BUNDLED_EXTENSIONS_DOC,
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

if not (DR_RESTORE_DEPTH_CHECK.stat().st_mode & 0o111):
    fail("ci/ai-blaise/dr-restore-depth-check.sh must be executable")

for phrase in (
    "dr-restore-depth-check:",
    "REQUIRE_DOCKER=1 ci/ai-blaise/dr-restore-depth-check.sh",
    "gate-close:",
    "dr-restore-depth-check",
):
    if phrase not in makefile:
        fail(f"Makefile.ai-blaise must wire the DR restore-depth gate: {phrase}")

for phrase in (
    "cargo test -q -p ai_blaise_citus_e2e dr_restore_depth",
    "dr_restore_depth_report",
    "pg_basebackup",
    "archive_command",
    "recovery_target_time",
    "pg_switch_wal",
    "dr_restore_depth_postgres_smoke",
):
    if phrase not in dr_restore_depth_check:
        fail(f"DR restore-depth check must preserve executable evidence: {phrase}")

for phrase in (
    "ci/ai-blaise/dr-restore-depth-check.sh",
    "fail-closed",
    "WAL archive continuity",
    "PITR evidence",
    "dr_restore_depth_postgres_smoke",
):
    if compact(phrase) not in compact(pitr_runbook):
        fail(f"pitr-restore.md must document DR restore-depth evidence: {phrase}")

for phrase in (
    "ci/ai-blaise/dr-restore-depth-check.sh",
    "read-only branch",
    "WAL archive continuity",
    "PostgreSQL PITR smoke",
    "not production evidence by itself",
):
    if compact(phrase) not in compact(dr_runbook):
        fail(f"disaster-recovery.md must document DR restore-depth gate: {phrase}")

for phrase in (
    "restore-depth gate",
    "ci/ai-blaise/dr-restore-depth-check.sh",
    "PostgreSQL PITR smoke",
):
    if compact(phrase) not in audit_compact:
        fail(f"PRODUCTION_READINESS_AUDIT.md must mention DR restore-depth correction: {phrase}")

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
    f"feature_headings={len(entries)}\t"
    f"production_ready={len(production_entries)}\t"
    f"alpha_headings={len(alpha_entries)}\t"
    f"source_only_alpha={len(source_only_ids)}\t"
    "v2_acceptance=model_only\t"
    "production_release_blocked=true\t"
    "live_sql_guards=true\t"
    "chart_folded_to_command_center=2026-05-22"
)
PY
