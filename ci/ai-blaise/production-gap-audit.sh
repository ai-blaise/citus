#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

python3 <<'PY'
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(".")
DOCS = ROOT / "docs/ai-blaise/NEW_FEATURES.md"
AUDIT = ROOT / "docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md"
RELEASING = ROOT / "docs/ai-blaise/RELEASING.md"
RUNBOOK = ROOT / "docs/ai-blaise/RUNBOOKS/production.md"
E2E_DOC = ROOT / "docs/ai-blaise/E2E.md"
RELEASE_GATES = ROOT / "e2e/src/release_gates.rs"
V2_ACCEPTANCE = ROOT / "ci/ai-blaise/v2-acceptance-check.sh"
SQL_SMOKE = ROOT / "ci/ai-blaise/sql-extension-smoke.sh"
IMAGE_CHECK = ROOT / "ci/ai-blaise/image-check.sh"
POOL_SMOKE = ROOT / "ci/ai-blaise/pool-proxy-smoke.sh"
KIND_SMOKE = ROOT / "ci/ai-blaise/kind-production-smoke.sh"
DEPLOY_CHECK = ROOT / "ci/ai-blaise/deploy-check.sh"
PROD_VALUES = ROOT / "deploy/k8s/helm/citus-overlay/values-prod.yaml"
PROD_READINESS = ROOT / "ci/ai-blaise/production-readiness-check.sh"

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

PRODUCTION_STATUSES = {
    "ga",
    "stable",
    "production",
    "production-ready",
    "production ready",
}

LIVE_EVIDENCE_WORDS = (
    "vm",
    "github actions",
    "postgres:17",
    "postgresql 17",
    "real postgres",
    "real postgresql",
    "kind production smoke",
    "live sql",
    "live operator",
    "live sidecar",
    "real pods",
    "real backend",
    "real rust image",
    "container",
)

MODEL_ONLY_WORDS = (
    "canonical model",
    "model-only",
    "contract-only",
    "deterministic model",
    "pure-rust model",
)


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    sys.exit(1)


def read(path: pathlib.Path) -> str:
    if not path.exists():
        fail(f"missing production gap audit input: {path}")
    return path.read_text(encoding="utf-8", errors="ignore")


def compact(text: str) -> str:
    return " ".join(text.split()).lower()


def require_text(path: pathlib.Path, needle: str) -> None:
    text = read(path)
    if needle not in text:
        fail(f"{path} must contain: {needle}")


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
            chunks.append(read(path))
    return "\n".join(chunks)


def feature_entries(docs: str):
    heading_re = re.compile(r"^###\s+([A-Za-z][A-Za-z0-9]*):\s+(.+)$", re.M)
    status_re = re.compile(r"^\*\*Status\*\*:\s*(.+)$", re.M)
    headings = list(heading_re.finditer(docs))
    entries = []
    for index, match in enumerate(headings):
        end = headings[index + 1].start() if index + 1 < len(headings) else len(docs)
        body = docs[match.start():end]
        status_match = status_re.search(body)
        status = status_match.group(1).strip() if status_match else ""
        evidence = ""
        evidence_match = re.search(
            r"Production evidence:\s*(.+?)(?:\n\n|\Z)", body, re.S
        )
        if evidence_match:
            evidence = compact(evidence_match.group(1))
        entries.append(
            {
                "id": match.group(1),
                "title": match.group(2).strip(),
                "status": status,
                "body": body,
                "evidence": evidence,
            }
        )
    return entries


def table_statuses(docs: str):
    table_re = re.compile(
        r"^\|\s*([A-Za-z][A-Za-z0-9]*)\s*\|[^|]*\|[^|]*\|\s*([^|]+?)\s*\|",
        re.M,
    )
    statuses = {}
    for match in table_re.finditer(docs):
        if match.group(1) != "ID":
            statuses[match.group(1)] = match.group(2).strip()
    return statuses


def number_forms(value: int):
    words = {
        0: "zero",
        1: "one",
        2: "two",
        3: "three",
        4: "four",
        5: "five",
        6: "six",
        7: "seven",
        8: "eight",
        9: "nine",
        10: "ten",
    }
    forms = {str(value)}
    if value in words:
        forms.add(words[value])
    return forms


docs = read(DOCS)
audit = read(AUDIT)
releasing = read(RELEASING)
runbook = read(RUNBOOK)
e2e_doc = read(E2E_DOC)
release_gates = read(RELEASE_GATES)
v2_acceptance = read(V2_ACCEPTANCE)
sql_smoke = read(SQL_SMOKE)
image_check = read(IMAGE_CHECK)
pool_smoke = read(POOL_SMOKE)
kind_smoke = read(KIND_SMOKE)
deploy_check = read(DEPLOY_CHECK)
prod_values = read(PROD_VALUES)
sources = source_text()

