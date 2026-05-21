#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

python3 <<'PY'
import pathlib
import json
import re
import subprocess
import sys

ROOT = pathlib.Path(".")
DOCS = ROOT / "docs/ai-blaise/NEW_FEATURES.md"
AUDIT = ROOT / "docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md"
RELEASING = ROOT / "docs/ai-blaise/RELEASING.md"
RUNBOOK = ROOT / "docs/ai-blaise/RUNBOOKS/production.md"
UPGRADE_RUNBOOK = ROOT / "docs/ai-blaise/RUNBOOKS/upgrade.md"
DR_RUNBOOK = ROOT / "docs/ai-blaise/RUNBOOKS/disaster-recovery.md"
E2E_DOC = ROOT / "docs/ai-blaise/E2E.md"
COHABITATION_DOC = ROOT / "docs/ai-blaise/COHABITATION.md"
ARCHITECTURE_DOC = ROOT / "docs/ai-blaise/ARCHITECTURE.md"
BUNDLED_EXTENSIONS_DOC = ROOT / "docs/ai-blaise/BUNDLED_EXTENSIONS.md"
IMAGES_OVERVIEW = ROOT / "images/README.ai-blaise.md"
PG_OVERLAY_README = ROOT / "images/citus-pg-overlay/README.md"
RELEASE_GATES = ROOT / "e2e/src/release_gates.rs"
V2_ACCEPTANCE = ROOT / "ci/ai-blaise/v2-acceptance-check.sh"
SQL_SMOKE = ROOT / "ci/ai-blaise/sql-extension-smoke.sh"
IMAGE_CHECK = ROOT / "ci/ai-blaise/image-check.sh"
POOL_SMOKE = ROOT / "ci/ai-blaise/pool-proxy-smoke.sh"
LSP_SMOKE = ROOT / "ci/ai-blaise/citus-lsp-smoke.sh"
TIMESCALE_SMOKE = ROOT / "ci/ai-blaise/timescale-bridge-smoke.sh"
TIMESCALE_COHABITATION_SMOKE = ROOT / "ci/ai-blaise/timescale-cohabitation-smoke.sh"
OBSERVABILITY_REPLICATION_SMOKE = ROOT / "ci/ai-blaise/observability-replication-smoke.sh"
KIND_SMOKE = ROOT / "ci/ai-blaise/kind-production-smoke.sh"
DEPLOY_CHECK = ROOT / "ci/ai-blaise/deploy-check.sh"
DEPLOY_SCRIPT = ROOT / "scripts/citus-scale/deploy.sh"
SIDECAR_SHARED_README = ROOT / "sidecar/shared/README.md"
PROD_VALUES = ROOT / "deploy/k8s/helm/citus-overlay/values-prod.yaml"
DEFAULT_VALUES = ROOT / "deploy/k8s/helm/citus-overlay/values.yaml"
EXHAUSTIVE_VALUES = ROOT / "deploy/k8s/helm/citus-overlay/values-exhaustive.yaml"
ARGO_APP = ROOT / "deploy/k8s/argo/app.yaml"
HELPER_TEMPLATE = ROOT / "deploy/k8s/helm/citus-overlay/templates/_helpers.tpl"
OPERATOR_DEPLOYMENT_TEMPLATE = ROOT / "deploy/k8s/helm/citus-overlay/templates/operator-deployment.yaml"
POOL_DEPLOYMENT_TEMPLATE = ROOT / "deploy/k8s/helm/citus-overlay/templates/pool-deployment.yaml"
POOL_NETWORKPOLICY_TEMPLATE = ROOT / "deploy/k8s/helm/citus-overlay/templates/pool-networkpolicy.yaml"
SIDECAR_DEPLOYMENT_TEMPLATE = ROOT / "deploy/k8s/helm/citus-overlay/templates/sidecar-deployments.yaml"
TOOLS_DEPLOYMENT_TEMPLATE = ROOT / "deploy/k8s/helm/citus-overlay/templates/tools-deployment.yaml"
DASHBOARD_TEMPLATE = ROOT / "deploy/k8s/helm/citus-overlay/templates/observability-dashboards.yaml"
PROMRULE_TEMPLATE = ROOT / "deploy/k8s/helm/citus-overlay/templates/observability-prometheusrules.yaml"
OPERATOR_RBAC_TEMPLATE = ROOT / "deploy/k8s/helm/citus-overlay/templates/operator-rbac.yaml"
PROD_READINESS = ROOT / "ci/ai-blaise/production-readiness-check.sh"
IMAGE_WORKFLOW = ROOT / ".github/workflows/ci-image.yml"
DEPLOY_WORKFLOW = ROOT / ".github/workflows/ci-deploy.yml"
POOL_WORKFLOW = ROOT / ".github/workflows/ci-pool.yml"
OPERATOR_WORKFLOW = ROOT / ".github/workflows/ci-operator.yml"
SIDECAR_WORKFLOW = ROOT / ".github/workflows/ci-sidecar.yml"
SLOP_WORKFLOW = ROOT / ".github/workflows/ci-slop-scan.yml"
TOOLS_WORKFLOW = ROOT / ".github/workflows/ci-tools.yml"
CUSTOM_CI_WORKFLOWS = sorted((ROOT / ".github/workflows").glob("ci-*.yml"))
MAKEFILE = ROOT / "Makefile.ai-blaise"
TS6_PATCH = ROOT / "patches/0001-allow-trusted-hook-coextensions.patch"
SHARED_LIBRARY_INIT = ROOT / "src/backend/distributed/shared_library_init.c"
KIND_TIMESCALE_SMOKE = ROOT / "tests/e2e/kind-timescale-citus-smoke.sh"
CUSTOM_CONTRACT_READMES = [
    ROOT / path
    for path in (
        "companion/README.md",
        "benchmarks/README.md",
        "operator/README.md",
        "operator/CRDS.md",
        "pool/README.md",
        "sidecar/README.md",
        "e2e/README.md",
        "tests/e2e/README.md",
        "tools/README.md",
        "deploy/k8s/README.md",
        "images/README.ai-blaise.md",
        "images/citus-pg-overlay/README.md",
        "images/operator/README.md",
        "images/pool/README.md",
        "images/tools/README.md",
        "sidecar/analytical/README.md",
        "sidecar/auth/README.md",
        "sidecar/backup/README.md",
        "sidecar/cdc/README.md",
        "sidecar/coldtier/README.md",
        "sidecar/edge_functions/README.md",
        "sidecar/graphql/README.md",
        "sidecar/hlc/README.md",
        "sidecar/mcp/README.md",
        "sidecar/postgrest/README.md",
        "sidecar/raft/README.md",
        "sidecar/realtime/README.md",
        "sidecar/repack/README.md",
        "sidecar/schema_job/README.md",
        "sidecar/shared/README.md",
        "sidecar/storage/README.md",
        "sidecar/txn_status/README.md",
        "sidecar/vectorizer/README.md",
        "tools/citus-admin/README.md",
        "tools/citus-lsp/README.md",
        "tools/citus-mcp/README.md",
        "tools/citus-schema-designer/README.md",
        "tools/citus-tui/README.md",
        "tools/citus-watch/README.md",
        "tools/citusctl/README.md",
    )
]

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
upgrade_runbook = read(UPGRADE_RUNBOOK)
dr_runbook = read(DR_RUNBOOK)
e2e_doc = read(E2E_DOC)
cohabitation_doc = read(COHABITATION_DOC)
architecture_doc = read(ARCHITECTURE_DOC)
bundled_extensions_doc = read(BUNDLED_EXTENSIONS_DOC)
images_overview = read(IMAGES_OVERVIEW)
pg_overlay_readme = read(PG_OVERLAY_README)
release_gates = read(RELEASE_GATES)
v2_acceptance = read(V2_ACCEPTANCE)
sql_smoke = read(SQL_SMOKE)
image_check = read(IMAGE_CHECK)
pool_smoke = read(POOL_SMOKE)
timescale_smoke = read(TIMESCALE_SMOKE)
timescale_cohabitation_smoke = read(TIMESCALE_COHABITATION_SMOKE)
observability_replication_smoke = read(OBSERVABILITY_REPLICATION_SMOKE)
kind_smoke = read(KIND_SMOKE)
kind_timescale_smoke = read(KIND_TIMESCALE_SMOKE)
deploy_check = read(DEPLOY_CHECK)
deploy_script = read(DEPLOY_SCRIPT)
prod_values = read(PROD_VALUES)
default_values = read(DEFAULT_VALUES)
exhaustive_values = read(EXHAUSTIVE_VALUES)
argo_app = read(ARGO_APP)
helper_template = read(HELPER_TEMPLATE)
operator_deployment_template = read(OPERATOR_DEPLOYMENT_TEMPLATE)
pool_deployment_template = read(POOL_DEPLOYMENT_TEMPLATE)
pool_networkpolicy_template = read(POOL_NETWORKPOLICY_TEMPLATE)
sidecar_deployment_template = read(SIDECAR_DEPLOYMENT_TEMPLATE)
tools_deployment_template = read(TOOLS_DEPLOYMENT_TEMPLATE)
dashboard_template = read(DASHBOARD_TEMPLATE)
promrule_template = read(PROMRULE_TEMPLATE)
operator_rbac_template = read(OPERATOR_RBAC_TEMPLATE)
image_workflow = read(IMAGE_WORKFLOW)
deploy_workflow = read(DEPLOY_WORKFLOW)
pool_workflow = read(POOL_WORKFLOW)
operator_workflow = read(OPERATOR_WORKFLOW)
sidecar_workflow = read(SIDECAR_WORKFLOW)
slop_workflow = read(SLOP_WORKFLOW)
tools_workflow = read(TOOLS_WORKFLOW)
makefile = read(MAKEFILE)
lsp_smoke = read(LSP_SMOKE)
ts6_patch = read(TS6_PATCH)
shared_library_init = read(SHARED_LIBRARY_INIT)
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
entry_by_id = {entry["id"]: entry for entry in entries}

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

