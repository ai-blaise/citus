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
import pathlib
import re
import sys

ROOT = pathlib.Path(".")
AI_EXTENSION_DIR = ROOT / "images/citus-pg-overlay/extensions"
AI_MANIFEST = AI_EXTENSION_DIR / "ai_blaise_citus-upgrade-manifest.tsv"
AI_OVERLAY_CONTROL = AI_EXTENSION_DIR / "ai_blaise_citus.control"
AI_COMPANION_CONTROL = ROOT / "companion/ai_blaise_citus.control"
CITUS_CONTROL = ROOT / "src/backend/distributed/citus.control"
CITUS_BOUNDED_FROM = "14.0-1"

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
    match = re.search(r"(?m)^\s*default_version\s*=\s*'([^']+)'\s*$", text)
    if not match:
        fail(f"{path} is missing default_version")
    return match.group(1)


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


def validate_ai_blaise_manifest(default_version: str) -> tuple[int, int]:
    rows = parse_manifest()
    sql_files = set(AI_EXTENSION_DIR.glob("ai_blaise_citus--*.sql"))
    manifest_sql_files: set[pathlib.Path] = set()
    edges: dict[tuple[str, str], dict[str, str]] = {}
    installs = 0

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
            installs += 1
            if row["from_version"] != "none":
                fail(f"{AI_MANIFEST}:{line} install rows must use from_version=none")
            if row["to_version"] != default_version:
                fail(
                    f"{AI_MANIFEST}:{line} install to_version must match control "
                    f"default_version {default_version}"
                )
            expected_name = f"ai_blaise_citus--{default_version}.sql"
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
            reverse_sql_path = normalized_repo_path(row["reverse_sql_file"])
            require_file(reverse_sql_path)
            edges[(row["from_version"], row["to_version"])] = row

    if installs != 1:
        fail(f"{AI_MANIFEST} must contain exactly one install row for the control default version")

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
        if reverse_row is None:
            fail(
                f"{AI_MANIFEST}:{row['line_number']} transition "
                f"{edge[0]}->{edge[1]} lacks reverse manifest row"
            )
        if row["reverse_sql_file"] != reverse_row["sql_file"]:
            fail(
                f"{AI_MANIFEST}:{row['line_number']} reverse_sql_file must point at "
                "the reverse row sql_file"
            )

    return len(rows), len(sql_files)


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

manifest_rows, companion_sql_files = validate_ai_blaise_manifest(overlay_default)
citus_edge = validate_citus_bounded_edge()

require_contains(ROOT / "images/citus-pg-overlay/Dockerfile", "ai_blaise_citus-upgrade-manifest.tsv")
require_contains(ROOT / "images/citus-pg-overlay/README.md", "ai_blaise_citus-upgrade-manifest.tsv")
require_contains(ROOT / "ci/ai-blaise/image-check.sh", "ai_blaise_citus-upgrade-manifest.tsv")
require_contains(ROOT / "docs/ai-blaise/RUNBOOKS/upgrade.md", "ci/ai-blaise/upgrade-rollback-guardrails.sh")
require_contains(ROOT / "docs/ai-blaise/RUNBOOKS/upgrade.md", "ALTER EXTENSION ai_blaise_citus UPDATE")
require_contains(ROOT / "docs/ai-blaise/RUNBOOKS/upgrade.md", "version-skew")
require_contains(ROOT / "docs/ai-blaise/RUNBOOKS/upgrade.md", "PITR")
require_contains(ROOT / "docs/ai-blaise/RUNBOOKS/upgrade.md", "not production evidence")
require_contains(ROOT / "docs/ai-blaise/RELEASING.md", "upgrade-rollback-guardrails")

makefile = read(ROOT / "Makefile.ai-blaise")
if "upgrade-rollback-guardrails:" not in makefile:
    fail("Makefile.ai-blaise must expose upgrade-rollback-guardrails")
if "ci/ai-blaise/upgrade-rollback-guardrails.sh" not in makefile:
    fail("Makefile.ai-blaise target must call ci/ai-blaise/upgrade-rollback-guardrails.sh")
gate_close = next((line for line in makefile.splitlines() if line.startswith("gate-close:")), "")
if "upgrade-rollback-guardrails" not in gate_close:
    fail("gate-close must include upgrade-rollback-guardrails")

print(
    "upgrade_rollback_guardrails\t"
    f"ai_blaise_version={overlay_default}\t"
    f"manifest_rows={manifest_rows}\t"
    f"companion_sql_files={companion_sql_files}\t"
    f"citus_edge={citus_edge}"
)
PY
