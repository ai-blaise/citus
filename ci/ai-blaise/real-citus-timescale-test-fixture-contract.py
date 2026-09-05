#!/usr/bin/env python3
"""Validate the source-bound Citus + TimescaleDB test-fixture contract."""

# FEATURE: TS6 TS18

from __future__ import annotations

import csv
import io
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
DOCKERFILE = ROOT / "images/citus-timescale-cohabitation/Dockerfile"
BASE_LOCK = ROOT / "images/citus-timescale-cohabitation/base-image.lock.tsv"
MATERIALIZER = ROOT / "ci/ai-blaise/materialize-real-citus-timescale-test-fixture.py"
BUILDER = ROOT / "ci/ai-blaise/build-real-citus-timescale-test-fixture.sh"
TEST = ROOT / "ci/ai-blaise/real-citus-timescale-test-fixture-contract_test.py"
IMAGE_CHECK = ROOT / "ci/ai-blaise/image-check.sh"
IMAGE_WORKFLOW = ROOT / ".github/workflows/ci-image.yml"
UPGRADE_GUARD = ROOT / "ci/ai-blaise/upgrade-rollback-guardrails.sh"
COHABITATION_SMOKE = ROOT / "ci/ai-blaise/timescale-cohabitation-smoke.sh"
BRIDGE_SMOKE = ROOT / "ci/ai-blaise/timescale-bridge-smoke.sh"
MATRIX_SMOKE = ROOT / "ci/ai-blaise/ts-version-matrix-smoke.sh"
MATRIX_DIR = ROOT / "tests/cohab-matrix"
MATRIX_COMPARATOR = MATRIX_DIR / "compare-hook-claims.sh"
EXACT_BASES = {
    "2.27": (
        "docker.io/timescale/timescaledb-ha:pg17-ts2.27@sha256:"
        "4f61167e11c7c95bedf96433c720d671a53aa29ad7f52b142b529a6d0e9f0b20"
    ),
    "2.28": (
        "docker.io/timescale/timescaledb-ha:pg17-ts2.28@sha256:"
        "bc9e09875460aa69fb536362fef7c8e92c51ad6aab3d13f91a2487d3547dc71a"
    ),
}


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(1)


def read(path: pathlib.Path) -> str:
    if not path.is_file():
        fail(f"missing real-Citus Timescale fixture artifact: {path.relative_to(ROOT)}")
    text = path.read_text(encoding="utf-8")
    if not text.strip():
        fail(f"empty real-Citus Timescale fixture artifact: {path.relative_to(ROOT)}")
    return text


def require_once(text: str, token: str, context: str) -> None:
    if text.count(token) != 1:
        fail(f"{context} must contain exactly one occurrence of: {token}")


def require_order(text: str, tokens: tuple[str, ...], context: str) -> None:
    position = -1
    for token in tokens:
        next_position = text.find(token, position + 1)
        if next_position < 0:
            fail(f"{context} is missing ordered token: {token}")
        if next_position <= position:
            fail(f"{context} has an invalid operation order at: {token}")
        position = next_position


def validate_init_wait(text: str, logs_variable: str, context: str) -> None:
    for token in (
        f'{logs_variable}="$(docker logs --tail 200 "${{container}}" 2>&1 || true)"',
        f'[[ "${{{logs_variable}}}" == *"PostgreSQL init process complete"* ]]',
    ):
        if token not in text:
            fail(f"{context} must use the bounded non-pipeline init wait: {token}")
    if re.search(
        r"docker logs[^\n]*\|[^\n]*grep[^\n]*PostgreSQL init process complete",
        text,
    ):
        fail(f"{context} must not use a pipefail-sensitive docker-logs pipeline")


def validate_base_lock() -> None:
    rows = list(csv.DictReader(io.StringIO(read(BASE_LOCK)), delimiter="\t"))
    if not rows or list(rows[0]) != ["pg_major", "timescaledb_minor", "base_image"]:
        fail("Timescale fixture base lock must have its exact header")
    expected = [
        {"pg_major": "17", "timescaledb_minor": minor, "base_image": base}
        for minor, base in EXACT_BASES.items()
    ]
    if rows != expected:
        fail("Timescale fixture base lock must contain both exact reviewed PG17 rows")