def v2_addendum_rows(docs):
    marker = "## V2 Completion Register Addendum"
    if marker not in docs:
        fail("NEW_FEATURES.md is missing the V2 completion register addendum")
    section = docs.split(marker, 1)[1]
    header = None
    rows = {}
    for line in section.splitlines():
        if not line.startswith("|"):
            if header and rows:
                break
            continue
        if set(line.replace("|", "").strip()) == {"-"}:
            continue
        columns = [column.strip() for column in line.strip("|").split("|")]
        if columns[0] == "ID":
            header = columns
            continue
        if header is None:
            continue
        if len(columns) != len(header):
            fail(f"V2 addendum row has {len(columns)} columns but header has {len(header)}: {line}")
        row = dict(zip(header, columns))
        rows[row["ID"]] = row
    return rows

addendum_by_id = v2_addendum_rows(docs)
if set(addendum_by_id) != source_only_ids:
    fail(
        "V2 addendum rows must exactly cover source-only alpha ids; missing rows: "
        + ", ".join(sorted(source_only_ids - set(addendum_by_id)))
        + "; extra rows: "
        + ", ".join(sorted(set(addendum_by_id) - source_only_ids))
    )

source_only_evidence_requirements = {
    "cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical": {
        "A7",
        "A12",
        "C11",
        "C12",
        "C13",
        "EF6",
        "F2",
        "F5",
        "G1",
        "Geo1",
        "IA1",
        "IA2",
        "JS1",
        "L11",
        "M6",
        "M10",
        "M12",
        "MR7",
        "O7",
        "O8",
        "O9",
        "O11",
        "O12",
        "PM1",
        "PM2",
        "R6",
        "R11",
        "Search1",
        "Search4",
        "Search5",
        "Search6",
        "Sec3",
        "Sec4",
        "Sec10",
        "Sec11",
        "Sec14",
        "Sec15",
        "WF1",
    },
    "cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical": {
        "A10",
        "A11",
        "Edge1",
        "Edge2",
        "F3",
        "F4",
        "L7",
        "L10",
        "M4",
        "MR3",
        "MR6",
        "R3",
        "R8",
        "R12",
        "S1",
        "S3",
        "S8",
        "S12",
        "Sto2",
        "T4",
        "T10",
        "T11",
        "T13",
        "T14",
        "TS10",
        "TS11",
    },
    "cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical": {
        "A9",
        "D9",
        "D10",
        "MR9",
        "RT5",
        "S7",
        "Sec7",
        "Sec8",
        "Sec9",
        "T6",
    },
    "cargo run -p ai_blaise_citus_mcp -- run-canonical": {
        "D11",
    },
    "cargo run -p ai_blaise_citus_pool -- run-canonical": {
        "T7",
    },
}
expected_source_only_evidence_ids = set().union(*source_only_evidence_requirements.values())
if expected_source_only_evidence_ids != source_only_ids:
    fail(
        "production gap audit source-only evidence requirement ids drifted; missing: "
        + ", ".join(sorted(source_only_ids - expected_source_only_evidence_ids))
        + "; extra: "
        + ", ".join(sorted(expected_source_only_evidence_ids - source_only_ids))
    )
for command, feature_ids in sorted(source_only_evidence_requirements.items()):
    evidence = f"`{command}`"
    for feature_id in sorted(feature_ids):
        row = addendum_by_id[feature_id]
        if row.get("Status", "").lower() != "alpha":
            fail(f"{feature_id} addendum row must remain alpha")
        if f"`FEATURE: {feature_id}`" not in row.get("Reference", ""):
            fail(f"{feature_id} addendum row must cite its source FEATURE marker")
        if evidence not in row.get("Evidence", ""):
            fail(f"{feature_id} addendum row must cite executable evidence command {command}")

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

t15_entry = entry_by_id.get("T15")
if t15_entry is None:
    fail("T15 feature heading is required for pool pipeline evidence")
if t15_entry["status"].lower() not in PRODUCTION_STATUSES:
    fail("T15 must remain production-ready only for the measured pool simple-query pipeline")
for phrase in (
    "pipelined PostgreSQL simple-query frames",
    "without waiting for the first result",
    "pipeline_one",
    "pipeline_two",
    "broader transaction-batching, shard-aware routing, and `FEATURE: T7` source-only pipeline contract remain alpha",
    "ci/ai-blaise/pool-proxy-smoke.sh",
):
    if compact(phrase) not in compact(t15_entry["body"]):
        fail(f"T15 production-ready boundary is missing: {phrase}")

auth2_entry = entry_by_id.get("Auth2")
if auth2_entry is None:
    fail("Auth2 feature heading is required for SQL session-claim evidence")
