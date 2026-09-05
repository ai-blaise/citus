#!/usr/bin/env bash
set -euo pipefail

# FEATURE: D9
# Static, fail-closed upgrade/rollback guardrails for the ai-blaise overlay.
# This is intentionally bounded: it validates the local extension transition
# manifest and the current upstream Citus edge instead of running the full
# upstream Citus upgrade matrix on every PR.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

python3 - <<'PY'
import csv
import hashlib
import pathlib
import re
import sys

ROOT = pathlib.Path(".")
AI_EXTENSION_DIR = ROOT / "images/citus-pg-overlay/extensions"
AI_MANIFEST = AI_EXTENSION_DIR / "ai_blaise_citus-upgrade-manifest.tsv"
AI_OVERLAY_CONTROL = AI_EXTENSION_DIR / "ai_blaise_citus.control"
AI_COMPANION_CONTROL = ROOT / "companion/ai_blaise_citus.control"
CANARY_UPGRADE_SMOKE = ROOT / "ci/ai-blaise/canary-upgrade-rollback-smoke.sh"
OPERATOR_VERSION_SOURCE = ROOT / "operator/src/crds/citus_cluster.rs"
OPERATOR_RECONCILE_SOURCE = ROOT / "operator/src/reconcile/citus_cluster.rs"
OPERATOR_CONTROLLER_SOURCE = ROOT / "operator/src/controllers/citus_cluster.rs"
OPERATOR_PRODUCTION_DOC = ROOT / "operator/CITUS_CLUSTER_PRODUCTION.md"
CITUS_CONTROL = ROOT / "src/backend/distributed/citus.control"
CITUS_BOUNDED_FROM = "14.0-1"
FROZEN_INSTALL_ROOT_VERSION = "0.1.0"
FROZEN_INSTALL_ROOT_SHA256 = "c23c0887753118915c12b40ee6058ddd8920d95c33258353448c68b4e6c0ddb5"

EXPECTED_MANIFEST_COLUMNS = [
    "extension",
    "from_version",
    "to_version",
    "direction",
    "sql_file",
    "reverse_sql_file",
    "rollback_contract",
    "version_skew_contract",
    "evidence_boundary",
]
VALID_DIRECTIONS = {"install", "upgrade", "downgrade"}
FORWARD_ONLY_SECURITY_EDGE = ("0.1.1", "0.1.2")
DEFAULT_VERSION_SMOKES = [
    ROOT / "ci/ai-blaise/ai-sql-contract-smoke.sh",
]
REAL_CITUS_DEFAULT_VERSION_SMOKES = [
    ROOT / "ci/ai-blaise/a10-a11-ai-sql-live-smoke.sh",
    ROOT / "ci/ai-blaise/migration-invariants-smoke.sh",
    ROOT / "ci/ai-blaise/observability-replication-smoke.sh",
    ROOT / "ci/ai-blaise/otel-trace-propagation-smoke.sh",
    ROOT / "ci/ai-blaise/schema-job-f1-2vi-smoke.sh",
]
REAL_CITUS_TIMESCALE_DEFAULT_VERSION_SMOKES = [
    ROOT / "ci/ai-blaise/timescale-bridge-smoke.sh",
]
REAL_CITUS_FIXTURE_DOCKERFILE = ROOT / "images/citus-test-fixture/Dockerfile"
COHABITATION_DOCKERFILES = [
    ROOT / "images/citus-pg-cron-cohabitation/Dockerfile",
    ROOT / "images/citus-timescale-cohabitation/Dockerfile",
]
SHIPPED_IMAGE_VERSION_SMOKES = [
    ROOT / "ci/ai-blaise/operator-hypertable-live-smoke.sh",
    ROOT / "ci/ai-blaise/operator-reconcilers-batch-c-smoke.sh",
    ROOT / "ci/ai-blaise/pg-cron-cohabitation-smoke.sh",
    ROOT / "ci/ai-blaise/timescale-advanced-live-smoke.sh",
    ROOT / "ci/ai-blaise/timescale-cohabitation-smoke.sh",
    ROOT / "tests/e2e/kind-timescale-citus-smoke.sh",
]


