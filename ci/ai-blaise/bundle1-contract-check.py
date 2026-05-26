#!/usr/bin/env python3
"""Fail-closed Bundle1 source-build/image contract checker.

This script intentionally validates the narrow Bundle1 evidence boundary rather
than promoting the whole feature. It cross-checks the manifest, lockfile,
Dockerfile, SQL smoke, tracked evidence file, and docs so Bundle1 cannot drift
into a prose-only production claim.
"""
from __future__ import annotations

import csv
import pathlib
import sys
from typing import Iterable

ROOT = pathlib.Path(__file__).resolve().parents[2]
IMAGE_DIR = ROOT / "images/citus-pg-overlay"
MANIFEST = IMAGE_DIR / "extension-manifest.tsv"
LOCKFILE = IMAGE_DIR / "bundle1-source-build.lock.tsv"
DOCKERFILE = IMAGE_DIR / "Dockerfile"
SQL_SMOKE = ROOT / "ci/ai-blaise/sql-extension-smoke.sh"
IMAGE_CHECK = ROOT / "ci/ai-blaise/image-check.sh"
EVIDENCE = IMAGE_DIR / "bundle1-source-build-evidence.tsv"
README = IMAGE_DIR / "README.md"
BUNDLED_DOC = ROOT / "docs/ai-blaise/BUNDLED_EXTENSIONS.md"
FEATURES_DOC = ROOT / "docs/ai-blaise/NEW_FEATURES.md"
AUDIT_DOC = ROOT / "docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md"
INITDB = IMAGE_DIR / "initdb.d/00-ai-blaise-extensions.sql"

EXPECTED_LOCK_ORDER = [
    "citus",
    "pgsodium",
    "topn",
    "pg_jsonschema",
    "pg_graphql",
    "pg_search",
    "plv8",
    "pg_warm",
    "plrust",
]
LIGHT_EXTENSIONS = {
    "ai_blaise_citus",
    "citus",
    "pgsodium",
    "topn",
    "pg_jsonschema",
    "pg_graphql",
    "pg_prewarm",
    "pg_warm",
}
# Bundle1 production-ready bar: every required-tier manifest extension whose
# control file is supplied by the bundle1-pgdg-runtime layer or the local SQL
# shims. pg_failover_slots is preload-only (no SQL extension) and plrust is
# optional/deferred upstream, so neither participates in this set.
REQUIRED_PRODUCTION_EXTENSIONS = LIGHT_EXTENSIONS | {
    "timescaledb",
    "vector",
    "pg_cron",
    "pg_partman",
    "pgaudit",
    "pgauditlogtofile",
    "hll",
    "tdigest",
    "pgnodemx",
    "postgis",
    "age",
    "pg_uuidv7",
    "pg_repack",
    "pgcrypto",
    "pg_trgm",
    "citext",
    "rum",
}
HEAVY_EXTENSIONS = LIGHT_EXTENSIONS | {"pg_search", "plv8"}


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    sys.exit(1)


def read(path: pathlib.Path) -> str:
    if not path.exists() or not path.read_text(encoding="utf-8", errors="ignore").strip():
        fail(f"missing or empty Bundle1 contract artifact: {path.relative_to(ROOT)}")
    return path.read_text(encoding="utf-8", errors="ignore")


def compact(text: str) -> str:
    return " ".join(text.split()).lower()


def parse_manifest() -> dict[str, dict[str, str]]:
    rows: dict[str, dict[str, str]] = {}
    for line_no, raw in enumerate(read(MANIFEST).splitlines(), start=1):
        if not raw or raw.startswith("#"):
            continue
        parts = raw.split("|")
        if len(parts) != 7:
            fail(f"extension manifest line {line_no} must have 7 pipe-delimited fields")
        name, tier, license_id, source, feature_ids, pg_versions, policy = parts
        if name in rows:
            fail(f"duplicate extension manifest row: {name}")
        if not feature_ids:
            fail(f"manifest row {name} must name feature_ids")
        rows[name] = {
            "tier": tier,
            "license": license_id,
            "source": source,
            "feature_ids": feature_ids,
            "pg_versions": pg_versions,
            "policy": policy,
        }
    return rows