if auth2_entry["status"].lower() not in PRODUCTION_STATUSES:
    fail("Auth2 must remain production-ready only for installable SQL session-claim helpers")
for phrase in (
    "installable SQL session-claim helpers",
    "custom GUCs",
    "companion_set_session_claims",
    "companion_current_session_claims",
    "companion_current_tenant_id",
    "Auth1 JWT issuance, Sec1 RLS enforcement, Sec2 JWT verification, and Auth3 token caching remain alpha",
    "ci/ai-blaise/sql-extension-smoke.sh",
):
    if compact(phrase) not in compact(auth2_entry["body"]):
        fail(f"Auth2 production-ready boundary is missing: {phrase}")

d2_entry = entry_by_id.get("D2")
if d2_entry is None:
    fail("D2 feature heading is required for citusctl plan-id evidence")
if d2_entry["status"].lower() not in PRODUCTION_STATUSES:
    fail("D2 must remain production-ready only for the real citusctl plan-id guard")
for phrase in (
    "explicit plan ID before apply-mode CLI execution",
    "citusctl apply",
    "citusctl: plan_id must not be empty",
    "plan inspect cluster",
    "apply plan-123 apply",
    "Broader citusctl dev cluster lifecycle, full plan/apply execution, migrations, backups, PITR, WAL replay, and operator mutation workflows remain alpha",
    "ci/ai-blaise/citusctl-smoke.sh",
):
    if compact(phrase) not in compact(d2_entry["body"]):
        fail(f"D2 production-ready boundary is missing: {phrase}")

for feature_id, feature_name in (
    ("D4", "file-backed Citus/Timescale SQL diagnostics"),
    ("M5", "file-backed quick-fix action emission"),
    ("TS8", "file-backed distributed hypertable invariant diagnostics"),
):
    entry = entry_by_id.get(feature_id)
    if entry is None:
        fail(f"{feature_id} feature heading is required for citus-lsp production evidence")
    if entry["status"].lower() not in PRODUCTION_STATUSES:
        fail(f"{feature_id} must remain production-ready for the narrow citus-lsp {feature_name} surface")
    for phrase in (
        "citus-lsp analyze --metadata <metadata.tsv> --sql <migration.sql>",
        "ci/ai-blaise/citus-lsp-smoke.sh",
        "real SQL file",
        "metadata TSV",
        "Broader JSON-RPC language-server protocol integration, editor transport, workspace indexing, automatic file rewrites, and full PostgreSQL grammar coverage remain alpha",
    ):
        if compact(phrase) not in compact(entry["body"]):
            fail(f"{feature_id} production-ready boundary is missing: {phrase}")

non_production_with_prod_evidence = sorted(
    entry["id"] for entry in alpha_entries if "Production evidence:" in entry["body"]
)
if non_production_with_prod_evidence:
    fail(
        "non-production headings must not carry Production evidence fields: "
        + ", ".join(non_production_with_prod_evidence)
    )

stale_alpha_production_phrases = [
    ("T1", "stable GUC"),
    ("S6", "stable helper APIs"),
    ("M9", "live shard-placement overlays"),
    ("M9", "live shard-map overlay model"),
    ("L6", "stable federation contract"),
    ("API2", "stable view contract"),
    ("PM3", "freezing a stable plan"),
    ("PM3", "stable executions"),
    ("Sto4", "live only in object-store policy"),
    ("D6", "stable model"),
    ("D6", "live CRD or companion state"),
    ("O5", "live VM/Kubernetes evidence"),
    ("T15", "production `serve` data plane"),
    ("A5", "provider calls run in production"),
    ("PM3", "stable production queries"),
    ("MCP3", "multi-tenant production usage"),
    ("D12", "`citus-watch` live view"),
    ("O13", "live operations need"),
    ("O13", "dedicated live operations tui"),
    ("O13", "`citus-watch` unified live view"),
]
for feature_id, phrase in stale_alpha_production_phrases:
    entry = entry_by_id.get(feature_id)
    if entry is None:
        fail(f"{feature_id} feature heading is required for alpha wording guard")
    if compact(phrase) in compact(entry["body"]):
        fail(f"{feature_id} alpha heading contains production-sounding stale phrase: {phrase}")

stale_alpha_addendum_phrases = [
    ("D10", "Production hardening runbook"),
    ("O12", "pg_show_plans live plans"),
    ("O12", "live plan inspection"),
]
for feature_id, phrase in stale_alpha_addendum_phrases:
    row = addendum_by_id.get(feature_id)
    if row is None:
        fail(f"{feature_id} source-only addendum row is required for alpha wording guard")
    row_text = " ".join(row.values())
    if compact(phrase) in compact(row_text):
        fail(f"{feature_id} source-only addendum row contains production-sounding stale phrase: {phrase}")

stale_alpha_readme_phrases = {
    ROOT / "tools/README.md": (
        "unified live view",
        "live shard-map overlays",
    ),
    ROOT / "tools/citus-schema-designer/README.md": (
        "live shard placements",
    ),
    ROOT / "sidecar/shared/README.md": (
        "live VM/Kubernetes evidence",
    ),
}
for path, phrases in stale_alpha_readme_phrases.items():
    text = compact(read(path))
    for phrase in phrases:
        if compact(phrase) in text:
            fail(f"{path} contains production-sounding stale alpha phrase: {phrase}")

audit_compact = compact(audit)
docs_compact = compact(docs)
releasing_compact = compact(releasing)
runbook_compact = compact(runbook)
upgrade_runbook_compact = compact(upgrade_runbook)
dr_runbook_compact = compact(dr_runbook)
e2e_compact = compact(e2e_doc)
cohabitation_compact = compact(cohabitation_doc)
architecture_compact = compact(architecture_doc)
bundled_extensions_compact = compact(bundled_extensions_doc)
images_overview_compact = compact(images_overview)
pg_overlay_readme_compact = compact(pg_overlay_readme)

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
expected_source_only_count = (
    f"remaining {len(source_only_ids)} source markers are represented as v2 completion addendum rows"
)
if expected_source_only_count not in audit_compact:
    fail(
        "PRODUCTION_READINESS_AUDIT.md must report the current source-only addendum row count"
    )
if "every addendum row has a deterministic executable evidence command" not in audit_compact:
    fail("PRODUCTION_READINESS_AUDIT.md must state source-only addendum evidence coverage")

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
for pattern in (
    "idle transaction reaper",
    "o2: distributed stats view",
    "coordinator and worker behavior to debug distributed plans",
    "validated cohabitation",
    "ts18` for the installable timescale bridge-state sql surface",
):
    if pattern in docs_compact:
        fail(f"NEW_FEATURES.md contains stale production-ready overclaim: {pattern}")
for phrase in (
    "ts6 source changes are now integrated into the fork",
    "timescale-cohabitation-smoke.sh",
    "cohabiting server without defining any citus stub",
    "validates the configured timescale/citus cohabitation precondition",
):
    if phrase not in docs_compact:
        fail(f"NEW_FEATURES.md must preserve cohabitation evidence boundary: {phrase}")

for phrase in (
    "release prerequisites, not a waiver for alpha features",
    "production-release mode intentionally fails",
    "contract-only, or model-only without measured evidence",
    "fail closed if docker is unavailable",
    "production-gap-audit",
):
    if phrase not in releasing_compact:
        fail(f"RELEASING.md must preserve guardrail phrase: {phrase}")