class UpgradeGraphError(ValueError):
    pass


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    sys.exit(1)


def read(path: pathlib.Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError:
        fail(f"missing required upgrade/rollback artifact: {path}")


def require_file(path: pathlib.Path) -> None:
    if not path.is_file() or path.stat().st_size == 0:
        fail(f"missing or empty upgrade/rollback artifact: {path}")


def require_contains(path: pathlib.Path, needle: str) -> None:
    if needle not in read(path):
        fail(f"{path} must contain: {needle}")


def control_default_version(path: pathlib.Path) -> str:
    text = read(path)
    matches = re.findall(r"(?m)^\s*default_version\s*=\s*'([^']+)'\s*$", text)
    if len(matches) != 1:
        fail(f"{path} must contain exactly one default_version")
    return matches[0]


def operator_shipped_companion_version(path: pathlib.Path) -> str:
    text = read(path)
    matches = re.findall(
        r'(?m)^pub\(crate\) const SHIPPED_COMPANION_EXTENSION_VERSION: &str = "([^"]+)";$',
        text,
    )
    if len(matches) != 1:
        fail(f"{path} must contain exactly one SHIPPED_COMPANION_EXTENSION_VERSION")
    return matches[0]


def shell_double_quoted_assignment(path: pathlib.Path, name: str) -> str:
    matches = re.findall(
        rf'(?m)^{re.escape(name)}="([^"]+)"$',
        read(path),
    )
    if len(matches) != 1:
        fail(f"{path} must assign {name} exactly once with a literal value")
    return matches[0]


def normalized_repo_path(value: str) -> pathlib.Path:
    path = pathlib.Path(value)
    if path.is_absolute():
        fail(f"manifest paths must be repo-relative, got {value}")
    return path


def parse_manifest() -> list[dict[str, str]]:
    require_file(AI_MANIFEST)
    with AI_MANIFEST.open(newline="", encoding="utf-8") as manifest_file:
        reader = csv.DictReader(manifest_file, delimiter="|")
        if reader.fieldnames != EXPECTED_MANIFEST_COLUMNS:
            fail(
                f"{AI_MANIFEST} header must be "
                + "|".join(EXPECTED_MANIFEST_COLUMNS)
            )
        rows = []
        for line_number, row in enumerate(reader, start=2):
            clean = {key: (value or "").strip() for key, value in row.items()}
            if not any(clean.values()):
                continue
            clean["line_number"] = str(line_number)
            rows.append(clean)
    if not rows:
        fail(f"{AI_MANIFEST} must contain at least one transition row")
    return rows


def validate_upgrade_graph(
    install_version: str,
    default_version: str,
    upgrade_edges: list[tuple[str, str]],
) -> None:
    if install_version == "none" or default_version == "none":
        raise UpgradeGraphError("install and default versions must be concrete")

    adjacency: dict[str, list[str]] = {}
    nodes = {install_version}
    seen_edges: set[tuple[str, str]] = set()
    for source, target in upgrade_edges:
        edge = (source, target)
        if edge in seen_edges:
            raise UpgradeGraphError(f"duplicate upgrade edge {source}->{target}")
        seen_edges.add(edge)
        adjacency.setdefault(source, []).append(target)
        nodes.update(edge)

    visiting: set[str] = set()
    visited: set[str] = set()

    def reject_cycles(version: str) -> None:
        if version in visiting:
            raise UpgradeGraphError(f"upgrade graph contains a cycle at {version}")
        if version in visited:
            return
        visiting.add(version)
        for target in adjacency.get(version, []):
            reject_cycles(target)
        visiting.remove(version)
        visited.add(version)

    for version in sorted(nodes):
        reject_cycles(version)

    reachable = {install_version}
    pending = [install_version]
    while pending:
        source = pending.pop()
        for target in adjacency.get(source, []):
            if target not in reachable:
                reachable.add(target)
                pending.append(target)

    if default_version not in reachable:
        raise UpgradeGraphError(
            f"control default_version {default_version} is unreachable from install root "
            f"{install_version}"
        )
    unreachable = sorted(nodes - reachable)
    if unreachable:
        raise UpgradeGraphError(
            "upgrade graph contains versions unreachable from install root "
            f"{install_version}: {', '.join(unreachable)}"
        )

    terminals = sorted(
        version for version in reachable if not adjacency.get(version)
    )
    if terminals != [default_version]:
        raise UpgradeGraphError(
            f"control default_version {default_version} must be the sole terminal upgrade "
            f"version; found {', '.join(terminals) or 'none'}"
        )

    path_counts: dict[str, int] = {}

    def paths_to_default(version: str) -> int:
        if version == default_version:
            return 1
        if version in path_counts:
            return path_counts[version]
        count = sum(paths_to_default(target) for target in adjacency.get(version, []))
        path_counts[version] = min(count, 2)
        return path_counts[version]

    path_count = paths_to_default(install_version)
    if path_count != 1:
        raise UpgradeGraphError(
            f"upgrade path from {install_version} to default {default_version} "
            f"must be unique; found {path_count}"
        )


def run_upgrade_graph_regressions() -> int:
    validate_upgrade_graph("0.1.0", "0.1.1", [("0.1.0", "0.1.1")])
    rejected = [
        (
            "duplicate-edge",
            "0.1.0",
            "0.1.1",
            [("0.1.0", "0.1.1"), ("0.1.0", "0.1.1")],
            "duplicate upgrade edge",
        ),
        (
            "cycle",
            "0.1.0",
            "0.1.1",
            [("0.1.0", "0.1.1"), ("0.1.1", "0.1.0")],
            "contains a cycle",
        ),
        (
            "unreachable-default",
            "0.1.0",
            "0.2.0",
            [("0.1.0", "0.1.1")],
            "is unreachable",
        ),
        (
            "orphan-branch",
            "0.1.0",
            "0.1.1",
            [("0.1.0", "0.1.1"), ("0.2.0", "0.2.1")],
            "versions unreachable",
        ),
        (
            "ambiguous-path",
            "0.1.0",
            "0.2.0",
            [
                ("0.1.0", "0.1.1"),
                ("0.1.0", "0.1.2"),
                ("0.1.1", "0.2.0"),
                ("0.1.2", "0.2.0"),
            ],
            "must be unique",
        ),
        (
            "default-behind-terminal",
            "0.1.0",
            "0.1.1",
            [("0.1.0", "0.1.1"), ("0.1.1", "0.2.0")],
            "must be the sole terminal",
        ),
    ]
    for name, install_version, default_version, edges, expected in rejected:
        try:
            validate_upgrade_graph(install_version, default_version, edges)
        except UpgradeGraphError as error:
            if expected not in str(error):
                fail(
                    f"upgrade graph regression {name} returned unexpected error: {error}"
                )
        else:
            fail(f"upgrade graph regression {name} unexpectedly passed")
    return len(rejected)


def validate_forward_only_security_contract(edge, row, reverse_present):
    if edge != FORWARD_ONLY_SECURITY_EDGE:
        raise UpgradeGraphError("unreviewed forward-only transition")
    if row["direction"] != "upgrade" or row["reverse_sql_file"] != "none" or reverse_present:
        raise UpgradeGraphError("security floor must not have a downgrade edge")
    rollback = row["rollback_contract"].lower()
    if not all(phrase in rollback for phrase in (
        "forward-only security floor", "no in-place downgrade", "pre-upgrade backup", "pitr", "separate cluster"
    )):
        raise UpgradeGraphError("security floor requires an explicit isolated backup/PITR rollback contract")


def run_forward_only_regressions():
    valid = {
        "direction": "upgrade", "reverse_sql_file": "none",
        "rollback_contract": "Forward-only security floor; no in-place downgrade; pre-upgrade backup/PITR into a separate cluster",
    }
    validate_forward_only_security_contract(FORWARD_ONLY_SECURITY_EDGE, valid, False)
    for edge, row, reverse_present in (
        (("0.1.2", "0.1.3"), valid, False),
        (FORWARD_ONLY_SECURITY_EDGE, {**valid, "direction": "downgrade"}, False),
        (FORWARD_ONLY_SECURITY_EDGE, {**valid, "reverse_sql_file": "unsafe.sql"}, False),
        (FORWARD_ONLY_SECURITY_EDGE, {**valid, "rollback_contract": "none"}, False),
        (FORWARD_ONLY_SECURITY_EDGE, valid, True),
    ):
        try:
            validate_forward_only_security_contract(edge, row, reverse_present)
        except UpgradeGraphError:
            continue
        fail("unsafe forward-only regression unexpectedly passed")
    return 5


def validate_ai_blaise_manifest(default_version: str) -> tuple[int, int, str]:
    rows = parse_manifest()
    sql_files = set(AI_EXTENSION_DIR.glob("ai_blaise_citus--*.sql"))
    manifest_sql_files: set[pathlib.Path] = set()
    edges: dict[tuple[str, str], dict[str, str]] = {}
    install_versions: list[str] = []
    upgrade_edges: list[tuple[str, str]] = []

    for row in rows:
        line = row["line_number"]
        if row["extension"] != "ai_blaise_citus":
            fail(f"{AI_MANIFEST}:{line} extension must be ai_blaise_citus")
        if row["direction"] not in VALID_DIRECTIONS:
            fail(f"{AI_MANIFEST}:{line} direction must be one of {sorted(VALID_DIRECTIONS)}")
        for field in [
            "from_version",
            "to_version",
            "sql_file",
            "rollback_contract",
            "version_skew_contract",
            "evidence_boundary",
        ]:
            if not row[field]:
                fail(f"{AI_MANIFEST}:{line} field {field} must not be empty")

        sql_path = normalized_repo_path(row["sql_file"])
        require_file(sql_path)
        if sql_path.parent != AI_EXTENSION_DIR:
            fail(f"{AI_MANIFEST}:{line} sql_file must live under {AI_EXTENSION_DIR}")
        if sql_path in manifest_sql_files:
            fail(f"{AI_MANIFEST}:{line} duplicates sql_file {sql_path}")
        manifest_sql_files.add(sql_path)

        evidence_boundary = row["evidence_boundary"].lower()
        if "not full upstream citus matrix" not in evidence_boundary:
            fail(
                f"{AI_MANIFEST}:{line} evidence_boundary must state it is not "
                "full upstream Citus matrix evidence"
            )
        if "version" not in row["version_skew_contract"].lower():
            fail(f"{AI_MANIFEST}:{line} version_skew_contract must explicitly discuss versions")

        if row["direction"] == "install":
            if row["from_version"] != "none":
                fail(f"{AI_MANIFEST}:{line} install rows must use from_version=none")
            install_versions.append(row["to_version"])
            expected_name = f"ai_blaise_citus--{row['to_version']}.sql"
            if sql_path.name != expected_name:
                fail(f"{AI_MANIFEST}:{line} install sql_file must be {expected_name}")
            if row["reverse_sql_file"] != "none":
                fail(f"{AI_MANIFEST}:{line} install rows must use reverse_sql_file=none")
        else:
            if row["from_version"] == "none" or row["to_version"] == "none":
                fail(f"{AI_MANIFEST}:{line} transition rows must use concrete versions")
            if row["from_version"] == row["to_version"]:
                fail(f"{AI_MANIFEST}:{line} transition versions must differ")
            expected_name = f"ai_blaise_citus--{row['from_version']}--{row['to_version']}.sql"
            if sql_path.name != expected_name:
                fail(f"{AI_MANIFEST}:{line} transition sql_file must be {expected_name}")
            if row["reverse_sql_file"] != "none":
                reverse_sql_path = normalized_repo_path(row["reverse_sql_file"])
                require_file(reverse_sql_path)
            edge = (row["from_version"], row["to_version"])
            if edge in edges:
                fail(
                    f"{AI_MANIFEST}:{line} duplicates transition edge "
                    f"{edge[0]}->{edge[1]}"
                )
            edges[edge] = row
            if row["direction"] == "upgrade":
                upgrade_edges.append(edge)

    if len(install_versions) != 1:
        fail(f"{AI_MANIFEST} must contain exactly one install root row")
    install_version = install_versions[0]
    if install_version != FROZEN_INSTALL_ROOT_VERSION:
        fail(
            f"{AI_MANIFEST} install root must remain the frozen released version "
            f"{FROZEN_INSTALL_ROOT_VERSION}; add a versioned transition instead of "
            "rewriting release history"
        )
    install_path = AI_EXTENSION_DIR / f"ai_blaise_citus--{install_version}.sql"
    install_sha256 = hashlib.sha256(install_path.read_bytes()).hexdigest()
    if install_sha256 != FROZEN_INSTALL_ROOT_SHA256:
        fail(
            f"{install_path} changed after release-history freeze: "
            f"expected sha256={FROZEN_INSTALL_ROOT_SHA256}, got sha256={install_sha256}; "
            "restore it and add a new extension version plus transition"
        )

    missing_from_manifest = sorted(str(path) for path in sql_files - manifest_sql_files)
    if missing_from_manifest:
        fail(
            "ai_blaise_citus SQL files missing from upgrade manifest: "
            + ", ".join(missing_from_manifest)
        )

    extra_in_manifest = sorted(str(path) for path in manifest_sql_files - sql_files)
    if extra_in_manifest:
        fail("upgrade manifest references non-extension SQL files: " + ", ".join(extra_in_manifest))

    for edge, row in edges.items():
        reverse = (edge[1], edge[0])
        reverse_row = edges.get(reverse)
        if row["reverse_sql_file"] == "none" or edge == FORWARD_ONLY_SECURITY_EDGE:
            try:
                validate_forward_only_security_contract(edge, row, reverse_row is not None)
            except UpgradeGraphError as error:
                fail(f"{AI_MANIFEST}:{row['line_number']} {error}")
            continue
        if reverse_row is None:
            fail(
                f"{AI_MANIFEST}:{row['line_number']} transition "
                f"{edge[0]}->{edge[1]} lacks reverse manifest row"
            )
        expected_reverse_direction = (
            "downgrade" if row["direction"] == "upgrade" else "upgrade"
        )
        if reverse_row["direction"] != expected_reverse_direction:
            fail(
                f"{AI_MANIFEST}:{row['line_number']} reverse transition must use "
                f"direction={expected_reverse_direction}"
            )
        if row["reverse_sql_file"] != reverse_row["sql_file"]:
            fail(
                f"{AI_MANIFEST}:{row['line_number']} reverse_sql_file must point at "
                "the reverse row sql_file"
            )

    try:
        validate_upgrade_graph(install_version, default_version, upgrade_edges)
    except UpgradeGraphError as error:
        fail(f"{AI_MANIFEST} has an invalid release upgrade graph: {error}")

    return len(rows), len(sql_files), install_version


def validate_citus_bounded_edge() -> str:
    citus_default = control_default_version(CITUS_CONTROL)
    upgrade = ROOT / f"src/backend/distributed/sql/citus--{CITUS_BOUNDED_FROM}--{citus_default}.sql"
    downgrade = ROOT / f"src/backend/distributed/sql/downgrades/citus--{citus_default}--{CITUS_BOUNDED_FROM}.sql"
    require_file(upgrade)
    require_file(downgrade)
    require_contains(upgrade, f"-- citus--{CITUS_BOUNDED_FROM}--{citus_default}")
    require_contains(downgrade, f"-- citus--{citus_default}--{CITUS_BOUNDED_FROM}")
    return f"{CITUS_BOUNDED_FROM}->{citus_default}"


overlay_default = control_default_version(AI_OVERLAY_CONTROL)
companion_default = control_default_version(AI_COMPANION_CONTROL)
if overlay_default != companion_default:
    fail(
        "ai_blaise_citus control default_version mismatch: "
        f"overlay={overlay_default} companion={companion_default}"
    )

operator_version = operator_shipped_companion_version(OPERATOR_VERSION_SOURCE)
if operator_version != overlay_default:
    fail(
        "operator shipped companion version must match control default_version: "
        f"operator={operator_version} control={overlay_default}"
    )

graph_regressions = run_upgrade_graph_regressions()
forward_only_regressions = run_forward_only_regressions()
manifest_rows, companion_sql_files, install_root = validate_ai_blaise_manifest(
    overlay_default
)
citus_edge = validate_citus_bounded_edge()

require_contains(ROOT / "images/citus-pg-overlay/Dockerfile", "ai_blaise_citus-upgrade-manifest.tsv")
require_contains(ROOT / "images/citus-pg-overlay/README.md", "ai_blaise_citus-upgrade-manifest.tsv")
require_contains(ROOT / "ci/ai-blaise/image-check.sh", "ai_blaise_citus-upgrade-manifest.tsv")
require_contains(ROOT / "docs/ai-blaise/RUNBOOKS/upgrade.md", "ci/ai-blaise/upgrade-rollback-guardrails.sh")
require_contains(ROOT / "docs/ai-blaise/RUNBOOKS/upgrade.md", "ALTER EXTENSION ai_blaise_citus UPDATE")
require_contains(ROOT / "docs/ai-blaise/RUNBOOKS/upgrade.md", "version-skew")
require_contains(ROOT / "docs/ai-blaise/RUNBOOKS/upgrade.md", "PITR")
require_contains(ROOT / "docs/ai-blaise/RUNBOOKS/upgrade.md", "not production evidence")
require_contains(ROOT / "docs/ai-blaise/RUNBOOKS/upgrade.md", "canary-upgrade-rollback-smoke.sh")
require_contains(
    ROOT / "docs/ai-blaise/RUNBOOKS/upgrade.md",
    f"sha256:{FROZEN_INSTALL_ROOT_SHA256}",
)
require_contains(
    ROOT / "docs/ai-blaise/RUNBOOKS/upgrade.md",
    "future SQL changes require a new versioned file and transition",
)
require_contains(ROOT / "docs/ai-blaise/RELEASING.md", "upgrade-rollback-guardrails")
require_contains(ROOT / "docs/ai-blaise/RELEASING.md", "canary-upgrade-rollback-smoke")
require_contains(
    OPERATOR_RECONCILE_SOURCE,
    "companion: SHIPPED_COMPANION_EXTENSION_VERSION.to_string()",
)
require_contains(
    OPERATOR_CONTROLLER_SOURCE,
    "companion: SHIPPED_COMPANION_EXTENSION_VERSION.to_string()",
)
require_contains(OPERATOR_PRODUCTION_DOC, f"companion: {overlay_default}")

for smoke in DEFAULT_VERSION_SMOKES:
    require_contains(smoke, "ai_blaise_citus--0.1.0--0.1.1.sql")
    require_contains(smoke, "ai_blaise_citus--0.1.1--0.1.0.sql")
    require_contains(smoke, "ai_blaise_citus--0.1.1--0.1.2.sql")
    require_contains(smoke, f"IS DISTINCT FROM '{overlay_default}'")
    require_contains(
        smoke,
        f"expected shipped ai_blaise_citus version {overlay_default}",
    )

for smoke in REAL_CITUS_DEFAULT_VERSION_SMOKES:
    if (
        "build-real-citus-test-fixture.sh" not in read(smoke)
        and "build-real-citus-http-test-fixture.sh" not in read(smoke)
    ):
        fail(f"{smoke} must use a shared real-Citus fixture builder")
    require_contains(smoke, "real-citus-test-fixture-contract.py")
    require_contains(smoke, "CREATE EXTENSION citus;")
    require_contains(smoke, "CREATE EXTENSION pgcrypto;")
    require_contains(smoke, "CREATE EXTENSION ai_blaise_citus;")
    require_contains(smoke, f"IS DISTINCT FROM '{overlay_default}'")
    require_contains(
        smoke,
        f"expected shipped ai_blaise_citus version {overlay_default}",
    )

for smoke in REAL_CITUS_TIMESCALE_DEFAULT_VERSION_SMOKES:
    require_contains(smoke, "build-real-citus-timescale-test-fixture.sh")
    require_contains(smoke, "real-citus-timescale-test-fixture-contract.py")
    require_contains(smoke, "CREATE EXTENSION IF NOT EXISTS citus;")
    require_contains(smoke, "CREATE EXTENSION IF NOT EXISTS timescaledb;")
    require_contains(smoke, "CREATE EXTENSION IF NOT EXISTS pgcrypto;")
    require_contains(smoke, "CREATE EXTENSION IF NOT EXISTS ai_blaise_citus;")
    require_contains(smoke, f"IS DISTINCT FROM '{overlay_default}'")
    require_contains(
        smoke,
        f"expected shipped ai_blaise_citus version {overlay_default}",
    )

for filename in (
    "ai_blaise_citus--0.1.0--0.1.1.sql",
    "ai_blaise_citus--0.1.1--0.1.0.sql",
    "ai_blaise_citus--0.1.1--0.1.2.sql",
):
    require_contains(REAL_CITUS_FIXTURE_DOCKERFILE, filename)

sql_extension_smoke = ROOT / "ci/ai-blaise/sql-extension-smoke.sh"
require_contains(sql_extension_smoke, "build-real-citus-test-fixture.sh")
require_contains(sql_extension_smoke, "real-citus-test-fixture-contract.py")
require_contains(sql_extension_smoke, "CREATE EXTENSION citus;")
require_contains(sql_extension_smoke, f"<> '{overlay_default}'")
require_contains(
    sql_extension_smoke,
    f"bare CREATE EXTENSION did not install shipped default {overlay_default}",
)

for dockerfile in COHABITATION_DOCKERFILES:
    require_contains(dockerfile, "ai_blaise_citus--0.1.0.sql")
    require_contains(dockerfile, "ai_blaise_citus--0.1.0--0.1.1.sql")
    require_contains(dockerfile, "ai_blaise_citus--0.1.1--0.1.0.sql")
    require_contains(dockerfile, "ai_blaise_citus--0.1.1--0.1.2.sql")

for smoke in SHIPPED_IMAGE_VERSION_SMOKES:
    require_contains(
        smoke,
        f"expected shipped ai_blaise_citus version {overlay_default}",
    )
    if smoke == ROOT / "ci/ai-blaise/operator-reconcilers-batch-c-smoke.sh":
        require_contains(smoke, f'[[ "${{extension_version}}" != "{overlay_default}" ]]')
    else:
        require_contains(smoke, f"IS DISTINCT FROM '{overlay_default}'")


require_file(CANARY_UPGRADE_SMOKE)
canary_install_version = shell_double_quoted_assignment(
    CANARY_UPGRADE_SMOKE, "install_version"
)
canary_current_version = shell_double_quoted_assignment(
    CANARY_UPGRADE_SMOKE, "current_version"
)
if canary_install_version != install_root:
    fail(
        "canary install_version must match manifest install root: "
        f"canary={canary_install_version} manifest={install_root}"
    )
if canary_current_version != overlay_default:
    fail(
        "canary current_version must match control default_version: "
        f"canary={canary_current_version} control={overlay_default}"
    )
for phrase in (
    "FEATURE: D9",
    f"CREATE EXTENSION ai_blaise_citus VERSION '{install_root}';",
    "CREATE EXTENSION ai_blaise_citus;",
    f"CREATE EXTENSION ai_blaise_citus VERSION '{overlay_default}';",
    "ALTER EXTENSION ai_blaise_citus UPDATE;",
    "ALTER EXTENSION ai_blaise_citus UPDATE TO '0.1.1';",
    f"ALTER EXTENSION ai_blaise_citus UPDATE TO '{install_root}'",
    "pg_extension_update_paths('ai_blaise_citus')",
    "canary image major mismatch",
    "container_logs=",
    "docker rm --force --volumes",
    "companion_internal.record_extension_upgrade_event",
    "companion_extension_upgrade_events",
    "version_after_rollback",
):
    require_contains(CANARY_UPGRADE_SMOKE, phrase)
require_contains(AI_EXTENSION_DIR / "ai_blaise_citus--0.1.0--0.1.1.sql", "CREATE TABLE companion_internal.extension_upgrade_events")
require_contains(AI_EXTENSION_DIR / "ai_blaise_citus--0.1.1--0.1.0.sql", "DROP TABLE IF EXISTS companion_internal.extension_upgrade_events")
security_smoke = ROOT / "ci/ai-blaise/extension-security-backup-smoke.sh"
for phrase in (
    "extension-backup-seed.sql", "extension-backup-state.sql", "extension-security-assert.sql",
    "pg_dump", "pg_restore", "assert_upgrade_rolls_back", "before_update", "before_commit",
    "delegated", "EXECUTE WITH GRANT OPTION", "987655", "--network none",
    "build-real-citus-test-fixture.sh", "CREATE EXTENSION citus;",
):
    require_contains(security_smoke, phrase)
require_file(ROOT / "images/citus-pg-overlay/upgrades/ai_blaise_citus--0.1.2.sql")
require_contains(ROOT / ".github/workflows/ci-production-readiness.yml", "extension-security-backup-smoke.sh")
require_contains(ROOT / "docs/ai-blaise/RUNBOOKS/companion-security-backup.md", "pg_init_privs")

makefile = read(ROOT / "Makefile.ai-blaise")
if "upgrade-rollback-guardrails:" not in makefile:
    fail("Makefile.ai-blaise must expose upgrade-rollback-guardrails")
if "ci/ai-blaise/upgrade-rollback-guardrails.sh" not in makefile:
    fail("Makefile.ai-blaise target must call ci/ai-blaise/upgrade-rollback-guardrails.sh")
if "canary-upgrade-rollback-smoke:" not in makefile:
    fail("Makefile.ai-blaise must expose canary-upgrade-rollback-smoke")
if "ci/ai-blaise/canary-upgrade-rollback-smoke.sh" not in makefile:
    fail("Makefile.ai-blaise target must call ci/ai-blaise/canary-upgrade-rollback-smoke.sh")
gate_close = "\n".join(line for line in makefile.splitlines() if line.startswith("gate-close:"))
if "upgrade-rollback-guardrails" not in gate_close:
    fail("gate-close must include upgrade-rollback-guardrails")
if "canary-upgrade-rollback-smoke" not in gate_close:
    fail("gate-close must include canary-upgrade-rollback-smoke")
if "extension-security-backup-smoke" not in gate_close:
    fail("gate-close must include extension-security-backup-smoke")

print(
    "upgrade_rollback_guardrails\t"
    f"ai_blaise_version={overlay_default}\t"
    f"install_root={install_root}\t"
    f"install_root_sha256={FROZEN_INSTALL_ROOT_SHA256}\t"
    f"manifest_rows={manifest_rows}\t"
    f"companion_sql_files={companion_sql_files}\t"
    f"graph_regressions={graph_regressions}\t"
    f"forward_only_regressions={forward_only_regressions}\t"
    f"citus_edge={citus_edge}"
)
PY
