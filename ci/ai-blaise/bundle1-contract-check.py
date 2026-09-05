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
import re
import sys
from typing import Callable, Iterable

ROOT = pathlib.Path(__file__).resolve().parents[2]
IMAGE_DIR = ROOT / "images/citus-pg-overlay"
MANIFEST = IMAGE_DIR / "extension-manifest.tsv"
LOCKFILE = IMAGE_DIR / "bundle1-source-build.lock.tsv"
DOCKERFILE = IMAGE_DIR / "Dockerfile"
PGCORE_PATCHES_DOCKERFILE = IMAGE_DIR / "Dockerfile.pgcore-patches"
TIMESCALE_COHABITATION_DOCKERFILE = (
    ROOT / "images/citus-timescale-cohabitation/Dockerfile"
)
PG_CRON_COHABITATION_DOCKERFILE = ROOT / "images/citus-pg-cron-cohabitation/Dockerfile"
SQL_SMOKE = ROOT / "ci/ai-blaise/sql-extension-smoke.sh"
REAL_CITUS_FIXTURE_DOCKERFILE = ROOT / "images/citus-test-fixture/Dockerfile"
IMAGE_CHECK = ROOT / "ci/ai-blaise/image-check.sh"
BUILD_CITUS = ROOT / "ci/build-citus.sh"
EVIDENCE = IMAGE_DIR / "bundle1-source-build-evidence.tsv"
README = IMAGE_DIR / "README.md"
BUNDLED_DOC = ROOT / "docs/ai-blaise/BUNDLED_EXTENSIONS.md"
AUDIT_DOC = ROOT / "docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md"
INITDB = IMAGE_DIR / "initdb.d/00-ai-blaise-extensions.sql"
PRELOAD = IMAGE_DIR / "shared-preload-libraries.conf"
DEFAULT_BOOT_SMOKE = ROOT / "ci/ai-blaise/bundle1-default-boot-smoke.sh"
IMAGE_WORKFLOW = ROOT / ".github/workflows/ci-image.yml"

LIGHT_TARGET = "bundle1-final-light"
FULL_TARGET = "bundle1-final-full"
LIGHT_SCOPE = "light-required-subset-minus-heavy-and-plrust"
FULL_SCOPE = "full-bundle-required-minus-plrust"

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


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    sys.exit(1)


def read(path: pathlib.Path) -> str:
    if (
        not path.exists()
        or not path.read_text(encoding="utf-8", errors="ignore").strip()
    ):
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


def normalized_docker_commands(block: str) -> list[str]:
    commands: list[str] = []
    for raw in block.splitlines():
        command = raw.strip()
        if command.endswith("\\"):
            command = command[:-1].rstrip()
        commands.append(command)
    return commands


def validate_custom_citus_install(
    dockerfile: str, stage_marker: str, expected_command: str, context: str
) -> None:
    """Prove that a real in-tree Citus build uses the downgrade-aware target."""

    if dockerfile.count(stage_marker) != 1:
        fail(f"{context} must contain exactly one in-tree Citus build marker")
    citus_stage = dockerfile.split(stage_marker, 1)[1].split("\nFROM ", 1)[0]
    commands = normalized_docker_commands(citus_stage)
    if commands.count(expected_command) != 1:
        fail(
            f"{context} must execute exact install-all packaging so downgrade "
            "SQL is never omitted"
        )
    if any("install-all" in command and "||" in command for command in commands):
        fail(f"{context} must not fall back from install-all to install")
    if any(
        re.search(r"\bmake\s+install(?!-all)(?:\s|;|$)", command)
        for command in commands
    ):
        fail(f"{context} must not execute install without downgrade SQL")


