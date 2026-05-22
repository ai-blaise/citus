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
E2E_DOC = ROOT / "docs/ai-blaise/E2E.md"
ARCHITECTURE_DOC = ROOT / "docs/ai-blaise/ARCHITECTURE.md"
BUNDLED_EXTENSIONS_DOC = ROOT / "docs/ai-blaise/BUNDLED_EXTENSIONS.md"
IMAGES_OVERVIEW = ROOT / "images/README.ai-blaise.md"
PG_OVERLAY_README = ROOT / "images/citus-pg-overlay/README.md"

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
docs_compact = compact(docs)

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
        "D11",
        "MR9",
        "RT5",
        "S7",
        "Sec7",
        "Sec8",
        "Sec9",
        "T6",
        "T7",
    },
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
    "cargo run -p ai_blaise_citus_mcp -- run-canonical": {
        "D11",
    },
    "cargo run -p ai_blaise_citus_pool -- run-canonical": {
        "T7",
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

for feature_id in ("MCP1", "MCP2", "MCP3", "D11"):
    entry = entry_by_id.get(feature_id)
    if entry is None:
        fail(f"{feature_id} feature heading is required for MCP runtime contract evidence")
    if entry["status"].lower() in PRODUCTION_STATUSES:
        fail(f"{feature_id} must remain alpha until broader MCP auth, mutation, Kubernetes, and sidecar production contracts are implemented")
    body_compact = compact(entry["body"])
    for phrase in (
        "ci/ai-blaise/mcp-stdio-smoke.sh",
        "ci/ai-blaise/mcp-sidecar-stdio-smoke.sh",
        "ci/ai-blaise/mcp-sidecar-http-smoke.sh",
        "serve-stdio",
        "json-rpc",
        "mcp4 covers read-only database execution",
        "authentication, mutating database execution, kubernetes tool execution, and production sidecar enablement remain alpha",
    ):
        if phrase not in body_compact:
            fail(f"{feature_id} must cite MCP runtime contract evidence marker: {phrase}")

mcp4 = entry_by_id.get("MCP4")
if mcp4 is None:
    fail("MCP4 feature heading is required for the production-ready MCP database runtime")
if mcp4["status"].lower() not in PRODUCTION_STATUSES:
    fail("MCP4 must be marked production-ready for the narrow read-only database runtime")
mcp4_body = compact(mcp4["body"])
for phrase in (
    "ci/ai-blaise/mcp-db-smoke.sh",
    "ai_blaise_mcp_database_url",
    "postgres:17",
    "maintained postgresql rust client with native tls support",
    "begin read only",
    "set local statement_timeout",
    "capped at 1000 rows",
    "caps caller-supplied query timeouts at 300000 ms",
    "rejects `explain analyze`",
    "query_with_timeout",
    "run_explain",
    "list_shards",
    "schema tenant_b is outside allowed_schemas",
    "safe mode denied a destructive tool",
    "authentication, mutating database execution, kubernetes tool execution, and production sidecar enablement remain alpha",
):
    if phrase not in mcp4_body:
        fail(f"MCP4 production-ready entry is missing boundary/evidence marker: {phrase}")
for phrase in (
    "FEATURE: MCP4",
    "MCP_DATABASE_URL_ENV",
    "AI_BLAISE_MCP_DATABASE_URL",
    "MCP_MAX_ROWS_ENV",
    "MCP_MAX_ROWS_CEILING",
    "MCP_MAX_TIMEOUT_MS",
    "native_tls::TlsConnector",
    "postgres::Client",
    "postgres_native_tls::MakeTlsConnector",
    "BEGIN READ ONLY",
    "SET LOCAL statement_timeout",
    "query_rows_as_json",
    "TimeoutTooLarge",
    "safe-mode MCP EXPLAIN must not use ANALYZE",
    "tenant_archive",
):
    require_text(ROOT / "tools/citus-mcp/src/lib.rs", phrase)
for phrase in (
    "FEATURE: MCP4",
    "postgres:17",
    "POSTGRES_HOST_AUTH_METHOD=trust",
    "AI_BLAISE_MCP_DATABASE_URL",
    "AI_BLAISE_MCP_MAX_ROWS",
    "\"name\": \"query_with_timeout\"",
    "\"name\": \"run_explain\"",
    "\"name\": \"list_shards\"",
    "executed query_with_timeout",
    "executed run_explain",
    "executed list_shards",
    "schema tenant_b is outside allowed_schemas",
    "safe mode denied a destructive tool",
    "ai_blaise_citus_mcp database smoke passed",
):
    if phrase not in mcp_db_smoke:
        fail(f"mcp-db-smoke.sh is missing real database proof marker: {phrase}")
if "REQUIRE_DOCKER=1 bash ci/ai-blaise/mcp-db-smoke.sh" not in tools_workflow:
    fail("tools workflow must run the MCP database smoke")
if "mcp-db-smoke" not in makefile:
    fail("Makefile gate-close must include the MCP database smoke")
for phrase in (
    "FEATURE: MCP1 MCP2 MCP3 D11",
    "serve-stdio",
    "\"method\": \"initialize\"",
    "\"method\": \"tools/list\"",
    "\"name\": \"query_with_timeout\"",
    "\"name\": \"tenant_archive\"",
    "safe mode denied a destructive tool",
    "tenant_scope is required",
    "schema tenant_b is outside allowed_schemas",
    "ai_blaise_citus_mcp stdio smoke passed",
):
    if phrase not in mcp_smoke:
        fail(f"mcp-stdio-smoke.sh is missing real stdio proof marker: {phrase}")
if "bash ci/ai-blaise/mcp-stdio-smoke.sh" not in tools_workflow:
    fail("tools workflow must run the MCP stdio smoke")
if "mcp-stdio-smoke" not in makefile:
    fail("Makefile gate-close must include the MCP stdio smoke")
for phrase in (
    "FEATURE: MCP1 MCP2 MCP3 D11",
    "ai_blaise_citus_sidecar_mcp",
    "serve-stdio",
    "ai-blaise-citus-mcp-sidecar",
    "\"method\": \"initialize\"",
    "\"method\": \"tools/list\"",
    "\"name\": \"query_with_timeout\"",
    "\"name\": \"tenant_archive\"",
    "safe mode denied a destructive tool",
    "tenant_scope is required",
    "schema tenant_b is outside allowed_schemas",
    "ai_blaise_citus_sidecar_mcp stdio smoke passed",
):
    if phrase not in mcp_sidecar_smoke:
        fail(f"mcp-sidecar-stdio-smoke.sh is missing real stdio proof marker: {phrase}")
if "bash ci/ai-blaise/mcp-sidecar-stdio-smoke.sh" not in sidecar_workflow:
    fail("sidecar workflow must run the MCP sidecar stdio smoke")
if "mcp-sidecar-stdio-smoke" not in makefile:
    fail("Makefile gate-close must include the MCP sidecar stdio smoke")
for phrase in (
    "FEATURE: MCP1 MCP2 MCP3 D11",
    "ai_blaise_citus_sidecar_mcp",
    "serve",
    "GET\", \"/readyz\"",
    "POST\", \"/mcp\"",
    "ai-blaise-citus-mcp-sidecar",
    "\"method\": \"initialize\"",
    "\"name\": \"query_with_timeout\"",
    "\"name\": \"tenant_archive\"",
    "schema tenant_b is outside allowed_schemas",
    "safe mode denied a destructive tool",
    "ai_blaise_citus_sidecar_mcp HTTP smoke passed",
):
    if phrase not in mcp_sidecar_http_smoke:
        fail(f"mcp-sidecar-http-smoke.sh is missing real HTTP proof marker: {phrase}")
for phrase in (
    "probe_mcp_sidecar_jsonrpc",
    "POST /mcp HTTP/1.1",
    "ai-blaise-citus-mcp-sidecar",
    "validated query_with_timeout",
    "schema tenant_b is outside allowed_schemas",
    "safe mode denied a destructive tool",
):
    if phrase not in kind_smoke:
        fail(f"kind-production-smoke.sh must prove deployed MCP sidecar JSON-RPC: {phrase}")
if "bash ci/ai-blaise/mcp-sidecar-http-smoke.sh" not in sidecar_workflow:
    fail("sidecar workflow must run the MCP sidecar HTTP smoke")
if "mcp-sidecar-http-smoke" not in makefile:
    fail("Makefile gate-close must include the MCP sidecar HTTP smoke")

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
if "expected=$'16\\t16\\t3\\tfalse" not in v2_acceptance:
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
    "CREATE EXTENSION pgcrypto;",
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
    "companion_internal.plan_freeze",
    "companion_internal.plan_auto_promote",
    "companion_internal.plan_regression_guard",
    "companion_plan_regression_violates",
    "companion_plan_freezes",
    "PM3 plan freeze state was not visible with policy metadata",
    "PM4 regression guard did not flag latency regression",
    "PM4 regression guard flagged an allowed candidate",
    "PM4 regression samples were not recorded",
    "PM3 plan_freeze accepted an empty query hash",
    "PM4 regression guard accepted an unknown frozen plan",
    "companion_internal.migrate_start",
    "companion_internal.migration_add_column",
    "companion_internal.migration_online_type_change",
    "companion_migration_runs",
    "M1 migration run was not completed and visible",
    "M1/M11 migration operations were not recorded",
    "M11 online type-change accepted identical types",
    "companion_internal.index_advisor_record_candidate",
    "companion_index_advisor_ranked",
    "IA3 ranked advisor did not render CREATE INDEX CONCURRENTLY SQL",
    "IA3 accepted a non-improving candidate",
    "companion_internal.webhook_register",
    "companion_internal.install_webhook_trigger",
    "companion_webhook_events",
    "WH2 webhook trigger did not enqueue INSERT and UPDATE rows",
    "WH2 accepted a non-http webhook URL",
    "companion_internal.bump_placement_generation",
    "companion_placement_generation",
    "companion_local_placement_matches",
    "companion_hash_shard_index",
    "companion_range_shard_index",
    "S6 placement generation did not advance from 1 to 2",
    "S6 unknown shard should return generation zero",
    "S13 hash routing helper was not deterministic and bounded",
    "S13 range routing helper returned",
    "S13 range routing helper accepted an out-of-bounds value",
    "companion_verify_jwt_hs256",
    "Sec2 JWT verification did not return expected claims",
    "Sec2 verified JWT claims did not feed Auth2 tenant claims",
    "Sec2 JWT verification accepted a bad signature",
    "Sec2 JWT verification accepted a wrong audience",
    "Sec2 JWT verification accepted an expired token",
    "Sec2 JWT verification accepted a missing tenant_id claim",
    "companion_require_tenant_id",
    "companion_tenant_id_matches",
    "companion_internal.ledger_transfer",
    "companion_ledger_chain_valid",
    "companion_ledger_seal",
    "ALTER TABLE rls_smoke_orders ENABLE ROW LEVEL SECURITY",
    "CREATE POLICY rls_smoke_tenant_isolation",
    "SET ROLE ai_blaise_rls_smoke",
    "Sec1 RLS WITH CHECK allowed a cross-tenant insert",
    "Sec5 ledger transfer did not return a sha256 entry hash",
    "Sec5 ledger entries must reject mutation",
    "Sec6 ledger seals must reject deletion",
    "Sec6 ledger seal accepted an unsupported algorithm",
    "uid claim must not be empty",
    "companion_internal.seed_extension_catalog",
    "companion_extension_feature_coverage",
    "companion_extension_required('A7')",
    "companion_required_preload_libraries",
    "extension catalog hard-block conflict check did not flag orioledb",
    "extension catalog accepted empty feature ids",
    "companion_pg_stat_local_activity",
    "companion_idle_transactions('100 milliseconds'::interval)",
):
    if phrase not in image_check:
        fail(f"image-check.sh must statically guard SQL smoke marker: {phrase}")

