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
SIDECAR_CONTROLLER_LIVE_SMOKE = ROOT / "ci/ai-blaise/sidecar-controller-live-smoke.sh"
SIDECAR_SHARED_README = ROOT / "sidecar/shared/README.md"
SIDECAR_CDC_SMOKE = ROOT / "ci/ai-blaise/sidecar-cdc-smoke.sh"
SIDECAR_CDC_README = ROOT / "sidecar/cdc/README.md"
SIDECAR_CDC_MODIFICATION = ROOT / "sidecar/cdc/MODIFICATION.md"
OPERATOR_RECONCILERS_BATCH_C_SMOKE = ROOT / "ci/ai-blaise/operator-reconcilers-batch-c-smoke.sh"
COMPANION_RUNTIME_DEPTH_A_SMOKE = ROOT / "ci/ai-blaise/companion-runtime-depth-a-smoke.sh"
GRAPHQL_POSTGREST_RUNTIME_SMOKE = ROOT / "ci/ai-blaise/graphql-postgrest-runtime-smoke.sh"
POSTGREST_LIVE_DATA_PLANE_SMOKE = ROOT / "ci/ai-blaise/postgrest-live-data-plane-smoke.sh"
STRUCTURED_LOG_INGESTION_SMOKE = ROOT / "ci/ai-blaise/structured-log-ingestion-smoke.sh"
OBSERVABILITY_WORKFLOW = ROOT / ".github/workflows/ci-observability-contracts.yml"
SIDECAR_REALTIME_SMOKE = ROOT / "ci/ai-blaise/sidecar-realtime-smoke.sh"
SIDECAR_REALTIME_README = ROOT / "sidecar/realtime/README.md"
STORAGE_RUNTIME_SMOKE = ROOT / "ci/ai-blaise/storage-sidecar-runtime-smoke.sh"
POOL_PROXY_SMOKE = ROOT / "ci/ai-blaise/pool-proxy-smoke.sh"
SQL_EXTENSION_SMOKE = ROOT / "ci/ai-blaise/sql-extension-smoke.sh"
POOL_ROUTING_SECURITY_SMOKE = ROOT / "ci/ai-blaise/pool-routing-security-smoke.sh"
PLACEMENT_GENERATION_UDF_SMOKE = ROOT / "ci/ai-blaise/placement-generation-udf-contract-smoke.sh"
SECURITY_SUPPLY_CHAIN_SMOKE = ROOT / "ci/ai-blaise/security-supply-chain-smoke.sh"
SECURITY_EXTERNAL_SECRETS_TLS_LIVE_SMOKE = ROOT / "ci/ai-blaise/security-external-secrets-tls-live-smoke.sh"
SECURITY_SBOM_COSIGN_LIVE_SMOKE = ROOT / "ci/ai-blaise/security-sbom-cosign-live-smoke.sh"
PATCHES_WORKFLOW = ROOT / ".github/workflows/ci-patches.yml"
OPERATOR_WORKFLOW = ROOT / ".github/workflows/ci-operator.yml"
PRODUCTION_WORKFLOW = ROOT / ".github/workflows/ci-production-readiness.yml"
CITUS_PATCH_AUDIT = ROOT / "ci/ai-blaise/citus-patch-production-audit.sh"
RUNBOOK_CHECK = ROOT / "ci/ai-blaise/runbook-command-check.sh"
K8S_GUARDRAIL_RENDERER = ROOT / "deploy/contracts/render_k8s_guardrails.py"
K8S_GUARDRAIL_MANIFEST = ROOT / "deploy/contracts/k8s-production-guardrails.yaml"
K8S_GUARDRAIL_KUSTOMIZATION = ROOT / "deploy/contracts/kustomization.yaml"
DEPLOY_CHECK = ROOT / "ci/ai-blaise/deploy-check.sh"
K8S_GUARDRAIL_CHECK = ROOT / "ci/ai-blaise/k8s-guardrails-check.sh"
KIND_PRODUCTION_SMOKE = ROOT / "ci/ai-blaise/kind-production-smoke.sh"
K8S_PRODUCTION_VALUES_LIVE_SMOKE = ROOT / "ci/ai-blaise/k8s-production-values-live-smoke.sh"
LIVE_K8S_E2E = ROOT / "ci/ai-blaise/live-k8s-e2e.sh"
DEPLOY_README = ROOT / "deploy/README.md"
DR_RESTORE_DEPTH_CHECK = ROOT / "ci/ai-blaise/dr-restore-depth-check.sh"
TIMESCALE_BRIDGE_SMOKE = ROOT / "ci/ai-blaise/timescale-bridge-smoke.sh"
TIMESCALE_COHABITATION_SMOKE = ROOT / "ci/ai-blaise/timescale-cohabitation-smoke.sh"
OPERATOR_HYPERTABLE_LIVE_SMOKE = ROOT / "ci/ai-blaise/operator-hypertable-live-smoke.sh"
TIMESCALE_COHABITATION_DOCKERFILE = ROOT / "images/citus-timescale-cohabitation/Dockerfile"
PG_CRON_COHABITATION_SMOKE = ROOT / "ci/ai-blaise/pg-cron-cohabitation-smoke.sh"
TS_VERSION_MATRIX_SMOKE = ROOT / "ci/ai-blaise/ts-version-matrix-smoke.sh"
SQL_EXTENSION = ROOT / "images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql"
AI_SQL_CONTRACT_SMOKE = ROOT / "ci/ai-blaise/ai-sql-contract-smoke.sh"
CI_IMAGE_WORKFLOW = ROOT / ".github/workflows/ci-image.yml"
TOOLS_WORKFLOW = ROOT / ".github/workflows/ci-tools.yml"
CITUSCTL_SMOKE = ROOT / "ci/ai-blaise/citusctl-smoke.sh"
CITUSCTL_DEV_LIFECYCLE_SMOKE = ROOT / "ci/ai-blaise/citusctl-dev-lifecycle-smoke.sh"
CITUSCTL_LIB = ROOT / "tools/citusctl/src/lib.rs"
BUNDLE1_LOCK = ROOT / "images/citus-pg-overlay/bundle1-source-build.lock.tsv"
BUNDLE1_CONTRACT_CHECK = ROOT / "ci/ai-blaise/bundle1-contract-check.py"
IMAGE_CHECK = ROOT / "ci/ai-blaise/image-check.sh"

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


def feature_section(docs: str, feature_id: str) -> str:
    heading_re = re.compile(r"^###\s+([A-Za-z][A-Za-z0-9]*):\s+(.+)$", re.M)
    headings = list(heading_re.finditer(docs))
    for index, heading in enumerate(headings):
        if heading.group(1) != feature_id:
            continue
        end = headings[index + 1].start() if index + 1 < len(headings) else len(docs)
        return docs[heading.start():end]
    fail(f"missing feature heading in NEW_FEATURES.md: {feature_id}")


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
                "body": body,
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