def validate_dockerfile(dockerfile: str) -> None:
    context = "real-Citus Timescale fixture Dockerfile"
    if re.search(r"(?m)^ARG BASE_IMAGE=", dockerfile):
        fail(f"{context} must not define a floating default base")
    if re.search(r"(?m)^COPY\s+\.\s", dockerfile):
        fail(f"{context} must not copy a broad mutable checkout")
    if any("install-all" in line and "||" in line for line in dockerfile.splitlines()):
        fail(f"{context} must not fall back from install-all")
    if re.search(r"\bmake\s+install(?!-all)(?:\s|;|$)", dockerfile):
        fail(f"{context} must not omit downgrade SQL with plain install")
    for token in (
        "# FEATURE: TS6 TS18",
        "ARG BASE_IMAGE\nFROM ${BASE_IMAGE}",
        f'"2.27:{EXACT_BASES["2.27"]}"|',
        f'"2.28:{EXACT_BASES["2.28"]}")',
        'test "${PG_MAJOR}" = "17";',
        'case "${timescaledb_version}" in "${TIMESCALEDB_MINOR}"|"${TIMESCALEDB_MINOR}".*)',
        'postgresql_package_version="$(dpkg-query -W -f=\'${Version}\' "postgresql-${PG_MAJOR}")";',
        '"postgresql-server-dev-${PG_MAJOR}=${postgresql_package_version}"',
        "COPY config ./config",
        "COPY src ./src",
        "COPY vendor ./vendor",
        "COPY images/citus-pg-overlay/upgrades/ai_blaise_citus--0.1.2.sql /build/companion/archive/",
        'make install-all with_llvm="${WITH_LLVM}";',
        "for sql_file in src/backend/distributed/build/sql/citus--*.sql src/backend/columnar/build/sql/citus_columnar--*.sql; do",
        'cmp "${sql_file}" "${extension_dir}/$(basename "${sql_file}")";',
        'ai-blaise.citus.test-fixture.timescale="true"',
        'ai-blaise.citus.test-fixture.scope="source-built-citus-timescaledb-companion-test-only"',
        'ai-blaise.citus.test-fixture.release-target="false"',
        'ai-blaise.citus.test-fixture.base-image="${BASE_IMAGE}"',
        'ai-blaise.citus.test-fixture.timescale-id="${AI_BLAISE_COHAB_FIXTURE_ID}"',
        'ai-blaise.citus.source-content-sha256="${AI_BLAISE_SOURCE_CONTENT_SHA256}"',
        'ai-blaise.citus.source-tree-state="${AI_BLAISE_SOURCE_TREE_STATE}"',
        "USER postgres",
    ):
        require_once(dockerfile, token, context)
    for artifact in (
        "postgres.sha256",
        "timescaledb-control.sha256",
        "timescaledb-libraries.sha256",
    ):
        if (
            dockerfile.count(f"sha256sum --check /build/vendor-baseline/{artifact}")
            != 2
        ):
            fail(
                f"{context} must verify {artifact} after dependency install and Citus build"
            )
    if not (
        dockerfile.index('case "${TIMESCALEDB_MINOR}:${BASE_IMAGE}"')
        < dockerfile.index("/build/vendor-baseline/postgres.sha256")
        < dockerfile.index("apt-get update")
        < dockerfile.index("COPY config ./config")
        < dockerfile.index('make install-all with_llvm="${WITH_LLVM}";')
        < dockerfile.index("USER postgres")
    ):
        fail(
            f"{context} must validate its base before building and drop privileges last"
        )


def validate_materializer(materializer: str) -> None:
    context = "real-Citus Timescale fixture materializer"
    for token in (
        "# FEATURE: TS6 TS18",
        '"materialize-real-citus-test-fixture.py"',
        '"images/citus-timescale-cohabitation/Dockerfile"',
        '"images/citus-timescale-cohabitation/base-image.lock.tsv"',
        '"images/citus-pg-overlay/upgrades/ai_blaise_citus--0.1.2.sql"',
        "args.source, args.destination, inputs=SOURCE_INPUTS",
    ):
        require_once(materializer, token, context)
    if '"images/citus-test-fixture/Dockerfile"' in materializer:
        fail(f"{context} must not stage the unrelated base-fixture Dockerfile")
    if "copytree" in materializer or "shutil" in materializer:
        fail(f"{context} must reuse the reviewed nonignored Git inventory")