for path in CUSTOM_CONTRACT_READMES:
    readme_compact = compact(read(path))
    for phrase in (
        "production boundary",
        "status: production-ready",
        "surfaces listed here are alpha",
        "deterministic canonical reports and local runtime models are ci",
        "artifacts, not production evidence",
        "production_readiness_audit.md",
        "production-gap-audit.sh",
    ):
        if phrase not in readme_compact:
            fail(f"{path} must preserve component production-boundary phrase: {phrase}")

if "benchmark targets, not production evidence" not in compact(read(ROOT / "docs/ai-blaise/BENCHMARKS.md")):
    fail("BENCHMARKS.md must preserve benchmark-target not-production-evidence guardrail")

for phrase in (
    "not a blanket production certification",
    "v2 acceptance model",
    "production-gap-audit",
    "probe-only traffic is insufficient",
    "deploy wrapper defaults to `values-prod.yaml`",
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

for phrase in (
    "ts6 source changes",
    "timescale-cohabitation-smoke.sh",
    "shared_preload_libraries=timescaledb,citus",
    "without defining a citus stub",
    "broader ts1/ts2/ts3/ts4/ts5/ts12 distributed timescale features remain alpha",
):
    if phrase not in cohabitation_compact:
        fail(f"COHABITATION.md must preserve cohabitation evidence boundary: {phrase}")

ts6_patch_compact = compact(ts6_patch)
for pattern in (
    "operator-controlled path for validated",
    "validated with citus",
):
    if pattern in ts6_patch_compact:
        fail(f"TS6 patch metadata/code hint contains stale validation overclaim: {pattern}")
for phrase in (
    "deployment-level trust contract, not production evidence",
    "real citus+timescaledb cohabitation smoke",
    "image identity, command path, and ci or vm run",
):
    if phrase not in ts6_patch_compact:
        fail(f"TS6 patch metadata must preserve cohabitation evidence boundary: {phrase}")

kind_timescale_smoke_compact = compact(kind_timescale_smoke)
for phrase in (
    "contract-only check verified",
    "run live cohabitation evidence",
):
    if phrase not in kind_timescale_smoke_compact:
        fail(f"kind-timescale-citus-smoke.sh must preserve contract-only evidence boundary: {phrase}")

for phrase in (
    "manifest/init contract, not production evidence",
    "feature: bundle1` remains alpha",
    "real operand image build smoke verifies",
):
    if phrase not in bundled_extensions_compact:
        fail(f"BUNDLED_EXTENSIONS.md must preserve operand-image alpha guardrail: {phrase}")

for phrase in (
    "feature: bundle1` alpha contract",
    "not production evidence that the full required binary extension bundle is installed",
    "real operand image build/initdb smoke",
):
    if phrase not in images_overview_compact:
        fail(f"images/README.ai-blaise.md must preserve operand-image alpha guardrail: {phrase}")

for phrase in (
    "not production evidence that every binary package",
    "feature: bundle1` remains alpha",
    "real image build smoke verifies",
):
    if phrase not in pg_overlay_readme_compact:
        fail(f"images/citus-pg-overlay/README.md must preserve operand-image alpha guardrail: {phrase}")

for phrase in (
    "feature: bundle1` alpha operand-image contract",
    "not production evidence for the full operand image",
    "real operand image build/initdb smoke",
    "timescale-cohabitation-smoke.sh",
    "broader ts1/ts2/ts3/ts4/ts5/ts12 distributed feature entries remain alpha",
):
    if phrase not in architecture_compact:
        fail(f"ARCHITECTURE.md must preserve alpha evidence boundary: {phrase}")

shared_readme_compact = compact(read(SIDECAR_SHARED_README))
if "### o5: opentelemetry traces" in docs_compact:
    fail("O5 must not claim OpenTelemetry traces before trace emission/export is implemented")
o5_entry = next((entry for entry in entries if entry["id"] == "O5"), None)
if o5_entry is None:
    fail("O5 feature heading is required while operator sidecar deployment contracts exist")
if "Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`" not in o5_entry["body"]:
    fail("O5 must cite the operator canonical runner as alpha contract evidence")
operator_canonical_ids = {
    "A8",
    "B2",
    "B6",
    "C4",
    "C5",
    "C6",
    "C7",
    "C8",
    "C9",
    "EF3",
    "F1",
    "M3",
    "MR1",
    "MR2",
    "MR4",
    "MR8",
    "O5",
    "R2",
    "R7",
    "S2",
    "S4",
    "S10",
    "S11",
    "Search2",
    "Search7",
    "TO1",
    "TO2",
    "TO5",
    "TS7",
    "WH1",
}
for feature_id in sorted(operator_canonical_ids):
    entry = entry_by_id.get(feature_id)
    if entry is None:
        fail(f"{feature_id} feature heading is required for operator canonical evidence")
    if "Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`" not in entry["body"]:
        fail(f"{feature_id} must cite the operator canonical runner as alpha contract evidence")
evidence_runner_requirements = {
    "cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical": {
        "A1",
        "API4",
        "G2",
        "G3",
        "Geo2",
        "Geo3",
        "IA3",
        "JS2",
        "L9",
        "M1",
        "M11",
        "M13",
        "M2",
        "M7",
        "PM3",
        "PM4",
        "S13",
        "S14",
        "S6",
        "Search3",
        "Search9",
        "Sec1",
        "Sec2",
        "Sec5",
        "Sec6",
        "T8",
        "TO3",
        "TO4",
        "TS13",
        "TS14",
        "TS15",
        "TS16",
        "TS17",
        "TS9",
        "WH2",
    },
    "cargo run -p ai_blaise_citus_sidecar_vectorizer -- run-canonical": {
        "A3",
        "A4",
        "A6",
    },
    "cargo run -p ai_blaise_citus_sidecar_postgrest -- run-canonical": {
        "API2",
        "API5",
        "API6",
    },
    "cargo run -p ai_blaise_citus_sidecar_auth -- run-canonical": {
        "Auth2",
        "Auth4",
        "Auth5",
    },
    "cargo run -p ai_blaise_citusctl -- run-canonical": {
        "B5",
        "D1",
        "D2",
        "M8",
        "WF2",
    },
}
for command, feature_ids in sorted(evidence_runner_requirements.items()):
    line = f"Executable: `{command}`"
    for feature_id in sorted(feature_ids):
        entry = entry_by_id.get(feature_id)
        if entry is None:
            fail(f"{feature_id} feature heading is required for executable alpha evidence")
        if line not in entry["body"]:
            fail(f"{feature_id} must cite {command} as alpha contract evidence")
for phrase in (
    "the current implementation does not emit or export opentelemetry traces",
    "trace propagation remains unimplemented until real runtime code",
):
    if phrase not in docs_compact:
        fail(f"NEW_FEATURES.md must preserve the O5 tracing boundary: {phrase}")
for phrase in (
    "tracing and opentelemetry export are not implemented in this shared runtime",
    "`feature: o5` remains alpha for the sidecar deployment contract",
    "configuration loading and postgresql connection helpers are also outside the current shared runtime surface",
):
    if phrase not in shared_readme_compact:
        fail(f"sidecar/shared README must preserve the O5 tracing boundary: {phrase}")
for stale in (
    "metrics, tracing",
    "postgresql connection helpers.",
):
    if stale in shared_readme_compact:
        fail(f"sidecar/shared README must not overclaim unimplemented runtime helpers: {stale}")

require_text(ROOT / "ci/ai-blaise/citusctl-smoke.sh", "citusctl apply without a plan id unexpectedly succeeded")
require_text(ROOT / "ci/ai-blaise/citusctl-smoke.sh", "citusctl: plan_id must not be empty")
require_text(ROOT / "ci/ai-blaise/citusctl-smoke.sh", "apply plan-123 apply deploy/k8s/helm/citus-overlay/values-prod.yaml")
if "bash ci/ai-blaise/citusctl-smoke.sh" not in tools_workflow:
    fail("tools workflow must run the citusctl plan-id smoke for D2")
if "citusctl-smoke" not in makefile:
    fail("Makefile gate-close must include the citusctl plan-id smoke for D2")

for phrase in (
    "citus-lsp file-backed smoke passed",
    "CREATE TABLE tenant_a.invoices",
    "SELECT create_distributed_table('public.shipments', 'tenant_id')",
    "SELECT create_hypertable('public.events', 'created_at')",
    "SELECT apply_distribute_hypertable('public.events', 'device_id', 'created_at', '1 day')",
    "missing_distribution_column",
    "non_colocated_join",
    "distribution_column_alter",
    "hypertable_invariant",
    "missing_tenant_filter",
    "missing_search_analyzer",
    "add_distribution_column table=tenant_a.invoices column=tenant_id",
    "align_colocation left_table=public.orders right_table=public.events distribution_column=tenant_id",
    "add_tenant_filter table=public.orders tenant_column=tenant_id",
    "use_distributed_hypertable_bridge table=public.events time_column=created_at",
    "set_search_analyzer index_name=orders_search analyzer=english",
    "bad metadata unexpectedly succeeded",
    "missing metadata unexpectedly succeeded",
):
    if phrase not in lsp_smoke:
        fail(f"citus-lsp-smoke.sh must preserve file-backed diagnostic proof marker: {phrase}")
if "bash ci/ai-blaise/citus-lsp-smoke.sh" not in tools_workflow:
    fail("tools workflow must run the citus-lsp file-backed smoke for D4/M5/TS8")
if "citus-lsp-smoke" not in makefile:
    fail("Makefile gate-close must include the citus-lsp file-backed smoke")

for phrase in (
    "while bundle1 remains alpha",
    "must not be used as production release evidence",
    "real operand image build/initdb smoke",
):
    if phrase not in upgrade_runbook_compact:
        fail(f"upgrade runbook must preserve operand-image alpha guardrail: {phrase}")

for phrase in (
    "release prerequisite and operational checklist, not production evidence by itself",
    "feature: mr9` remains alpha",
    "live multi-region failover drill",
    "pitr restore",
    "backup artifact restore",
    "sidecar readiness check",
    "conflict-policy report",
    "completing the document checklist alone does not promote",
):
    if phrase not in dr_runbook_compact:
        fail(f"disaster recovery runbook must preserve MR9 alpha guardrail: {phrase}")

for path, text in (
    (IMAGES_OVERVIEW, images_overview_compact),
    (ARCHITECTURE_DOC, architecture_compact),
    (BUNDLED_EXTENSIONS_DOC, bundled_extensions_compact),
    (PG_OVERLAY_README, pg_overlay_readme_compact),
):
    for pattern in (
        "image directories under this tree build the citus operand image",
        "`images/citus-pg-overlay` builds the postgres operand",
        "sql fallback extension packaged in the operand image",
        "required bundle is installed for every ai-blaise/citus postgres operand image",
        "cloudnativepg operand image containing citus, companion, and bundled extension dependencies",
    ):
        if pattern in text:
            fail(f"{path} contains operand-image overclaim: {pattern}")

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
    "PostgreSQL init process complete",
    "CREATE EXTENSION pg_stat_statements;",
    "ai_blaise_pg_stat_statements_seed",
    "companion_pg_stat_statements_p95",
    "companion_pg_stat_local_activity",
    "docker exec -d",
    "companion_idle_transactions('100 milliseconds'::interval)",
):
    if phrase not in sql_smoke:
        fail(f"SQL extension smoke is missing runtime proof marker: {phrase}")

