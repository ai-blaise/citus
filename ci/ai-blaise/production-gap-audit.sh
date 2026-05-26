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
SIDECAR_COLDTIER_SMOKE = ROOT / "ci/ai-blaise/sidecar-coldtier-runtime-smoke.sh"
SIDECAR_COLDTIER_LIB = ROOT / "sidecar/coldtier/src/lib.rs"
SIDECAR_COLDTIER_MAIN = ROOT / "sidecar/coldtier/src/main.rs"
SIDECAR_CONTROLLER_LIVE_SMOKE = ROOT / "ci/ai-blaise/sidecar-controller-live-smoke.sh"
SIDECAR_SHARED_README = ROOT / "sidecar/shared/README.md"
SIDECAR_CDC_SMOKE = ROOT / "ci/ai-blaise/sidecar-cdc-smoke.sh"
SIDECAR_CDC_README = ROOT / "sidecar/cdc/README.md"
SIDECAR_CDC_MODIFICATION = ROOT / "sidecar/cdc/MODIFICATION.md"
OPERATOR_RECONCILERS_BATCH_C_SMOKE = ROOT / "ci/ai-blaise/operator-reconcilers-batch-c-smoke.sh"
COMPANION_RUNTIME_DEPTH_A_SMOKE = ROOT / "ci/ai-blaise/companion-runtime-depth-a-smoke.sh"
GRAPHQL_POSTGREST_RUNTIME_SMOKE = ROOT / "ci/ai-blaise/graphql-postgrest-runtime-smoke.sh"
GRAPHQL_PGGRAPHQL_LIVE_SMOKE = ROOT / "ci/ai-blaise/graphql-pggraphql-live-smoke.sh"
POSTGREST_LIVE_DATA_PLANE_SMOKE = ROOT / "ci/ai-blaise/postgrest-live-data-plane-smoke.sh"
EDGE_DENO_LIVE_SMOKE = ROOT / "ci/ai-blaise/edge-functions-deno-live-smoke.sh"
EDGE_BUN_LIVE_SMOKE = ROOT / "ci/ai-blaise/edge-functions-bun-live-smoke.sh"
EDGE_DB_CALLBACK_UDS_SMOKE = ROOT / "ci/ai-blaise/edge-functions-db-callback-uds-smoke.sh"
STRUCTURED_LOG_INGESTION_SMOKE = ROOT / "ci/ai-blaise/structured-log-ingestion-smoke.sh"
OBSERVABILITY_WORKFLOW = ROOT / ".github/workflows/ci-observability-contracts.yml"
SIDECAR_REALTIME_SMOKE = ROOT / "ci/ai-blaise/sidecar-realtime-smoke.sh"
SIDECAR_REALTIME_README = ROOT / "sidecar/realtime/README.md"
STORAGE_RUNTIME_SMOKE = ROOT / "ci/ai-blaise/storage-sidecar-runtime-smoke.sh"
POOL_PROXY_SMOKE = ROOT / "ci/ai-blaise/pool-proxy-smoke.sh"
SQL_EXTENSION_SMOKE = ROOT / "ci/ai-blaise/sql-extension-smoke.sh"
POOL_ROUTING_SECURITY_SMOKE = ROOT / "ci/ai-blaise/pool-routing-security-smoke.sh"
POOL_GEOIP_LIVE_SMOKE = ROOT / "ci/ai-blaise/pool-geoip-live-smoke.sh"
BULK_DISTSQL_LIVE_SMOKE = ROOT / "ci/ai-blaise/bulk-distsql-live-smoke.sh"
TIMESCALE_ADVANCED_LIVE_SMOKE = ROOT / "ci/ai-blaise/timescale-advanced-live-smoke.sh"
PLACEMENT_GENERATION_UDF_SMOKE = ROOT / "ci/ai-blaise/placement-generation-udf-contract-smoke.sh"
SECURITY_SUPPLY_CHAIN_SMOKE = ROOT / "ci/ai-blaise/security-supply-chain-smoke.sh"
SECURITY_EXTERNAL_SECRETS_TLS_LIVE_SMOKE = ROOT / "ci/ai-blaise/security-external-secrets-tls-live-smoke.sh"
SECURITY_SBOM_COSIGN_LIVE_SMOKE = ROOT / "ci/ai-blaise/security-sbom-cosign-live-smoke.sh"
PATCHES_WORKFLOW = ROOT / ".github/workflows/ci-patches.yml"
OPERATOR_WORKFLOW = ROOT / ".github/workflows/ci-operator.yml"
PRODUCTION_WORKFLOW = ROOT / ".github/workflows/ci-production-readiness.yml"
COORDINATORLESS_MX_LIVE_SMOKE = ROOT / "ci/ai-blaise/coordinatorless-mx-live-smoke.sh"
CITUS_PATCH_AUDIT = ROOT / "ci/ai-blaise/citus-patch-production-audit.sh"
RUNBOOK_CHECK = ROOT / "ci/ai-blaise/runbook-command-check.sh"
RELEASE_HARDENING_SMOKE = ROOT / "ci/ai-blaise/release-hardening-runbook-smoke.sh"
CANARY_UPGRADE_SMOKE = ROOT / "ci/ai-blaise/canary-upgrade-rollback-smoke.sh"
UPGRADE_MANIFEST = ROOT / "images/citus-pg-overlay/extensions/ai_blaise_citus-upgrade-manifest.tsv"
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
CITUSCTL_K8S_APPLY_LIVE_SMOKE = ROOT / "ci/ai-blaise/citusctl-k8s-apply-live-smoke.sh"
CITUSCTL_TIME_TRAVEL_INTENT_SMOKE = ROOT / "ci/ai-blaise/citusctl-time-travel-intent-smoke.sh"
CITUSCTL_LIB = ROOT / "tools/citusctl/src/lib.rs"
FDW_CREDENTIAL_ROTATION_SMOKE = ROOT / "ci/ai-blaise/fdw-credential-rotation-live-smoke.sh"
SCHEMA_DRIFT_LIVE_SMOKE = ROOT / "ci/ai-blaise/schema-drift-live-smoke.sh"
SIDECAR_RAFT_SMOKE = ROOT / "ci/ai-blaise/sidecar-raft-smoke.sh"
SIDECAR_HLC_SMOKE = ROOT / "ci/ai-blaise/sidecar-hlc-smoke.sh"
TXN_STATUS_NETWORKED_RAFT_SMOKE = ROOT / "ci/ai-blaise/txn-status-networked-raft-smoke.sh"
COMPANION_CONTRACTS = ROOT / "companion/src/bin/companion_contracts.rs"
COMPANION_WORKFLOW = ROOT / ".github/workflows/ci-companion.yml"
EDGE2_LIBSQL_GUARD_SMOKE = ROOT / "ci/ai-blaise/edge2-libsql-research-guard-smoke.sh"
EDGE2_LIBSQL_ADR = ROOT / "docs/ai-blaise/ADR/0009-libsql-read-tier-research-guard.md"
SHARD_TEMPERATURE_RANKING_LIVE_SMOKE = ROOT / "ci/ai-blaise/shard-temperature-ranking-live-smoke.sh"
COLUMNAR_TIERING_LIVE_SMOKE = ROOT / "ci/ai-blaise/columnar-tiering-live-smoke.sh"
CROSS_TIER_QUERY_LIVE_SMOKE = ROOT / "ci/ai-blaise/cross-tier-query-live-smoke.sh"
POSTGRES_CORE_PATCHES_LIVE_SMOKE = ROOT / "ci/ai-blaise/postgres-core-patches-live-smoke.sh"
PGCORE_PATCHES_DOCKERFILE = ROOT / "images/citus-pg-overlay/Dockerfile.pgcore-patches"
PGC_PROBE_C = ROOT / "ci/ai-blaise/pgc_probe/ai_blaise_pgc_probe.c"
PGC_PROBE_SQL = ROOT / "ci/ai-blaise/pgc_probe/ai_blaise_pgc_probe--0.1.0.sql"
POSTGRES_PATCH_SERIES = ROOT / "patches/postgres/series"
REGIONAL_PLACEMENT_LIVE_SMOKE = ROOT / "ci/ai-blaise/regional-placement-live-smoke.sh"
REGIONAL_ROW_PLACEMENT = ROOT / "companion/src/regional_row_placement.rs"
TRANSACTION_STATE_LIVE_SMOKE = ROOT / "ci/ai-blaise/transaction-state-live-smoke.sh"
SHARD_SPLIT_LIVE_SMOKE = ROOT / "ci/ai-blaise/shard-split-live-smoke.sh"
CLONE_NODE_LIVE_SMOKE = ROOT / "ci/ai-blaise/clone-node-live-smoke.sh"
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
    if status_by_id.get(feature_id) != "production-ready":
        fail(f"{feature_id} branch lifecycle must be production-ready once live kind+CSI snapshot evidence is wired")
if status_by_id.get("MR3") != "production-ready":
    fail("MR3 must be production-ready once live multi-worker regional row-placement evidence is wired")
if status_by_id.get("MR9") != "production-ready":
    fail("MR9 must be production-ready once live regional failover smoke evidence is wired")
for feature_id in ("PGC1", "PGC2"):
    if status_by_id.get(feature_id) != "production-ready":
        fail(f"{feature_id} must be production-ready once patched PG17+Citus runtime evidence is wired")

pgc_truth = "\n".join(
    read(path)
    for path in (
        POSTGRES_CORE_PATCHES_LIVE_SMOKE,
        PGCORE_PATCHES_DOCKERFILE,
        PGC_PROBE_C,
        PGC_PROBE_SQL,
        MAKEFILE,
        DOCS,
        AUDIT,
        PG_OVERLAY_README,
        POSTGRES_PATCH_SERIES,
    )
)
for phrase in (
    "Dockerfile.pgcore-patches",
    "REL_17_10",
    "patches/postgres/series",
    "while IFS= read -r patch_name",
    "git apply \"/patches/postgres/${patch_name}\"",
    "0001-logical-commit-clock.patch",
    "0002-per-subtrans-commit-ts.patch",
    "SubTransactionIdSetCommitTsData",
    "XLogSetLastTransactionStopTimestamp",
    "CREATE EXTENSION citus",
    "CREATE EXTENSION ai_blaise_pgc_probe",
    "pg_xact_commit_timestamp",
    "pg_waldump",
    "SUBTRANS_TS",
    "pgc_citus_built_against_patched_pg=true",
    "pgc_logical_clock_hook_executed=true",
    "pgc_subtrans_commit_ts_override_executed=true",
    "pgc_pgactive_traffic_exercised=false",
    "pgc_spock_apply_traffic_exercised=false",
    "pgc_pg18_exercised=false",
    "postgres-core-patches-live-smoke:",
    "gate-close:",
):
    if compact(phrase) not in compact(pgc_truth):
        fail(f"PGC1/PGC2 patched-core runtime boundary missing truth phrase: {phrase}")
for feature_id in ("PGC1", "PGC2"):
    section = feature_section(docs, feature_id)
    for phrase in (
        "Production evidence:",
        "postgres-core-patches-live-smoke.sh",
        "Citus build-against-patched-`pg_config`",
        "PG18",
        "full Bundle1 operand image",
    ):
        if compact(phrase) not in compact(section):
            fail(f"{feature_id} docs lost patched-core production boundary phrase: {phrase}")
    if compact("alpha-with-placeholder") in compact(section):
        fail(f"{feature_id} docs still claim alpha placeholder status")

audit_compact = compact(audit)

sidecar_coldtier_smoke = read(SIDECAR_COLDTIER_SMOKE)
sidecar_coldtier_lib = read(SIDECAR_COLDTIER_LIB)
sidecar_coldtier_main = read(SIDECAR_COLDTIER_MAIN)
coldtier_makefile_text = read(MAKEFILE)
sidecar_workflow = read(SIDECAR_WORKFLOW)

for feature_id in ("R1", "R5", "R9", "Search8"):
    if status_by_id.get(feature_id) != "production-ready":
        fail(f"{feature_id} cold-tier local file materialization must be production-ready")
    section = compact(feature_section(docs, feature_id))
    for phrase in (
        "Production evidence:",
        "ci/ai-blaise/sidecar-coldtier-runtime-smoke.sh",
        "run-local-file-materialization-canonical",
        "local `file://`",
        "materialized_artifact_count=4",
        "materialized_bytes=1408",
        "object_store_io_attempted=false",
        "citus_cold_read_serving=false",
        "S3/GCS/Azure object-store",
        "pageserver deployment",
        "Citus cold-read serving",
    ):
        if compact(phrase) not in section:
            fail(f"{feature_id} docs lost cold-tier production boundary phrase: {phrase}")

for phrase in (
    "run-local-file-materialization-canonical",
    "coldtier_local_file_materialization=passed",
    "local_file_materialized=true",
    "materialized_artifact_count=4",
    "materialized_bytes=1408",
    "materialized_layer_files=2",
    "search_indexes_materialized=2",
    "planner_routes_refreshed=1",
    "cold_tier_reads=1",
    "object_store_io_attempted=false",
    "citus_cold_read_serving=false",
    "/tmp/ai-blaise-coldtier/events/42/image.parquet",
):
    if phrase not in sidecar_coldtier_smoke:
        fail(f"sidecar-coldtier-runtime-smoke.sh lost local materialization assertion: {phrase}")

for phrase in (
    "materialize_file_artifacts",
    "ColdTierMaterializationReport",
    "UnsupportedMaterializationUri",
    "materialization supports only local file:// artifact URIs",
    "file://",
):
    if phrase not in sidecar_coldtier_lib:
        fail(f"sidecar/coldtier/src/lib.rs lost local materialization code: {phrase}")