def validate_citus_downgrade_install(dockerfile: str, build_citus: str) -> None:
    """Require every release packaging path to install downgrade SQL."""

    stage_marker = "FROM build-base AS build-citus"
    validate_custom_citus_install(
        dockerfile,
        stage_marker,
        "make install-all DESTDIR=/out;",
        "Bundle1 Citus operand",
    )

    ci_install = 'make DESTDIR="${installdir}" install-all'
    if build_citus.count(ci_install) != 1:
        fail("Citus CI packaging must require exact install-all downgrade coverage")
    if any("install-all" in line and "||" in line for line in build_citus.splitlines()):
        fail("Citus CI packaging must not fall back from install-all to install")
    if re.search(
        r'\bmake\s+DESTDIR="\$\{installdir\}"\s+install(?:\s|;|$)', build_citus
    ):
        fail("Citus CI packaging must not use install without downgrade SQL")


class ContractViolation(ValueError):
    """A target claims or covers a scope other than its derived contract."""


def required_extensions(manifest: dict[str, dict[str, str]]) -> set[str]:
    return {name for name, row in manifest.items() if row["tier"] == "required"}


def full_only_extensions(lock_rows: list[dict[str, str]]) -> set[str]:
    return {row["extension"] for row in lock_rows if row["target"] == "full"}


def target_extensions(
    manifest: dict[str, dict[str, str]],
    lock_rows: list[dict[str, str]],
    target: str,
) -> set[str]:
    required = required_extensions(manifest)
    full_only = full_only_extensions(lock_rows)
    if target == LIGHT_TARGET:
        return required - full_only
    if target == FULL_TARGET:
        return required
    raise ContractViolation(f"unknown Bundle1 target: {target}")


def validate_target_observation(
    manifest: dict[str, dict[str, str]],
    lock_rows: list[dict[str, str]],
    target: str,
    scope: str,
    covered_extensions: set[str],
) -> None:
    expected_scope = LIGHT_SCOPE if target == LIGHT_TARGET else FULL_SCOPE
    if scope != expected_scope:
        raise ContractViolation(
            f"{target} must claim {expected_scope}, observed {scope}"
        )
    missing = target_extensions(manifest, lock_rows, target) - covered_extensions
    if missing:
        raise ContractViolation(
            f"{target} is missing required entries: {sorted(missing)}"
        )


def expect_contract_violation(callback: Callable[[], None], context: str) -> None:
    try:
        callback()
    except ContractViolation:
        return
    fail(f"Bundle1 negative contract did not fail closed: {context}")


def validate_boundary_claims(docs: str, source_metadata: str) -> None:
    for forbidden in (
        "FEATURE: Bundle1 is production-ready",
        "full Bundle1 production evidence exists",
        "plrust PG17 source-build is supported",
        "plrust source-build subset is production-ready",
    ):
        if compact(forbidden) in compact(docs):
            raise ContractViolation(f"Bundle1 docs misstate boundary: {forbidden}")
    for forbidden in (
        "Bundle1 is production-ready",
        "Bundle1 production-ready evidence",
    ):
        if compact(forbidden) in compact(source_metadata):
            raise ContractViolation(
                f"Bundle1 source metadata misstates boundary: {forbidden}"
            )


def validate_trusted_preload_order(
    preloaded_libraries: list[str], trusted_cohabit_libraries: list[str]
) -> None:
    """Validate the opted-in Bundle1 hook-chain order.

    Plain upstream deployments retain Citus's normal first-load rule. Bundle1
    explicitly opts trusted coextensions into the fork's cohabitation policy,
    so those hooks must load before Citus and Citus must be the final preload.
    """

    preloaded = [library.strip().lower() for library in preloaded_libraries]
    trusted = [library.strip().lower() for library in trusted_cohabit_libraries]
    if not preloaded or any(not library for library in preloaded):
        raise ContractViolation("Bundle1 preload list must not be empty")
    if len(preloaded) != len(set(preloaded)):
        raise ContractViolation("Bundle1 preload list must not contain duplicates")
    if not trusted or any(not library for library in trusted):
        raise ContractViolation("Bundle1 trusted cohabit list must not be empty")
    missing = sorted(set(trusted) - set(preloaded))
    if missing:
        raise ContractViolation(
            f"Bundle1 preload list is missing trusted coextensions: {missing}"
        )
    if preloaded[-1] != "citus":
        raise ContractViolation(
            "Bundle1 trusted coextensions must load before a final Citus entry"
        )