for phrase in (
    'docker exec -i "${container}" psql',
    "shared_preload_libraries=pg_stat_statements",
    "PostgreSQL init process complete",
    "ai_blaise_pg_stat_statements_seed",
    "companion_set_session_claims",
    "companion_current_session_claims",
    "companion_current_tenant_id",
    "uid claim must not be empty",
    "companion_pg_stat_local_activity",
    "companion_idle_transactions('100 milliseconds'::interval)",
):
    if phrase not in image_check:
        fail(f"image-check.sh must statically guard SQL smoke marker: {phrase}")

for phrase in (
    "CREATE FUNCTION companion_set_session_claims",
    "CREATE FUNCTION companion_current_session_claims",
    "CREATE FUNCTION companion_current_tenant_id",
    "'Auth2', 'tenant-aware claims', 'sql-runtime'",
):
    if phrase not in sources:
        fail(f"ai_blaise_citus SQL extension is missing Auth2 runtime marker: {phrase}")
    if phrase not in image_check:
        fail(f"image-check.sh must statically guard Auth2 SQL extension marker: {phrase}")

for phrase in (
    'psql -h 127.0.0.1 -p "${pool_port}"',
    "AI_BLAISE_POOL_UPSTREAM_ADDR",
    "AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST",
    "PostgreSQL init process complete",
    "ai_blaise_citus_pool_requests_total",
    "ai_blaise_citus_pool_rejected_connections_total",
    "pool CIDR deny smoke unexpectedly allowed PostgreSQL traffic",
    "raw PostgreSQL pipelined simple-query smoke passed through pool proxy",
    "pack_simple_query(\"SELECT 'pipeline_one'::text\")",
    "pack_simple_query(\"SELECT 'pipeline_two'::text\")",
    'expected = [["pipeline_one"], ["pipeline_two"]]',
):
    if phrase not in pool_smoke:
        fail(f"pool-proxy-smoke.sh is missing live SQL proof marker: {phrase}")
    escaped_phrase = phrase.replace('"', '\\"')
    if phrase not in image_check and escaped_phrase not in image_check:
        fail(f"image-check.sh must statically guard pool smoke marker: {phrase}")

for phrase in (
    "timescale/timescaledb:latest-pg17",
    "PostgreSQL init process complete",
    "CREATE EXTENSION IF NOT EXISTS timescaledb",
    "SELECT apply_distribute_hypertable",
    "SELECT apply_compression_policy_distributed",
    "SELECT apply_retention_policy_distributed",
    "SELECT apply_reorder_policy_distributed",
    "SELECT apply_continuous_aggregate_distributed",
    "SELECT apply_time_range_shard_pruner",
    "_timescaledb_catalog.hypertable",
    "companion_timescale_bridge_state",
):
    if phrase not in timescale_smoke:
        fail(f"timescale-bridge-smoke.sh is missing real Timescale proof marker: {phrase}")
    if phrase not in image_check:
        fail(f"image-check.sh must statically guard Timescale smoke marker: {phrase}")