for phrase in (
    "run-local-file-materialization-canonical",
    "materialize_file_artifacts",
):
    if phrase not in sidecar_coldtier_main:
        fail(f"sidecar/coldtier/src/main.rs lost local materialization command: {phrase}")
for phrase in (
    "sidecar-coldtier-runtime-smoke:",
    "ci/ai-blaise/sidecar-coldtier-runtime-smoke.sh",
    "gate-close:",
    "sidecar-coldtier-runtime-smoke",
):
    if phrase not in coldtier_makefile_text:
        fail(f"Makefile.ai-blaise must wire cold-tier materialization smoke: {phrase}")
if "sidecar-coldtier-runtime-smoke.sh" not in sidecar_workflow:
    fail("ci-sidecar workflow must run sidecar-coldtier-runtime-smoke.sh")
for phrase in (
    "R1/R5/R9/Search8 cold-tier local file materialization is production-ready",
    "local `file://` runtime",
    "coldtier_local_file_materialization=passed",
    "local_file_materialized=true",
    "materialized_artifact_count=4",
    "materialized_bytes=1408",
    "search_indexes_materialized=2",
    "object_store_io_attempted=false",
    "citus_cold_read_serving=false",
    "real Tantivy/LanceDB query execution",
):
    if compact(phrase) not in audit_compact:
        fail(f"PRODUCTION_READINESS_AUDIT.md missing cold-tier boundary phrase: {phrase}")

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
b5_section = feature_section(docs, "B5")
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
    "**Status**: production-ready",
    "M8 is production-ready for two real binary paths",
    "citusctl plan/apply apply <manifest>",
    "kubectl apply --dry-run=server",
    "deterministic `k8s-apply-*` plan id",
    "requires apply to match that rendered plan id",
    "k8s-manifest-apply.audit.tsv",
    "live-kubernetes-manifest-apply",
    "does not claim Docker/kind lifecycle orchestration",
    "deterministic JSON/TSV output",
    "local audit append",
):
    if compact(phrase) not in compact(m8_section):
        fail(f"M8 citusctl plan/apply boundary missing phrase: {phrase}")
for phrase in (
    "**Status**: production-ready",
    "citusctl plan/apply time-travel <target_time>",
    "strict RFC3339 UTC calendar validation",
    "rejects ahead-of-now targets",
    "rejects targets older than the explicit staleness window",
    "deterministic `time-travel-*` plan id",
    "time-travel-intent.audit.tsv",
    "time-travel-intent-validation-only",
    "does not execute follower reads",
):
    if compact(phrase) not in compact(b5_section):
        fail(f"B5 citusctl time-travel boundary missing phrase: {phrase}")
for phrase in (
    "explicit `--state-dir` invocations",
    "deterministic JSON/TSV output",
    "local audit append",
    "M8 Kubernetes manifest path",
    "server-side dry-run",
    "apply-time plan-id match guard",
    "real `kubectl apply`",
    "kubectl get -f",
    "k8s-manifest-apply.audit.tsv",
    "production cluster lifecycle management",
):
    if compact(phrase) not in compact(audit):
        fail(f"PRODUCTION_READINESS_AUDIT.md must preserve D1/M8 citusctl boundary: {phrase}")
for phrase in (
    "B5 time-travel intent",
    "strict RFC3339 UTC timestamp parsing",
    "explicit `--max-staleness-seconds` enforcement",
    "deterministic `time-travel-*`",
    "time-travel-intent.audit.tsv",
    "follower-read execution",
):
    if compact(phrase) not in compact(audit):
        fail(f"PRODUCTION_READINESS_AUDIT.md must preserve B5 citusctl boundary: {phrase}")