source_ids = set(re.findall(r"FEATURE:\s+([A-Za-z][A-Za-z0-9]*)", sources))
doc_ids = set(re.findall(r"FEATURE:\s+([A-Za-z][A-Za-z0-9]*)", docs))
entries = feature_entries(docs)
entry_ids = {entry["id"] for entry in entries}
statuses = table_statuses(docs)
production_entries = [
    entry for entry in entries if entry["status"].lower() in PRODUCTION_STATUSES
]
alpha_entries = [
    entry for entry in entries if entry["status"].lower() not in PRODUCTION_STATUSES
]
source_only_ids = source_ids - entry_ids

if not source_ids:
    fail("no source FEATURE markers found")

if source_ids - doc_ids:
    fail("source FEATURE markers missing from NEW_FEATURES.md: " + ", ".join(sorted(source_ids - doc_ids)))

if doc_ids - source_ids:
    fail("NEW_FEATURES.md references FEATURE ids missing from source: " + ", ".join(sorted(doc_ids - source_ids)))

source_only_non_alpha = sorted(
    feature_id
    for feature_id in source_only_ids
    if statuses.get(feature_id, "").lower() in PRODUCTION_STATUSES
)
if source_only_non_alpha:
    fail(
        "source-only addendum rows cannot be production-like without feature headings: "
        + ", ".join(source_only_non_alpha)
    )

if len(production_entries) < 1:
    fail("production gap audit expects at least one promoted, evidence-backed feature")

for entry in production_entries:
    if "Production evidence:" not in entry["body"]:
        fail(f"{entry['id']} is production-like but lacks Production evidence")
    evidence = entry["evidence"]
    if not any(word in evidence for word in LIVE_EVIDENCE_WORDS):
        fail(f"{entry['id']} production evidence lacks live VM/container/CI wording")
    if any(word in evidence for word in MODEL_ONLY_WORDS):
        fail(f"{entry['id']} production evidence relies on model-only wording")

non_production_with_prod_evidence = sorted(
    entry["id"] for entry in alpha_entries if "Production evidence:" in entry["body"]
)
if non_production_with_prod_evidence:
    fail(
        "non-production headings must not carry Production evidence fields: "
        + ", ".join(non_production_with_prod_evidence)
    )

audit_compact = compact(audit)
docs_compact = compact(docs)
releasing_compact = compact(releasing)
runbook_compact = compact(runbook)
e2e_compact = compact(e2e_doc)

expected_inventory = (
    f"contains {len(source_ids)} source `feature:` markers and {len(entries)} "
    "feature headings"
)
if expected_inventory not in audit_compact:
    fail(
        "PRODUCTION_READINESS_AUDIT.md must report the current computed feature inventory"
    )

expected_production_counts = [
    f"{form} narrow headings are `status: production-ready`"
    for form in number_forms(len(production_entries))
]
if not any(phrase in audit_compact for phrase in expected_production_counts):
    fail(
        "PRODUCTION_READINESS_AUDIT.md must report the current production-ready heading count"
    )

expected_alpha_count = f"other {len(alpha_entries)} feature headings remain `status: alpha`"
if expected_alpha_count not in audit_compact:
    fail(
        "PRODUCTION_READINESS_AUDIT.md must report the current alpha heading count"
    )

expected_source_only = f"remaining {len(source_only_ids)} source markers"
if expected_source_only not in audit_compact:
    fail(
        "PRODUCTION_READINESS_AUDIT.md must report the current source-only/addendum count"
    )

for phrase in (
    "not production-ready as a whole",
    "modeled release gates",
    "canonical model data rather than results from live performance",
    "v2 acceptance model must not be cited as production evidence",
):
    if phrase not in audit_compact:
        fail(f"PRODUCTION_READINESS_AUDIT.md must preserve guardrail phrase: {phrase}")
if "production-release" not in audit_compact or "fails while any feature heading" not in audit_compact:
    fail("PRODUCTION_READINESS_AUDIT.md must state that production-release fails on remaining non-production headings")

for phrase in (
    "alpha means not production-ready",
    "contract, model, catalog, sql-plan, and runbook entries are implementation artifacts",
    "production-gap-audit",
):
    if phrase not in docs_compact:
        fail(f"NEW_FEATURES.md must preserve guardrail phrase: {phrase}")

for phrase in (
    "release prerequisites, not a waiver for alpha features",
    "production-release mode intentionally fails",
    "contract-only, or model-only without measured evidence",
    "production-gap-audit",
):
    if phrase not in releasing_compact:
        fail(f"RELEASING.md must preserve guardrail phrase: {phrase}")

for phrase in (
    "not a blanket production certification",
    "v2 acceptance model",
    "production-gap-audit",
    "probe-only traffic is insufficient",
):
    if phrase not in runbook_compact:
        fail(f"production runbook must preserve guardrail phrase: {phrase}")