def run_negative_contract_tests(
    manifest: dict[str, dict[str, str]], lock_rows: list[dict[str, str]]
) -> None:
    light = target_extensions(manifest, lock_rows, LIGHT_TARGET)
    full = target_extensions(manifest, lock_rows, FULL_TARGET)
    expect_contract_violation(
        lambda: validate_target_observation(
            manifest, lock_rows, LIGHT_TARGET, FULL_SCOPE, light
        ),
        "light target carrying full scope",
    )
    full_only = full - light
    if not full_only:
        fail("Bundle1 target lock must retain at least one full-only required entry")
    missing = sorted(full_only)[0]
    expect_contract_violation(
        lambda: validate_target_observation(
            manifest, lock_rows, FULL_TARGET, FULL_SCOPE, full - {missing}
        ),
        f"full target missing required entry {missing}",
    )
    expect_contract_violation(
        lambda: validate_trusted_preload_order(
            ["citus", "timescaledb"], ["timescaledb"]
        ),
        "trusted Bundle1 Citus-first preload",
    )
    expect_contract_violation(
        lambda: validate_trusted_preload_order(
            ["timescaledb", "citus"], ["timescaledb", "pg_cron"]
        ),
        "trusted Bundle1 missing required preload",
    )
    expect_contract_violation(
        lambda: validate_boundary_claims("", "Bundle1 is production-ready"),
        "manifest promotion claim",
    )
    expect_contract_violation(
        lambda: validate_boundary_claims("", "Bundle1 production-ready evidence"),
        "source lock promotion claim",
    )


