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
PITR_RUNBOOK = ROOT / "docs/ai-blaise/RUNBOOKS/pitr-restore.md"
COHABITATION_DOC = ROOT / "docs/ai-blaise/COHABITATION.md"
COHAB_MATRIX_README = ROOT / "tests/cohab-matrix/README.md"
BENCHMARKS_DOC = ROOT / "docs/ai-blaise/BENCHMARKS.md"
IMAGES_OVERVIEW = ROOT / "images/README.ai-blaise.md"
PG_OVERLAY_README = ROOT / "images/citus-pg-overlay/README.md"
PERF_THRESHOLDS = ROOT / "benchmarks/performance-evidence-thresholds.json"
PERF_CHECK = ROOT / "ci/ai-blaise/performance-evidence-check.sh"
MAKEFILE = ROOT / "Makefile.ai-blaise"
SIDECAR_WORKFLOW = ROOT / ".github/workflows/ci-sidecar.yml"
SIDECAR_API_SMOKE = ROOT / "ci/ai-blaise/sidecar-api-runtime-smoke.sh"
STORAGE_RUNTIME_SMOKE = ROOT / "ci/ai-blaise/storage-sidecar-runtime-smoke.sh"
POOL_PROXY_SMOKE = ROOT / "ci/ai-blaise/pool-proxy-smoke.sh"
SQL_EXTENSION_SMOKE = ROOT / "ci/ai-blaise/sql-extension-smoke.sh"
PATCHES_WORKFLOW = ROOT / ".github/workflows/ci-patches.yml"
PRODUCTION_WORKFLOW = ROOT / ".github/workflows/ci-production-readiness.yml"
CITUS_PATCH_AUDIT = ROOT / "ci/ai-blaise/citus-patch-production-audit.sh"
RUNBOOK_CHECK = ROOT / "ci/ai-blaise/runbook-command-check.sh"
K8S_GUARDRAIL_RENDERER = ROOT / "deploy/contracts/render_k8s_guardrails.py"
K8S_GUARDRAIL_MANIFEST = ROOT / "deploy/contracts/k8s-production-guardrails.yaml"
K8S_GUARDRAIL_KUSTOMIZATION = ROOT / "deploy/contracts/kustomization.yaml"
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
    PITR_RUNBOOK,
    COHABITATION_DOC,
    COHAB_MATRIX_README,
    BENCHMARKS_DOC,
    IMAGES_OVERVIEW,
    PG_OVERLAY_README,
    MAKEFILE,
    SIDECAR_WORKFLOW,
    SIDECAR_API_SMOKE,
    STORAGE_RUNTIME_SMOKE,
    POOL_PROXY_SMOKE,
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

makefile = read(MAKEFILE)
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

sidecar_smoke = read(SIDECAR_API_SMOKE)
for required in (
    "run-bun-runtime-canonical",
    "POST",
    "/drain",
    "invalid listen address",
    "definitely-not-a-command",
    "ai_blaise_sidecar_accepting_new_work",
):
    if required not in sidecar_smoke:
        fail(f"sidecar API runtime smoke lost required assertion: {required}")

if "sidecar-api-runtime-smoke:" not in makefile:
    fail("Makefile.ai-blaise must expose sidecar-api-runtime-smoke")
if (
    "gate-close:" not in makefile
    or "sidecar-api-runtime-smoke" not in makefile.split("gate-close:", 1)[1]
):
    fail("gate-close must run sidecar-api-runtime-smoke")

if (
    "storage-sidecar-runtime-smoke:" not in makefile
    or "gate-close:" not in makefile
    or "storage-sidecar-runtime-smoke" not in makefile.split("gate-close:", 1)[1]
):
    fail("gate-close must run storage-sidecar-runtime-smoke")

storage_smoke = read(STORAGE_RUNTIME_SMOKE)
for required in (
    "/storage/policy",
    "/storage/presign",
    "/storage/upload",
    "malware:eicar-test",
    "quarantined",
    "/drain",
):
    if required not in storage_smoke:
        fail(f"storage sidecar runtime smoke lost required assertion: {required}")


sql_extension_smoke = read(SQL_EXTENSION_SMOKE)
for required in (
    "storage.file_attachment(",
    "storage.file_attachment_refs",
    "storage.file_attachment_uri",
    "Sto2 accepted invalid bucket",
    "Sto2 accepted path traversal",
    "Sto2 accepted malformed sha256",
    "Sto2 accepted negative size_bytes",
):
    if required not in sql_extension_smoke:
        fail(f"SQL extension smoke lost Sto2 assertion: {required}")