for phrase in (
    "pure rust model",
    "not measured production evidence",
    "production-gap-audit",
):
    if phrase not in e2e_compact:
        fail(f"E2E.md must preserve model-disclosure phrase: {phrase}")

require_text(V2_ACCEPTANCE, "bash ci/ai-blaise/production-readiness-check.sh")
if "production-readiness-check.sh production-release" in v2_acceptance:
    fail("v2-acceptance-check.sh must not invoke production-release mode")
if "production gap audit treats this as modeled acceptance" not in v2_acceptance:
    fail("v2-acceptance-check.sh must explicitly disclose modeled acceptance")
if "expected=$'15\\t15\\t3\\tfalse" not in v2_acceptance:
    fail("v2-acceptance-check.sh must keep the canonical expected V2 TSV row")

for phrase in (
    "pub fn canonical() -> Self",
    "canonical v2 gate shape",
    "not measured production evidence",
    "parallel_commit_p95_us: 55_000",
    "primary_failure_recovery_p99_ms: 4_500",
    "timescale-ingest",
    "random-kill",
    "network-partition",
):
    haystack = compact(release_gates) if phrase == phrase.lower() else release_gates
    if phrase not in haystack:
        fail(f"release_gates.rs must preserve modeled-gate marker: {phrase}")

release_probe = subprocess.run(
    ["bash", str(PROD_READINESS), "production-release"],
    text=True,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
)
if release_probe.returncode == 0:
    fail(
        "production-readiness-check.sh production-release unexpectedly passed "
        "while non-production features remain"
    )
if "production release blocked: non-production feature statuses remain" not in release_probe.stderr:
    fail("production-release failure must clearly report non-production blockers")

if 'docker exec -i "${container}" psql' not in sql_smoke:
    fail("SQL extension smoke must attach stdin to psql")
for phrase in (
    "shared_preload_libraries=pg_stat_statements",
    "CREATE EXTENSION pg_stat_statements;",
    "ai_blaise_pg_stat_statements_seed",
    "companion_pg_stat_statements_p95",
    "docker exec -d",
    "companion_idle_transactions('100 milliseconds'::interval)",
):
    if phrase not in sql_smoke:
        fail(f"SQL extension smoke is missing runtime proof marker: {phrase}")

for phrase in (
    'docker exec -i "${container}" psql',
    "shared_preload_libraries=pg_stat_statements",
    "ai_blaise_pg_stat_statements_seed",
    "companion_idle_transactions('100 milliseconds'::interval)",
):
    if phrase not in image_check:
        fail(f"image-check.sh must statically guard SQL smoke marker: {phrase}")

for phrase in (
    'psql -h 127.0.0.1 -p "${pool_port}"',
    "AI_BLAISE_POOL_UPSTREAM_ADDR",
    "ai_blaise_citus_pool_requests_total",
):
    if phrase not in pool_smoke:
        fail(f"pool-proxy-smoke.sh is missing live SQL proof marker: {phrase}")

for phrase in (
    "kind create cluster",
    "scripts/citus-scale/build-app-images.sh",
    "helm upgrade --install",
    "port-forward",
    "/healthz",
    "/readyz",
    "/metrics",
    "psql -h ai-blaise-citus-pool",
    "probe_pool_admin_pods",
    "ai_blaise_citus_pool_requests_total",
):
    if phrase not in kind_smoke:
        fail(f"kind-production-smoke.sh is missing live deployment proof marker: {phrase}")

if "values-prod.yaml must not enable alpha sidecars by default" not in deploy_check:
    fail("deploy-check.sh must reject alpha sidecars enabled in production values")
inside_sidecars = False
current_sidecar = ""
enabled_sidecars = []
for line in prod_values.splitlines():
    if line == "sidecars:":
        inside_sidecars = True
        continue
    if inside_sidecars and line and not line.startswith((" ", "-")):
        inside_sidecars = False
    if not inside_sidecars:
        continue
    name_match = re.match(r"^\s*-\s+name:\s+(.+?)\s*$", line)
    if name_match:
        current_sidecar = name_match.group(1)
    if re.match(r"^\s+enabled:\s+true\s*$", line):
        enabled_sidecars.append(current_sidecar)
if enabled_sidecars:
    fail(
        "values-prod.yaml must not enable alpha sidecars by default: "
        + ", ".join(enabled_sidecars)
    )

for path in (
    DOCS,
    AUDIT,
    RELEASING,
    RUNBOOK,
    E2E_DOC,
):
    text = read(path)
    for pattern in (
        "full plan is production-ready",
        "entire plan is production-ready",
        "all custom features are production-ready",
        "production certified by v2-acceptance",
        "v2 acceptance proves production",
    ):
        if pattern in compact(text):
            fail(f"{path} contains overclaiming wording: {pattern}")

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
    "live_kubernetes_guards=true"
)
PY
