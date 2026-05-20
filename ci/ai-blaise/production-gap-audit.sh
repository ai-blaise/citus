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
BUNDLED_EXTENSIONS_DOC = ROOT / "docs/ai-blaise/BUNDLED_EXTENSIONS.md"
PG_OVERLAY_README = ROOT / "images/citus-pg-overlay/README.md"
RELEASE_GATES = ROOT / "e2e/src/release_gates.rs"
V2_ACCEPTANCE = ROOT / "ci/ai-blaise/v2-acceptance-check.sh"
SQL_SMOKE = ROOT / "ci/ai-blaise/sql-extension-smoke.sh"
IMAGE_CHECK = ROOT / "ci/ai-blaise/image-check.sh"
POOL_SMOKE = ROOT / "ci/ai-blaise/pool-proxy-smoke.sh"
TIMESCALE_SMOKE = ROOT / "ci/ai-blaise/timescale-bridge-smoke.sh"
OBSERVABILITY_REPLICATION_SMOKE = ROOT / "ci/ai-blaise/observability-replication-smoke.sh"
KIND_SMOKE = ROOT / "ci/ai-blaise/kind-production-smoke.sh"
DEPLOY_CHECK = ROOT / "ci/ai-blaise/deploy-check.sh"
PROD_VALUES = ROOT / "deploy/k8s/helm/citus-overlay/values-prod.yaml"
DEFAULT_VALUES = ROOT / "deploy/k8s/helm/citus-overlay/values.yaml"
ARGO_APP = ROOT / "deploy/k8s/argo/app.yaml"
DASHBOARD_TEMPLATE = ROOT / "deploy/k8s/helm/citus-overlay/templates/observability-dashboards.yaml"
PROMRULE_TEMPLATE = ROOT / "deploy/k8s/helm/citus-overlay/templates/observability-prometheusrules.yaml"
PROD_READINESS = ROOT / "ci/ai-blaise/production-readiness-check.sh"
IMAGE_WORKFLOW = ROOT / ".github/workflows/ci-image.yml"
DEPLOY_WORKFLOW = ROOT / ".github/workflows/ci-deploy.yml"
POOL_WORKFLOW = ROOT / ".github/workflows/ci-pool.yml"
OPERATOR_WORKFLOW = ROOT / ".github/workflows/ci-operator.yml"
SIDECAR_WORKFLOW = ROOT / ".github/workflows/ci-sidecar.yml"
MAKEFILE = ROOT / "Makefile.ai-blaise"

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
bundled_extensions_doc = read(BUNDLED_EXTENSIONS_DOC)
pg_overlay_readme = read(PG_OVERLAY_README)
release_gates = read(RELEASE_GATES)
v2_acceptance = read(V2_ACCEPTANCE)
sql_smoke = read(SQL_SMOKE)
image_check = read(IMAGE_CHECK)
pool_smoke = read(POOL_SMOKE)
timescale_smoke = read(TIMESCALE_SMOKE)
observability_replication_smoke = read(OBSERVABILITY_REPLICATION_SMOKE)
kind_smoke = read(KIND_SMOKE)
deploy_check = read(DEPLOY_CHECK)
prod_values = read(PROD_VALUES)
default_values = read(DEFAULT_VALUES)
argo_app = read(ARGO_APP)
dashboard_template = read(DASHBOARD_TEMPLATE)
promrule_template = read(PROMRULE_TEMPLATE)
image_workflow = read(IMAGE_WORKFLOW)
deploy_workflow = read(DEPLOY_WORKFLOW)
pool_workflow = read(POOL_WORKFLOW)
operator_workflow = read(OPERATOR_WORKFLOW)
sidecar_workflow = read(SIDECAR_WORKFLOW)
makefile = read(MAKEFILE)
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
bundled_extensions_compact = compact(bundled_extensions_doc)
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
):
    if pattern in docs_compact:
        fail(f"NEW_FEATURES.md contains stale production-ready overclaim: {pattern}")

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

for phrase in (
    "manifest/init contract, not production evidence",
    "feature: bundle1` remains alpha",
    "real operand image build smoke verifies",
):
    if phrase not in bundled_extensions_compact:
        fail(f"BUNDLED_EXTENSIONS.md must preserve operand-image alpha guardrail: {phrase}")