for phrase in (
    "timescale/timescaledb:latest-pg17",
    "shared_preload_libraries=timescaledb,citus",
    "citus.cohabit_extensions=timescaledb",
    "CREATE EXTENSION IF NOT EXISTS citus",
    "CREATE EXTENSION IF NOT EXISTS timescaledb",
    "CREATE EXTENSION IF NOT EXISTS ai_blaise_citus",
    "SELECT create_distributed_table('citus_smoke_events', 'tenant_id')",
    "SELECT apply_distribute_hypertable",
    "SELECT apply_compression_policy_distributed",
    "SELECT apply_retention_policy_distributed",
    "SELECT apply_reorder_policy_distributed",
    "SELECT apply_continuous_aggregate_distributed",
    "SELECT apply_time_range_shard_pruner",
    "pg_dist_partition",
    "expected six Timescale bridge feature ids",
    "timescale-cohabitation-evidence.tsv",
    "stable image identity",
    "git_sha",
    "command_path",
):
    if phrase not in timescale_cohabitation_smoke:
        fail(
            "timescale-cohabitation-smoke.sh is missing real cohabitation marker: "
            + phrase
        )
    if phrase not in image_check:
        fail(f"image-check.sh must statically guard cohabitation smoke marker: {phrase}")
if "CREATE FUNCTION create_distributed_table" in timescale_cohabitation_smoke:
    fail("timescale-cohabitation-smoke.sh must not stub Citus create_distributed_table")
for phrase in (
    "IsTrustedHookCoextension",
    'pg_strcasecmp(coextensionName, "timescaledb")',
):
    if phrase not in shared_library_init:
        fail(f"shared_library_init.c must constrain trusted coextensions: {phrase}")
    if phrase not in image_check:
        fail(f"image-check.sh must statically guard trusted coextension marker: {phrase}")

for phrase in (
    "wal_level=replica",
    "pg_basebackup",
    "PostgreSQL init process complete",
    "pg_is_in_recovery()",
    "companion_pg_stat_local_activity",
    "companion_pg_stat_distributed",
    "companion_pg_dist_replication_lag",
    "state = 'streaming'",
):
    if phrase not in observability_replication_smoke:
        fail(
            "observability-replication-smoke.sh is missing live replication proof marker: "
            + phrase
        )
    if phrase not in image_check:
        fail(f"image-check.sh must statically guard observability smoke marker: {phrase}")

for phrase in (
    "kind create cluster",
    "scripts/citus-scale/build-app-images.sh",
    "helm upgrade --install",
    "values-exhaustive.yaml",
    "global.requireImageDigest=false",
    "apply_monitoring_crds",
    "assert_observability_resources",
    "configmap/${chart_name}-dashboards",
    "prometheusrules.monitoring.coreos.com/${chart_name}-alerts",
    "ai-blaise-citus-overview.json",
    "ai-blaise-citus-sidecars.json",
    "AiBlaiseCitusSidecarNotReady",
    "clamp_min",
    "> 0",
    "DEFAULT_VALUES_NAMESPACE",
    "DEPLOY_PROFILE=prod",
    "MODE=install",
    "ALLOW_MUTABLE_IMAGE_TAGS=1",
    "scripts/citus-scale/deploy.sh",
    "assert_no_alpha_workload_deployments",
    "exhaustive image-matrix smoke passed",
    'helm uninstall "${release}"',
    "ClusterRole cleanup",
    "values.yaml default production-safe profile smoke passed",
    "values-prod.yaml production profile smoke passed",
    "port-forward",
    "/healthz",
    "/readyz",
    "/metrics",
    "psql -h ai-blaise-citus-pool",
    "probe_pool_admin_pods",
    "pool admin smoke did not observe ready upstream metrics",
    "ai_blaise_citus_pool_requests_total",
    "run_pool_cidr_deny_smoke",
    "ai-blaise-pool-cidr-deny-smoke",
    "pool CIDR deny smoke passed",
    "ai_blaise_citus_pool_rejected_connections_total",
    "run_citusctl_image_smoke",
    "ai-blaise-citusctl-image-smoke",
    "citusctl inspect destructive=false requires_plan_id=true steps=3",
):
    if phrase not in kind_smoke:
        fail(f"kind-production-smoke.sh is missing live deployment proof marker: {phrase}")

build_app_images = read(ROOT / "scripts/citus-scale/build-app-images.sh")
for phrase in (
    "DIGEST_FILE",
    "push_output",
    "DEFAULT_ARGS",
    "plan inspect cluster",
    "ai-blaise-image-digests.tsv",
    "did not report an immutable repo digest",
):
    if phrase not in build_app_images:
        fail(f"build-app-images.sh must preserve release digest manifest marker: {phrase}")
    if phrase not in image_check:
        fail(f"image-check.sh must guard release digest manifest marker: {phrase}")
for phrase in (
    "app-image-digest-manifest-smoke.sh",
    "FAKE_DOCKER_DIGEST_MODE=missing",
    "FAKE_DOCKER_PUSH_DIGEST_MODE=missing",
):
    if phrase not in image_check:
        fail(f"image-check.sh must guard release digest smoke marker: {phrase}")
if "repository\\timage\\ttag\\tdigest\\tpackage\\tbinary\\tpushed" not in build_app_images:
    fail("build-app-images.sh must write the release digest manifest header")
if "repository\\\\timage\\\\ttag\\\\tdigest\\\\tpackage\\\\tbinary\\\\tpushed" not in image_check:
    fail("image-check.sh must guard the release digest manifest header")

if "values-prod.yaml" not in argo_app or "valueFiles:" not in argo_app:
    fail("Argo application must install the production values profile")
if "targetRevision: main" not in argo_app:
    fail("Argo application must target the main release branch")
for phrase in ("prune: true", "selfHeal: true", "CreateNamespace=true", "PruneLast=true"):
    if phrase not in argo_app:
        fail(f"Argo production application must preserve sync guardrail: {phrase}")

if "kind-production-smoke:" not in deploy_workflow:
    fail("deploy workflow must include the live Kubernetes production smoke job")
if "bash ci/ai-blaise/kind-production-smoke.sh" not in deploy_workflow:
    fail("deploy workflow must run ci/ai-blaise/kind-production-smoke.sh")
if "Install Helm for rendered chart checks" not in deploy_workflow:
    fail("deploy workflow must install Helm before rendered deploy checks")

gate_close_dependencies = makefile.split("gate-close:", 1)[-1].splitlines()[0]
if "kind-production-smoke" not in gate_close_dependencies:
    fail("gate-close must include the live Kubernetes production smoke")
if "image-check" not in gate_close_dependencies:
    fail("gate-close must include image-check")
if "deploy-check" not in gate_close_dependencies:
    fail("gate-close must include deploy-check")
if "image-check:\n\t@bash ci/ai-blaise/image-check.sh" not in makefile:
    fail("Makefile image-check target must run bash ci/ai-blaise/image-check.sh")
if "deploy-check:\n\t@REQUIRE_HELM=1 bash ci/ai-blaise/deploy-check.sh" not in makefile:
    fail("Makefile deploy-check target must fail closed with REQUIRE_HELM=1")
for target, command in (
    ("pool-proxy-smoke", "REQUIRE_DOCKER=1 ci/ai-blaise/pool-proxy-smoke.sh"),
    ("sql-extension-smoke", "REQUIRE_DOCKER=1 ci/ai-blaise/sql-extension-smoke.sh"),
    ("timescale-bridge-smoke", "REQUIRE_DOCKER=1 ci/ai-blaise/timescale-bridge-smoke.sh"),
    (
        "timescale-cohabitation-smoke",
        "REQUIRE_DOCKER=1 ci/ai-blaise/timescale-cohabitation-smoke.sh",
    ),
    (
        "observability-replication-smoke",
        "REQUIRE_DOCKER=1 ci/ai-blaise/observability-replication-smoke.sh",
    ),
):
    target_marker = f"{target}:\n\t@{command}"
    if target_marker not in makefile:
        fail(f"Makefile {target} target must fail closed with {command}")