status_by_id = {entry["id"]: entry["status"] for entry in entries}
entry_by_id = {entry["id"]: entry for entry in entries}
for feature_id in ("C6", "C7", "C8"):
    if status_by_id.get(feature_id) != "alpha":
        fail(f"{feature_id} branch lifecycle must remain alpha until live CSI/Kubernetes execution evidence exists")
for feature_id in ("MR3", "MR5", "MR9"):
    if status_by_id.get(feature_id) != "alpha":
        fail(f"{feature_id} must remain alpha until live multi-region runtime evidence exists")

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

d1_section = feature_section(docs, "D1")
m8_section = feature_section(docs, "M8")
for phrase in (
    "**Status**: production-ready",
    "explicit `--state-dir`",
    "json and tsv outputs are deterministic",
    "local `dev-lifecycle.audit.tsv` log",
    "not evidence for docker/kind startup",
):
    if compact(phrase) not in compact(d1_section):
        fail(f"D1 citusctl dev lifecycle production boundary missing phrase: {phrase}")
for phrase in (
    "**Status**: alpha",
    "M8 is not production-ready as a whole",
    "bounded D1 local dev lifecycle subpath",
    "deterministic JSON/TSV output",
    "local audit append",
    "does not execute manifests against Kubernetes",
):
    if compact(phrase) not in compact(m8_section):
        fail(f"M8 citusctl plan/apply boundary missing phrase: {phrase}")
for phrase in (
    "explicit `--state-dir` invocations",
    "deterministic JSON/TSV output",
    "local audit append",
    "M8 remains alpha outside that bounded D1 subpath",
    "production cluster lifecycle management",
):
    if compact(phrase) not in compact(audit):
        fail(f"PRODUCTION_READINESS_AUDIT.md must preserve D1/M8 citusctl boundary: {phrase}")

citusctl_lib = read(CITUSCTL_LIB)
for phrase in (
    "render_dev_lifecycle_cli_report_from_args",
    "validate_plan_id(plan_id)",
    "append_dev_audit_record",
    "DevLifecycleCliReport",
    "state-file-only-no-recursive-delete",
):
    if phrase not in citusctl_lib:
        fail(f"tools/citusctl runtime lost production D1/M8 contract code: {phrase}")

citusctl_smoke = read(CITUSCTL_SMOKE)
for phrase in (
    "apply \"not ok\" inspect cluster",
    "plan_id must be stable ascii and non-empty",
):
    if phrase not in citusctl_smoke:
        fail(f"citusctl-smoke.sh lost invalid plan-id guard: {phrase}")

citusctl_dev_smoke = read(CITUSCTL_DEV_LIFECYCLE_SMOKE)
for phrase in (
    "--state-dir",
    "--format json",
    "--format tsv",
    "state_dir must not be empty",
    "plan_id must be stable ascii and non-empty",
    "audit_record_written",
    "dev-lifecycle.audit.tsv",
    "state-file-only-no-recursive-delete",
    "local-state-file-only",
):
    if phrase not in citusctl_dev_smoke:
        fail(f"citusctl-dev-lifecycle-smoke.sh lost required D1/M8 assertion: {phrase}")

makefile_text = read(MAKEFILE)
for phrase in (
    "citusctl-dev-lifecycle-smoke:",
    "ci/ai-blaise/citusctl-dev-lifecycle-smoke.sh",
    "gate-close:",
    "citusctl-dev-lifecycle-smoke",
):
    if phrase not in makefile_text:
        fail(f"Makefile.ai-blaise must wire citusctl dev lifecycle smoke: {phrase}")

tools_workflow = read(TOOLS_WORKFLOW)
if "citusctl-dev-lifecycle-smoke.sh" not in tools_workflow:
    fail("ci-tools workflow must run citusctl-dev-lifecycle-smoke.sh")

for phrase in (
    "not production-ready as a whole",
    "modeled release gates",
    "canonical model data rather than results from live performance",
    "v2 acceptance model must not be cited as production evidence",
    "WF2 fixture-backed WAL replay debugger plan",
    "not evidence for real WAL segment inspection",
    "pg_cron cohabitation smoke is production evidence for the TS19 clock-reservation",
    "does not make broad Bundle1 cohabitation production-ready",
    "TS20 SQL-visible C API proof remains limited to role/configuration classification",
):
    if compact(phrase) not in audit_compact:
        fail(
            f"PRODUCTION_READINESS_AUDIT.md must preserve guardrail phrase: {phrase}"
        )

branch_lifecycle_truth = compact(docs + "\n" + audit + "\n" + read(ROOT / "ci/ai-blaise/operator-branch-lifecycle-smoke.sh"))
for phrase in (
    "ci/ai-blaise/operator-branch-lifecycle-smoke.sh",
    "conservative admission guards",
    "live Kubernetes CSI `VolumeSnapshot` creation",
    "traffic cut-over",
    "remains alpha contract evidence",
):
    if compact(phrase) not in branch_lifecycle_truth:
        fail(f"Branch lifecycle docs must preserve alpha evidence boundary: {phrase}")

multiregion_truth = compact(docs + "\n" + audit + "\n" + read(ROOT / "ci/ai-blaise/operator-multiregion-contracts-smoke.sh"))
for phrase in (
    "ci/ai-blaise/operator-multiregion-contracts-smoke.sh",
    "RegionalRowPlacementPlan",
    "live_k8s_exercised=false",
    "GeoIP pool routing",
    "regional failover",
    "remain alpha",
):
    if compact(phrase) not in multiregion_truth:
        fail(f"Multi-region docs must preserve alpha evidence boundary: {phrase}")

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
    SIDECAR_CONTROLLER_LIVE_SMOKE,
    SIDECAR_SHARED_README,
    SIDECAR_CDC_SMOKE,
    SIDECAR_CDC_README,
    SIDECAR_CDC_MODIFICATION,
    OPERATOR_RECONCILERS_BATCH_C_SMOKE,
    COMPANION_RUNTIME_DEPTH_A_SMOKE,
    STRUCTURED_LOG_INGESTION_SMOKE,
    OBSERVABILITY_WORKFLOW,
    SIDECAR_REALTIME_SMOKE,
    SIDECAR_REALTIME_README,
    STORAGE_RUNTIME_SMOKE,
    POOL_PROXY_SMOKE,
    POOL_ROUTING_SECURITY_SMOKE,
    PLACEMENT_GENERATION_UDF_SMOKE,
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


# Bundle1 remains alpha until the complete operand initdb path is proven. Keep
# the source-build subset tied to structured manifest/lock/smoke evidence so it
# cannot drift into a prose-only production claim.
for path in (BUNDLE1_LOCK, BUNDLE1_CONTRACT_CHECK):
    if not path.exists() or not read(path).strip():
        fail(f"missing Bundle1 source-build contract artifact: {path}")