def parse_lockfile() -> list[dict[str, str]]:
    with LOCKFILE.open(encoding="utf-8", newline="") as fh:
        rows = list(csv.DictReader(fh, delimiter="\t"))
    expected_fields = [
        "extension",
        "target",
        "manifest_source",
        "pg_versions",
        "repo",
        "tag",
        "ref",
        "docker_arg_prefix",
        "docker_stage",
        "status",
    ]
    if not rows or list(rows[0].keys()) != expected_fields:
        fail("Bundle1 source-build lockfile header changed or is empty")
    order = [row["extension"] for row in rows]
    if order != EXPECTED_LOCK_ORDER:
        fail(f"Bundle1 lockfile order changed: {order}")
    for row in rows:
        for field in expected_fields:
            if not row[field]:
                fail(f"Bundle1 lockfile row {row['extension']} has empty {field}")
    return rows


def require_all(text: str, phrases: Iterable[str], context: str) -> None:
    body = compact(text)
    for phrase in phrases:
        if compact(phrase) not in body:
            fail(f"{context} missing Bundle1 contract phrase: {phrase}")


def main() -> None:
    manifest = parse_manifest()
    lock_rows = parse_lockfile()
    dockerfile = read(DOCKERFILE)
    sql_smoke = read(SQL_SMOKE)
    image_check = read(IMAGE_CHECK)
    initdb = read(INITDB)
    docs = "\n".join(read(path) for path in (README, BUNDLED_DOC, FEATURES_DOC, AUDIT_DOC))

    for row in lock_rows:
        extension = row["extension"]
        if extension not in manifest:
            fail(f"Bundle1 lockfile extension missing from manifest: {extension}")
        manifest_row = manifest[extension]
        if manifest_row["source"] != row["manifest_source"]:
            fail(
                f"Bundle1 source mismatch for {extension}: "
                f"manifest={manifest_row['source']} lock={row['manifest_source']}"
            )
        if manifest_row["pg_versions"] != row["pg_versions"]:
            fail(
                f"Bundle1 pg_versions mismatch for {extension}: "
                f"manifest={manifest_row['pg_versions']} lock={row['pg_versions']}"
            )
        if row["tag"] not in manifest_row["policy"] and row["tag"] not in {"in-tree", "0.1.0"}:
            fail(f"manifest policy for {extension} does not mention locked tag {row['tag']}")
        if row["ref"] not in manifest_row["policy"] and row["ref"] != "in-tree":
            fail(f"manifest policy for {extension} does not mention locked ref {row['ref']}")

    for row in lock_rows:
        extension = row["extension"]
        prefix = row["docker_arg_prefix"]
        stage = row["docker_stage"]
        if row["manifest_source"] == "source-build":
            for phrase in (
                f"ARG {prefix}_TAG={row['tag']}",
                f"ARG {prefix}_REF={row['ref']}",
                f"AS {stage}",
                f'git clone --branch "${{{prefix}_TAG}}"',
                f'test "$(git rev-parse HEAD)" = "${{{prefix}_REF}}"',
                f"COPY --from={stage}",
            ):
                if phrase not in dockerfile:
                    fail(f"Dockerfile missing locked source-build contract for {extension}: {phrase}")
        elif extension == "citus":
            for phrase in (
                "ARG CITUS_TAG=v13.3.0",
                "AS build-citus",
                "COPY --from=build-citus",
                "ai-blaise.citus.source-git-sha",
                "ai-blaise.citus.source-tree-state",
            ):
                if phrase not in dockerfile:
                    fail(f"Dockerfile missing in-tree Citus source contract: {phrase}")
        elif extension == "pg_warm":
            for phrase in (
                "COPY images/citus-pg-overlay/extensions/pg_warm.control",
                "COPY images/citus-pg-overlay/extensions/pg_warm--0.1.0.sql",
            ):
                if phrase not in dockerfile:
                    fail(f"Bundle1 pg_warm local shim contract missing: {phrase}")
            if (
                "CREATE EXTENSION pg_warm;" not in sql_smoke
                and "CREATE EXTENSION IF NOT EXISTS pg_warm;" not in initdb
            ):
                fail("Bundle1 pg_warm local shim must be created by either smoke or initdb")
        elif extension == "plrust":
            for phrase in (
                "ARG PLRUST_TAG=v1.2.8",
                "ARG PLRUST_REF=bd76906a43c05a2afdb7839263431a066f5b42fb",
                "alpha-upstream-pg17-blocked",
                "source-build-deferred|EF6|none",
            ):
                if phrase not in (dockerfile + "\n" + MANIFEST.read_text(encoding="utf-8")):
                    fail(f"Bundle1 plrust deferred boundary missing: {phrase}")

    # Bundle1 production-ready: every required extension created by initdb or smoke.
    initdb_or_smoke = initdb + sql_smoke
    for extension in sorted(REQUIRED_PRODUCTION_EXTENSIONS):
        if extension == "pg_prewarm":
            continue
        if (
            f"CREATE EXTENSION {extension};" not in initdb_or_smoke
            and f"CREATE EXTENSION IF NOT EXISTS {extension};" not in initdb_or_smoke
        ):
            fail(f"Bundle1 initdb/smoke does not create required extension {extension}")
    for extension in sorted(HEAVY_EXTENSIONS - LIGHT_EXTENSIONS):
        if f"CREATE EXTENSION {extension};" not in sql_smoke:
            fail(f"Bundle1 heavy source-build smoke does not create {extension}")

    for phrase in (
        "BUNDLE1_BUILD_IMAGE",
        "BUNDLE1_BUILD_HEAVY",
        "BUNDLE1_EVIDENCE_FILE",
        "AI_BLAISE_SOURCE_GIT_SHA",
        "ai-blaise.citus.source-git-sha",
        "ai-blaise.citus.bundle1.evidence-scope",
        "full-bundle-required-minus-plrust",
        "pgsodium key unavailable",
        "PGSODIUM_KEY=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "bundle1-source-build.lock.tsv",
    ):
        if phrase not in sql_smoke + "\n" + dockerfile:
            fail(f"Bundle1 smoke/Dockerfile lost required fail-closed phrase: {phrase}")

    # pg_failover_slots has no SQL extension surface (shared_preload_libraries-only).
    preload_only_extensions = {"pg_failover_slots"}
    required_extensions = [name for name, row in manifest.items() if row["tier"] == "required"]
    for extension in required_extensions:
        if extension in preload_only_extensions:
            continue
        if f"CREATE EXTENSION IF NOT EXISTS {extension};" not in initdb:
            fail(f"initdb contract does not create required extension {extension}")
    for extension in EXPECTED_LOCK_ORDER:
        if extension == "plrust":
            continue
        if extension not in required_extensions:
            fail(f"source-build lock extension must remain required until policy changes: {extension}")

    if EVIDENCE.exists():
        with EVIDENCE.open(encoding="utf-8", newline="") as fh:
            evidence_rows = list(csv.DictReader(fh, delimiter="\t"))
        if evidence_rows and list(evidence_rows[0].keys()) != ["observed_at", "git_sha", "target", "image_id", "extensions"]:
            fail("Bundle1 evidence TSV header changed")
        light_rows = [row for row in evidence_rows if row["target"] == "bundle1-final-light"]
        if not light_rows:
            fail("Bundle1 evidence TSV must keep at least one light source-build proof row")
        latest_light_extensions = set(light_rows[-1]["extensions"].split())
        missing = REQUIRED_PRODUCTION_EXTENSIONS - latest_light_extensions
        if missing:
            fail(f"latest Bundle1 light evidence row missing required production extensions: {sorted(missing)}")
        if "plrust" in latest_light_extensions:
            fail("Bundle1 light evidence must not imply plrust PG17 support")

    require_all(
        docs,
        (
            "bundle1-source-build.lock.tsv",
            "full-bundle-required-minus-plrust",
            "structured Bundle1 contract check",
            "BUNDLE1_BUILD_IMAGE=1",
            "BUNDLE1_BUILD_HEAVY=1",
            "complete initdb path",
            "plrust PG17 upstream gap",
            "FEATURE: Bundle1 is production-ready",
        ),
        "Bundle1 docs",
    )
    require_all(
        image_check,
        (
            "bundle1-contract-check.py",
            "bundle1-source-build.lock.tsv",
            "full-bundle-required-minus-plrust",
        ),
        "image-check.sh",
    )

    for forbidden in (
        "FEATURE: Bundle1 remains alpha",
        "full Bundle1 production evidence exists",
        "plrust PG17 source-build is supported",
        "plrust source-build subset is production-ready",
    ):
        if compact(forbidden) in compact(docs):
            fail(f"Bundle1 docs misstate boundary: {forbidden}")

    print("bundle1-contract-check passed")


if __name__ == "__main__":
    main()