for path, text in (
    (DEPLOY_WORKFLOW, deploy_workflow),
    (POOL_WORKFLOW, pool_workflow),
    (OPERATOR_WORKFLOW, operator_workflow),
    (SIDECAR_WORKFLOW, sidecar_workflow),
    (SLOP_WORKFLOW, slop_workflow),
):
    if "- main" not in text or "- ai-blaise/dev" not in text:
        fail(f"{path} must run on main and ai-blaise/dev pushes")

for path in CUSTOM_CI_WORKFLOWS:
    text = read(path)
    if "- main" not in text or "- ai-blaise/dev" not in text:
        fail(f"{path} must run on main and ai-blaise/dev pushes")
    if "- ai-blaise/bootstrap-v2" in text:
        fail(f"{path} must not target stale ai-blaise/bootstrap-v2")

for path, text in (
    (DASHBOARD_TEMPLATE, dashboard_template),
    (PROMRULE_TEMPLATE, promrule_template),
):
    if "ai_blaise_sidecar_ready" not in text:
        fail(f"{path} must query the sidecar metric emitted by runtime.rs")
    if "ai_blaise_citus_sidecar_ready" in text:
        fail(f"{path} contains stale sidecar metric name ai_blaise_citus_sidecar_ready")

def dashboard_json_blocks(template: str):
    blocks = {}
    pattern = re.compile(r"^  ([A-Za-z0-9_.-]+\.json): \|-\n((?:    .*\n)+)", re.M)
    for match in pattern.finditer(template):
        name = match.group(1)
        raw_json = "\n".join(line[4:] for line in match.group(2).splitlines())
        try:
            blocks[name] = json.loads(raw_json)
        except json.JSONDecodeError as error:
            fail(f"{name} contains invalid Grafana dashboard JSON: {error}")
    return blocks

dashboards = dashboard_json_blocks(dashboard_template)
expected_dashboards = {
    "ai-blaise-citus-overview.json",
    "ai-blaise-citus-sidecars.json",
}
if set(dashboards) != expected_dashboards:
    fail(
        "observability dashboard template must contain exactly "
        + ", ".join(sorted(expected_dashboards))
    )

expected_panel_exprs = {
    "ai-blaise-citus-overview.json": {
        "Coordinator query latency p95": "histogram_quantile(0.95, sum(rate(ai_blaise_citus_query_duration_seconds_bucket[5m])) by (le, query_class))",
        "Distributed replication lag": "max(ai_blaise_citus_replication_lag_seconds) by (region)",
        "Vectorizer backlog": "sum(ai_blaise_citus_vectorizer_backlog_jobs) by (tenant)",
    },
    "ai-blaise-citus-sidecars.json": {
        "Sidecar readiness": "min(ai_blaise_sidecar_ready) by (component)",
        "Pool error rate percent": "100 * sum(rate(ai_blaise_citus_pool_errors_total[5m])) / clamp_min(sum(rate(ai_blaise_citus_pool_requests_total[5m])), 0.001)",
    },
}
for dashboard_name, panels in expected_panel_exprs.items():
    actual_panels = {
        panel.get("title"): {
            target.get("expr")
            for target in panel.get("targets", [])
            if isinstance(target, dict)
        }
        for panel in dashboards[dashboard_name].get("panels", [])
        if isinstance(panel, dict)
    }
    for title, expected_expr in panels.items():
        if expected_expr not in actual_panels.get(title, set()):
            fail(f"{dashboard_name} must preserve panel {title!r} expression {expected_expr!r}")

for phrase in (
    "clamp_min(sum(rate(ai_blaise_citus_pool_requests_total[5m])), 0.001)",
    "and sum(rate(ai_blaise_citus_pool_requests_total[5m])) > 0",
):
    if phrase not in promrule_template:
        fail(f"PrometheusRule must preserve guarded pool error-rate expression: {phrase}")
if "/ sum(rate(ai_blaise_citus_pool_requests_total[5m]))" in dashboard_template:
    fail("Grafana pool error-rate panel must not divide by raw request rate")
if "/ sum(rate(ai_blaise_citus_pool_requests_total[5m]))" in promrule_template:
    fail("Prometheus pool error-rate alert must not divide by raw request rate")

if 'resources: ["*"]' in operator_rbac_template:
    fail("operator RBAC must enumerate ai-blaise resources explicitly")
if '"secrets"' in operator_rbac_template:
    fail("operator RBAC must not grant Secret access while secret binding is alpha")
for resource in ("citusclusters", "hypertables", "scheduledrepacks"):
    if resource not in operator_rbac_template:
        fail(f"operator RBAC must include explicit resource: {resource}")

for phrase in (
    "FEATURE: Sec13",
    "kind: NetworkPolicy",
    "cidrAllowlist",
    "ipBlock:",
):
    if phrase not in pool_networkpolicy_template:
        fail(f"pool NetworkPolicy template must preserve Sec13 render marker: {phrase}")
for phrase in (
    "AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST",
    "pool.networkPolicy.cidrAllowlist",
):
    if phrase not in pool_deployment_template:
        fail(f"pool deployment must pass Sec13 CIDR allowlist to runtime: {phrase}")

for phrase in (
    "bash ci/ai-blaise/app-image-digest-manifest-smoke.sh",
    "REQUIRE_DOCKER=1 bash ci/ai-blaise/sql-extension-smoke.sh",
    "REQUIRE_DOCKER=1 bash ci/ai-blaise/timescale-bridge-smoke.sh",
    "REQUIRE_DOCKER=1 bash ci/ai-blaise/timescale-cohabitation-smoke.sh",
    "REQUIRE_DOCKER=1 bash ci/ai-blaise/observability-replication-smoke.sh",
):
    if phrase not in image_workflow:
        fail(f"ci-image.yml must run production runtime smoke: {phrase}")

for target in (
    "app-image-digest-manifest-smoke",
    "sql-extension-smoke",
    "timescale-bridge-smoke",
    "timescale-cohabitation-smoke",
    "observability-replication-smoke",
):
    if target not in makefile.split("gate-close:", 1)[-1]:
        fail(f"gate-close must include {target}")

if "values-prod.yaml must not enable alpha sidecars by default" not in deploy_check:
    fail("deploy-check.sh must reject alpha sidecars enabled in production values")
if "values-prod.yaml must not enable alpha runtime/security intent controls by default" not in deploy_check:
    fail("deploy-check.sh must reject alpha runtime/security intent controls enabled in production values")
if "values-prod.yaml render must require immutable operator/pool image digests" not in deploy_check:
    fail("deploy-check.sh must reject production values renders without immutable image digests")
if "values.yaml default render must require immutable operator/pool image digests" not in deploy_check:
    fail("deploy-check.sh must reject default Helm renders without immutable image digests")
if "values.yaml default render must not include alpha sidecar deployments" not in deploy_check:
    fail("deploy-check.sh must reject alpha sidecars in the default Helm render")