for phrase in (
    "not production evidence that every binary package",
    "feature: bundle1` remains alpha",
    "real image build smoke verifies",
):
    if phrase not in pg_overlay_readme_compact:
        fail(f"images/citus-pg-overlay/README.md must preserve operand-image alpha guardrail: {phrase}")

for path, text in (
    (BUNDLED_EXTENSIONS_DOC, bundled_extensions_compact),
    (PG_OVERLAY_README, pg_overlay_readme_compact),
):
    for pattern in (
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
    "ai_blaise_pg_stat_statements_seed",
    "companion_pg_stat_local_activity",
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
    "timescale/timescaledb:latest-pg17",
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
    "wal_level=replica",
    "pg_basebackup",
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
    "apply_monitoring_crds",
    "-f deploy/k8s/helm/citus-overlay/values-prod.yaml",
    "assert_no_alpha_workload_deployments",
    "exhaustive image-matrix smoke passed",
    'helm uninstall "${release}"',
    "ClusterRole cleanup",
    "values-prod.yaml production profile smoke passed",
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

if "values-prod.yaml" not in argo_app or "valueFiles:" not in argo_app:
    fail("Argo application must install the production values profile")

if "kind-production-smoke:" not in deploy_workflow:
    fail("deploy workflow must include the live Kubernetes production smoke job")
if "bash ci/ai-blaise/kind-production-smoke.sh" not in deploy_workflow:
    fail("deploy workflow must run ci/ai-blaise/kind-production-smoke.sh")
if "Install Helm for rendered chart checks" not in deploy_workflow:
    fail("deploy workflow must install Helm before rendered deploy checks")

gate_close_dependencies = makefile.split("gate-close:", 1)[-1].splitlines()[0]
if "kind-production-smoke" not in gate_close_dependencies:
    fail("gate-close must include the live Kubernetes production smoke")

for path, text in (
    (POOL_WORKFLOW, pool_workflow),
    (OPERATOR_WORKFLOW, operator_workflow),
    (SIDECAR_WORKFLOW, sidecar_workflow),
):
    if "ai-blaise/bootstrap-v2" not in text:
        fail(f"{path} must run on ai-blaise/bootstrap-v2 pushes")

for path, text in (
    (DASHBOARD_TEMPLATE, dashboard_template),
    (PROMRULE_TEMPLATE, promrule_template),
):
    if "ai_blaise_sidecar_ready" not in text:
        fail(f"{path} must query the sidecar metric emitted by runtime.rs")
    if "ai_blaise_citus_sidecar_ready" in text:
        fail(f"{path} contains stale sidecar metric name ai_blaise_citus_sidecar_ready")

for phrase in (
    "REQUIRE_DOCKER=1 bash ci/ai-blaise/sql-extension-smoke.sh",
    "REQUIRE_DOCKER=1 bash ci/ai-blaise/timescale-bridge-smoke.sh",
    "REQUIRE_DOCKER=1 bash ci/ai-blaise/observability-replication-smoke.sh",
):
    if phrase not in image_workflow:
        fail(f"ci-image.yml must run production runtime smoke: {phrase}")

for target in (
    "sql-extension-smoke",
    "timescale-bridge-smoke",
    "observability-replication-smoke",
):
    if target not in makefile.split("gate-close:", 1)[-1]:
        fail(f"gate-close must include {target}")

if "values-prod.yaml must not enable alpha sidecars by default" not in deploy_check:
    fail("deploy-check.sh must reject alpha sidecars enabled in production values")
if "values-prod.yaml must not enable alpha runtime/security intent controls by default" not in deploy_check:
    fail("deploy-check.sh must reject alpha runtime/security intent controls enabled in production values")
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
if re.search(
    r"networkPolicy:\n(?:    .*\n)*    cidrAllowlist:\n(?:      - .+\n)+",
    prod_values,
):
    alpha_intent_findings.append("Sec13 pool.networkPolicy.cidrAllowlist")
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
    if phrase not in default_values:
        fail(f"values.yaml must preserve alpha security intent field: {phrase}")

for phrase in (
    "runtime and security controls are alpha intent",
    "not active production enforcement",
    "production values keep those alpha controls disabled",
):
    if phrase not in runbook_compact:
        fail(f"production runbook must preserve runtime/security alpha guardrail: {phrase}")

for path in (
    DOCS,
    AUDIT,
    RELEASING,
    RUNBOOK,
    E2E_DOC,
    BUNDLED_EXTENSIONS_DOC,
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