bundle1_truth = "\n".join(
    read(path)
    for path in (
        BUNDLED_EXTENSIONS_DOC,
        PG_OVERLAY_README,
        DOCS,
        AUDIT,
        SQL_EXTENSION_SMOKE,
        IMAGE_CHECK,
        BUNDLE1_LOCK,
        BUNDLE1_CONTRACT_CHECK,
    )
)
for phrase in (
    "bundle1-source-build.lock.tsv",
    "structured Bundle1 contract check",
    "BUNDLE1_BUILD_IMAGE=1",
    "BUNDLE1_BUILD_HEAVY=1",
    "source-build-subset-no-complete-initdb",
    "ai-blaise.citus.source-git-sha",
    "ai-blaise.citus.source-tree-state",
    "plrust PG17 upstream gap",
    "complete initdb path",
):
    if compact(phrase) not in compact(bundle1_truth):
        fail(f"Bundle1 source-build contract boundary missing phrase: {phrase}")

bundle1_docs_truth = "\n".join(
    read(path)
    for path in (BUNDLED_EXTENSIONS_DOC, PG_OVERLAY_README, DOCS, AUDIT)
)
for pattern in (
    "FEATURE: Bundle1 is production-ready",
    "Bundle1 is production-ready",
    "full Bundle1 production evidence exists",
    "plrust PG17 source-build is supported",
):
    if compact(pattern) in compact(bundle1_docs_truth):
        fail(f"Bundle1 docs overclaim production readiness: {pattern}")

# A10/A11 SQL-visible contract guardrail: these features remain alpha and prove
# deterministic intent validation only. This audit prevents accidental promotion
# to live provider/model execution or generated-query execution without evidence.
entry_status = {entry["id"]: entry["status"] for entry in entries}
for feature_id in ("A10", "A11"):
    if entry_status.get(feature_id) != "alpha":
        fail(f"{feature_id} must remain Status: alpha until live AI SQL execution is verified")

section_a10 = feature_section(docs, "A10")
section_a11 = feature_section(docs, "A11")
for phrase in (
    "sql-intent-fail-closed-only",
    "does not call a live model provider",
    "does not produce real streaming provider chunks",
    "not production-ready",
):
    if compact(phrase) not in compact(section_a10):
        fail(f"A10 docs must preserve SQL intent caveat: {phrase}")
for phrase in (
    "sql-intent-fail-closed-only",
    "does not call a live text-to-SQL model",
    "does not execute generated SQL",
    "not production-ready",
):
    if compact(phrase) not in compact(section_a11):
        fail(f"A11 docs must preserve SQL intent caveat: {phrase}")
for phrase in (
    "A10 and A11 remain alpha",
    "sql-intent-fail-closed-only",
    "no live provider call",
    "no generated-query execution",
):
    if compact(phrase) not in audit_compact:
        fail(f"PRODUCTION_READINESS_AUDIT.md must preserve AI SQL contract caveat: {phrase}")

sql_extension = read(SQL_EXTENSION)
ai_sql_smoke = read(AI_SQL_CONTRACT_SMOKE)
ci_image_workflow = read(CI_IMAGE_WORKFLOW)
makefile = read(MAKEFILE)
for phrase in (
    "CREATE TABLE IF NOT EXISTS companion_internal.ai_provider_bindings",
    "CREATE TABLE IF NOT EXISTS companion_internal.semantic_catalog_objects",
    "CREATE VIEW companion_ai_provider_bindings",
    "CREATE VIEW companion_semantic_catalog_objects",
    "CREATE FUNCTION companion_internal.register_ai_provider_binding",
    "CREATE FUNCTION companion_ai_chat_stream",
    "CREATE FUNCTION companion_internal.register_semantic_catalog_object",
    "CREATE FUNCTION companion_semantic_text_to_sql_intent",
    "provider_runtime_available",
    "AI provider runtime is unavailable; this SQL surface emits request intent only",
    "text-to-SQL execution is unavailable; this SQL surface emits request intent only",
):
    if phrase not in sql_extension:
        fail(f"AI SQL extension contract missing phrase: {phrase}")
for phrase in (
    "sql-intent-fail-closed-only",
    "provider_runtime_available",
    "secret_bound",
    "AI provider runtime is unavailable; this SQL surface emits request intent only",
    "text-to-SQL execution is unavailable; this SQL surface emits request intent only",
    "does not call a live LLM provider or execute generated SQL",
):
    if compact(phrase) not in compact(ai_sql_smoke):
        fail(f"AI SQL contract smoke missing fail-closed assertion: {phrase}")
for phrase in (
    "ai-sql-contract-smoke:",
    "REQUIRE_DOCKER=1 ci/ai-blaise/ai-sql-contract-smoke.sh",
    "gate-close:",
    "ai-sql-contract-smoke",
):
    if phrase not in makefile:
        fail(f"Makefile.ai-blaise must wire AI SQL contract smoke: {phrase}")
if "ai-sql-contract-smoke.sh" not in ci_image_workflow:
    fail("ci-image workflow must run ai-sql-contract-smoke.sh")

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

realtime_docs = "\n".join([docs, audit, read(SIDECAR_REALTIME_README)])
for phrase in (
    "runtime_boundary=single-node-raw-ws-cdc-ingest",
    "websocket_network_exercised=true",
    "browser_client_exercised=false",
    "cdc_tailing_integrated=false",
    "multi_node_pubsub=false",
    "kubernetes_traffic_exercised=false",
    "browser client behavior",
    "WebSocket extension negotiation",
    "live CDC tailing",
    "multi-node pubsub",
    "Kubernetes traffic",
):
    if phrase not in realtime_docs:
        fail(f"realtime production boundary caveat missing phrase: {phrase}")

realtime_smoke = read(SIDECAR_REALTIME_SMOKE)
for phrase in (
    "single-node-raw-ws-cdc-ingest",
    "invalid join did not fail closed",
    "Sec-WebSocket-Extensions: permessage-deflate",
):
    if phrase not in realtime_smoke:
        fail(f"realtime smoke missing fail-closed runtime proof phrase: {phrase}")

sidecar_cdc_smoke = read(SIDECAR_CDC_SMOKE)
cdc_truth = compact(
    docs
    + "\n"
    + audit
    + "\n"
    + sidecar_cdc_smoke
    + "\n"
    + read(SIDECAR_CDC_README)
    + "\n"
    + read(SIDECAR_CDC_MODIFICATION)
)
if status_by_id.get("C2") != "production-ready":
    fail("C2 must be production-ready after live PostgreSQL DDL capture parsing evidence")