def validate_builder(builder: str) -> None:
    context = "real-Citus Timescale fixture builder"
    for token in (
        "# FEATURE: TS6 TS18",
        'context_builder="${repo_root}/ci/ai-blaise/materialize-real-citus-timescale-test-fixture.py"',
        'timescaledb_minor="${CITUS_TIMESCALE_TEST_FIXTURE_MINOR:-2.27}"',
        "--timescaledb-minor)",
        '[[ "${timescaledb_minor}" =~ ^2\\.(27|28)$ ]]',
        'python3 "${contract_check}" >&2',
        'fixture_context="$(mktemp -d "${fixture_tmp_parent}/ai-blaise-citus-timescale-fixture.XXXXXX")"',
        '--destination "${fixture_context}"',
        'staged_lock="${fixture_context}/images/citus-timescale-cohabitation/base-image.lock.tsv"',
        '"${staged_lock}" "${timescaledb_minor}" <<\'PY\'',
        'digest = hashlib.sha256(b"ai-blaise/real-citus-timescale-test-fixture/v1\\0")',
        '"${pg_major}" "${timescaledb_minor}" "${base_image}"',
        '"${citus_extension_version}" "${companion_extension_version}"',
        "\"${source_content_sha256}\" <<'PY'",
        'image="ai-blaise-citus-timescale-test-fixture:pg${pg_major}-ts${timescaledb_minor}-${fixture_identity}"',
        'verify_label "ai-blaise.citus.test-fixture.base-image" "${base_image}"',
        'verify_label "ai-blaise.citus.test-fixture.timescale-id" "${fixture_identity}"',
        'verify_label "ai-blaise.citus.source-content-sha256" "${source_content_sha256}"',
        '[[ "${provenance_tree_state}" =~ ^(clean|dirty)$ ]]',
        '--build-arg "BASE_IMAGE=${base_image}"',
        '--build-arg "AI_BLAISE_COHAB_FIXTURE_ID=${fixture_identity}"',
        '"${fixture_context}" >&2',
    ):
        require_once(builder, token, context)
    if (
        builder.count(
            'image_id="$(docker image inspect --format \'{{.Id}}\' "${image}")"'
        )
        != 2
    ):
        fail(f"{context} must resolve immutable IDs after cache selection and build")
    if builder.count("verify_fixture") != 3:
        fail(f"{context} must verify both cache-hit and newly built images")
    if "docker pull" in builder or 'timescale/timescaledb-ha:pg17-ts2.27"' in builder:
        fail(f"{context} must use only the staged digest-pinned base lock")
    if (
        '"${repo_root}" >&2' in builder
        or '"${repo_root}"\n' in builder.split("docker build", 1)[-1]
    ):
        fail(f"{context} must build only the staged immutable context")
    if not (
        builder.index('--destination "${fixture_context}"')
        < builder.index('staged_lock="${fixture_context}/')
        < builder.index("fixture_identity=")
        < builder.index('if docker image inspect "${image}"')
        < builder.index("docker build \\")
    ):
        fail(f"{context} must snapshot before identity, cache selection, and build")


def validate_cohabitation_smoke(smoke: str) -> None:
    context = "real-Citus Timescale cohabitation smoke"
    for token in (
        "# FEATURE: TS6 TS18",
        'expected_ts_minor="${TIMESCALE_COHABITATION_EXPECTED_TS_MINOR:-${CITUS_TIMESCALE_TEST_FIXTURE_MINOR:-2.27}}"',
        'builder_args=(--timescaledb-minor "${expected_ts_minor}")',
        'builder_args+=(--image "${image}")',
        'if [[ ! "${image}" =~ ^sha256:[0-9a-f]{64}$ ]]; then',
        "--network none",
        'docker rm --force --volumes "${container}"',
        "PostgreSQL init process complete",
        "CREATE EXTENSION IF NOT EXISTS citus;\nCREATE EXTENSION IF NOT EXISTS timescaledb;\nCREATE EXTENSION IF NOT EXISTS pgcrypto;\nCREATE EXTENSION IF NOT EXISTS ai_blaise_citus;",
        "FROM pg_dist_partition",
        "FROM _timescaledb_catalog.hypertable",
        '"true" \\\n    "false"',
    ):
        if token not in smoke:
            fail(f"{context} is missing: {token}")
    for minor, base in EXACT_BASES.items():
        if f'{minor})\n    expected_base_image="{base}"' not in smoke:
            fail(f"{context} does not bind TimescaleDB {minor} to its exact base")
    if "CREATE FUNCTION create_distributed_table" in smoke:
        fail(f"{context} must not stub the Citus distribution entrypoint")
    if "docker build" in smoke or "docker pull" in smoke:
        fail(f"{context} must use only the shared verified fixture builder")
    validate_init_wait(smoke, "container_logs", context)


