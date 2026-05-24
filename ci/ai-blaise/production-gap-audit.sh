#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

# After the 2026-05-22 Helm chart fold into ai-blaise/command-center, this
# audit verifies only the source-of-truth pieces that remain in this repo:
# the feature inventory in NEW_FEATURES.md vs PRODUCTION_READINESS_AUDIT.md,
# the doc overclaim guardrail, the deploy/k8s/-may-not-be-reintroduced
# negative gate, and the external-chart live Kubernetes harness contract that
# remains in this repo. Chart object ownership / digest policy / Argo / sidecar
# HA assertions moved with the chart to ai-blaise/command-center.

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
K8S_GUARDRAIL_RENDERER = ROOT / "deploy/contracts/render_k8s_guardrails.py"
K8S_GUARDRAIL_MANIFEST = ROOT / "deploy/contracts/k8s-production-guardrails.yaml"
K8S_GUARDRAIL_KUSTOMIZATION = ROOT / "deploy/contracts/kustomization.yaml"
MAKEFILE = ROOT / "Makefile.ai-blaise"
DEPLOY_CHECK = ROOT / "ci/ai-blaise/deploy-check.sh"
K8S_GUARDRAIL_CHECK = ROOT / "ci/ai-blaise/k8s-guardrails-check.sh"
KIND_PRODUCTION_SMOKE = ROOT / "ci/ai-blaise/kind-production-smoke.sh"
LIVE_K8S_E2E = ROOT / "ci/ai-blaise/live-k8s-e2e.sh"
DEPLOY_README = ROOT / "deploy/README.md"
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
    E2E_DOC,
    PITR_RUNBOOK,
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


makefile = read(MAKEFILE)
dr_runbook = read(DR_RUNBOOK)
deploy_check = read(DEPLOY_CHECK)
kind_smoke = read(KIND_PRODUCTION_SMOKE)
live_k8s = read(LIVE_K8S_E2E)
deploy_readme = read(DEPLOY_README)

for target in ("deploy-check:", "kind-production-smoke:"):
    if target not in makefile:
        fail(f"Makefile.ai-blaise must define {target}")

for path, text in (
    (DEPLOY_CHECK, deploy_check),
    (KIND_PRODUCTION_SMOKE, kind_smoke),
    (LIVE_K8S_E2E, live_k8s),
):
    if "FEATURE:" not in text:
        fail(f"{path} must carry a FEATURE marker")

for phrase in (
    "LIVE_K8S_MODE=dry-run",
    "LIVE_K8S_MODE=real",
    "LIVE_K8S_MODE=kind",
    "CHART_DIR",
    "COMMAND_CENTER_DIR",
    "LOCAL_IMAGE_REFS",
    "AI_BLAISE_STACK_IMAGE_REF",
    "ALLOW_UNPUBLISHED_IMAGES",
    "docker manifest inspect",
    "kubectl port-forward",
    "curl -fsS",
    "psql",
    "/healthz /readyz /metrics",
    "collect_diagnostics",
    "kubectl-events.txt",
    "dry-run does not send live HTTP or SQL traffic",
):
    if phrase not in live_k8s:
        fail(f"live Kubernetes e2e harness missing required contract phrase: {phrase}")

for phrase in ("REQUIRE_CHART", "REQUIRE_HELM", "dry-run"):
    if phrase not in deploy_check:
        fail(f"deploy-check wrapper missing required contract phrase: {phrase}")

for phrase in ("REAL_K8S", "LIVE_K8S_MODE=kind", "REQUIRE_HTTP", "REQUIRE_SQL"):
    if phrase not in kind_smoke:
        fail(f"kind production smoke wrapper missing required contract phrase: {phrase}")

for phrase in (
    "ci/ai-blaise/live-k8s-e2e.sh",
    "CHART_DIR",
    "COMMAND_CENTER_DIR",
    "AI_BLAISE_STACK_IMAGE_REF",
    "LOCAL_IMAGE_REFS",
):
    if phrase not in deploy_readme:
        fail(f"deploy/README.md must document live Kubernetes e2e input: {phrase}")

if not (DR_RESTORE_DEPTH_CHECK.stat().st_mode & 0o111):
    fail("ci/ai-blaise/dr-restore-depth-check.sh must be executable")

dr_restore_depth_check = read(DR_RESTORE_DEPTH_CHECK)
pitr_runbook = read(PITR_RUNBOOK)

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

for path in (
    K8S_GUARDRAIL_RENDERER,
    K8S_GUARDRAIL_MANIFEST,
    K8S_GUARDRAIL_KUSTOMIZATION,
    DEPLOY_CHECK,
    K8S_GUARDRAIL_CHECK,
):
    if not path.exists() or not path.read_text(encoding="utf-8", errors="ignore").strip():
        fail(f"missing Kubernetes guardrail contract artifact: {path}")

guardrail_text = read(K8S_GUARDRAIL_MANIFEST)
for phrase in (
    'kind: "HorizontalPodAutoscaler"',
    'kind: "PodDisruptionBudget"',
    'kind: "NetworkPolicy"',
    'app.kubernetes.io/name: "ai-blaise-citus"',
    'ai-blaise.com/chart-fold-date: "2026-05-22"',
    'name: "ai-blaise-citus-pool-postgres"',
    "ai_blaise_sidecar_queue_depth",
):
    if phrase not in guardrail_text:
        fail(f"Kubernetes guardrail contract missing phrase: {phrase}")

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
    "k8s_guardrail_contract=true\t"
    "live_k8s_e2e_harness=true\t"
    "chart_folded_to_command_center=2026-05-22"
)
PY