for phrase in (
    "postgres:17-bookworm",
    "CREATE EVENT TRIGGER ai_blaise_capture_ddl",
    "CREATE TABLE public.cdc_schema_smoke",
    "ddl_events_total",
    "ddl_stream_table",
    "command_tag",
    "object_schema",
    "object_identity",
    "ddl_event",
    "same /ingest",
    "long-running logical replication slot tailing",
):
    if compact(phrase) not in cdc_truth:
        fail(f"C2 DDL capture production boundary must preserve phrase: {phrase}")
cdc_executable_truth = (
    sidecar_cdc_smoke
    + "\n"
    + read(ROOT / "sidecar/cdc/src/lib.rs")
    + "\n"
    + read(ROOT / "sidecar/cdc/src/live.rs")
)
for phrase in (
    "DdlStreamEvent",
    "parse_ddl_stream_event",
    "POSTGRES_HOST_AUTH_METHOD=trust",
    "command_tag = 'CREATE TABLE'",
    "OK cdc-sidecar live Postgres DDL capture parsed through /ingest",
):
    if phrase not in cdc_executable_truth:
        fail(f"C2 DDL capture executable proof must preserve phrase: {phrase}")

conflict_truth = compact(
    docs
    + "\n"
    + audit
    + "\n"
    + read(OPERATOR_RECONCILERS_BATCH_C_SMOKE)
    + "\n"
    + read(COMPANION_RUNTIME_DEPTH_A_SMOKE)
    + "\n"
    + read(ROOT / "operator/src/main.rs")
    + "\n"
    + read(ROOT / "operator/src/reconcile/conflict_policy.rs")
    + "\n"
    + read(ROOT / "companion/src/replication_conflict.rs")
)
if status_by_id.get("C4") != "production-ready":
    fail("C4 must be production-ready after live conflict-policy metadata apply evidence")
if status_by_id.get("C5") != "production-ready":
    fail("C5 must be production-ready after seven-class resolver and live metadata apply evidence")
for phrase in (
    "run-conflict-policy-runtime-canonical",
    "CONFLICT_POLICY_IMAGE",
    "conflict_policy_live_row",
    "accounts-lww",
    "accounts-merge",
    "update_origin_differs",
    "apply_remote_if_newer",
    "update_exists",
    "merge_function",
    "public.merge_remote_into_local",
    "replication_conflict_status",
    "conflict_classes",
    "7",
    "companion.replication_conflict_audit",
    "does not claim live pgactive",
    "does not claim live Spock",
):
    if compact(phrase) not in conflict_truth:
        fail(f"C4/C5 conflict-policy production boundary must preserve phrase: {phrase}")

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

sidecar_controller_live_smoke = read(SIDECAR_CONTROLLER_LIVE_SMOKE)
if not (SIDECAR_CONTROLLER_LIVE_SMOKE.stat().st_mode & 0o111):
    fail("sidecar-controller-live-smoke.sh must be executable")
for required in (
    "FEATURE: O5",
    "kind create cluster",
    "images/rust-runtime/Dockerfile",
    "build_and_push",
    "sha256:[0-9a-f]",
    "AI_BLAISE_OPERATOR_EXECUTION_MODE",
    "AI_BLAISE_OPERATOR_CONTROLLERS",
    "print-sidecar-crd",
    "sidecars/status",
    "--subresource=status",
    "ownerReferences",
    "port-forward --address 127.0.0.1",
    "ai_blaise_sidecar_ready",
    "mutable_image_fail_closed",
    "requires an immutable sha256 digest image",
):
    if required not in sidecar_controller_live_smoke:
        fail(f"O5 live sidecar controller smoke lost required assertion: {required}")
if status_by_id.get("O5") != "production-ready":
    fail("O5 must be production-ready after live Sidecar controller apply evidence")
o5_body = compact(entry_by_id["O5"]["body"])
for phrase in (
    "sidecar-controller-live-smoke.sh",
    "digest-pinned images",
    "AI_BLAISE_OPERATOR_EXECUTION_MODE=apply",
    "AI_BLAISE_OPERATOR_CONTROLLERS=sidecar",
    "generated Deployment, Service, owner references, status fields",
    "sidecars/status",
    "rejects it before creating a Deployment",
    "does not claim OpenTelemetry trace propagation",
    "full production semantics for every sidecar application",
):
    if compact(phrase) not in o5_body:
        fail(f"O5 docs lost live apply proof/boundary phrase: {phrase}")
shared_sidecar_readme = read(SIDECAR_SHARED_README)
shared_sidecar_readme_compact = compact(shared_sidecar_readme)
for phrase in (
    "O5` is production-ready only for the operator `Sidecar` CR",
    "sidecar-controller-live-smoke.sh",
    "Trace emission, collector wiring",
    "broader sidecar application behavior remain outside",
):
    if compact(phrase) not in shared_sidecar_readme_compact:
        fail(f"sidecar/shared README lost O5 boundary phrase: {phrase}")
if (
    "sidecar-controller-live-smoke:" not in makefile
    or "gate-close:" not in makefile
    or "sidecar-controller-live-smoke" not in makefile.split("gate-close:", 1)[1]
):
    fail("gate-close must run sidecar-controller-live-smoke")
for phrase in (
    "O5 register entry",
    "sidecar-controller-live-smoke.sh",
    "real operator and",
    "sidecars/status",
    "mutable image tag before Deployment creation",
    "all sidecar app semantics beyond the realtime probe container remain outside",
):
    if compact(phrase) not in audit_compact:
        fail(f"production audit lost O5 live apply boundary phrase: {phrase}")

graphql_postgrest_smoke = read(GRAPHQL_POSTGREST_RUNTIME_SMOKE)
for required in (
    "json.loads(body)",
    'openapi["openapi"] == "3.0.0"',
    'openapi["x-ai-blaise"]["schemas"] == ["public", "api"]',
    'openapi["x-ai-blaise"]["rls_required"] is True',
    'openapi["x-ai-blaise"]["tenant_claim"] == "tenant_id"',
    'orders = openapi["paths"]["/orders"]',
    'assert POSTGRES_URL not in body',
    'assert JWT_SECRET not in body',
):
    if required not in graphql_postgrest_smoke:
        fail(f"GraphQL/PostgREST smoke lost API6 OpenAPI assertion: {required}")

postgrest_live_smoke = read(POSTGREST_LIVE_DATA_PLANE_SMOKE)
for required in (
    "DEFAULT_DATABASE_IMAGE",
    "ai-blaise-citus-timescale-cohabitation:local",
    "run-live-postgrest",
    "AI_BLAISE_POSTGREST_UPSTREAM",
    "create_distributed_table('public.orders', 'tenant_id')",
    "pg_dist_partition",
    "api.orders",
    "Accept-Profile api to api.orders",
    "role=web_user",
    "tenant_id",
    "cross-tenant INSERT",
    "dependency report and postgrest.conf retained env refs without URI/JWT leakage",
):
    if required not in postgrest_live_smoke:
        fail(f"live PostgREST data-plane smoke lost required assertion: {required}")