if "values.yaml must not enable alpha sidecars by default" not in deploy_check:
    fail("deploy-check.sh must reject alpha sidecars enabled in default values")
if "values.yaml must not enable alpha runtime/security intent controls by default" not in deploy_check:
    fail("deploy-check.sh must reject alpha runtime/security intent controls enabled in default values")
if "deploy.sh default production render must require immutable operator/pool image digests" not in deploy_check:
    fail("deploy-check.sh must reject deploy wrapper production renders without immutable image digests")
if "requires an immutable digest" not in deploy_check:
    fail("deploy-check.sh must assert Helm emits the immutable digest failure")
if 'require_helm="${REQUIRE_HELM:-0}"' not in deploy_check:
    fail("deploy-check.sh must expose REQUIRE_HELM fail-closed mode")
if "helm is required for rendered chart profile checks" not in deploy_check:
    fail("deploy-check.sh must fail when required Helm is missing")
for phrase in (
    "DEPLOY_PROFILE=prod",
    "MODE=install",
    "ALLOW_MUTABLE_IMAGE_TAGS=1",
    "scripts/citus-scale/deploy.sh",
):
    if phrase not in deploy_check or phrase not in kind_smoke:
        fail(f"kind-production-smoke.sh/deploy-check.sh must live-gate deploy wrapper install marker: {phrase}")

for phrase in (
    "requireImageDigest:",
    "digest: \"\"",
):
    if phrase not in default_values:
        fail(f"values.yaml must preserve image digest field: {phrase}")
if "requireImageDigest: true" not in prod_values:
    fail("values-prod.yaml must require immutable image digests")
if "requireImageDigest: true" not in default_values:
    fail("values.yaml default profile must require immutable image digests")
if "requireImageDigest: false" not in exhaustive_values:
    fail("values-exhaustive.yaml must remain the explicit non-production image-matrix profile")

for phrase in (
    "global.requireImageDigest",
    "requires an immutable digest",
    "@%s",
):
    if phrase not in helper_template:
        fail(f"Helm image helper must preserve digest guard/render marker: {phrase}")
for path, text, phrase in (
    (OPERATOR_DEPLOYMENT_TEMPLATE, operator_deployment_template, "operator.image.digest"),
    (POOL_DEPLOYMENT_TEMPLATE, pool_deployment_template, "pool.image.digest"),
    (SIDECAR_DEPLOYMENT_TEMPLATE, sidecar_deployment_template, "sidecarDefaults.digest"),
    (TOOLS_DEPLOYMENT_TEMPLATE, tools_deployment_template, "tools.image.digest"),
):
    if phrase not in text:
        fail(f"{path} must pass image digest into the image helper")
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

alpha_intent_findings = []
if re.search(r"protocolPipeline:\n(?:    .*\n)*    enabled:\s+true\b", prod_values):
    alpha_intent_findings.append("T7 pool.protocolPipeline.enabled")
if re.search(r"ioMethod:\s+io_uring\b", prod_values):
    alpha_intent_findings.append("T6 postgres.ioMethod")
if re.search(r"externalSecrets:\n(?:    .*\n)*    enabled:\s+true\b", prod_values):
    alpha_intent_findings.append("Sec7 security.externalSecrets.enabled")
if re.search(r"tls:\n(?:    .*\n)*    (clients|postgres|sidecars):\s+true\b", prod_values):
    alpha_intent_findings.append("Sec8 security.tls")
if re.search(
    r"releaseAttestation:\n(?:    .*\n)*    (sbom|cosign):\s+true\b",
    prod_values,
):
    alpha_intent_findings.append("Sec9 security.releaseAttestation")
if alpha_intent_findings:
    fail(
        "values-prod.yaml must not enable alpha runtime/security intent controls by default: "
        + ", ".join(alpha_intent_findings)
    )

for phrase in (
    "protocolPipeline:",
    "ioMethod: io_uring",
    "externalSecrets:",
    "tls:",
    "releaseAttestation:",
    "cidrAllowlist:",
):
    if phrase not in exhaustive_values:
        fail(f"values-exhaustive.yaml must preserve alpha security intent field: {phrase}")

for phrase in (
    "runtime and security controls are alpha intent",
    "not active production enforcement",
    "production values keep those alpha controls disabled",
    "deploy wrapper defaults to `values-prod.yaml`",
    "global.requireimagedigest: true",
    "operator_image_digest=sha256:",
    "ai-blaise-image-digests.tsv",
    "pushed image without a reported",
    "makefile smoke targets set `require_docker=1`",
    "allow_mutable_image_tags=1",
    "must not be used as release image-pinning evidence",
    "gitops sync intentionally fails closed",
    "`feature: sec13` pool cidr access control is production-ready",
    "ai_blaise_citus_pool_rejected_connections_total",
):
    if phrase not in runbook_compact:
        fail(f"production runbook must preserve runtime/security alpha guardrail: {phrase}")

for phrase in (
    'deploy_profile="${DEPLOY_PROFILE:-prod}"',
    "values-prod.yaml",
    "ALLOW_ALPHA_INSTALL",
    "OPERATOR_IMAGE_DIGEST",
    "POOL_IMAGE_DIGEST",
    "ALLOW_MUTABLE_IMAGE_TAGS",
    "refusing to install non-production values file",
):
    if phrase not in deploy_script:
        fail(f"deploy.sh must preserve production-safe default marker: {phrase}")

for phrase in (
    'deploy_profile="${DEPLOY_PROFILE:-prod}"',
    "deploy.sh default render must use production values without alpha sidecars",
    "deploy.sh must refuse non-production installs unless ALLOW_ALPHA_INSTALL=1",
    "values-prod.yaml render must require immutable operator/pool image digests",
    "deploy.sh default production render must require immutable operator/pool image digests",
    "dashboard_json_blocks",
    "Pool error rate percent",
    "pool error-rate dashboard expression must use clamp_min denominator guard",
    "pool error-rate alert expression must preserve marker",
):
    if phrase not in deploy_check:
        fail(f"deploy-check.sh must enforce deploy wrapper production-safe default: {phrase}")

for phrase in (
    "deploy wrapper defaults to `values-prod.yaml`",
    "allow_alpha_install=1",
    "default `values.yaml` profile and `values-prod.yaml` both set `global.requireimagedigest: true`",
    "runtime behavior, not release image pinning",
    "gitops sync fails closed",
    "ai-blaise-image-digests.tsv",
    "release pushes fail",
    "release gate could silently skip live docker smokes",
    "makefile live-smoke targets now set `require_docker=1`",
    "release gate could silently skip rendered helm chart checks",
    "makefile release gate now runs `image-check` and `deploy-check`",
    "require_helm=1",
    "deploy wrapper install path is now live-gated",
    "`ts18` now has real citus+timescaledb cohabitation evidence",
    "ts6 and ts18 are therefore production-ready narrow surfaces",
    "tools deployment remains dev-only",
    "argo application is a gitops render contract, not live controller evidence",
    "sec13 pool cidr access control is now enforced by the live pool data path",
    "ai_blaise_citus_pool_rejected_connections_total",
    "parses the embedded grafana dashboard json",
    "guarded pool error-rate denominator",
    "positive request traffic before firing",
    "parsed grafana json",
    "exact panel/promql contracts",
):
    if phrase not in audit_compact:
        fail(f"PRODUCTION_READINESS_AUDIT.md must preserve deploy-wrapper guardrail: {phrase}")

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