if "'Sto2'" not in sql_extension_smoke or "<> 44" not in sql_extension_smoke:
    fail("SQL extension smoke must count Sto2 as a sql-runtime feature")

sidecar_workflow = read(SIDECAR_WORKFLOW)
if (
    "api-runtime-smoke:" not in sidecar_workflow
    or "sidecar-api-runtime-smoke.sh" not in sidecar_workflow
):
    fail("ci-sidecar workflow must run sidecar-api-runtime-smoke.sh")

if "storage-sidecar-runtime-smoke.sh" not in sidecar_workflow:
    fail("ci-sidecar workflow must run storage-sidecar-runtime-smoke.sh")

pool_smoke = read(POOL_PROXY_SMOKE)
for required in (
    "AI_BLAISE_POOL_AUTH_INTROSPECTION_URL",
    "ai_blaise.jwt",
    "pool auth valid-token admission",
    "pool auth revoked-token fail-closed",
    "ai_blaise_citus_pool_auth_verified_connections_total",
    "ai_blaise_citus_pool_auth_rejections_total",
):
    if required not in pool_smoke:
        fail(f"pool proxy smoke lost Auth3 data-plane assertion: {required}")

phony_lines = "\n".join(line for line in makefile.splitlines() if line.startswith(".PHONY:"))
gate_deps = "\n".join(line for line in makefile.splitlines() if line.startswith("gate-close:"))
for target in (
    "citus-patch-production-audit",
    "sidecar-api-runtime-smoke",
    "storage-sidecar-runtime-smoke",
    "runbook-command-check",
):
    if target not in phony_lines:
        fail(f"Makefile.ai-blaise .PHONY missing integration gate: {target}")
    if target not in gate_deps:
        fail(f"gate-close must run integration gate: {target}")

patch_audit = read(CITUS_PATCH_AUDIT)
for phrase in (
    "production-gates.json",
    "roster-only",
    "not production-ready",
    "required_mode",
    "measured",
):
    if phrase not in patch_audit:
        fail(f"citus-patch-production-audit.sh lost fail-closed phrase: {phrase}")
patches_workflow = read(PATCHES_WORKFLOW)
if "make -f Makefile.ai-blaise citus-patch-production-audit" not in patches_workflow:
    fail("ci-patches workflow must run citus-patch-production-audit")

runbook_check = read(RUNBOOK_CHECK)
for phrase in (
    "shell_syntax_errors",
    "script_ref_errors",
    "sidecar_binary_errors",
    "make_target_errors",
):
    if phrase not in runbook_check:
        fail(f"runbook-command-check.sh lost validator: {phrase}")
production_workflow = read(PRODUCTION_WORKFLOW)
if "runbook-command-check.sh" not in production_workflow:
    fail("ci-production-readiness workflow must run runbook-command-check.sh")


live_k8s = read(LIVE_K8S_E2E)
deploy_check = read(DEPLOY_CHECK)
kind_smoke = read(KIND_PRODUCTION_SMOKE)
deploy_readme = read(DEPLOY_README)
for phrase in ("CHART_DIR", "COMMAND_CENTER_DIR", "AI_BLAISE_STACK_IMAGE_REF", "LOCAL_IMAGE_REFS"):
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
dr_runbook = read(DR_RUNBOOK)

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

matrix_truth = compact(read(COHAB_MATRIX_README) + "\n" + read(COHABITATION_DOC) + "\n" + audit)
for phrase in (
    "skip-with-note",
    "does not promote TS 2.28 to production-ready",
    "VM registry probe on 2026-05-24",
):
    if compact(phrase) not in matrix_truth:
        fail(f"Timescale 2.28 matrix docs must preserve truth phrase: {phrase}")

for pattern in ("TS 2.28 production-ready", "TimescaleDB 2.28 production-ready"):
    if compact(pattern) in matrix_truth:
        fail(f"Timescale 2.28 matrix overclaims production readiness: {pattern}")

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
    "k8s_guardrail_contract=true\t"
    "live_k8s_e2e_harness=true\t"
    "chart_folded_to_command_center=2026-05-22"
)
PY