for feature_id in ("API1", "API2", "API5", "API6"):
    if status_by_id.get(feature_id) != "production-ready":
        fail(f"{feature_id} must be production-ready after live PostgREST REST data-plane evidence")
if status_by_id.get("API3") != "alpha":
    fail("API3 must remain alpha until live pg_graphql execution is proven")
api_rest_body = compact(
    entry_by_id["API1"]["body"]
    + entry_by_id["API2"]["body"]
    + entry_by_id["API5"]["body"]
)
for phrase in (
    "production evidence",
    "postgrest-live-data-plane-smoke.sh",
    "run-live-postgrest",
    "AI_BLAISE_POSTGREST_UPSTREAM",
    "create_distributed_table('public.orders', 'tenant_id')",
    "pg_dist_partition",
    "security-invoker `api.orders` view",
    "tenant A cross-tenant INSERT for tenant B is rejected",
    "live `pg_graphql` execution remains bounded by `FEATURE: API3` alpha status",
):
    if compact(phrase) not in api_rest_body:
        fail(f"API1/API2/API5 docs lost live PostgREST data-plane phrase: {phrase}")
api6_body = compact(entry_by_id["API6"]["body"])
for phrase in (
    "production evidence",
    "graphql-postgrest-runtime-smoke.sh",
    "/openapi.json",
    "openapi 3.0 metadata",
    "absence of database uri or jwt secret material",
    "API1/API2/API5 have separate production evidence",
    "API3 remains alpha",
):
    if compact(phrase) not in api6_body:
        fail(f"API6 docs lost bounded production evidence phrase: {phrase}")

structured_log_smoke = read(STRUCTURED_LOG_INGESTION_SMOKE)
for required in (
    "POSTGRES_IMAGE=${POSTGRES_IMAGE:-postgres:17}",
    "run-log-view-sql-canonical",
    "log-schema-records-canonical",
    "companion.sidecar_log_raw",
    "docker run -d",
    "typed_view_rows",
    "vectorizer_types",
):
    if required not in structured_log_smoke:
        fail(f"structured-log ingestion smoke lost O15 runtime proof: {required}")

if status_by_id.get("O14") != "alpha":
    fail("O14 must remain alpha until full trace propagation and dashboard correlation are measured")
otel_trace_smoke = read(ROOT / "ci/ai-blaise/otel-trace-propagation-smoke.sh")
for phrase in (
    "resourceSpans",
    "http://jaeger:4318/v1/traces",
    "http://jaeger:16686/api/traces/${trace_id}",
    "pool.trace_tap",
    "synthetic-jaeger-correlation-harness",
    "automatic pool/companion/sidecar span",
):
    if phrase not in otel_trace_smoke:
        fail(f"O14 KIND Jaeger correlation smoke lost required phrase: {phrase}")
if "Jaeger correlation harness, not automatic pool/companion/sidecar span export" not in entry_by_id["O14"]["body"]:
    fail("O14 docs must keep the Jaeger correlation proof boundary explicit")
if status_by_id.get("O15") != "production-ready":
    fail("O15 structured-log schema must be production-ready after PostgreSQL ingestion smoke evidence")
o15_body = compact(entry_by_id["O15"]["body"])
for phrase in (
    "production evidence",
    "structured-log-ingestion-smoke.sh",
    "postgres:17",
    "companion.sidecar_log_raw",
    "applies all 17 generated typed views",
    "ingests all 17 sidecar records as jsonb",
    "does not claim vector",
    "broader o14 trace propagation path",
):
    if compact(phrase) not in o15_body:
        fail(f"O15 docs lost PostgreSQL typed-view evidence phrase: {phrase}")

observability_workflow = read(OBSERVABILITY_WORKFLOW)
for required in (
    "bootstrap-v2",
    "postgresql-client",
    "observability-contracts-check.sh",
    "structured-log-ingestion-smoke.sh",
):
    if required not in observability_workflow:
        fail(f"observability workflow lost O15 CI runtime proof: {required}")

if (
    "structured-log-ingestion-smoke:" not in makefile
    or "gate-close:" not in makefile
    or "structured-log-ingestion-smoke" not in makefile.split("gate-close:", 1)[1]
):
    fail("gate-close must run structured-log-ingestion-smoke")

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

for required in (
    "Send both query frames before reading either result",
    "pack_simple_query",
    "pipeline_one",
    "pipeline_two",
    "raw PostgreSQL pipelined simple-query",
    "passed through pool proxy",
):
    if required not in pool_smoke:
        fail(f"pool proxy smoke lost T7 raw-wire pipelining assertion: {required}")

if "### T7: Pipelined Client Protocol In Pool" in docs:
    t7_section = docs.split("### T7: Pipelined Client Protocol In Pool", 1)[1].split("### T10:", 1)[0]
    for required in (
        "**Status**: production-ready",
        "Production evidence:",
        "raw PostgreSQL client",
        "Extended-query `Parse`/`Bind`/`Execute` buffering",
        "remain alpha",
    ):
        if required not in t7_section:
            fail(f"T7 production boundary lost required docs phrase: {required}")

pool_routing_smoke = read(POOL_ROUTING_SECURITY_SMOKE)
for required in (
    "mirror_decision_bucket",
    "htap_fail_closed_rejections",
    "geo_invalid_cidr_rejections",
    "tls_key_fingerprint_len",
    "pool-routing-security-smoke ok",
):
    if required not in pool_routing_smoke:
        fail(f"pool routing/security smoke lost required assertion: {required}")

for phrase in (
    "live canary mirroring",
    "managed GeoIP databases",
    "rustls listener/session-resumption traffic",
    "analytical sidecar query execution",
):
    if phrase.lower() not in audit_compact:
        fail(f"production audit lost pool routing/security caveat: {phrase}")

security_supply_chain_smoke = read(SECURITY_SUPPLY_CHAIN_SMOKE)
for required in (
    "run-security-supply-chain-canonical",
    "external-secrets.io/v1beta1",
    "MutableImageReference",
    "InvalidSbomPath",
    "slsa.dev/provenance/v1",
    "security-supply-chain-smoke ok",
):
    if required not in security_supply_chain_smoke:
        fail(f"security supply-chain smoke lost required assertion: {required}")

security_external_tls_smoke = read(SECURITY_EXTERNAL_SECRETS_TLS_LIVE_SMOKE)
if not (SECURITY_EXTERNAL_SECRETS_TLS_LIVE_SMOKE.stat().st_mode & 0o111):
    fail("security-external-secrets-tls-live-smoke.sh must be executable")