def validate_bridge_smoke(smoke: str) -> None:
    context = "real-Citus Timescale bridge smoke"
    for token in (
        "# FEATURE: TS1 TS2 TS3 TS4 TS5 TS12",
        'timescaledb_minor="${TIMESCALE_BRIDGE_EXPECTED_TS_MINOR:-${CITUS_TIMESCALE_TEST_FIXTURE_MINOR:-2.27}}"',
        '"${fixture_builder}" --timescaledb-minor "${timescaledb_minor}"',
        'if [[ ! "${timescale_image}" =~ ^sha256:[0-9a-f]{64}$ ]]; then',
        "--network none",
        'docker rm --force --volumes "${container}"',
        "PostgreSQL init process complete",
        "CREATE DATABASE timescale_bridge_positive;",
        "-d timescale_bridge_positive",
        "negative bridge database unexpectedly contains Citus",
        "requires visible function create_distributed_table from extension citus",
        "FROM _timescaledb_catalog.hypertable",
        "FROM pg_dist_partition",
        "FROM pg_dist_shard",
        "real Timescale/Citus bridge row did not round trip",
        "real_citus_distribution",
        "stubbed_citus_distribution",
    ):
        if token not in smoke:
            fail(f"{context} is missing: {token}")
    for minor, base in EXACT_BASES.items():
        if f'{minor})\n    expected_base_image="{base}"' not in smoke:
            fail(f"{context} does not bind TimescaleDB {minor} to its exact base")
    negative_order = (
        "CREATE EXTENSION IF NOT EXISTS timescaledb;",
        "CREATE EXTENSION IF NOT EXISTS pgcrypto;",
        "CREATE EXTENSION IF NOT EXISTS ai_blaise_citus;",
        "CREATE TABLE timescale_missing_citus",
        "PERFORM apply_distribute_hypertable('timescale_missing_citus'",
        "CREATE DATABASE timescale_bridge_positive;",
    )
    require_order(smoke, negative_order, f"{context} negative database")
    positive_start = smoke.find(
        "psql -U postgres -d timescale_bridge_positive -v ON_ERROR_STOP=1"
    )
    if positive_start < 0:
        fail(f"{context} must connect explicitly to its positive database")
    positive = smoke[positive_start:]
    require_order(
        positive,
        (
            "CREATE EXTENSION IF NOT EXISTS citus;",
            "CREATE EXTENSION IF NOT EXISTS timescaledb;",
            "CREATE EXTENSION IF NOT EXISTS pgcrypto;",
            "CREATE EXTENSION IF NOT EXISTS ai_blaise_citus;",
            "SELECT apply_distribute_hypertable",
            "FROM pg_dist_partition",
            "FROM pg_dist_shard",
        ),
        f"{context} positive database",
    )
    if "CREATE FUNCTION create_distributed_table" in smoke:
        fail(f"{context} must not stub the Citus distribution entrypoint")
    if "docker build" in smoke or "docker pull" in smoke:
        fail(f"{context} must use only the shared verified fixture builder")
    if '"true" \\\n    "true" \\\n    "false" \\\n    "true"' not in smoke:
        fail(f"{context} must record real Timescale, real Citus, and no stub")
    validate_init_wait(smoke, "container_logs", context)