def main() -> None:
    manifest = parse_manifest()
    lock_rows = parse_lockfile()
    dockerfile = read(DOCKERFILE)
    pgcore_patches_dockerfile = read(PGCORE_PATCHES_DOCKERFILE)
    timescale_cohabitation_dockerfile = read(TIMESCALE_COHABITATION_DOCKERFILE)
    pg_cron_cohabitation_dockerfile = read(PG_CRON_COHABITATION_DOCKERFILE)
    sql_smoke = read(SQL_SMOKE)
    real_citus_fixture_dockerfile = read(REAL_CITUS_FIXTURE_DOCKERFILE)
    image_check = read(IMAGE_CHECK)
    build_citus = read(BUILD_CITUS)
    initdb = read(INITDB)
    preload = read(PRELOAD)
    default_boot_smoke = read(DEFAULT_BOOT_SMOKE)
    image_workflow = read(IMAGE_WORKFLOW)
    docs = "\n".join(read(path) for path in (README, BUNDLED_DOC, AUDIT_DOC))
    try:
        validate_boundary_claims(docs, read(MANIFEST) + "\n" + read(LOCKFILE))
    except ContractViolation as exc:
        fail(str(exc))
    validate_citus_downgrade_install(dockerfile, build_citus)
    if "--without-pg-version-check" in dockerfile:
        fail("Bundle1 Citus build must not bypass its supported PostgreSQL version check")
    validate_custom_citus_install(
        timescale_cohabitation_dockerfile,
        "WORKDIR /build/citus",
        'make install-all with_llvm="${WITH_LLVM}";',
        "Timescale cohabitation Citus image",
    )
    validate_custom_citus_install(
        pg_cron_cohabitation_dockerfile,
        "WORKDIR /build/citus",
        "make install-all;",
        "pg_cron cohabitation Citus image",
    )
    validate_custom_citus_install(
        pgcore_patches_dockerfile,
        "WORKDIR /src/citus",
        "make install-all;",
        "patched PostgreSQL core Citus image",
    )

    required = required_extensions(manifest)
    light_required = target_extensions(manifest, lock_rows, LIGHT_TARGET)
    full_required = target_extensions(manifest, lock_rows, FULL_TARGET)
    full_only = full_only_extensions(lock_rows)
    if not full_only:
        fail("Bundle1 source-build lock must define full-only extensions")
    if not full_only <= required:
        fail(
            "Bundle1 full-only source-build entries must remain required in the manifest: "
            f"{sorted(full_only - required)}"
        )
    if full_required != required or light_required != required - full_only:
        fail("Bundle1 target extension sets are not derived from manifest plus lock")
    run_negative_contract_tests(manifest, lock_rows)

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
        if row["tag"] not in manifest_row["policy"] and row["tag"] not in {
            "in-tree",
            "0.1.0",
        }:
            fail(
                f"manifest policy for {extension} does not mention locked tag {row['tag']}"
            )
        if row["ref"] not in manifest_row["policy"] and row["ref"] != "in-tree":
            fail(
                f"manifest policy for {extension} does not mention locked ref {row['ref']}"
            )

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
                    fail(
                        f"Dockerfile missing locked source-build contract for {extension}: {phrase}"
                    )
        elif extension == "citus":
            for phrase in (
                "ARG CITUS_TAG=v13.3.0",
                "AS build-citus",
                "COPY --from=build-citus",
                "ai-blaise-citus-historical-tracking-tag",
                "ai-blaise.citus.bundle1.citus.historical-tracking-tag",
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
                fail(
                    "Bundle1 pg_warm local shim must be created by either smoke or initdb"
                )
        elif extension == "plrust":
            for phrase in (
                "ARG PLRUST_TAG=v1.2.8",
                "ARG PLRUST_REF=bd76906a43c05a2afdb7839263431a066f5b42fb",
                "alpha-upstream-pg17-blocked",
                "source-build-deferred|EF6|none",
            ):
                if phrase not in (
                    dockerfile + "\n" + MANIFEST.read_text(encoding="utf-8")
                ):
                    fail(f"Bundle1 plrust deferred boundary missing: {phrase}")

    # Every required manifest capability must be represented by initdb SQL or
    # the canonical preload contract. Target membership itself is derived from
    # the manifest plus the light/full source-build lock above.
    initdb_or_smoke = initdb + sql_smoke
    preload_match = re.search(
        r"^shared_preload_libraries = '([^']*)'$", preload, flags=re.MULTILINE
    )
    if preload_match is None:
        fail("Bundle1 preload contract does not define shared_preload_libraries")
    preloaded_order = preload_match.group(1).split(",")
    cohabit_match = re.search(
        r"^citus\.cohabit_extensions = '([^']*)'$", preload, flags=re.MULTILINE
    )
    if cohabit_match is None:
        fail("Bundle1 preload contract does not define citus.cohabit_extensions")
    trusted_cohabit_order = cohabit_match.group(1).split(",")
    try:
        validate_trusted_preload_order(preloaded_order, trusted_cohabit_order)
    except ContractViolation as exc:
        fail(str(exc))
    preloaded = {library.strip().lower() for library in preloaded_order}
    sql_created_required: set[str] = set()
    for extension in sorted(required):
        if (
            f"CREATE EXTENSION {extension};" not in initdb_or_smoke
            and f"CREATE EXTENSION IF NOT EXISTS {extension};" not in initdb_or_smoke
        ):
            if extension not in preloaded:
                fail(
                    "Bundle1 required extension has neither initdb creation nor "
                    f"preload coverage: {extension}"
                )
        else:
            sql_created_required.add(extension)
    for extension in sorted(full_only):
        if f"CREATE EXTENSION {extension};" not in sql_smoke:
            fail(f"Bundle1 heavy source-build smoke does not create {extension}")

    validate_target_observation(
        manifest, lock_rows, LIGHT_TARGET, LIGHT_SCOPE, light_required
    )
    validate_target_observation(
        manifest, lock_rows, FULL_TARGET, FULL_SCOPE, full_required
    )

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
        "postgresql.conf.sample",
        "include = '/etc/postgresql/ai-blaise/shared-preload-libraries.conf'",
    ):
        if phrase not in sql_smoke + "\n" + dockerfile:
            fail(f"Bundle1 smoke/Dockerfile lost required fail-closed phrase: {phrase}")

    require_all(
        real_citus_fixture_dockerfile,
        (
            "ai_blaise_citus--0.1.0--0.1.1.sql",
            "ai_blaise_citus--0.1.1--0.1.0.sql",
            "ai_blaise_citus--0.1.1--0.1.2.sql",
        ),
        "real-Citus fixture companion transition packaging",
    )
    require_all(
        sql_smoke,
        (
            "build-real-citus-test-fixture.sh",
            "CREATE EXTENSION citus",
            "CREATE EXTENSION ai_blaise_citus",
            "extversion FROM pg_extension",
            "shipped default 0.1.2",
        ),
        "Bundle1 companion default-version smoke",
    )
    require_all(
        dockerfile,
        (
            "COPY images/citus-pg-overlay/upgrades/ai_blaise_citus--0.1.2.sql",
            "test -s /usr/local/share/ai-blaise/citus/upgrades/ai_blaise_citus--0.1.2.sql",
        ),
        "Bundle1 transactional security-upgrade wrapper packaging",
    )

    light_marker = f"FROM bundle1-source-runtime AS {LIGHT_TARGET}"
    full_marker = f"FROM {LIGHT_TARGET} AS {FULL_TARGET}"
    contract_marker = "FROM ${BASE_IMAGE} AS bundle1-contract"
    if light_marker not in dockerfile or full_marker not in dockerfile:
        fail("Dockerfile lost explicit Bundle1 light/full target stages")
    light_stage = dockerfile.split(light_marker, 1)[1].split(full_marker, 1)[0]
    full_stage = dockerfile.split(full_marker, 1)[1].split(contract_marker, 1)[0]
    require_all(
        light_stage,
        (
            'ai-blaise.citus.bundle1.target="bundle1-final-light"',
            f'ai-blaise.citus.bundle1.evidence-scope="{LIGHT_SCOPE}"',
            'ai-blaise.citus.bundle1.release-target="false"',
        ),
        "Bundle1 light target labels",
    )
    if FULL_SCOPE in light_stage:
        fail("Bundle1 light target must never carry the full-bundle evidence scope")
    require_all(
        full_stage,
        (
            'ai-blaise.citus.bundle1.target="bundle1-final-full"',
            f'ai-blaise.citus.bundle1.evidence-scope="{FULL_SCOPE}"',
            'ai-blaise.citus.bundle1.release-target="true"',
        ),
        "Bundle1 full target labels",
    )
    if dockerfile.count(f'ai-blaise.citus.bundle1.evidence-scope="{FULL_SCOPE}"') != 1:
        fail("only Bundle1 final-full may carry the full-bundle evidence scope")

    require_all(
        default_boot_smoke,
        (
            "Deliberately pass no postgres command or -c override",
            "SHOW shared_preload_libraries",
            "SHOW citus.cohabit_extensions",
            "pg_file_settings",
            "/etc/postgresql/ai-blaise/shared-preload-libraries.conf",
            "PostgreSQL init process complete",
            "docker rm -fv",
            "server_version_num",
            "observed_pg_major",
            "BUNDLE1_EXPECTED_SOURCE_GIT_SHA",
            "BUNDLE1_EXPECTED_SOURCE_TREE_STATE",
            "ai-blaise.citus.source-git-sha",
            "ai-blaise.citus.source-tree-state",
            "observed_companion_version",
            'expected_companion_version="0.1.2"',
            "src/backend/distributed/citus.control",
            "observed_citus_version",
            "expected_citus_version",
            "SELECT companion_internal.assert_citus_cohabit_extension_order();",
            "Bundle1 negative order control accepted Citus-first preload",
            "Bundle1 negative required-library control accepted a missing library",
            "BUNDLE1_EXPECTED_TARGET",
            "bundle1-source-build.lock.tsv",
            "full_only_extensions",
            "required_manifest_count",
            "full Bundle1 check did not cover every required manifest entry",
        ),
        "Bundle1 default-boot smoke",
    )
    require_all(
        image_workflow,
        (
            "target: bundle1-final-light",
            "--target ${{ matrix.target }}",
            "bundle1-default-boot-smoke.sh",
            "BUNDLE1_EXPECTED_TARGET: bundle1-final-light",
            "BUNDLE1_EXPECTED_TARGET: bundle1-final-full",
            "BUNDLE1_EXPECTED_SOURCE_GIT_SHA: ${{ github.sha }}",
            "BUNDLE1_EXPECTED_SOURCE_TREE_STATE: clean",
            "Build full PG17 release-boundary operand",
            "if: github.event_name == 'push'",
            "postgres:17-bookworm@sha256:7bade6d532592ca8ce7ee32def7399dad2607c4ea5583839fc4352a095a11ea6",
            "--pull=false",
            "--iidfile",
            'BUNDLE1_IMAGE="$(cat',
        ),
        "Bundle1 image workflow",
    )

    for extension in EXPECTED_LOCK_ORDER:
        if extension == "plrust":
            continue
        if extension not in required:
            fail(
                f"source-build lock extension must remain required until policy changes: {extension}"
            )

    if EVIDENCE.exists():
        with EVIDENCE.open(encoding="utf-8", newline="") as fh:
            evidence_rows = list(csv.DictReader(fh, delimiter="\t"))
        if evidence_rows and list(evidence_rows[0].keys()) != [
            "observed_at",
            "git_sha",
            "target",
            "image_id",
            "extensions",
        ]:
            fail("Bundle1 evidence TSV header changed")
        light_rows = [
            row for row in evidence_rows if row["target"] == "bundle1-final-light"
        ]
        if not light_rows:
            fail(
                "Bundle1 evidence TSV must keep at least one light source-build proof row"
            )
        latest_light_extensions = set(light_rows[-1]["extensions"].split())
        expected_light_catalog = light_required & sql_created_required
        missing = expected_light_catalog - latest_light_extensions
        if missing:
            fail(
                f"latest Bundle1 light evidence row missing required production extensions: {sorted(missing)}"
            )
        leaked_full_only = latest_light_extensions & full_only
        if leaked_full_only:
            fail(
                "Bundle1 light evidence must not claim full-only extensions: "
                f"{sorted(leaked_full_only)}"
            )
        if "plrust" in latest_light_extensions:
            fail("Bundle1 light evidence must not imply plrust PG17 support")
        full_rows = [
            row for row in evidence_rows if row["target"] == "bundle1-final-full"
        ]
        if full_rows:
            latest_full_extensions = set(full_rows[-1]["extensions"].split())
            missing_full = (
                full_required & sql_created_required
            ) - latest_full_extensions
            if missing_full:
                fail(
                    "latest Bundle1 full evidence row missing required manifest entries: "
                    f"{sorted(missing_full)}"
                )
        elif (
            "no current release-qualified full-target default-boot receipt"
            not in compact(docs)
        ):
            fail(
                "Bundle1 docs must disclose that no current release-qualified "
                "full-target receipt exists"
            )

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
            "FEATURE: Bundle1 remains alpha",
            "no current release-qualified full-target default-boot receipt",
            "release-target=true",
            "workflow does not publish",
            "historical tracking metadata",
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

    print("bundle1-contract-check passed")


if __name__ == "__main__":
    main()