for required in (
    "FEATURE: Sec7 Sec8",
    "external-secrets/external-secrets",
    "SEC78_ESO_CHART_VERSION",
    "0.10.7",
    "SecretStore",
    "provider:",
    "fake:",
    "ExternalSecret",
    "kubectl -n \"$ns\" wait externalsecret",
    "auth can-i get secrets",
    "ssl.TLSVersion.TLSv1_3",
    "ssl.CERT_REQUIRED",
    "mode == \"tls12\"",
    "mode != \"no-cert\"",
    "runtime_secret_api_denied",
    "tls13_mtls_success",
    "client_cert_required",
    "tls12_rejected",
):
    if required not in security_external_tls_smoke:
        fail(f"Sec7/Sec8 live smoke lost required assertion: {required}")
if (
    "security-external-secrets-tls-live-smoke:" not in makefile
    or "gate-close:" not in makefile
    or "security-external-secrets-tls-live-smoke" not in makefile.split("gate-close:", 1)[1]
):
    fail("gate-close must run security-external-secrets-tls-live-smoke")

sec9_live_smoke = read(SECURITY_SBOM_COSIGN_LIVE_SMOKE)
for required in (
    "SEC9_SYFT_IMAGE",
    "ghcr.io/anchore/syft:v1.18.1",
    "SEC9_COSIGN_IMAGE",
    "gcr.io/projectsigstore/cosign:v2.4.1",
    "registry:2",
    "spdx-json=/work/sec9.spdx.json",
    "slsa.dev/provenance/v1=sec9-live-smoke",
    "verify-attestation",
    "slsaprovenance1",
    "sec9.spdx.sigstore.json",
    "verify-blob",
):
    if required not in sec9_live_smoke:
        fail(f"Sec9 live SBOM/cosign smoke lost required assertion: {required}")

operator_workflow = read(OPERATOR_WORKFLOW)
if "security-supply-chain-smoke.sh" not in operator_workflow:
    fail("operator workflow must run security-supply-chain-smoke.sh")

if status_by_id.get("Sec9") != "production-ready":
    fail("Sec9 must remain production-ready after registry-backed SBOM/cosign proof")
for feature_id in ("Sec7", "Sec8"):
    if status_by_id.get(feature_id) != "production-ready":
        fail(f"{feature_id} must be production-ready after live External Secrets and TLS proof")
sec7_body = compact(entry_by_id["Sec7"]["body"])
for phrase in (
    "External Secrets Operator chart `0.10.7`",
    "fake-provider `ExternalSecret` objects into real Kubernetes Secrets",
    "runtime ServiceAccount is denied Secret API reads",
    "does not claim cloud provider authentication",
    "production rotation SLOs",
    "security-external-secrets-tls-live-smoke.sh",
):
    if compact(phrase) not in sec7_body:
        fail(f"Sec7 docs lost live proof/boundary phrase: {phrase}")
sec8_body = compact(entry_by_id["Sec8"]["body"])
for phrase in (
    "TLS 1.3 mTLS success",
    "no-client-cert and TLS 1.2 clients fail",
    "does not claim cloud certificate issuance",
    "automatic rotation",
    "every application protocol path",
    "security-external-secrets-tls-live-smoke.sh",
):
    if compact(phrase) not in sec8_body:
        fail(f"Sec8 docs lost live proof/boundary phrase: {phrase}")
sec9_body = compact(entry_by_id["Sec9"]["body"])
for phrase in (
    "Production evidence",
    "security-sbom-cosign-live-smoke.sh",
    "local OCI registry",
    "SPDX 2.3 SBOM with Syft",
    "Cosign",
    "SLSA provenance attestations",
    ".sigstore.json` bundle",
    "Kubernetes admission-policy enforcement",
    "public release registry publication",
):
    if compact(phrase) not in sec9_body:
        fail(f"Sec9 production boundary lost docs phrase: {phrase}")

for phrase in (
    "security-external-secrets-tls-live-smoke.sh",
    "External Secrets Operator chart `0.10.7`",
    "runtime Secret API reads are denied",
    "TLS 1.3 mTLS success",
    "no-client-cert and TLS 1.2 clients fail",
    "Cloud provider authentication",
    "production rotation SLOs",
    "ExternalSecret manifest",
    "TLS Secret-reference",
    "SBOM/cosign metadata",
    "registry-backed generation/sign/verify flow",
    "Kubernetes admission enforcement",
    "public registry publication",
):
    if compact(phrase) not in audit_compact:
        fail(f"production audit lost security supply-chain boundary phrase: {phrase}")