def validate_matrix_smoke(smoke: str, comparator: str) -> None:
    context = "real-Citus Timescale version matrix"
    for minor, expected_base in EXACT_BASES.items():
        image_tag = read(MATRIX_DIR / minor / "image-tag.txt").strip()
        if image_tag != expected_base:
            fail(f"{context} {minor} image-tag must be its exact reviewed base")
        inventory = list(
            csv.DictReader(
                io.StringIO(read(MATRIX_DIR / minor / "expected-hook-claims.tsv")),
                delimiter="\t",
            )
        )
        if not inventory or list(inventory[0]) != [
            "hook_symbol",
            "claim_status",
            "notes",
        ]:
            fail(f"{context} {minor} hook inventory must have its exact schema")
        symbols = [row["hook_symbol"] for row in inventory]
        if (
            len(symbols) != len(set(symbols))
            or any(not symbol for symbol in symbols)
            or any(
                row["claim_status"] not in {"claimed", "not_claimed"}
                for row in inventory
            )
            or any(not row["notes"] for row in inventory)
        ):
            fail(
                f"{context} {minor} hook inventory must be nonempty and structurally closed"
            )
    for token in (
        'required_versions_value="${TS_VERSION_MATRIX_REQUIRED-2.27 2.28}"',
        'for required_version in "${required_versions[@]}"; do',
        'echo "required TS version was not selected: ${required_version}"',
        'fixture_image="$("${fixture_builder}" --timescaledb-minor "${ts_version}")"',
        '[[ ! "${fixture_image}" =~ ^sha256:[0-9a-f]{64}$ ]]',
        'TIMESCALE_COHABITATION_IMAGE="${fixture_image}"',
        'TIMESCALE_COHABITATION_EXPECTED_TS_MINOR="${ts_version}"',
        "--network none",
        "PostgreSQL init process complete",
        'docker rm --force --volumes "${container}"',
        'compare-hook-claims.sh" "${ts_version}" "${container}"',
    ):
        if token not in smoke:
            fail(f"{context} is missing: {token}")
    for forbidden in ("docker pull", "docker manifest inspect", "docker buildx"):
        if forbidden in smoke:
            fail(f"{context} must not discover or build a floating vendor image")
    validate_init_wait(smoke, "probe_logs", context)
    for token in (
        "cannot observe or compare live C hook pointers",
        "hook_runtime_comparison=unavailable",
        "static hook inventory is structurally closed",
    ):
        if token not in comparator:
            fail(f"{context} must state its non-runtime hook boundary: {token}")
    for forbidden in (
        "cohabitation seam matches expected hook-claim table",
        'production proxy for "the trusted-hook chain is wired"',
    ):
        if forbidden in comparator:
            fail(f"{context} must not claim runtime hook-pointer comparison")


def validate_wiring(image_check: str, workflow: str, upgrade_guard: str) -> None:
    for token in (
        "build-real-citus-timescale-test-fixture.sh",
        "materialize-real-citus-timescale-test-fixture.py",
        "real-citus-timescale-test-fixture-contract.py",
        "real-citus-timescale-test-fixture-contract_test.py",
        "base-image.lock.tsv",
    ):
        if token not in image_check + workflow:
            fail(f"real-Citus Timescale fixture CI/static wiring missing: {token}")
    if 'python3 "${real_citus_timescale_fixture_contract}"' not in image_check:
        fail("image-check must execute the Timescale fixture contract")
    if (
        "python3 ci/ai-blaise/real-citus-timescale-test-fixture-contract.py"
        not in workflow
        or "python3 ci/ai-blaise/real-citus-timescale-test-fixture-contract_test.py"
        not in workflow
    ):
        fail("ci-image must execute the Timescale fixture contract and mutations")
    for smoke in (
        "timescale-cohabitation-smoke.sh",
        "ts-version-matrix-smoke.sh",
    ):
        if f"REQUIRE_DOCKER=1 bash ci/ai-blaise/{smoke}" not in workflow:
            fail(f"ci-image must execute the source-bound {smoke}")
    for token in (
        "for timescaledb_minor in 2.27 2.28; do",
        'TIMESCALE_BRIDGE_EXPECTED_TS_MINOR="${timescaledb_minor}"',
        "REQUIRE_DOCKER=1 bash ci/ai-blaise/timescale-bridge-smoke.sh",
    ):
        if token not in workflow:
            fail(f"ci-image must execute both exact Timescale bridge minors: {token}")
    for token in (
        "REAL_CITUS_TIMESCALE_DEFAULT_VERSION_SMOKES",
        'ROOT / "ci/ai-blaise/timescale-bridge-smoke.sh"',
        'require_contains(smoke, "build-real-citus-timescale-test-fixture.sh")',
        'require_contains(smoke, "real-citus-timescale-test-fixture-contract.py")',
    ):
        if token not in upgrade_guard:
            fail(
                f"upgrade guard is missing the source-bound Timescale bridge contract: {token}"
            )


def main() -> None:
    validate_base_lock()
    validate_dockerfile(read(DOCKERFILE))
    validate_materializer(read(MATERIALIZER))
    validate_builder(read(BUILDER))
    validate_cohabitation_smoke(read(COHABITATION_SMOKE))
    validate_bridge_smoke(read(BRIDGE_SMOKE))
    validate_matrix_smoke(read(MATRIX_SMOKE), read(MATRIX_COMPARATOR))
    validate_wiring(read(IMAGE_CHECK), read(IMAGE_WORKFLOW), read(UPGRADE_GUARD))
    read(TEST)
    print("real-citus-timescale-test-fixture-contract passed")


if __name__ == "__main__":
    main()