citusctl_lib = read(CITUSCTL_LIB)
for phrase in (
    "render_dev_lifecycle_cli_report_from_args",
    "validate_plan_id(plan_id)",
    "append_dev_audit_record",
    "DevLifecycleCliReport",
    "state-file-only-no-recursive-delete",
    "render_k8s_manifest_cli_report_from_args",
    "append_k8s_manifest_audit_record",
    "live-kubernetes-manifest-apply",
    "kubectl_apply_server_dry_run",
    "PlanIdMismatch",
    "render_time_travel_intent_cli_report_from_args",
    "append_time_travel_intent_audit_record",
    "time-travel-intent-validation-only",
    "utc_timestamp_epoch_seconds",
    "TimeTravelPlanIdMismatch",
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

citusctl_k8s_smoke = read(CITUSCTL_K8S_APPLY_LIVE_SMOKE)
for phrase in (
    "FEATURE: M8",
    "kind create cluster",
    "run_citusctl plan apply",
    "run_citusctl apply wrong-plan-id apply",
    "plan_id does not match current Kubernetes manifest plan",
    "kubectl -n \"${namespace}\" get configmap ai-blaise-citusctl-live",
    "k8s-manifest-apply.audit.tsv",
    "live-kubernetes-manifest-apply",
):
    if phrase not in citusctl_k8s_smoke:
        fail(f"citusctl-k8s-apply-live-smoke.sh lost required M8 live assertion: {phrase}")

citusctl_time_travel_smoke = read(CITUSCTL_TIME_TRAVEL_INTENT_SMOKE)
for phrase in (
    "FEATURE: B5",
    "run_citusctl plan time-travel",
    "run_citusctl apply \"${plan_id}\" time-travel",
    "plan_id does not match current time-travel intent plan",
    "target_time must be an RFC3339 UTC timestamp",
    "older than max_staleness_seconds 60",
    "must not be in the future",
    "time-travel-intent.audit.tsv",
    "time-travel-intent-validation-only",
):
    if phrase not in citusctl_time_travel_smoke:
        fail(f"citusctl-time-travel-intent-smoke.sh lost required B5 assertion: {phrase}")

makefile_text = read(MAKEFILE)
for phrase in (
    "citusctl-dev-lifecycle-smoke:",
    "ci/ai-blaise/citusctl-dev-lifecycle-smoke.sh",
    "citusctl-k8s-apply-live-smoke:",
    "ci/ai-blaise/citusctl-k8s-apply-live-smoke.sh",
    "citusctl-time-travel-intent-smoke:",
    "ci/ai-blaise/citusctl-time-travel-intent-smoke.sh",
    "gate-close:",
    "citusctl-dev-lifecycle-smoke",
    "citusctl-k8s-apply-live-smoke",
    "citusctl-time-travel-intent-smoke",
):
    if phrase not in makefile_text:
        fail(f"Makefile.ai-blaise must wire citusctl smoke: {phrase}")

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

branch_lifecycle_smoke = read(ROOT / "ci/ai-blaise/operator-branch-lifecycle-smoke.sh") + read(ROOT / "ci/ai-blaise/operator-branch-lifecycle-live-smoke.sh")
branch_scale_live_smoke = read(ROOT / "ci/ai-blaise/operator-branch-scale-to-zero-live-smoke.sh")
r2_truth = compact(
    feature_section(docs, "R2")
    + "\n"
    + audit
    + "\n"
    + branch_lifecycle_smoke
    + "\n"
    + branch_scale_live_smoke
    + "\n"
    + read(MAKEFILE)
    + "\n"
    + read(OPERATOR_WORKFLOW)
)
if status_by_id.get("R2") != "production-ready":
    fail("R2 scale-to-zero compute must be production-ready after live Kubernetes scale-down evidence")
for phrase in (
    "operator-branch-lifecycle-smoke.sh",
    "operator-branch-scale-to-zero-live-smoke.sh",
    "run-branch-lifecycle-canonical",
    "ScaleTargetComputeToZero",
    "branch_scale_to_zero_live=passed",
    "kubernetes_deployment_scaled_to_zero=true",
    "spec_replicas_after_scale=0",
    "observed_replicas_after_scale=0",
    "active_sessions_fail_closed=true",
    "pending_migrations_fail_closed=true",
    "REQUIRE_DOCKER=1",
    "kubectl scale deployment/branch-review --replicas=0",
    "CSI `VolumeSnapshot` creation",
    "PVC cloning",
    "traffic cut-over",
    "branch promotion",
):
    if compact(phrase) not in r2_truth:
        fail(f"R2 scale-to-zero production boundary missing truth phrase: {phrase}")
r12_section = feature_section(docs, "R12")
r12_smoke = read(SHARD_TEMPERATURE_RANKING_LIVE_SMOKE)
r12_truth = compact(
    r12_section
    + "\n"
    + audit
    + "\n"
    + r12_smoke
    + "\n"
    + read(ROOT / "companion/src/shard_temperature.rs")
    + "\n"
    + read(COMPANION_CONTRACTS)
    + "\n"
    + read(MAKEFILE)
    + "\n"
    + read(COMPANION_WORKFLOW)
)
if status_by_id.get("R12") != "production-ready":
    fail("R12 per-shard temperature ranking must be production-ready after live Citus catalog ranking evidence")
for phrase in (
    "companion/src/shard_temperature.rs",
    "run-shard-temperature-ranking-canonical",
    "run-shard-temperature-ranking-sql-canonical",
    "shard-temperature-ranking-live-smoke.sh",
    "CREATE EXTENSION IF NOT EXISTS citus",
    "create_distributed_table('public.temperature_orders', 'tenant_id')",
    "FROM pg_dist_shard ds",
    "JOIN pg_class c ON c.oid = ds.logicalrelid",
    "JOIN pg_namespace n ON n.oid = c.relnamespace",
    "DENSE_RANK() OVER",
    "shard_temperature_ranking_live=passed",
    "citus_pg_dist_shard_joined=true",
    "temperature_scores_ranked=true",
    "hot_shards=1",
    "warm_shards=1",
    "cold_shards=1",
    "automatic_tier_movement=false",
    "coldtier_moves_executed=false",
    "does not collect production telemetry",
    "does not claim telemetry collection",
    "does not claim automatic tier movement",
    "does not claim cold-tier artifact moves",
    "does not claim Citus placement changes",
    "does not claim distributed planner integration",
):
    if compact(phrase) not in r12_truth:
        fail(f"R12 temperature ranking production boundary missing truth phrase: {phrase}")

columnar_tiering_truth = compact(
    feature_section(docs, "L7")
    + "\n"
    + feature_section(docs, "R3")
    + "\n"
    + feature_section(docs, "R8")
    + "\n"
    + audit
    + "\n"
    + read(ROOT / "companion/src/columnar_tiering.rs")
    + "\n"
    + read(COLUMNAR_TIERING_LIVE_SMOKE)
    + "\n"
    + read(COMPANION_CONTRACTS)
    + "\n"
    + read(MAKEFILE)
    + "\n"
    + read(COMPANION_WORKFLOW)
)
for feature_id in ("L7", "R3", "R8"):
    if status_by_id.get(feature_id) != "production-ready":
        fail(f"{feature_id} must be production-ready after live Citus columnar tiering evidence")
for phrase in (
    "companion/src/columnar_tiering.rs",
    "run-columnar-tiering-canonical",
    "run-columnar-tiering-sql-canonical",
    "columnar-tiering-live-smoke.sh",
    "CREATE EXTENSION IF NOT EXISTS citus_columnar",
    "USING columnar",
    "create_distributed_table('public.columnar_orders', 'tenant_id', shard_count => 4)",
    "pg_dist_partition",
    "pg_dist_shard",
    "pg_dist_placement",
    "pg_am",
    "_timescaledb_catalog.hypertable",
    "Custom Scan (Citus",
    "ColumnarScan",
    "columnar_tiering_live=passed",
    "l7_distributed_columnar_table=true",
    "l7_columnar_access_method=true",
    "l7_columnar_query_rows=12",
    "l7_columnar_query_total=3024",
    "l7_citus_custom_scan_executed=true",
    "l7_columnar_scan_executed=true",
    "r3_worker_columnstore_policy_live=true",
    "r3_worker_access_method=columnar",
    "r8_non_hypertable_cold_columnar_path=true",
    "cost_model_selection_exercised=false",
    "automatic_tier_movement_executed=false",
    "workload_routing_exercised=false",
    "kubernetes_traffic_exercised=false",
    "cost-model tier selection",
    "automatic tier movement",
    "workload-routing rewrites",
    "object-store cold reads",
    "Kubernetes traffic",
):
    if compact(phrase) not in columnar_tiering_truth:
        fail(f"columnar tiering production boundary missing truth phrase: {phrase}")
for phrase in ("columnar-tiering-live-smoke", "ci/ai-blaise/columnar-tiering-live-smoke.sh"):
    if phrase not in read(MAKEFILE):
        fail(f"Makefile.ai-blaise must wire the columnar tiering live smoke: {phrase}")


cross_tier_query_truth = compact(
    feature_section(docs, "L10")
    + "\n"
    + audit
    + "\n"
    + read(ROOT / "companion/src/cross_tier_query.rs")
    + "\n"
    + read(CROSS_TIER_QUERY_LIVE_SMOKE)
    + "\n"
    + read(COMPANION_CONTRACTS)
    + "\n"
    + read(MAKEFILE)
    + "\n"
    + read(COMPANION_WORKFLOW)
)
if status_by_id.get("L10") != "production-ready":
    fail("L10 must be production-ready after live Citus cross-tier query execution evidence")
for phrase in (
    "companion/src/cross_tier_query.rs",
    "run-cross-tier-query-canonical",
    "run-cross-tier-query-sql-canonical",
    "cross-tier-query-live-smoke.sh",
    "CREATE EXTENSION IF NOT EXISTS citus_columnar",
    "USING columnar",
    "create_distributed_table('public.l10_hot_orders', 'tenant_id', shard_count => 4)",
    "create_distributed_table('public.l10_warm_orders', 'tenant_id', shard_count => 4)",
    "create_distributed_table('public.l10_cold_orders', 'tenant_id', shard_count => 4)",
    "pg_dist_shard",
    "pg_dist_placement",
    "pg_am",
    "UNION ALL",
    "Custom Scan (Citus",
    "ColumnarScan",
    "cross_tier_query_live=passed",
    "l10_hot_tier_rows=4",
    "l10_warm_tier_rows=4",
    "l10_cold_tier_rows=4",
    "l10_cross_tier_rows=12",
    "l10_cross_tier_total=6678",
    "l10_citus_custom_scan_executed=true",
    "l10_columnar_scan_executed=true",
    "automatic_workload_routing_exercised=false",
    "automatic_query_rewrite_exercised=false",
    "cost_model_selection_exercised=false",
    "object_store_cold_read_exercised=false",
    "kubernetes_traffic_exercised=false",
    "automatic workload routing",
    "automatic query rewrites",
    "cost-model tier selection",
    "object-store cold reads",
    "Kubernetes traffic",
):
    if compact(phrase) not in cross_tier_query_truth:
        fail(f"L10 cross-tier query production boundary missing truth phrase: {phrase}")
for phrase in ("cross-tier-query-live-smoke", "ci/ai-blaise/cross-tier-query-live-smoke.sh"):
    if phrase not in read(MAKEFILE):
        fail(f"Makefile.ai-blaise must wire the cross-tier query live smoke: {phrase}")
if "run-cross-tier-query-canonical" not in read(COMPANION_WORKFLOW):
    fail("ci-companion workflow must run the cross-tier query canonical report")
if "run-cross-tier-query-sql-canonical" not in read(COMPANION_WORKFLOW):
    fail("ci-companion workflow must run the cross-tier query SQL renderer")

regional_section = feature_section(docs, "S8") + "\n" + feature_section(docs, "S12")
regional_smoke = read(REGIONAL_PLACEMENT_LIVE_SMOKE)
regional_truth = compact(
    regional_section
    + "\n"
    + audit
    + "\n"
    + regional_smoke
    + "\n"
    + read(ROOT / "companion/src/regional_placement.rs")
    + "\n"
    + read(COMPANION_CONTRACTS)
    + "\n"
    + read(MAKEFILE)
    + "\n"
    + read(COMPANION_WORKFLOW)
)
for feature_id in ("S8", "S12"):
    if status_by_id.get(feature_id) != "production-ready":
        fail(f"{feature_id} regional placement catalog guard must be production-ready after live Citus/PostgreSQL evidence")
for phrase in (
    "companion/src/regional_placement.rs",
    "run-regional-placement-canonical",
    "run-regional-placement-sql-canonical",
    "regional-placement-live-smoke.sh",
    "CREATE TABLESPACE ai_blaise_us_east_1",
    "CREATE TABLESPACE ai_blaise_eu_west_1",
    "create_distributed_table('public.locality_orders', 'locality_key')",
    "FROM pg_index i",
    "FROM pg_dist_partition",
    "JOIN pg_tablespace spc",
    "regional_placement_live=passed",
    "locality_prefixed_pk_valid=true",
    "citus_distribution_present=true",
    "region_tablespace_mappings_valid=true",
    "region_tablespace_count=2",
    "automatic_rebalance_executed=false",
    "shard_movement_executed=false",
    "worker_placement_enforced=false",
    "multi_region_failover_exercised=false",
    "does not claim key rewrites",
    "does not claim foreign-key compatibility migration",
    "does not claim production tablespace creation",
    "does not claim operator reconciliation",
    "does not claim worker-level shard placement enforcement",
    "does not claim automatic rebalance",
    "does not claim shard movement",
    "does not claim multi-region failover",
):
    if compact(phrase) not in regional_truth:
        fail(f"S8/S12 regional placement production boundary missing truth phrase: {phrase}")

mr3_truth = compact(
    feature_section(docs, "MR3")
    + "\n"
    + audit
    + "\n"
    + regional_smoke
    + "\n"
    + read(REGIONAL_ROW_PLACEMENT)
    + "\n"
    + read(COMPANION_CONTRACTS)
    + "\n"
    + read(MAKEFILE)
)
for phrase in (
    "**Status**: production-ready",
    "companion/src/regional_row_placement.rs",
    "run-regional-row-placement-canonical",
    "run-regional-row-placement-sql-canonical",
    "regional-placement-live-smoke.sh",
    "isolate_tenant_to_new_shard",
    "citus_move_shard_placement",
    "regional_row_placement_live=passed",
    "mr3_live_multi_worker_citus=true",
    "mr3_shards_isolated=true",
    "mr3_citus_move_shard_placement_executed=true",
    "mr3_worker_placement_enforced=true",
    "mr3_matched_region_count=2",
    "mr3_rows_preserved=true",
    "mr3_multi_region_network_exercised=false",
    "mr3_kubernetes_operator_reconciliation_exercised=false",
    "WAN/multi-region network execution",
    "Kubernetes operator reconciliation",
    "automatic repartition scheduling",
    "regional traffic routing",
    "regional failover",
    "MR9 is production-ready for the bounded two-region drill",
    "gate-close:",
    "regional-placement-live-smoke",
):
    if compact(phrase) not in mr3_truth:
        fail(f"MR3 regional row-placement production boundary missing truth phrase: {phrase}")

transaction_state_section = feature_section(docs, "T13") + "\n" + feature_section(docs, "T14")
transaction_state_smoke = read(TRANSACTION_STATE_LIVE_SMOKE)
transaction_state_truth = compact(
    transaction_state_section
    + "\n"
    + audit
    + "\n"
    + transaction_state_smoke
    + "\n"
    + read(ROOT / "companion/src/transaction_state.rs")
    + "\n"
    + read(COMPANION_CONTRACTS)
    + "\n"
    + read(MAKEFILE)
    + "\n"
    + read(COMPANION_WORKFLOW)
)
for feature_id in ("T13", "T14"):
    if status_by_id.get(feature_id) != "production-ready":
        fail(f"{feature_id} transaction-state smoke must be production-ready after live Citus transaction evidence")
for phrase in (
    "companion/src/transaction_state.rs",
    "run-transaction-state-canonical",
    "run-transaction-state-sql-canonical",
    "transaction-state-live-smoke.sh",
    "DECLARE",
    "NO SCROLL CURSOR",
    "FETCH 2 FROM",
    "SAVEPOINT",
    "ROLLBACK TO SAVEPOINT",
    "create_distributed_table('public.txn_state_orders', 'tenant_id')",
    "Custom Scan (Citus Adaptive)",
    "Task Count: 1",
    "transaction_state_live=passed",
    "distributed_cursor_declared=true",
    "cursor_fetch_batches=2",
    "cursor_rows_fetched=5",
    "savepoint_rollback_verified=true",
    "count_after_insert=6",
    "count_after_rollback=5",
    "final_count=5",
    "citus_adaptive_plan_observed=true",
    "citus_task_count_observed=1",
    "coordinator_failover_exercised=false",
    "multi_worker_cleanup_exercised=false",
    "wire_protocol_portal_exercised=false",
    "does not claim PostgreSQL wire protocol portal implementation",
    "does not claim multi-worker cursor cleanup",
    "does not claim cursor holdability across transactions",
    "does not claim coordinator restart recovery",
    "does not claim distributed deadlock handling",
    "does not claim Kubernetes transaction-drain behavior",
):
    if compact(phrase) not in transaction_state_truth:
        fail(f"T13/T14 transaction-state production boundary missing truth phrase: {phrase}")

branch_lifecycle_makefile = read(MAKEFILE)
for phrase in (
    "operator-branch-lifecycle-smoke:",
    "operator-branch-scale-to-zero-live-smoke:",
    "REQUIRE_DOCKER=1 bash ci/ai-blaise/operator-branch-scale-to-zero-live-smoke.sh",
    "gate-close:",
    "operator-branch-scale-to-zero-live-smoke",
):
    if phrase not in branch_lifecycle_makefile:
        fail(f"Makefile.ai-blaise must wire R2 branch scale smoke: {phrase}")
if "operator-branch-lifecycle-smoke.sh" not in read(OPERATOR_WORKFLOW):
    fail("ci-operator workflow must run operator-branch-lifecycle-smoke.sh")

branch_lifecycle_truth = compact(docs + "\n" + audit + "\n" + branch_lifecycle_smoke)
for phrase in (
    "ci/ai-blaise/operator-branch-lifecycle-smoke.sh",
    "ci/ai-blaise/operator-branch-lifecycle-live-smoke.sh",
    "csi-driver-host-path",
    "VolumeSnapshot",
    "branch-review-0",
    "branch lifecycle live smoke",
):
    if compact(phrase) not in branch_lifecycle_truth:
        fail(f"Branch lifecycle docs must preserve production-ready evidence boundary: {phrase}")

multiregion_truth = compact(docs + "\n" + audit + "\n" + read(ROOT / "ci/ai-blaise/operator-multiregion-contracts-smoke.sh"))
for phrase in (
    "ci/ai-blaise/operator-multiregion-contracts-smoke.sh",
    "RegionalRowPlacementPlan",
    "live_k8s_exercised=false",
    "GeoIP pool routing",
    "regional failover",
    "MR9 is production-ready for the bounded two-region drill",
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


# Bundle1 is production-ready for the bundle1-final-light source-build subset.
# Keep the subset tied to structured manifest/lock/smoke evidence so it cannot
# drift into a prose-only production claim.
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
    "full-bundle-required-minus-plrust",
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
# After Bundle1 promotion (2026-05-26), the only forbidden language is
# overclaim of plrust/full bundle. The production-ready phrase itself is now
# allowed because the full-bundle-required-minus-plrust evidence exists.
for pattern in (
    "full Bundle1 production evidence exists",
    "plrust PG17 source-build is supported",
    "plrust source-build subset is production-ready",
):
    if compact(pattern) in compact(bundle1_docs_truth):
        fail(f"Bundle1 docs misstate boundary: {pattern}")
if "feature: bundle1 is production-ready" not in compact(bundle1_docs_truth):
    fail("Bundle1 docs must record the FEATURE: Bundle1 is production-ready promotion")

# A10/A11 SQL-visible contract guardrail: both features are production-ready
# under the live-provider-execution-safety-validated boundary. This audit pins
# their NEW_FEATURES status so a regression to alpha would fail the gate.
entry_status = {entry["id"]: entry["status"] for entry in entries}
for feature_id in ("A10", "A11"):
    if entry_status.get(feature_id) != "production-ready":
        fail(f"{feature_id} must be production-ready once live AI SQL execution evidence is wired")

section_a10 = feature_section(docs, "A10")
section_a11 = feature_section(docs, "A11")
for phrase in (
    "live-provider-execution",
    "http+jsonb live POST",
    "production-ready",
):
    if compact(phrase) not in compact(section_a10):
        fail(f"A10 docs must record live-provider-execution evidence: {phrase}")
for phrase in (
    "live-provider-execution-safety-validated",
    "safety validator",
    "statement_timeout",
    "production-ready",
):
    if compact(phrase) not in compact(section_a11):
        fail(f"A11 docs must record safety-validated-execution evidence: {phrase}")
for phrase in (
    "A10 and A11 are production-ready",
    "live-provider-execution-safety-validated",
    "live AI SQL execution",
    "no generated-query execution",
):
    if compact(phrase) not in audit_compact:
        fail(f"PRODUCTION_READINESS_AUDIT.md must preserve AI SQL contract caveat: {phrase}")

sql_extension = read(SQL_EXTENSION)
ai_sql_smoke = read(AI_SQL_CONTRACT_SMOKE)
ci_image_workflow = read(CI_IMAGE_WORKFLOW)
makefile = read(MAKEFILE)
fdw_smoke = read(FDW_CREDENTIAL_ROTATION_SMOKE)
schema_drift_smoke = read(SCHEMA_DRIFT_LIVE_SMOKE)
sidecar_raft_smoke = read(SIDECAR_RAFT_SMOKE)
sidecar_hlc_smoke = read(SIDECAR_HLC_SMOKE)
txn_status_networked_raft_smoke = read(TXN_STATUS_NETWORKED_RAFT_SMOKE)
coordinatorless_mx_live_smoke = read(COORDINATORLESS_MX_LIVE_SMOKE)
companion_contracts = read(COMPANION_CONTRACTS)
companion_workflow = read(COMPANION_WORKFLOW)

if status_by_id.get("S4") != "production-ready":
    fail("S4 must be Status: production-ready once live Citus MX worker-entry and pool-entry evidence is wired")
section_s4 = feature_section(docs, "S4")
s4_truth = compact(
    section_s4
    + "\n"
    + audit
    + "\n"
    + coordinatorless_mx_live_smoke
    + "\n"
    + read(MAKEFILE)
    + "\n"
    + read(ROOT / "operator/src/crds/citus_cluster.rs")
    + "\n"
    + read(ROOT / "operator/src/reconcile/citus_cluster.rs")
)
for phrase in (
    "Production evidence:",
    "ci/ai-blaise/coordinatorless-mx-live-smoke.sh",
    "Citus MX metadata sync",
    "worker entry point",
    "pool proxy",
    "Custom Scan (Citus Adaptive)",
    "Task Count: 1",
    "coordinator bootstrap removal",
    "dynamic shard-aware pool routing",
    "multi-shard plan-leader execution",
    "Kubernetes reconciliation",
    "WAN or cross-region behavior",
):
    if compact(phrase) not in compact(section_s4):
        fail(f"S4 docs missing production boundary phrase: {phrase}")
for phrase in (
    "FEATURE: S4",
    "POSTGRES_HOST_AUTH_METHOD=trust",
    "citus_set_coordinator_host",
    "citus_add_node",
    "start_metadata_sync_to_node",
    "AI_BLAISE_POOL_UPSTREAM_ADDR",
    "coordinatorless_mx_live=passed",
    "operator_coordinatorless_admission_checked=true",
    "dedicated_coordinators=0",
    "citus_mx_metadata_synced=true",
    "metadata_synced_workers=2",
    "worker_entry_query_served=true",
    "worker_entry_sum=550",
    "pool_worker_entry_query_served=true",
    "pool_worker_entry_sum=550",
    "citus_adaptive_plan_observed=true",
    "citus_task_count_observed=1",
    "coordinator_reroute_observed=false",
    "coordinator_bootstrap_removed=false",
    "dynamic_shard_aware_pool_routing=false",
    "multi_shard_plan_leader_executed=false",
    "kubernetes_reconciliation_exercised=false",
    "wan_or_cross_region_exercised=false",
    "coordinatorless-mx-live-smoke:",
    "gate-close:",
):
    if compact(phrase) not in s4_truth:
        fail(f"S4 coordinator-less MX production boundary missing truth phrase: {phrase}")
for phrase in (
    "S4 coordinator-less topology mode is production-ready only for the bounded Citus MX worker-entry and pool-entry smoke",
    "does not claim coordinator bootstrap removal",
    "does not claim dynamic shard-aware pool routing",
    "does not claim multi-shard plan-leader execution",
    "does not claim Kubernetes reconciliation",
    "does not claim WAN or cross-region behavior",
):
    if compact(phrase) not in audit_compact:
        fail(f"PRODUCTION_READINESS_AUDIT.md missing S4 boundary phrase: {phrase}")

if status_by_id.get("S5") != "production-ready":
    fail("S5 must be Status: production-ready once live multi-process Raft transport evidence is wired")
section_s5 = feature_section(docs, "S5")
for phrase in (
    "Production evidence:",
    "ci/ai-blaise/sidecar-raft-smoke.sh",
    "three separate `ai_blaise_citus_sidecar_raft serve` OS processes",
    "/raft/campaign",
    "/raft/propose",
    "/raft/message",
    "/raft/status",
    "networked-placement-intent",
    "follower proposals",
    "operator-driven membership changes",
    "Citus placement synchronization",
):
    if compact(phrase) not in compact(section_s5):
        fail(f"S5 docs missing production boundary phrase: {phrase}")
for phrase in (
    "networked_raft_transport=passed",
    "start_raft_node worker-a",
    "start_raft_node worker-b",
    "start_raft_node worker-c",
    "/raft/campaign",
    "/raft/propose",
    "/raft/message",
    "/raft/status",
    "networked-placement-intent",
    "follower-should-not-commit",
    "not-a-valid-wire-message",
):
    if phrase not in sidecar_raft_smoke:
        fail(f"sidecar-raft-smoke.sh missing live network transport assertion: {phrase}")
if "sidecar-raft-smoke.sh" not in read(SIDECAR_WORKFLOW):
    fail("ci-sidecar workflow must run sidecar-raft-smoke.sh for S5 production evidence")
for phrase in (
    "S5 Raft per shard group is production-ready",
    "live multi-process HTTP transport",
    "three separate `ai_blaise_citus_sidecar_raft serve` OS processes",
    "networked-placement-intent",
    "follower proposals",
    "malformed",
    "operator-driven membership changes",
    "Citus placement synchronization",
):
    if compact(phrase) not in audit_compact:
        fail(f"PRODUCTION_READINESS_AUDIT.md missing S5 boundary phrase: {phrase}")

if status_by_id.get("S9") != "production-ready":
    fail("S9 must be Status: production-ready once live HLC follower-read gate evidence is wired")
section_s9 = feature_section(docs, "S9")
for phrase in (
    "Production evidence:",
    "ci/ai-blaise/sidecar-hlc-smoke.sh",
    "`ai_blaise_citus_sidecar_hlc serve`",
    "/clock/tick",
    "/clock/observe",
    "/closed_ts",
    "/follower_read",
    "HTTP 409",
    "unknown peers fail closed",
    "MVCC snapshot execution",
    "replica query routing",
    "planner integration",
):
    if compact(phrase) not in compact(section_s9):
        fail(f"S9 docs missing production boundary phrase: {phrase}")
for phrase in (
    "hlc_live_gate=passed",
    "/clock/tick",
    "/clock/observe",
    "/closed_ts",
    "/follower_read",
    "reject_not_closed",
    "unknown HLC peer",
    "AI_BLAISE_HLC_PEERS",
    "AI_BLAISE_HLC_MAX_OFFSET_MS",
):
    if phrase not in sidecar_hlc_smoke:
        fail(f"sidecar-hlc-smoke.sh missing live HLC assertion: {phrase}")
if "sidecar-hlc-smoke.sh" not in read(SIDECAR_WORKFLOW):
    fail("ci-sidecar workflow must run sidecar-hlc-smoke.sh for S9 production evidence")
if "sidecar-hlc-smoke:" not in makefile or "ci/ai-blaise/sidecar-hlc-smoke.sh" not in makefile:
    fail("Makefile.ai-blaise must expose sidecar-hlc-smoke target for S9 production evidence")
for phrase in (
    "S9 closed-timestamp follower-read gating is production-ready",
    "`ai_blaise_citus_sidecar_hlc serve`",
    "/clock/tick",
    "/clock/observe",
    "/closed_ts",
    "/follower_read",
    "HTTP 409",
    "unknown peers fail closed",
    "MVCC snapshot execution",
    "replica query routing",
    "planner integration",
):
    if compact(phrase) not in audit_compact:
        fail(f"PRODUCTION_READINESS_AUDIT.md missing S9 boundary phrase: {phrase}")

if status_by_id.get("MR6") != "production-ready":
    fail("MR6 must be Status: production-ready once live HLC time-travel gate evidence is wired")
section_mr6 = feature_section(docs, "MR6")
for phrase in (
    "Production evidence:",
    "ci/ai-blaise/sidecar-hlc-smoke.sh",
    "`ai_blaise_citus_sidecar_hlc serve`",
    "/clock/tick",
    "/clock/observe",
    "/closed_ts",
    "/follower_read",
    "HTTP 409",
    "unknown peers fail closed",
    "closed_timestamp_time_travel_gate=passed",
    "follower_read_as_of_closed_served=true",
    "follower_read_newer_than_closed_rejected=true",
    "closed_ts_peer_exchange_observed=true",
    "MVCC snapshot execution",
    "replica query routing",
    "stale-read SQL syntax",
    "planner integration",
    "Kubernetes reconciliation",
):
    if compact(phrase) not in compact(section_mr6):
        fail(f"MR6 docs missing production boundary phrase: {phrase}")
for phrase in (
    "FEATURE: S9/MR6",
    "hlc_live_gate=passed",
    "closed_timestamp_time_travel_gate=passed",
    "follower_read_as_of_closed_served=true",
    "follower_read_newer_than_closed_rejected=true",
    "unknown_peer_rejected=true",
    "closed_ts_peer_exchange_observed=true",
    "/clock/tick",
    "/clock/observe",
    "/closed_ts",
    "/follower_read",
    "reject_not_closed",
    "unknown HLC peer",
):
    if phrase not in sidecar_hlc_smoke:
        fail(f"sidecar-hlc-smoke.sh missing live MR6 assertion: {phrase}")
for phrase in (
    "MR6 closed-timestamp time-travel gate is production-ready",
    "/closed_ts",
    "/clock/tick",
    "/clock/observe",
    "exact-closed `AS OF` follower-read serving",
    "newer-than-closed HTTP 409 rejection",
    "unknown-peer fail-closed behavior",
    "closed_timestamp_time_travel_gate=passed",
    "follower_read_as_of_closed_served=true",
    "follower_read_newer_than_closed_rejected=true",
    "closed_ts_peer_exchange_observed=true",
    "MVCC snapshot execution",
    "replica query routing",
    "stale-read SQL syntax",
    "planner integration",
    "SQL/MVCC execution",
):
    if compact(phrase) not in audit_compact:
        fail(f"PRODUCTION_READINESS_AUDIT.md missing MR6 boundary phrase: {phrase}")


if status_by_id.get("Edge1") != "production-ready":
    fail("Edge1 must be Status: production-ready once live edge-read HLC gate evidence is wired")
section_edge1 = feature_section(docs, "Edge1")
for phrase in (
    "Production evidence:",
    "ci/ai-blaise/sidecar-hlc-smoke.sh",
    "`ai_blaise_citus_sidecar_hlc serve`",
    "AI_BLAISE_HLC_EDGE_REPLICAS",
    "/closed_ts",
    "/clock/tick",
    "/clock/observe",
    "/edge_read",
    "HTTP 409",
    "edge_bounded_staleness_gate=passed",
    "edge_read_as_of_closed_served=true",
    "edge_read_newer_than_closed_rejected=true",
    "edge_read_too_stale_rejected=true",
    "edge_read_replica_mismatch_rejected=true",
    "edge_unknown_region_rejected=true",
    "Edge replica provisioning",
    "POP/WAN network deployment",
    "SQL/MVCC snapshot execution",
    "planner integration",
    "data-plane query routing",
    "failover automation",
    "Kubernetes traffic",
):
    if compact(phrase) not in compact(section_edge1):
        fail(f"Edge1 docs missing production boundary phrase: {phrase}")
for phrase in (
    "FEATURE: S9/MR6/Edge1",
    "FEATURE: Edge1",
    "AI_BLAISE_HLC_EDGE_REPLICAS",
    "/edge_read",
    "edge_bounded_staleness_gate=passed",
    "edge_read_as_of_closed_served=true",
    "edge_read_newer_than_closed_rejected=true",
    "edge_read_too_stale_rejected=true",
    "edge_read_replica_mismatch_rejected=true",
    "edge_unknown_region_rejected=true",
    "edge_replica_provisioning_exercised=false",
    "edge_kubernetes_traffic_exercised=false",
    "reject_too_stale",
    "reject_replica_mismatch",
    "unknown edge region",
):
    if phrase not in sidecar_hlc_smoke:
        fail(f"sidecar-hlc-smoke.sh missing live Edge1 assertion: {phrase}")
if "EdgeReadPlan" not in read(ROOT / "sidecar/hlc/src/lib.rs") or "EdgeReadDecision" not in read(ROOT / "sidecar/hlc/src/lib.rs"):
    fail("sidecar HLC lib must expose Edge1 edge-read plan/decision types")
if "edge_read_decision" not in read(ROOT / "sidecar/hlc/src/runtime.rs"):
    fail("sidecar HLC runtime must expose Edge1 edge_read_decision")
if '\"/edge_read\"' not in read(ROOT / "sidecar/hlc/src/main.rs"):
    fail("sidecar HLC HTTP server must expose Edge1 /edge_read route")
for phrase in (
    "Edge1 bounded-staleness edge read gating is production-ready",
    "`ai_blaise_citus_sidecar_hlc serve`",
    "AI_BLAISE_HLC_EDGE_REPLICAS",
    "/closed_ts",
    "/clock/tick",
    "/clock/observe",
    "/edge_read",
    "HTTP 409",
    "edge_bounded_staleness_gate=passed",
    "edge_read_too_stale_rejected=true",
    "edge_read_replica_mismatch_rejected=true",
    "edge_unknown_region_rejected=true",
    "edge replica provisioning",
    "POP/WAN network deployment",
    "SQL/MVCC snapshot execution",
    "data-plane query routing",
    "Kubernetes traffic",
):
    if compact(phrase) not in audit_compact:
        fail(f"PRODUCTION_READINESS_AUDIT.md missing Edge1 boundary phrase: {phrase}")


if status_by_id.get("Edge2") != "production-ready":
    fail("Edge2 must be Status: production-ready once fail-closed libsql research guard evidence is wired")
section_edge2 = feature_section(docs, "Edge2")
edge2_truth = compact(
    section_edge2
    + "\n"
    + audit
    + "\n"
    + read(EDGE2_LIBSQL_GUARD_SMOKE)
    + "\n"
    + read(EDGE2_LIBSQL_ADR)
    + "\n"
    + read(ROOT / "companion/src/advanced_planner.rs")
    + "\n"
    + companion_contracts
    + "\n"
    + companion_workflow
    + "\n"
    + makefile
)
for phrase in (
    "Production evidence:",
    "docs/ai-blaise/ADR/0009-libsql-read-tier-research-guard.md",
    "run-libsql-read-tier-guard-canonical",
    "edge2-libsql-research-guard-smoke.sh",
    "edge2_libsql_research_guard_smoke",
    "guard_status=fail-closed",
    "libsql production read tier",
    "promotion evidence requirements",
    "forbidden runtime claims",
    "live_execution_claims=0",
    "replication_adapter_claimed=false",
    "workload_isolation_claimed=false",
    "production_query_routing_claimed=false",
    "libsql read-tier integration",
    "libsql replication adapter",
    "workload isolation",
    "production query routing to libsql",
    "operator reconciliation",
    "Kubernetes traffic",
):
    if compact(phrase) not in edge2_truth:
        fail(f"Edge2 libsql research guard production boundary missing phrase: {phrase}")
for phrase in (
    "cargo test -q -p ai_blaise_citus_companion edge2_libsql_research_guard_is_fail_closed",
    "run-libsql-read-tier-guard-canonical",
    "Edge2	fail-closed",
    "live_execution_claims=0",
    "replication_adapter_claimed=false",
    "workload_isolation_claimed=false",
    "production_query_routing_claimed=false",
):
    if phrase not in read(EDGE2_LIBSQL_GUARD_SMOKE):
        fail(f"Edge2 libsql guard smoke missing assertion: {phrase}")
if "edge2-libsql-research-guard-smoke.sh" not in companion_workflow:
    fail("ci-companion workflow must run edge2-libsql-research-guard-smoke.sh")
if "companion-edge2-libsql-research-guard-smoke" not in makefile:
    fail("Makefile.ai-blaise must wire companion-edge2-libsql-research-guard-smoke")

if status_by_id.get("T5") != "production-ready":
    fail("T5 must be Status: production-ready once networked txn-status Raft evidence is wired")
section_t5 = feature_section(docs, "T5")
for phrase in (
    "Production evidence:",
    "ci/ai-blaise/txn-status-networked-raft-smoke.sh",
    "three separate `ai_blaise_citus_sidecar_raft serve` OS processes",
    "`ai_blaise_citus_sidecar_txn_status serve`",
    "AI_BLAISE_TXN_RAFT_LEADER_ADDR",
    "stage:txn-live-raft-1:worker-a",
    "commit:txn-live-raft-1",
    "follower-backed replication failures fail closed",
    "Citus distributed executor",
    "PostgreSQL-core commit timestamp patches",
    "Kubernetes operator wiring",
):
    if compact(phrase) not in compact(section_t5):
        fail(f"T5 docs missing production boundary phrase: {phrase}")
for phrase in (
    "txn_status_networked_raft=passed",
    "AI_BLAISE_TXN_RAFT_LEADER_ADDR",
    "start_raft_node worker-a",
    "start_raft_node worker-b",
    "start_raft_node worker-c",
    "start_txn_status txn-status-leader",
    "start_txn_status txn-status-follower",
    "/raft/campaign",
    "stage:txn-live-raft-1:worker-a",
    "commit:txn-live-raft-1",
    "follower_replication_failure=fail_closed",
):
    if phrase not in txn_status_networked_raft_smoke:
        fail(f"txn-status-networked-raft-smoke.sh missing live assertion: {phrase}")
for phrase in (
    "txn-status-networked-raft-smoke:",
    "ci/ai-blaise/txn-status-networked-raft-smoke.sh",
    "gate-close:",
    "txn-status-networked-raft-smoke",
):
    if phrase not in makefile:
        fail(f"Makefile.ai-blaise must wire txn-status networked Raft smoke: {phrase}")
if "txn-status-networked-raft-smoke.sh" not in read(SIDECAR_WORKFLOW):
    fail("ci-sidecar workflow must run txn-status-networked-raft-smoke.sh")
for phrase in (
    "T5 parallel commit transaction status is production-ready",
    "`ai_blaise_citus_sidecar_txn_status serve`",
    "AI_BLAISE_TXN_RAFT_LEADER_ADDR",
    "stage:txn-live-raft-1:worker-a",
    "commit:txn-live-raft-1",
    "follower-backed replication failures fail closed",
    "Citus distributed executor integration",
    "PostgreSQL-core commit timestamp patch integration",
    "Kubernetes operator reconciliation",
):
    if compact(phrase) not in audit_compact:
        fail(f"PRODUCTION_READINESS_AUDIT.md missing T5 boundary phrase: {phrase}")

if status_by_id.get("F4") != "production-ready":
    fail("F4 must be Status: production-ready once live postgres_fdw rotation evidence is wired")
section_f4 = feature_section(docs, "F4")
for phrase in (
    "Production evidence:",
    "ci/ai-blaise/fdw-credential-rotation-live-smoke.sh",
    "old_password_rejected=true",
    "new_password_succeeded=true",
    "plan_secret_literals=false",
    "postgres_fdw_disconnect_all()",
    "Managed secret backends",
    "Kubernetes `ExternalSecret`",
):
    if compact(phrase) not in compact(section_f4):
        fail(f"F4 docs missing production boundary phrase: {phrase}")
for phrase in (
    "old_password_rejected=true",
    "new_password_succeeded=true",
    "plan_secret_literals=false",
    "postgres_fdw_disconnect_all",
    "ALTER USER MAPPING FOR CURRENT_USER",
    "fdw_new_password",
    "CREATE EXTENSION postgres_fdw",
):
    if phrase not in fdw_smoke:
        fail(f"fdw-credential-rotation-live-smoke.sh missing live assertion: {phrase}")
for phrase in (
    "run-fdw-credential-rotation-canonical",
    "run-fdw-credential-rotation-sql-canonical",
    "canonical_fdw_credential_rotation_report",
    "canonical_fdw_credential_rotation_sql_plan",
):
    if phrase not in companion_contracts:
        fail(f"companion_contracts missing FDW rotation command: {phrase}")
for phrase in (
    "fdw-credential-rotation-live-smoke:",
    "ci/ai-blaise/fdw-credential-rotation-live-smoke.sh",
    "gate-close:",
    "fdw-credential-rotation-live-smoke",
):
    if phrase not in makefile:
        fail(f"Makefile.ai-blaise must wire FDW rotation smoke: {phrase}")
if "fdw-credential-rotation-live-smoke.sh" not in companion_workflow:
    fail("ci-companion workflow must run fdw-credential-rotation-live-smoke.sh")
for phrase in (
    "F4 production evidence",
    "old_password_rejected=true",
    "new_password_succeeded=true",
    "plan_secret_literals=false",
    "managed secret backend",
    "Kubernetes `ExternalSecret`",
):
    if compact(phrase) not in audit_compact:
        fail(f"PRODUCTION_READINESS_AUDIT.md missing F4 boundary phrase: {phrase}")

if status_by_id.get("M4") != "production-ready":
    fail("M4 must be Status: production-ready once live schema drift evidence is wired")
section_m4 = feature_section(docs, "M4")
for phrase in (
    "Production evidence:",
    "ci/ai-blaise/schema-drift-live-smoke.sh",
    "missing_column",
    "type_mismatch",
    "nullability_mismatch",
    "unexpected_column",
    "clean_schema_zero_drift=true",
    "information_schema.columns",
    "Remediation planning",
    "operator apply behavior",
):
    if compact(phrase) not in compact(section_m4):
        fail(f"M4 docs missing production boundary phrase: {phrase}")
for phrase in (
    "CREATE TEMP TABLE ai_blaise_expected_schema_columns",
    "information_schema.columns",
    "missing_column=true",
    "type_mismatch=true",
    "nullability_mismatch=true",
    "unexpected_column=true",
    "clean_schema_zero_drift=true",
):
    if phrase not in schema_drift_smoke:
        fail(f"schema-drift-live-smoke.sh missing live assertion: {phrase}")
for phrase in (
    "run-schema-drift-canonical",
    "run-schema-drift-sql-canonical",
    "canonical_schema_drift_report",
    "canonical_schema_drift_sql_plan",
):
    if phrase not in companion_contracts:
        fail(f"companion_contracts missing schema drift command: {phrase}")
for phrase in (
    "schema-drift-live-smoke:",
    "ci/ai-blaise/schema-drift-live-smoke.sh",
    "gate-close:",
    "schema-drift-live-smoke",
):
    if phrase not in makefile:
        fail(f"Makefile.ai-blaise must wire schema drift smoke: {phrase}")
if "schema-drift-live-smoke.sh" not in companion_workflow:
    fail("ci-companion workflow must run schema-drift-live-smoke.sh")
for phrase in (
    "M4 production evidence",
    "missing_column",
    "type_mismatch",
    "nullability_mismatch",
    "unexpected_column",
    "clean_schema_zero_drift=true",
    "remediation planning",
    "operator apply behavior",
):
    if compact(phrase) not in audit_compact:
        fail(f"PRODUCTION_READINESS_AUDIT.md missing M4 boundary phrase: {phrase}")

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
    "live-provider-execution",
    "live-provider-execution-safety-validated",
):
    if phrase not in sql_extension:
        fail(f"AI SQL extension contract missing phrase: {phrase}")
for phrase in (
    "provider_runtime_available",
    "secret_bound",
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
graphql_pggraphql_live_smoke = read(GRAPHQL_PGGRAPHQL_LIVE_SMOKE)
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

for feature_id in ("API1", "API2", "API3", "API5", "API6"):
    if status_by_id.get(feature_id) != "production-ready":
        fail(f"{feature_id} must be production-ready after live API data-plane evidence")
for required in (
    "graphql_pggraphql_live=passed",
    "AI_BLAISE_GRAPHQL_LIVE_EXECUTION=1",
    "pg_graphql",
    "public.account",
    "account_tenant",
    "graphql.resolve",
    "tenant_a_rows=1",
    "tenant_b_rows=1",
    "rls_cross_tenant_hidden=true",
    "graphql_resolve_executed=true",
    "database_url not in raw",
    "jwt_secret not in raw",
):
    if required not in graphql_pggraphql_live_smoke:
        fail(f"live pg_graphql smoke lost API3 assertion: {required}")
api3_body = compact(entry_by_id["API3"]["body"])
for phrase in (
    "production evidence",
    "graphql-pggraphql-live-smoke.sh",
    "AI_BLAISE_GRAPHQL_LIVE_EXECUTION=1",
    "pg_graphql",
    "public.account",
    "graphql.resolve",
    "PostgreSQL RLS",
    "database URL/JWT secret material is absent",
    "durable GraphQL subscription fan-out",
    "multi-worker GraphQL planning",
    "Kubernetes traffic",
):
    if compact(phrase) not in api3_body:
        fail(f"API3 docs lost live pg_graphql data-plane phrase: {phrase}")
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
    "API3 has separate live `pg_graphql` execution evidence",
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
    "API3 has separate live `pg_graphql` query execution evidence",
    "does not claim GraphQL OpenAPI generation",
):
    if compact(phrase) not in api6_body:
        fail(f"API6 docs lost bounded production evidence phrase: {phrase}")

if status_by_id.get("EF1") != "production-ready":
    fail("EF1 inline Deno runtime must be production-ready after live Deno process smoke evidence")
if status_by_id.get("EF4") != "production-ready":
    fail("EF4 database callback over UDS must be production-ready after live PostgreSQL UDS smoke evidence")
if status_by_id.get("EF5") != "production-ready":
    fail("EF5 trigger dispatch must be production-ready after live scheduled/CDC Deno dispatch evidence")
if status_by_id.get("EF2") != "production-ready":
    fail("EF2 Bun runtime must be production-ready after live Bun process smoke evidence")
edge_deno_live_smoke = read(EDGE_DENO_LIVE_SMOKE)
for required in (
    "FEATURE: EF1",
    "EF5",
    "AI_BLAISE_EDGE_RUNTIME_EXECUTION",
    "AI_BLAISE_DENO_BIN",
    "edge_deno_live=passed",
    "user_code_executed=true",
    "runtime_response_contains=deno-live",
    "runtime_default_env_permission=permission_denied",
    "live_mode_without_executor_rejected=true",
    "live_timeout_rejected=true",
    "live_stdout_cap_rejected=true",
    "trigger_dispatch_scheduled_live=true",
    "trigger_dispatch_cdc_live=true",
):
    if required not in edge_deno_live_smoke:
        fail(f"EF1 live Deno smoke lost production assertion: {required}")
for path, required in (
    (MAKEFILE, "edge-functions-deno-live-smoke"),
    (SIDECAR_WORKFLOW, "edge-functions-deno-live-smoke.sh"),
):
    if required not in read(path):
        fail(f"EF1 live Deno smoke is not wired into {path}: {required}")
edge_bun_live_smoke = read(EDGE_BUN_LIVE_SMOKE)
for required in (
    "FEATURE: EF2",
    "AI_BLAISE_EDGE_RUNTIME_EXECUTION",
    "AI_BLAISE_BUN_BIN",
    "edge_bun_live=passed",
    "bun_version=",
    "user_code_executed=true",
    "runtime_response_contains=bun-live",
    "runtime_env_cleared=true",
    "live_mode_without_executor_rejected=true",
    "live_timeout_rejected=true",
    "live_stdout_cap_rejected=true",
    "trigger_dispatch_scheduled_live=true",
    "trigger_dispatch_cdc_live=true",
):
    if required not in edge_bun_live_smoke:
        fail(f"EF2 live Bun smoke lost production assertion: {required}")
for path, required in (
    (MAKEFILE, "edge-functions-bun-live-smoke"),
    (SIDECAR_WORKFLOW, "edge-functions-bun-live-smoke.sh"),
):
    if required not in read(path):
        fail(f"EF2 live Bun smoke is not wired into {path}: {required}")
ef2_body = compact(entry_by_id["EF2"]["body"])
for phrase in (
    "production evidence",
    "edge-functions-bun-live-smoke.sh",
    "AI_BLAISE_EDGE_RUNTIME_EXECUTION=1",
    "AI_BLAISE_BUN_BIN",
    "status=executed",
    "execution_mode=live",
    "user_code_executed=true",
    "runtime_response_json",
    "child environment is cleared",
    "HTTP 504",
    "runtime stdout cap",
    "scheduled and CDC trigger dispatch",
    "explicit opt-in inline Bun execution",
    "package installation",
    "bundle URI/Git source fetch",
    "Kubernetes deployment",
):
    if compact(phrase) not in ef2_body:
        fail(f"EF2 docs lost live Bun evidence phrase: {phrase}")
ef1_body = compact(entry_by_id["EF1"]["body"])
for phrase in (
    "production evidence",
    "edge-functions-deno-live-smoke.sh",
    "AI_BLAISE_EDGE_RUNTIME_EXECUTION=1",
    "AI_BLAISE_DENO_BIN",
    "status=executed",
    "execution_mode=live",
    "user_code_executed=true",
    "runtime_response_json",
    "environment access is denied",
    "HTTP 504",
    "explicit opt-in inline Deno execution",
    "Bun execution",
    "EF5 production boundary",
    "Kubernetes deployment",
):
    if compact(phrase) not in ef1_body:
        fail(f"EF1 docs lost live Deno evidence phrase: {phrase}")
ef5_body = compact(entry_by_id["EF5"]["body"])
for phrase in (
    "production evidence",
    "edge-functions-deno-live-smoke.sh",
    "POST /triggers/scheduled",
    "POST /triggers/cdc",
    "public.edge_orders insert",
    "matched=1",
    "dispatched=1",
    "execution_mode=live",
    "user_code_executed=true",
    "runtime_response_json",
    "sidecar-owned trigger ingress and dispatch",
    "Queue/broker integration",
    "long-running CDC slot tailing",
    "durable retry/DLQ",
    "Kubernetes deployment",
):
    if compact(phrase) not in ef5_body:
        fail(f"EF5 docs lost live trigger dispatch evidence phrase: {phrase}")
edge_db_callback_smoke = read(EDGE_DB_CALLBACK_UDS_SMOKE)
for required in (
    "FEATURE: EF4",
    "POSTGRES_IMAGE",
    "postgres:17",
    ".s.PGSQL.5432",
    "AI_BLAISE_EDGE_DB_CALLBACK_EXECUTION",
    "disabled_executor_rejected=true",
    "unsafe_statement_rejected=true",
    "db_callback_statement_executed=true",
    "db_callback_rows=1",
    "inserted_rows=1",
):
    if required not in edge_db_callback_smoke:
        fail(f"EF4 UDS callback smoke lost production assertion: {required}")
ef4_body = compact(entry_by_id["EF4"]["body"])
for phrase in (
    "production evidence",
    "edge-functions-db-callback-uds-smoke.sh",
    "postgres:17",
    ".s.PGSQL.5432",
    "db_callback_socket",
    "AI_BLAISE_EDGE_DB_CALLBACK_EXECUTION=1",
    "unsafe multi-statement callback SQL",
    "db_callback_statement_executed=true",
    "db_callback_rows=1",
    "sidecar-owned PostgreSQL UDS callback executor",
    "Bun DB-callback integration",
    "separate EF1/EF2 production boundaries",
    "explicit opt-in inline Deno execution",
    "EF5 covers sidecar-owned trigger dispatch",
    "Kubernetes deployment",
):
    if compact(phrase) not in ef4_body:
        fail(f"EF4 docs lost live UDS callback evidence phrase: {phrase}")
audit_body = compact(read(AUDIT))
for phrase in (
    "edge-functions-deno-live-smoke.sh",
    "AI_BLAISE_EDGE_RUNTIME_EXECUTION=1",
    "user_code_executed=true",
    "runtime_response_json",
    "HTTP 504",
    "/triggers/scheduled",
    "/triggers/cdc",
    "edge-functions-db-callback-uds-smoke.sh",
    "AI_BLAISE_EDGE_DB_CALLBACK_EXECUTION=1",
    "db_callback_rows=1",
    "edge-functions-bun-live-smoke.sh",
    "runtime_env_cleared=true",
    "live CDC slot tailing",
):
    if compact(phrase) not in audit_body:
        fail(f"production audit lost EF4 UDS callback boundary phrase: {phrase}")

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

if status_by_id.get("O14") != "production-ready":
    fail("O14 trace-context propagation must be production-ready after pool, SQL, sidecar, and Jaeger evidence")
otel_trace_smoke = read(ROOT / "ci/ai-blaise/otel-trace-propagation-smoke.sh")
for phrase in (
    "CREATE EXTENSION ai_blaise_citus;",
    "companion_current_traceparent",
    "companion_current_tracestate",
    "companion_projected",
    "companion_projected_traceparent",
    "companion_projected_tracestate",
    "companion_invalid_projected",
    "ai_blaise_citus_sidecar_shared -- serve",
    "/tracez",
    "shared sidecar did not project trace headers through /tracez",
    "shared sidecar did not report absent trace headers through /tracez",
    "AI_BLAISE_RELEASE_MODE",
    "REQUIRE_DOCKER=1 fail closed",
    "resourceSpans",
    "http://jaeger:4318/v1/traces",
    "http://jaeger:16686/api/traces/${trace_id}",
    "pool.trace_tap",
    "synthetic-jaeger-correlation-harness",
    "automatic pool/companion/sidecar span",
):
    if phrase not in otel_trace_smoke:
        fail(f"O14 trace propagation smoke lost required phrase: {phrase}")
o14_body = compact(entry_by_id["O14"]["body"])
for phrase in (
    "production evidence",
    "companion.current_traceparent",
    "companion.current_tracestate",
    "companion.project_traceparent_from_application_name",
    "/tracez",
    "trace-context extraction, propagation, SQL projection, sidecar ingress visibility, and Jaeger correlation harness evidence",
    "not automatic OTLP span export",
    "not a production dashboard/SLO certification",
    "not a claim that every business endpoint emits child spans",
):
    if compact(phrase) not in o14_body:
        fail(f"O14 docs lost production evidence or boundary phrase: {phrase}")
install_sql = read(SQL_EXTENSION)
transition_sql = read(ROOT / "images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0--0.1.1.sql")
for label, sql in (("install", install_sql), ("upgrade", transition_sql)):
    for phrase in (
        "FUNCTION companion.current_traceparent()",
        "FUNCTION companion.current_tracestate()",
        "FUNCTION companion.project_traceparent_from_application_name",
        "FUNCTION companion_current_traceparent()",
        "FUNCTION companion_current_tracestate()",
    ):
        if phrase not in sql:
            fail(f"O14 {label} SQL lost trace propagation function: {phrase}")
sql_extension_smoke_o14 = read(SQL_EXTENSION_SMOKE)
for phrase in (
    "O14 companion.current_traceparent",
    "companion.project_traceparent_from_application_name",
    "O14 invalid traceparent projection did not fail closed",
):
    if phrase not in sql_extension_smoke_o14:
        fail(f"O14 SQL extension smoke lost companion projection proof: {phrase}")
shared_runtime = read(ROOT / "sidecar/shared/src/runtime.rs")
for phrase in (
    "FEATURE: O14",
    '"/tracez"',
    "fn trace_response",
    "fn trace_json",
    "tracez_response_reports_trace_context",
):
    if phrase not in shared_runtime:
        fail(f"O14 shared sidecar runtime lost /tracez evidence: {phrase}")
image_check = read(IMAGE_CHECK)
for phrase in (
    "companion.current_traceparent()",
    "companion.project_traceparent_from_application_name",
    "ai_blaise_citus--0.1.0--0.1.1.sql",
):
    if phrase not in image_check:
        fail(f"O14 image packaging check lost trace SQL guard: {phrase}")
makefile_o14 = read(MAKEFILE)
if (
    "otel-trace-propagation-smoke:" not in makefile_o14
    or "gate-close:" not in makefile_o14
    or "otel-trace-propagation-smoke" not in makefile_o14.split("gate-close:", 1)[1]
):
    fail("gate-close must run otel-trace-propagation-smoke")
observability_workflow_o14 = read(OBSERVABILITY_WORKFLOW)
for phrase in (
    "Smoke trace propagation",
    'REQUIRE_DOCKER: "1"',
    "otel-trace-propagation-smoke.sh",
):
    if phrase not in observability_workflow_o14:
        fail(f"observability workflow lost O14 runtime proof: {phrase}")
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


bulk_distsql_live_smoke = read(BULK_DISTSQL_LIVE_SMOKE)
for feature_id in ("T10", "T11"):
    if status_by_id.get(feature_id) != "production-ready":
        fail(f"{feature_id} must be production-ready once live bulk/DistSQL evidence is wired")
section_t10 = feature_section(docs, "T10")
section_t11 = feature_section(docs, "T11")
bulk_distsql_truth = compact(
    section_t10
    + "\n"
    + section_t11
    + "\n"
    + audit
    + "\n"
    + bulk_distsql_live_smoke
    + "\n"
    + read(ROOT / "companion/src/bulk_distsql.rs")
    + "\n"
    + read(COMPANION_CONTRACTS)
    + "\n"
    + read(MAKEFILE)
)
for phrase in (
    "**Status**: production-ready",
    "ci/ai-blaise/bulk-distsql-live-smoke.sh",
    "FETCH 4096",
    "bulk_fetch_rows_returned=4096",
    "custom PostgreSQL wire-protocol",
    "backpressure",
    "cross-worker streaming fanout",
    "Kubernetes traffic",
):
    if compact(phrase) not in compact(section_t10):
        fail(f"T10 docs missing production boundary phrase: {phrase}")
for phrase in (
    "**Status**: production-ready",
    "Custom Scan (Citus Adaptive)",
    "citus_task_count_observed=1",
    "worker_task_budget=16",
    "physical plan rewrite engine",
    "multi-worker fanout",
    "Kubernetes traffic",
):
    if compact(phrase) not in compact(section_t11):
        fail(f"T11 docs missing production boundary phrase: {phrase}")
for phrase in (
    "FEATURE: T10",
    "FEATURE: T11",
    "BulkDistSqlPlan",
    "run-bulk-distsql-canonical",
    "run-bulk-distsql-sql-canonical",
    "bulk_distsql_live=passed",
    "bulk_fetch_rows_requested=4096",
    "bulk_fetch_rows_returned=4096",
    "distsql_physical_pushdown_explain=true",
    "citus_adaptive_plan_observed=true",
    "citus_task_count_observed=1",
    "worker_task_budget=16",
    "worker_task_budget_exceeded=false",
    "wire_protocol_implementation=false",
    "backpressure_scheduler_exercised=false",
    "physical_plan_rewrite_exercised=false",
    "multi_worker_fanout_exercised=false",
    "kubernetes_traffic_exercised=false",
    "bulk-distsql-live-smoke:",
    "gate-close:",
):
    if compact(phrase) not in bulk_distsql_truth:
        fail(f"T10/T11 live bulk/DistSQL production boundary missing truth phrase: {phrase}")
for phrase in (
    "T10` and",
    "T11` now also have bounded live Citus SQL evidence",
    "bulk_fetch_rows_returned=4096",
    "citus_adaptive_plan_observed=true",
    "worker_task_budget=16",
    "wire-protocol implementation",
    "adaptive backpressure",
    "optimizer rewrite engine",
    "multi-worker fanout",
    "Kubernetes traffic",
):
    if compact(phrase) not in audit_compact:
        fail(f"PRODUCTION_READINESS_AUDIT.md missing T10/T11 boundary phrase: {phrase}")


timescale_advanced_live_smoke = read(TIMESCALE_ADVANCED_LIVE_SMOKE)
for feature_id in ("TS10", "TS11"):
    if status_by_id.get(feature_id) != "production-ready":
        fail(f"{feature_id} must be production-ready once live Timescale advanced evidence is wired")
section_ts10 = feature_section(docs, "TS10")
section_ts11 = feature_section(docs, "TS11")
timescale_advanced_truth = compact(
    section_ts10
    + "\n"
    + section_ts11
    + "\n"
    + audit
    + "\n"
    + timescale_advanced_live_smoke
    + "\n"
    + read(ROOT / "companion/src/timescale_advanced.rs")
    + "\n"
    + read(COMPANION_CONTRACTS)
    + "\n"
    + read(MAKEFILE)
)
for phrase in (
    "**Status**: production-ready",
    "ci/ai-blaise/timescale-advanced-live-smoke.sh",
    "refresh_continuous_aggregate",
    "hierarchical_cagg_count=2",
    "hierarchical_cagg_daily_rows=4",
    "automated refresh scheduling",
    "multi-worker fanout",
    "Kubernetes traffic",
):
    if compact(phrase) not in compact(section_ts10):
        fail(f"TS10 docs missing production boundary phrase: {phrase}")
for phrase in (
    "**Status**: production-ready",
    "timescaledb.compress_segmentby",
    "compression_segmentby_columns=2",
    "segmentby_bloom_rows=16",
    "segmentby_bloom_bit_count=2048",
    "native Timescale bloom filters",
    "planner integration",
    "compressed-chunk scan pruning",
    "Kubernetes traffic",
):
    if compact(phrase) not in compact(section_ts11):
        fail(f"TS11 docs missing production boundary phrase: {phrase}")
for phrase in (
    "FEATURE: TS10",
    "FEATURE: TS11",
    "TimescaleAdvancedPlan",
    "run-timescale-advanced-canonical",
    "run-timescale-advanced-sql-canonical",
    "timescale_advanced_live=passed",
    "hierarchical_cagg_count=2",
    "hierarchical_cagg_daily_rows=4",
    "compression_segmentby_columns=2",
    "compression_segmentby_detail=tenant_id,device_id",
    "segmentby_bloom_rows=16",
    "segmentby_bloom_bit_count=2048",
    "segmentby_bloom_hash_count=3",
    "native_timescale_bloom_filter=false",
    "planner_integration_exercised=false",
    "compressed_chunk_scan_pruning_exercised=false",
    "multi_worker_fanout_exercised=false",
    "kubernetes_traffic_exercised=false",
    "timescale-advanced-live-smoke:",
    "gate-close:",
):
    if compact(phrase) not in timescale_advanced_truth:
        fail(f"TS10/TS11 live Timescale advanced production boundary missing truth phrase: {phrase}")
for phrase in (
    "TS10` and",
    "TS11` now have bounded live Citus+Timescale",
    "hierarchical_cagg_count=2",
    "segmentby_bloom_rows=16",
    "native Timescale bloom filters",
    "planner integration",
    "compressed-chunk scan pruning",
    "multi-worker fanout",
    "Kubernetes traffic",
):
    if compact(phrase) not in audit_compact:
        fail(f"PRODUCTION_READINESS_AUDIT.md missing TS10/TS11 boundary phrase: {phrase}")


shard_split_live_smoke = read(SHARD_SPLIT_LIVE_SMOKE)
if status_by_id.get("S1") != "production-ready":
    fail("S1 must be production-ready once live shard split evidence is wired")
section_s1 = feature_section(docs, "S1")
shard_split_truth = compact(
    section_s1
    + "\n"
    + audit
    + "\n"
    + shard_split_live_smoke
    + "\n"
    + read(ROOT / "companion/src/shard_split.rs")
    + "\n"
    + read(COMPANION_CONTRACTS)
    + "\n"
    + read(MAKEFILE)
)
for phrase in (
    "**Status**: production-ready",
    "ci/ai-blaise/shard-split-live-smoke.sh",
    "wal_level=logical",
    "isolate_tenant_to_new_shard",
    "split_shard_count_before=4",
    "split_shard_count_after=6",
    "split_tenant_rows_preserved=10",
    "split_isolated_range_exact=true",
    "policy scheduler",
    "threshold telemetry",
    "rollback automation",
    "multi-node movement",
    "Kubernetes traffic",
):
    if compact(phrase) not in compact(section_s1):
        fail(f"S1 docs missing production boundary phrase: {phrase}")
for phrase in (
    "FEATURE: S1",
    "ShardSplitPlan",
    "run-shard-split-canonical",
    "run-shard-split-sql-canonical",
    "shard_split_live=passed",
    "isolate_tenant_to_new_shard_executed=true",
    "wal_level_logical_required=true",
    "split_tenant_id=4",
    "split_shard_count_before=4",
    "split_shard_count_after=6",
    "split_new_shard_created=true",
    "split_tenant_rows_preserved=10",
    "split_tenant_shard_changed=true",
    "split_isolated_range_exact=true",
    "policy_scheduler_exercised=false",
    "threshold_telemetry_exercised=false",
    "rollback_automation_exercised=false",
    "multi_node_movement_exercised=false",
    "kubernetes_traffic_exercised=false",
    "shard-split-live-smoke:",
    "gate-close:",
):
    if compact(phrase) not in shard_split_truth:
        fail(f"S1 live shard split production boundary missing truth phrase: {phrase}")
for phrase in (
    "S1` now has bounded live Citus shard-split evidence",
    "wal_level=logical",
    "split_shard_count_before=4",
    "split_shard_count_after=6",
    "split_tenant_rows_preserved=10",
    "split_tenant_shard_changed=true",
    "split_isolated_range_exact=true",
    "policy scheduler",
    "threshold telemetry",
    "rollback automation",
    "multi-node movement",
    "Kubernetes traffic",
):
    if compact(phrase) not in audit_compact:
        fail(f"PRODUCTION_READINESS_AUDIT.md missing S1 boundary phrase: {phrase}")


clone_node_live_smoke = read(CLONE_NODE_LIVE_SMOKE)
if status_by_id.get("S3") != "production-ready":
    fail("S3 must be production-ready once live clone-node evidence is wired")
section_s3 = feature_section(docs, "S3")
clone_node_truth = compact(
    section_s3
    + "\n"
    + audit
    + "\n"
    + clone_node_live_smoke
    + "\n"
    + read(ROOT / "companion/src/clone_node.rs")
    + "\n"
    + read(COMPANION_CONTRACTS)
    + "\n"
    + read(MAKEFILE)
)
for phrase in (
    "**Status**: production-ready",
    "ci/ai-blaise/clone-node-live-smoke.sh",
    "pg_basebackup",
    "citus_add_clone_node",
    "citus_promote_clone_and_rebalance",
    "clone_rows_preserved=20",
    "clone_sum_preserved=5060",
    "clone_role_after_promote=primary",
    "clone_shard_placements_after=2",
    "primary_shard_placements_after=2",
    "Kubernetes clone orchestration",
    "CSI snapshot",
    "automatic capacity policy",
    "WAN/cross-region",
    "production traffic cutover",
):
    if compact(phrase) not in compact(section_s3):
        fail(f"S3 docs missing production boundary phrase: {phrase}")
for phrase in (
    "FEATURE: S3",
    "CloneNodePlan",
    "run-clone-node-canonical",
    "run-clone-node-setup-sql-canonical",
    "run-clone-node-promote-sql-canonical",
    "clone_node_live=passed",
    "pg_basebackup_clone_in_recovery_before=true",
    "citus_add_clone_node_executed=true",
    "citus_promote_clone_and_rebalance_executed=true",
    "pg_promote_clone_recovery_after=false",
    "clone_rows_preserved=20",
    "clone_sum_preserved=5060",
    "clone_should_have_shards_after_promote=true",
    "clone_shard_placements_after=2",
    "primary_shard_placements_after=2",
    "kubernetes_clone_orchestration_exercised=false",
    "csi_snapshot_exercised=false",
    "automatic_capacity_policy_exercised=false",
    "production_traffic_cutover_exercised=false",
    "clone-node-live-smoke:",
    "gate-close:",
):
    if compact(phrase) not in clone_node_truth:
        fail(f"S3 live clone-node production boundary missing truth phrase: {phrase}")
for phrase in (
    "S3 clone-node fast scale-out is production-ready",
    "pg_basebackup",
    "citus_add_clone_node",
    "citus_promote_clone_and_rebalance",
    "clone_rows_preserved=20",
    "clone_sum_preserved=5060",
    "clone_role_after_promote=primary",
    "clone_shard_placements_after=2",
    "primary_shard_placements_after=2",
    "Kubernetes clone orchestration",
    "CSI snapshot",
    "automatic capacity policy",
    "WAN/cross-region",
    "production traffic cutover",
):
    if compact(phrase) not in audit_compact:
        fail(f"PRODUCTION_READINESS_AUDIT.md missing S3 boundary phrase: {phrase}")

pool_routing_smoke = read(POOL_ROUTING_SECURITY_SMOKE)
pool_geoip_live_smoke = read(POOL_GEOIP_LIVE_SMOKE)
for required in (
    "mirror_decision_bucket",
    "htap_fail_closed_rejections",
    "geo_invalid_cidr_rejections",
    "tls_key_fingerprint_len",
    "pool-routing-security-smoke ok",
):
    if required not in pool_routing_smoke:
        fail(f"pool routing/security smoke lost required assertion: {required}")


if status_by_id.get("MR5") != "production-ready":
    fail("MR5 must be production-ready once live pool GeoIP routing evidence is wired")
section_mr5 = feature_section(docs, "MR5")
mr5_truth = compact(section_mr5 + "\n" + audit + "\n" + pool_geoip_live_smoke + "\n" + read(ROOT / "pool/src/proxy.rs") + "\n" + read(MAKEFILE))
for phrase in (
    "Production evidence:",
    "ci/ai-blaise/pool-geoip-live-smoke.sh",
    "AI_BLAISE_POOL_GEO_DEFAULT_REGION=us-east-1",
    "AI_BLAISE_POOL_GEO_RULES=127.0.0.0/8=us-east-1",
    "AI_BLAISE_POOL_GEO_REPLICAS",
    "geoip_pool_route_selected_region=us-east-1",
    "geoip_pool_fallback_region=us-east-1",
    "ai_blaise_citus_pool_geo_routes_total",
    "ai_blaise_citus_pool_geo_fallback_routes_total",
    "invalid CIDR fails closed",
    "managed MaxMind DB loading",
    "Region-CR synchronization",
    "hot-swap reloads",
    "cross-region/WAN traffic behavior",
    "edge-replica traffic",
    "Kubernetes traffic",
):
    if compact(phrase) not in compact(section_mr5):
        fail(f"MR5 docs missing production boundary phrase: {phrase}")
for phrase in (
    "FEATURE: MR5",
    "AI_BLAISE_POOL_GEO_DEFAULT_REGION",
    "AI_BLAISE_POOL_GEO_RULES",
    "AI_BLAISE_POOL_GEO_REPLICAS",
    "PoolGeoRoutingConfig",
    "route_upstream",
    "pool_geoip_live=passed",
    "geoip_pool_route_selected_region=us-east-1",
    "geoip_pool_fallback_region=us-east-1",
    "geoip_live_routes_total=1",
    "geoip_live_fallback_routes_total=1",
    "geoip_invalid_cidr_fail_closed=true",
    "managed_maxmind_db_loaded=false",
    "region_cr_synchronization=false",
    "hot_swap_reload_exercised=false",
    "cross_region_wan_exercised=false",
    "edge_replica_traffic_exercised=false",
    "kubernetes_traffic_exercised=false",
    "pool-geoip-live-smoke:",
    "gate-close:",
):
    if compact(phrase) not in mr5_truth:
        fail(f"MR5 live GeoIP routing production boundary missing truth phrase: {phrase}")
for phrase in (
    "MR5 now has a bounded live data-plane proof",
    "geoip_pool_route_selected_region=us-east-1",
    "ai_blaise_citus_pool_geo_routes_total",
    "managed GeoIP databases",
    "Region-CR synchronization",
    "hot-swap reloads",
    "cross-region/WAN behavior",
    "edge-replica traffic",
    "Kubernetes traffic",
):
    if compact(phrase) not in audit_compact:
        fail(f"PRODUCTION_READINESS_AUDIT.md missing MR5 boundary phrase: {phrase}")

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
    "FEATURE: A9 Sec7 Sec8",
    "external-secrets/external-secrets",
    "SEC78_ESO_CHART_VERSION",
    "0.10.7",
    "SecretStore",
    "provider:",
    "fake:",
    "ExternalSecret",
    "ai-blaise-vector-provider-openai",
    "/providers/openai/api-key",
    "vector_provider_secret_binding",
    "literal_manifest=false",
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

if status_by_id.get("A9") != "production-ready":
    fail("A9 must be production-ready after live vector-provider ExternalSecret reconciliation evidence")
a9_body = compact(entry_by_id["A9"]["body"])
for phrase in (
    "Production evidence",
    "security-external-secrets-tls-live-smoke.sh",
    "ai-blaise-vector-provider-openai",
    "External Secrets Operator chart `0.10.7`",
    "fake-provider `ExternalSecret`",
    "runtime ServiceAccount is denied Secret API reads",
    "does not claim cloud provider authentication",
    "provider credential rotation",
):
    if compact(phrase) not in a9_body:
        fail(f"A9 docs lost live proof/boundary phrase: {phrase}")
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
    "release-hardening-runbook-smoke",
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
if "release-hardening-runbook-smoke.sh" not in production_workflow:
    fail("ci-production-readiness workflow must run release-hardening-runbook-smoke.sh")
if "canary-upgrade-rollback-smoke.sh" not in production_workflow:
    fail("ci-production-readiness workflow must run canary-upgrade-rollback-smoke.sh")

if status_by_id.get("D10") != "production-ready":
    fail("D10 release hardening runbook must be production-ready after fail-closed release-record smoke evidence")
if status_by_id.get("D9") != "production-ready":
    fail("D9 canary upgrade runbook must be production-ready after live companion SQL upgrade/rollback smoke evidence")
canary_upgrade_smoke = read(CANARY_UPGRADE_SMOKE)
for phrase in (
    "FEATURE: D9",
    "ALTER EXTENSION ai_blaise_citus UPDATE TO '0.1.1'",
    "ALTER EXTENSION ai_blaise_citus UPDATE TO '0.1.0'",
    "companion_internal.record_extension_upgrade_event",
    "companion_extension_upgrade_events",
    "version_after_rollback",
    "AI_BLAISE_RELEASE_MODE",
    "event_table_after_rollback",
    "event_function_after_rollback",
    "canary_upgrade_rollback_smoke",
):
    if phrase not in canary_upgrade_smoke:
        fail(f"D9 canary upgrade rollback smoke lost assertion: {phrase}")
if "canary-upgrade-rollback-smoke:" not in makefile:
    fail("Makefile.ai-blaise must expose canary-upgrade-rollback-smoke")
if "canary-upgrade-rollback-smoke" not in makefile.split("gate-close:", 1)[1]:
    fail("gate-close must run canary-upgrade-rollback-smoke")
manifest = read(UPGRADE_MANIFEST)
for phrase in (
    "0.1.0|0.1.1|upgrade",
    "0.1.1|0.1.0|downgrade",
    "ai_blaise_citus--0.1.0--0.1.1.sql",
    "ai_blaise_citus--0.1.1--0.1.0.sql",
    "not full upstream Citus matrix evidence",
):
    if phrase not in manifest:
        fail(f"D9 upgrade manifest lost reversible transition phrase: {phrase}")
d9_body = compact(entry_by_id["D9"]["body"])
for phrase in (
    "production evidence",
    "canary-upgrade-rollback-smoke.sh",
    "real `postgres:17` container",
    "ALTER EXTENSION ai_blaise_citus UPDATE TO '0.1.1'",
    "ALTER EXTENSION ai_blaise_citus UPDATE TO '0.1.0'",
    "companion_internal.record_extension_upgrade_event",
    "companion_extension_upgrade_events",
    "0.1.1 event table and recorder are removed after rollback",
    "not full upstream Citus upgrade-matrix evidence",
    "does not certify an operand image release",
    "does not perform human production promotion",
):
    if compact(phrase) not in d9_body:
        fail(f"D9 docs lost canary upgrade evidence/boundary phrase: {phrase}")
upgrade_runbook = compact(read(UPGRADE_RUNBOOK))
for phrase in (
    "canary-upgrade-rollback-smoke.sh",
    "ai_blaise_citus--0.1.0--0.1.1.sql",
    "ai_blaise_citus--0.1.1--0.1.0.sql",
    "creates the extension at `0.1.0`, upgrades to `0.1.1`",
    "downgrades to `0.1.0`",
    "does not replace upstream Citus `check-citus-upgrade`",
):
    if compact(phrase) not in upgrade_runbook:
        fail(f"upgrade runbook lost D9 canary drill phrase: {phrase}")
releasing_doc = compact(read(RELEASING))
for phrase in (
    "canary-upgrade-rollback-smoke.sh",
    "companion SQL extension upgrade and rollback versions",
):
    if compact(phrase) not in releasing_doc:
        fail(f"release docs lost D9 canary evidence phrase: {phrase}")
audit_compact_for_d9 = compact(read(AUDIT))
for phrase in (
    "D9 canary upgrade runbook is now production-ready",
    "canary-upgrade-rollback-smoke.sh",
    "installs `ai_blaise_citus` at `0.1.0`",
    "upgrades to `0.1.1`",
    "rolls back to `0.1.0`",
    "does not claim full upstream Citus upgrade-matrix evidence",
):
    if compact(phrase) not in audit_compact_for_d9:
        fail(f"PRODUCTION_READINESS_AUDIT.md lost D9 evidence phrase: {phrase}")
release_hardening_smoke = read(RELEASE_HARDENING_SMOKE)
for phrase in (
    "FEATURE: D10",
    "run-release-hardening-canonical",
    "required_gates=19",
    "release_record_fields=10",
    "production-readiness-check.sh production-release",
    "production_release_blocked=true",
    "owner_signoff_required=true",
    "rollback_evidence_required=true",
    "D10 must not be listed as a production-release blocker",
    "release_record_source_revision",
):
    if phrase not in release_hardening_smoke:
        fail(f"D10 release hardening smoke lost assertion: {phrase}")
d10_body = compact(entry_by_id["D10"]["body"])
for phrase in (
    "production evidence",
    "release-hardening-runbook-smoke.sh",
    "run-release-hardening-canonical",
    "all 19 required release gates",
    "10 required release-record fields",
    "production-readiness-check.sh production-release",
    "requires it to fail closed while alpha features remain",
    "D10 is no longer listed as the blocker",
    "source revision",
    "rollback checkpoint requirement",
    "owner signoff requirement",
    "does not claim that a release candidate has been certified",
    "D9 canary upgrade/rollback drills",
):
    if compact(phrase) not in d10_body:
        fail(f"D10 docs lost release hardening evidence phrase: {phrase}")
production_runbook = read(RUNBOOK)
for phrase in (
    "release-hardening-runbook-smoke.sh",
    "run-release-hardening-canonical",
    "production_release_block_required=true",
    "owner_signoff_required=true",
    "rollback_evidence_required=true",
    "release_block_status",
    "alpha_feature_scope",
):
    if compact(phrase) not in compact(production_runbook):
        fail(f"production runbook lost D10 release-record phrase: {phrase}")


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
analytical_alpha_ids = set()
entry_status = {entry["id"]: entry["status"] for entry in entries}
not_alpha = sorted(feature_id for feature_id in analytical_alpha_ids if entry_status.get(feature_id) != "alpha")
if not_alpha:
    fail(
        "analytical/lakehouse features without local execution evidence must remain alpha: "
        + ", ".join(not_alpha)
    )
for feature_id in ("L2", "L4"):
    if entry_status.get(feature_id) != "production-ready":
        fail(f"{feature_id} must be production-ready once local DataFusion runtime evidence is wired")
if entry_status.get("L3") != "production-ready":
    fail("L3 must be production-ready once local Parquet read evidence is wired")
if entry_status.get("L8") != "production-ready":
    fail("L8 must be production-ready once live test_decoding mirror materialization evidence is wired")
if entry_status.get("L5") != "production-ready":
    fail("L5 must be production-ready once local Iceberg snapshot metadata commit evidence is wired")
if entry_status.get("L12") != "production-ready":
    fail("L12 must be production-ready once live DuckDB extension load evidence is wired")
if entry_status.get("L6") != "production-ready":
    fail("L6 must be production-ready once local federation catalog publication evidence is wired")
analytical_truth = compact(
    docs
    + "\n"
    + audit
    + "\n"
    + read(ROOT / "sidecar/analytical/README.md")
    + "\n"
    + read(ROOT / "sidecar/analytical/src/lib.rs")
    + "\n"
    + read(ROOT / "sidecar/analytical/src/main.rs")
    + "\n"
    + read(ROOT / "sidecar/analytical/Cargo.toml")
    + "\n"
    + read(ROOT / "ci/ai-blaise/sidecar-analytical-smoke.sh")
    + "\n"
    + read(ROOT / "ci/ai-blaise/sidecar-analytical-parquet-read-smoke.sh")
    + "\n"
    + read(ROOT / "ci/ai-blaise/sidecar-analytical-iceberg-snapshot-smoke.sh")
    + "\n"
    + read(ROOT / "ci/ai-blaise/sidecar-analytical-mirror-live-smoke.sh")
    + "\n"
    + read(ROOT / "ci/ai-blaise/sidecar-analytical-duckdb-extension-live-smoke.sh")
    + "\n"
    + read(ROOT / "ci/ai-blaise/sidecar-analytical-federation-catalog-live-smoke.sh")
)
for phrase in (
    "**Status**: production-ready",
    "datafusion = \"48.0.1\"",
    "query_engine_executed=true",
    "datafusion_output_rows=2",
    "datafusion_output_total=3000",
    "projection_pushdown_executed=true",
    "filter_pushdown_executed=true",
    "limit_pushdown_executed=true",
    "local-datafusion-recordbatch-only",
    "external_io_attempted=false",
    "pg_lake",
    "object-store IO",
    "Iceberg/Parquet/Delta",
    "DuckDB",
    "MotherDuck",
    "logical-replication mirror materialization",
    "Citus planner integration",
    "Kubernetes traffic",
):
    if compact(phrase) not in analytical_truth:
        fail(f"analytical L2/L4 production boundary missing truth phrase: {phrase}")
for feature_id in analytical_alpha_ids:
    section = feature_section(docs, feature_id)
    if "**Status**: alpha" not in section:
        fail(f"{feature_id} analytical feature must remain alpha")
for phrase in ("sidecar-analytical-smoke", "ci/ai-blaise/sidecar-analytical-smoke.sh"):
    if phrase not in makefile:
        fail(f"Makefile.ai-blaise must wire the analytical smoke: {phrase}")
if "sidecar-analytical-smoke.sh" not in read(SIDECAR_WORKFLOW):
    fail("ci-sidecar workflow must run sidecar-analytical-smoke.sh")

for phrase in (
    "run-local-parquet-read-canonical",
    "sidecar-analytical-parquet-read-smoke.sh",
    "parquet = \"55.2.0\"",
    "ArrowWriter",
    "ParquetReadOptions",
    "register_parquet",
    "parquet_lakehouse_read_live=passed",
    "l3_local_parquet_file_created=true",
    "l3_datafusion_parquet_read_executed=true",
    "l3_source_rows=4",
    "l3_source_total=5500",
    "l3_datafusion_output_rows=2",
    "l3_datafusion_output_total=3000",
    "local-datafusion-parquet-file-only",
    "object_store_io_attempted=false",
    "iceberg_runtime_exercised=false",
    "delta_runtime_exercised=false",
    "pg_lake_runtime_exercised=false",
    "motherduck_session_exercised=false",
    "kubernetes_traffic_exercised=false",
    "Iceberg runtime reads",
    "Delta runtime reads",
    "object-store IO",
    "pg_lake",
    "MotherDuck",
    "Citus planner integration",
    "warehouse federation",
    "Kubernetes traffic",
):
    if compact(phrase) not in analytical_truth:
        fail(f"analytical L3 production boundary missing truth phrase: {phrase}")
for phrase in ("sidecar-analytical-parquet-read-smoke", "ci/ai-blaise/sidecar-analytical-parquet-read-smoke.sh"):
    if phrase not in makefile:
        fail(f"Makefile.ai-blaise must wire the analytical Parquet read smoke: {phrase}")
if "sidecar-analytical-parquet-read-smoke.sh" not in read(SIDECAR_WORKFLOW):
    fail("ci-sidecar workflow must run sidecar-analytical-parquet-read-smoke.sh")

for phrase in (
    "run-local-iceberg-snapshot-commit-canonical",
    "sidecar-analytical-iceberg-snapshot-smoke.sh",
    "LocalIcebergSnapshotMetadata",
    "LocalIcebergManifest",
    "atomic_write_synced",
    "sync_all",
    "iceberg_snapshot_commit_live=passed",
    "l5_local_metadata_written=true",
    "l5_local_manifest_written=true",
    "l5_current_pointer_committed=true",
    "l5_prepare_lsn_recorded=true",
    "l5_snapshot_metadata_round_tripped=true",
    "atomic_rename_used=true",
    "fsync_executed=true",
    "local-iceberg-snapshot-metadata-commit-only",
    "iceberg_catalog_commit_exercised=false",
    "object_store_io_attempted=false",
    "citus_prepare_hook_exercised=false",
    "multi_writer_conflict_detection_exercised=false",
    "warehouse_federation_exercised=false",
    "kubernetes_traffic_exercised=false",
    "live Iceberg catalog commits",
    "object-store IO",
    "Citus prepare hook",
    "multi-writer conflict detection",
    "warehouse federation",
    "Kubernetes traffic",
):
    if compact(phrase) not in analytical_truth:
        fail(f"analytical L5 production boundary missing truth phrase: {phrase}")
for phrase in ("sidecar-analytical-iceberg-snapshot-smoke", "ci/ai-blaise/sidecar-analytical-iceberg-snapshot-smoke.sh"):
    if phrase not in makefile:
        fail(f"Makefile.ai-blaise must wire the analytical Iceberg snapshot smoke: {phrase}")
if "sidecar-analytical-iceberg-snapshot-smoke.sh" not in read(SIDECAR_WORKFLOW):
    fail("ci-sidecar workflow must run sidecar-analytical-iceberg-snapshot-smoke.sh")

for phrase in (
    "run-logical-mirror-materialization-from-stdin",
    "sidecar-analytical-mirror-live-smoke.sh",
    "test_decoding",
    "pg_logical_slot_get_changes",
    "CsvReadOptions",
    "register_csv",
    ".file_extension(\".tsv\")",
    "logical_mirror_live=passed",
    "l8_test_decoding_slot_consumed=true",
    "l8_local_mirror_artifact_created=true",
    "l8_materialized_rows=3",
    "l8_materialized_total=6000",
    "l8_datafusion_mirror_query_executed=true",
    "object_store_io_attempted=false",
    "long_running_slot_tailing=false",
    "checkpoint_persistence_exercised=false",
    "kubernetes_traffic_exercised=false",
    "object-store mirror writes",
    "long-running logical-replication mirror daemon",
    "exactly-once checkpoint persistence",
    "Citus distributed mirror routing",
    "Kubernetes traffic",
):
    if compact(phrase) not in analytical_truth:
        fail(f"analytical L8 production boundary missing truth phrase: {phrase}")
for phrase in ("sidecar-analytical-mirror-live-smoke", "ci/ai-blaise/sidecar-analytical-mirror-live-smoke.sh"):
    if phrase not in makefile:
        fail(f"Makefile.ai-blaise must wire the analytical mirror live smoke: {phrase}")
if "sidecar-analytical-mirror-live-smoke.sh" not in read(SIDECAR_WORKFLOW):
    fail("ci-sidecar workflow must run sidecar-analytical-mirror-live-smoke.sh")

for phrase in (
    "run-duckdb-extension-catalog-canonical",
    "sidecar-analytical-duckdb-extension-live-smoke.sh",
    "duckdb/duckdb@sha256:ddc7ffc382dfd3f8213ac3d29435a7ce0ea4446fb3fc966a57a28d39b46174b1",
    "INSTALL httpfs",
    "LOAD httpfs",
    "INSTALL iceberg",
    "LOAD iceberg",
    "duckdb_extensions()",
    "duckdb_extension_catalog_live=passed",
    "l12_extensions_installed=2",
    "l12_extensions_loaded=2",
    "l12_duckdb_extensions_catalog_queried=true",
    "live-duckdb-container-extension-load-only",
    "pg_duckdb_runtime_exercised=false",
    "motherduck_session_exercised=false",
    "object_store_io_attempted=false",
    "extension_repository_mirror_verified=false",
    "pg_duckdb inside PostgreSQL",
    "MotherDuck cloud sessions",
    "object-store reads",
    "warehouse federation",
    "internally mirrored DuckDB extension repository",
):
    if compact(phrase) not in analytical_truth:
        fail(f"analytical L12 production boundary missing truth phrase: {phrase}")
for phrase in ("sidecar-analytical-duckdb-extension-live-smoke", "ci/ai-blaise/sidecar-analytical-duckdb-extension-live-smoke.sh"):
    if phrase not in makefile:
        fail(f"Makefile.ai-blaise must wire the DuckDB extension live smoke: {phrase}")
if "sidecar-analytical-duckdb-extension-live-smoke.sh" not in read(SIDECAR_WORKFLOW):
    fail("ci-sidecar workflow must run sidecar-analytical-duckdb-extension-live-smoke.sh")

for phrase in (
    "run-federation-catalog-publication-canonical",
    "sidecar-analytical-federation-catalog-live-smoke.sh",
    "federation_catalog_publication_live=passed",
    "l6_catalog_version=v1",
    "l6_catalog_count=4",
    "l6_federation_targets=databricks,snowflake,trino,spark",
    "l6_local_catalog_artifact_created=true",
    "l6_local_http_catalog_served=true",
    "local-federation-catalog-artifact-http-only",
    "external_warehouse_connections_attempted=false",
    "object_store_io_attempted=false",
    "catalog_auth_exercised=false",
    "live Snowflake",
    "live Trino",
    "live Spark",
    "live Databricks",
    "warehouse connections",
    "catalog authentication",
    "object-store catalog reads",
    "F3 warehouse federation",
    "Kubernetes traffic",
):
    if compact(phrase) not in analytical_truth:
        fail(f"analytical L6 production boundary missing truth phrase: {phrase}")
for phrase in ("sidecar-analytical-federation-catalog-live-smoke", "ci/ai-blaise/sidecar-analytical-federation-catalog-live-smoke.sh"):
    if phrase not in makefile:
        fail(f"Makefile.ai-blaise must wire the federation catalog live smoke: {phrase}")
if "sidecar-analytical-federation-catalog-live-smoke.sh" not in read(SIDECAR_WORKFLOW):
    fail("ci-sidecar workflow must run sidecar-analytical-federation-catalog-live-smoke.sh")

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