phony_lines = "\n".join(line for line in makefile.splitlines() if line.startswith(".PHONY:"))
gate_deps = "\n".join(line for line in makefile.splitlines() if line.startswith("gate-close:"))
for target in (
    "citus-patch-production-audit",
    "sidecar-api-runtime-smoke",
    "storage-sidecar-runtime-smoke",
    "pool-routing-security-smoke",
    "security-enforcement-smoke",
    "security-supply-chain-smoke",
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

if not (K8S_PRODUCTION_VALUES_LIVE_SMOKE.stat().st_mode & 0o111):
    fail("ci/ai-blaise/k8s-production-values-live-smoke.sh must be executable")

k8s_prod_values_live = read(K8S_PRODUCTION_VALUES_LIVE_SMOKE)
for phrase in (
    "@sha256:",
    "alphaSidecarsEnabled: false",
    "mutableImagesAllowed: false",
    "helm template",
    "kubectl apply --dry-run=client",
    "kind create cluster",
    'kubectl -n "${namespace}" rollout status',
    "traffic=sql-service status=ok",
    "claim_boundary=postgres_substrate_only",
    "no_operator_pool_or_citus_data_plane_claim=true",
):
    if phrase not in k8s_prod_values_live:
        fail(f"k8s production-values live smoke lost contract phrase: {phrase}")

if "k8s-production-values-live-smoke:" not in makefile:
    fail("Makefile.ai-blaise must expose k8s-production-values-live-smoke")

if "PRODUCTION_VALUES_STRICT" not in live_k8s:
    fail("live Kubernetes e2e harness must expose PRODUCTION_VALUES_STRICT")

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

pool_smoke = read(POOL_PROXY_SMOKE)
for phrase in (
    "AI_BLAISE_POOL_SETTINGS_BUCKET_GUCS",
    "citus.enable_repartition_joins",
    "current_setting('citus.enable_repartition_joins', true)",
    "pg_backend_pid()",
    "ai_blaise_citus_pool_settings_bucket_unique_fingerprints",
    "ai_blaise_citus_pool_settings_bucket_backend_borrows_total",
    "ai_blaise_citus_pool_settings_bucket_assigned_connections",
    "ai_blaise_citus_pool_settings_bucket_release_errors_total",
):
    if phrase not in pool_smoke:
        fail(f"pool-proxy-smoke.sh must preserve T1 live settings-bucket evidence: {phrase}")

for phrase in (
    "T1 settings-bucket production evidence is limited",
    "live proxy startup",
    "borrow/release metrics",
    "must not be cited as proof of reusable transaction pooling",
):
    if compact(phrase) not in audit_compact:
        fail(f"PRODUCTION_READINESS_AUDIT.md must preserve T1 boundary: {phrase}")

placement_udf_smoke = read(PLACEMENT_GENERATION_UDF_SMOKE)
for required in (
    "PG_FUNCTION_INFO_V1(citus_placement_generation)",
    "CREATE OR REPLACE FUNCTION pg_catalog.citus_placement_generation()",
    "citus--14.0-1--15.0-1.sql",
    "udfs/citus_placement_generation/15.0-1.sql",
    "patches/0005-placement-generation-counter.patch",
):
    if required not in placement_udf_smoke:
        fail(f"placement-generation UDF smoke lost required proof: {required}")
for required_path, required_phrase in (
    (ROOT / "src/backend/distributed/sql/udfs/citus_placement_generation/latest.sql", "GRANT EXECUTE ON FUNCTION pg_catalog.citus_placement_generation() TO PUBLIC"),
    (ROOT / "src/backend/distributed/sql/udfs/citus_placement_generation/15.0-1.sql", "AS 'MODULE_PATHNAME', $$citus_placement_generation$$"),
    (ROOT / "src/backend/distributed/sql/udfs/citus_placement_generation/14.0-1.sql", "RETURNS bigint"),
    (ROOT / "src/backend/distributed/sql/citus--14.0-1--15.0-1.sql", "udfs/citus_placement_generation/15.0-1.sql"),
):
    if required_phrase not in read(required_path):
        fail(f"placement-generation SQL contract missing {required_phrase} in {required_path}")
if "placement-generation-udf-contract-smoke:" not in makefile or "placement-generation-udf-contract-smoke" not in makefile.split("gate-close:", 1)[1]:
    fail("gate-close must run placement-generation-udf-contract-smoke")
if status_by_id.get("T2") != "production-ready":
    fail("T2 must be production-ready after live patched-Citus placement-generation and GUC_REPORT evidence")
t2_body = compact(entry_by_id["T2"]["body"])
for phrase in (
    "placement-generation-udf-contract-smoke.sh",
    "pg-cron-cohabitation-smoke.sh",
    "pg_catalog.citus_placement_generation()",
    "fresh-install sql",
    "15.0 upgrade sql",
    "placement_generation_after_first_distribution",
    "placement_generation_after_second_distribution",
    "placement_generation_placements",
    "citus_shard_count_parameter_status",
    "ParameterStatus",
    "SET citus.shard_count TO 7",
    "does not claim production latency",
):
    if compact(phrase) not in t2_body:
        fail(f"T2 docs lost placement-generation runtime proof/boundary phrase: {phrase}")

pg_cron_cohabitation_smoke = read(PG_CRON_COHABITATION_SMOKE)
for phrase in (
    "FEATURE: Bundle1 T2 TS19 TS20",
    "placement_generation_initial",
    "placement_generation_after_first_distribution",
    "placement_generation_after_second_distribution",
    "placement_generation_placements",
    "citus_shard_count_parameter_status",
    "SET citus.shard_count TO 7",
    "ParameterStatus",
    "POSTGRES_HOST_AUTH_METHOD=trust",
):
    if phrase not in pg_cron_cohabitation_smoke:
        fail(f"pg_cron cohabitation smoke must preserve T2 runtime proof: {phrase}")

audit_compact = compact(read(AUDIT))
for phrase in (
    "pg_catalog.citus_placement_generation()",
    "fresh-install and 15.0 upgrade SQL",
    "placement_generation_after_second_distribution",
    "citus_shard_count_parameter_status",
    "GUC_REPORT",
    "production-ready for the bounded Citus patch surface",
    "does not claim production latency",
):
    if compact(phrase) not in audit_compact:
        fail(f"PRODUCTION_READINESS_AUDIT.md must preserve T2 runtime boundary: {phrase}")

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

timescale_bridge_smoke = read(TIMESCALE_BRIDGE_SMOKE)
timescale_cohabitation_smoke = read(TIMESCALE_COHABITATION_SMOKE)
ts_version_matrix_smoke = read(TS_VERSION_MATRIX_SMOKE)
timescale_runtime_truth = compact(docs + "\n" + audit + "\n" + timescale_bridge_smoke + "\n" + timescale_cohabitation_smoke + "\n" + read(TIMESCALE_COHABITATION_DOCKERFILE))
for phrase in (
    "missing_citus_fail_closed",
    "policy_execution_scope",
    "entrypoints-and-catalog-state-only",
    "stubbed_citus_distribution",
    "real_citus_distribution",
    "timescaledb_extversion",
    "does not claim full TimescaleDB functionality",
    "timescale/timescaledb-ha:pg17-ts2.27",
    "with_llvm=\"${WITH_LLVM}\"",
    "postgresql-server-dev-17",
):
    if compact(phrase) not in timescale_runtime_truth:
        fail(f"Timescale runtime evidence boundary must preserve phrase: {phrase}")
for feature_id, function_name in (
    ("TS1", "apply_distribute_hypertable"),
    ("TS2", "apply_compression_policy_distributed"),
    ("TS3", "apply_continuous_aggregate_distributed"),
    ("TS4", "apply_retention_policy_distributed"),
    ("TS5", "apply_time_range_shard_pruner"),
    ("TS12", "apply_reorder_policy_distributed"),
):
    if status_by_id.get(feature_id) != "production-ready":
        fail(f"{feature_id} must be production-ready for bounded live Timescale bridge apply/catalog-state evidence")
    body = compact(entry_by_id[feature_id]["body"])
    for phrase in (
        "Production evidence",
        function_name,
        "timescale/timescaledb-ha:pg17-ts2.27",
        "policy_execution_scope=entrypoints-and-catalog-state-only",
        "does not claim full TimescaleDB functionality",
        "operator reconciliation",
    ):
        if compact(phrase) not in body:
            fail(f"{feature_id} docs lost bounded Timescale production evidence phrase: {phrase}")
if status_by_id.get("TS7") != "production-ready":
    fail("TS7 must be production-ready after live Kubernetes Hypertable controller SQL execution and status reconciliation evidence")
ts7_truth = compact(docs + "\n" + audit + "\n" + read(OPERATOR_HYPERTABLE_LIVE_SMOKE))
for phrase in (
    "operator-hypertable-live-smoke.sh",
    "AI_BLAISE_OPERATOR_EXECUTION_MODE=apply",
    "status.phase=Applied",
    "observedGeneration",
    "skippedStepCount >= 5",
    "timeColumn=metric_time",
    "distributionColumn=tenant_id",
    "no duplicate bridge-state rows",
    "does not claim multi-worker fanout",
):
    if compact(phrase) not in ts7_truth:
        fail(f"TS7 live controller evidence boundary must preserve phrase: {phrase}")

if "pg-cron-cohabitation-smoke.sh" not in read(CI_IMAGE_WORKFLOW):
    fail("ci-image workflow must run pg-cron-cohabitation-smoke for TS19 production evidence")

for phrase in (
    "TIMESCALE_COHABITATION_EXPECTED_TS_MINOR",
    "TimescaleDB minor mismatch",
    "docker_manifest_available",
    "required_version",
):
    if phrase not in (timescale_cohabitation_smoke + "\n" + ts_version_matrix_smoke):
        fail(f"Timescale version matrix must preserve fail-closed/version evidence phrase: {phrase}")

for pattern in (
    "full TimescaleDB functionality is production-ready",
    "continuous aggregate execution is production-ready",
    "compression policies execution is production-ready",
    "distributed hypertables production-ready",
    "planner pushdown production-ready",
):
    if compact(pattern) in compact(docs + "\n" + audit):
        fail(f"Timescale docs overclaim production readiness: {pattern}")

pg_cron_truth = compact(docs + "\n" + audit + "\n" + pg_cron_cohabitation_smoke)
if status_by_id.get("TS19") != "production-ready":
    fail("TS19 pg_cron clock cohabitation must be production-ready after live clock-reservation worker evidence")
if status_by_id.get("TS20") != "production-ready":
    fail("TS20 cohabit role/configuration classifier must be production-ready after SQL-visible C API live proof")
for phrase in (
    "citus_cohabit_clock_tick_reserved",
    "clock_tick_reserved",
    "cron_clock_reserved_runs",
    "cron_node_clock_samples",
    "negative_clock_tick_reserved",
    "citus_cohabit_pg_cron_role",
    "citus_cohabit_pg_cron_configured",
    "citus_cohabit_timescaledb_role",
    "citus_cohabit_pg_partman_role",
    "citus_cohabit_unknown_role",
    "negative_pg_cron_citus_role",
    "negative_pg_cron_citus_configured",
    "scheduled pg_cron worker",
    "does not make `pg_cron` a trusted hook-chain coextension",
    "role/configuration classifier boundary only",
):
    if compact(phrase) not in pg_cron_truth:
        fail(f"pg_cron TS19 production boundary must preserve phrase: {phrase}")
for required_path in (
    ROOT / "src/backend/distributed/sql/udfs/citus_cohabit_clock_tick_reserved/14.0-1.sql",
    ROOT / "src/backend/distributed/sql/udfs/citus_cohabit_clock_tick_reserved/15.0-1.sql",
    ROOT / "src/backend/distributed/sql/udfs/citus_cohabit_clock_tick_reserved/latest.sql",
):
    sql = read(required_path)
    if "citus_cohabit_clock_tick_reserved" not in sql or "MODULE_PATHNAME" not in sql:
        fail(f"TS19 clock-reservation UDF SQL contract missing required function in {required_path}")

for required_path, required_symbol in (
    (ROOT / "src/backend/distributed/sql/udfs/citus_cohabit_extension_role/14.0-1.sql", "citus_cohabit_extension_role"),
    (ROOT / "src/backend/distributed/sql/udfs/citus_cohabit_extension_role/15.0-1.sql", "citus_cohabit_extension_role"),
    (ROOT / "src/backend/distributed/sql/udfs/citus_cohabit_extension_role/latest.sql", "citus_cohabit_extension_role"),
    (ROOT / "src/backend/distributed/sql/udfs/citus_cohabit_extension_configured/14.0-1.sql", "citus_cohabit_extension_configured"),
    (ROOT / "src/backend/distributed/sql/udfs/citus_cohabit_extension_configured/15.0-1.sql", "citus_cohabit_extension_configured"),
    (ROOT / "src/backend/distributed/sql/udfs/citus_cohabit_extension_configured/latest.sql", "citus_cohabit_extension_configured"),
):
    sql = read(required_path)
    if required_symbol not in sql or "MODULE_PATHNAME" not in sql:
        fail(f"TS20 cohabit classifier UDF SQL contract missing {required_symbol} in {required_path}")

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

repack_truth = compact(docs + "\n" + audit + "\n" + read(ROOT / "sidecar/repack/README.md"))
r7_entries = [entry for entry in entries if entry["id"] == "R7"]
if len(r7_entries) != 1 or r7_entries[0]["status"] != "production-ready":
    fail("R7 must be production-ready only with live pg_repack execution evidence")
for phrase in (
    "dry-run-plan-only",
    "run-live-pg-repack",
    "dry_run=false",
    "executed=true",
    "live-pg-repack-execution",
    "REQUIRE_DOCKER=1",
    "single local PostgreSQL target",
    "PostgreSQL 19 `REPACK CONCURRENTLY`",
    "Kubernetes-scheduled repack execution",
    "Citus shard fanout across workers",
):
    if compact(phrase) not in repack_truth:
        fail(f"R7 repack production boundary missing truth phrase: {phrase}")
for phrase in ("sidecar-repack-smoke", "ci/ai-blaise/sidecar-repack-smoke.sh"):
    if phrase not in makefile:
        fail(f"Makefile.ai-blaise must wire the repack smoke: {phrase}")
if "sidecar-repack-smoke.sh" not in read(SIDECAR_WORKFLOW):
    fail("ci-sidecar workflow must run sidecar-repack-smoke.sh")
if "run-live-pg-repack" not in read(ROOT / "sidecar/repack/src/main.rs"):
    fail("R7 sidecar must expose the live pg_repack execution command")
analytical_ids = {"L1", "L2", "L3", "L4", "L5", "L6", "L8", "L12", "L13"}
entry_status = {entry["id"]: entry["status"] for entry in entries}
not_alpha = sorted(feature_id for feature_id in analytical_ids if entry_status.get(feature_id) != "alpha")
if not_alpha:
    fail(
        "analytical/lakehouse features must remain alpha until live execution evidence exists: "
        + ", ".join(not_alpha)
    )
analytical_truth = compact(
    docs + "\\n" + audit + "\\n" + read(ROOT / "sidecar/analytical/README.md")
)
for phrase in (
    "external_io_attempted=false",
    "query_engine_executed=false",
    "deterministic-runtime-report-only",
    "must not be cited as production evidence for live DataFusion",
    "object-store IO",
    "Citus planner integration",
):
    if compact(phrase) not in analytical_truth:
        fail(f"analytical/lakehouse boundary missing truth phrase: {phrase}")
for phrase in ("sidecar-analytical-smoke", "ci/ai-blaise/sidecar-analytical-smoke.sh"):
    if phrase not in makefile:
        fail(f"Makefile.ai-blaise must wire the analytical smoke: {phrase}")
if "sidecar-analytical-smoke.sh" not in read(SIDECAR_WORKFLOW):
    fail("ci-sidecar workflow must run sidecar-analytical-smoke.sh")

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