for phrase in (
    "CREATE FUNCTION companion_set_session_claims",
    "CREATE FUNCTION companion_current_session_claims",
    "CREATE FUNCTION companion_current_tenant_id",
    "CREATE TABLE IF NOT EXISTS companion_internal.plan_freezes",
    "CREATE TABLE IF NOT EXISTS companion_internal.plan_promotion_policies",
    "CREATE TABLE IF NOT EXISTS companion_internal.plan_regression_policies",
    "CREATE TABLE IF NOT EXISTS companion_internal.plan_regression_samples",
    "CREATE VIEW companion_plan_freezes",
    "CREATE FUNCTION companion_internal.plan_freeze",
    "CREATE FUNCTION companion_internal.plan_auto_promote",
    "CREATE FUNCTION companion_internal.plan_regression_guard",
    "CREATE FUNCTION companion_plan_regression_violates",
    "CREATE TABLE IF NOT EXISTS companion_internal.migration_runs",
    "CREATE TABLE IF NOT EXISTS companion_internal.migration_operations",
    "CREATE VIEW companion_migration_runs",
    "CREATE VIEW companion_migration_operations",
    "CREATE FUNCTION companion_internal.migrate_start",
    "CREATE FUNCTION companion_internal.migration_add_column",
    "CREATE FUNCTION companion_internal.migration_online_type_change",
    "CREATE FUNCTION companion_internal.migrate_complete",
    "CREATE TABLE IF NOT EXISTS companion_internal.index_advisor_candidates",
    "CREATE VIEW companion_index_advisor_candidates",
    "CREATE FUNCTION companion_internal.index_advisor_record_candidate",
    "CREATE FUNCTION companion_index_advisor_ranked",
    "CREATE TABLE IF NOT EXISTS companion_internal.webhook_registrations",
    "CREATE TABLE IF NOT EXISTS companion_internal.webhook_triggers",
    "CREATE TABLE IF NOT EXISTS companion_internal.webhook_events",
    "CREATE VIEW companion_webhook_registrations",
    "CREATE VIEW companion_webhook_events",
    "CREATE FUNCTION companion_internal.webhook_register",
    "CREATE FUNCTION companion_internal.install_webhook_trigger",
    "CREATE TABLE IF NOT EXISTS companion_internal.shard_placement_generations",
    "CREATE VIEW companion_shard_placement_generations",
    "CREATE FUNCTION companion_internal.bump_placement_generation",
    "CREATE FUNCTION companion_placement_generation",
    "CREATE FUNCTION companion_local_placement_matches",
    "CREATE FUNCTION companion_hash_shard_index",
    "CREATE FUNCTION companion_range_shard_index",
    "CREATE FUNCTION companion_internal.base64url_encode",
    "CREATE FUNCTION companion_internal.base64url_decode",
    "CREATE FUNCTION companion_internal.jwt_audience_matches",
    "CREATE FUNCTION companion_verify_jwt_hs256",
    "CREATE FUNCTION companion_require_tenant_id",
    "CREATE FUNCTION companion_tenant_id_matches(row_tenant_id text)",
    "CREATE FUNCTION companion_tenant_id_matches(row_tenant_id uuid)",
    "CREATE TABLE IF NOT EXISTS companion_internal.ledger_entries",
    "CREATE TABLE IF NOT EXISTS companion_internal.ledger_seals",
    "CREATE FUNCTION companion_internal.ledger_transfer",
    "CREATE FUNCTION companion_ledger_chain_valid",
    "CREATE FUNCTION companion_ledger_seal",
    "CREATE VIEW companion_ledger_entries",
    "'Auth2', 'tenant-aware claims', 'sql-runtime'",
    "'PM3', 'plan freeze companion module', 'sql-runtime'",
    "'PM4', 'plan regression detection', 'sql-runtime'",
    "'M1', 'pgroll-style expand-contract migrations', 'sql-runtime'",
    "'M11', 'online column-type migration', 'sql-runtime'",
    "'IA3', 'companion index advisor', 'sql-runtime'",
    "'WH2', 'companion webhook helpers', 'sql-runtime'",
    "'Search2', 'distributed BM25 search index', 'sql-runtime'",
    "'Search3', 'hybrid BM25 and vector ranking', 'sql-runtime'",
    "'Search9', 'reranker UDF plan', 'sql-runtime'",
    "'G2', 'distributed graph bridge', 'sql-runtime'",
    "'G3', 'graph colocation policy', 'sql-runtime'",
    "'API4', 'GraphQL distributed graph metadata', 'sql-runtime'",
    "'JS2', 'distributed JSON Schema validation', 'sql-runtime'",
    "'M13', 'JSON Schema validation triggers', 'sql-runtime'",
    "'Geo2', 'geo-aware distribution', 'sql-runtime'",
    "'Geo3', 'geo shard pruning', 'sql-runtime'",
    "'A1', 'pgai-compatible vectorizer DSL', 'sql-runtime'",
    "'TS9', 'doctor rules for cohabitation', 'sql-runtime'",
    "'M7', 'pre-flight cohabit-extension check', 'sql-runtime'",
    "'T8', 'toolkit two-step aggregate pushdown', 'sql-runtime'",
    "'L9', 'worker partial aggregate pushdown', 'sql-runtime'",
    "'TS13', 'distributed time_bucket_gapfill', 'sql-runtime'",
    "'TS14', 'distributed metric toolkit aggregates', 'sql-runtime'",
    "'TS15', 'distributed approximate toolkit aggregates', 'sql-runtime'",
    "'TS16', 'distributed downsampler toolkit aggregates', 'sql-runtime'",
    "'TS17', 'distributed state toolkit aggregates', 'sql-runtime'",
    "'C10', 'online schema job state machine', 'sql-runtime'",
    "'M2', 'gh-ost-style online DDL', 'sql-runtime'",
    "'S14', 'tenant migration online', 'sql-runtime'",
    "'TO3', 'tenant migration online', 'sql-runtime'",
    "'TO4', 'tenant archive', 'sql-runtime'",
    "'TO5', 'tenant region affinity', 'sql-runtime'",
    "CREATE FUNCTION companion_internal.register_search_index",
    "CREATE FUNCTION companion_internal.hybrid_rank",
    "CREATE FUNCTION companion_internal.rerank_search",
    "CREATE FUNCTION companion_internal.ensure_graph_colocation",
    "CREATE FUNCTION companion_internal.register_graphql_distributed_graph",
    "CREATE FUNCTION companion_internal.register_json_schema",
    "CREATE FUNCTION companion_internal.install_jsonschema_trigger",
    "CREATE FUNCTION companion_internal.add_geohash_column",
    "CREATE FUNCTION companion_internal.enable_geo_shard_pruning",
    "CREATE FUNCTION companion_internal.register_vectorizer",
    "CREATE FUNCTION companion_internal.vectorizer_enqueue",
    "CREATE FUNCTION companion_internal.assert_shared_preload_libraries",
    "CREATE FUNCTION companion_internal.get_violations",
    "CREATE FUNCTION companion_internal.register_toolkit_aggregate_plan",
    "CREATE FUNCTION companion_internal.schema_job_start",
    "CREATE FUNCTION companion_internal.schema_job_advance",
    "CREATE FUNCTION companion_internal.plan_tenant_move",
    "CREATE FUNCTION companion_internal.plan_tenant_archive",
    "CREATE FUNCTION companion_internal.set_tenant_region_affinity",
    "CREATE TABLE IF NOT EXISTS companion_internal.extension_catalog_contracts",
    "CREATE VIEW companion_extension_catalog",
    "CREATE VIEW companion_extension_feature_coverage",
    "CREATE FUNCTION companion_internal.register_extension_contract",
    "CREATE FUNCTION companion_internal.seed_extension_catalog",
    "CREATE FUNCTION companion_extension_required",
    "CREATE FUNCTION companion_required_preload_libraries",
    "CREATE FUNCTION companion_extension_conflicts",
    "CREATE FUNCTION companion_internal.assert_extension_allowed",
    "'A7', 'pgvector cohabitation', 'extension-catalog-runtime'",
    "'Search1', 'pg_search bundled', 'extension-catalog-runtime'",
    "'Sec15', 'encryption-at-rest with CMK', 'extension-catalog-runtime'",
    "'S6', 'placement generation helpers', 'sql-runtime'",
    "'S13', 'range routing helpers', 'sql-runtime'",
    "'Sec1', 'RLS helpers', 'sql-runtime'",
    "'Sec2', 'JWT verification UDF', 'sql-runtime'",
    "'Sec5', 'immutable ledger', 'sql-runtime'",
    "'Sec6', 'ledger HMAC tamper evidence', 'sql-runtime'",
):
    if phrase not in sources:
        fail(f"ai_blaise_citus SQL extension is missing Auth2/Sec1 runtime marker: {phrase}")
    if phrase not in image_check:
        fail(f"image-check.sh must statically guard Auth2/Sec1 SQL extension marker: {phrase}")

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
