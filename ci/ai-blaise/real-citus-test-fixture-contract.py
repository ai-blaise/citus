#!/usr/bin/env python3
"""Validate the deterministic real-Citus companion test-fixture contract."""

from __future__ import annotations

import csv
import io
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
DOCKERFILE = ROOT / "images/citus-test-fixture/Dockerfile"
BASE_LOCK = ROOT / "images/citus-test-fixture/base-images.lock.tsv"
HTTP_DOCKERFILE = ROOT / "images/citus-test-fixture/Dockerfile.http"
HTTP_PACKAGE_LOCK = ROOT / "images/citus-test-fixture/http-packages.lock.tsv"
BUILDER = ROOT / "ci/ai-blaise/build-real-citus-test-fixture.sh"
HTTP_BUILDER = ROOT / "ci/ai-blaise/build-real-citus-http-test-fixture.sh"
CONTEXT_BUILDER = ROOT / "ci/ai-blaise/materialize-real-citus-test-fixture.py"
AI_SQL_SMOKE = ROOT / "ci/ai-blaise/ai-sql-contract-smoke.sh"
A10_A11_LIVE_SMOKE = ROOT / "ci/ai-blaise/a10-a11-ai-sql-live-smoke.sh"
MIGRATION_INVARIANTS_SMOKE = ROOT / "ci/ai-blaise/migration-invariants-smoke.sh"
SCHEMA_JOB_SMOKE = ROOT / "ci/ai-blaise/schema-job-f1-2vi-smoke.sh"
OTEL_SMOKE = ROOT / "ci/ai-blaise/otel-trace-propagation-smoke.sh"
SQL_EXTENSION_SMOKE = ROOT / "ci/ai-blaise/sql-extension-smoke.sh"
CANARY_UPGRADE_SMOKE = ROOT / "ci/ai-blaise/canary-upgrade-rollback-smoke.sh"
SECURITY_BACKUP_SMOKE = ROOT / "ci/ai-blaise/extension-security-backup-smoke.sh"
OBSERVABILITY_REPLICATION_SMOKE = (
    ROOT / "ci/ai-blaise/observability-replication-smoke.sh"
)
IMAGE_CHECK = ROOT / "ci/ai-blaise/image-check.sh"
IMAGE_WORKFLOW = ROOT / ".github/workflows/ci-image.yml"
SIDECAR_WORKFLOW = ROOT / ".github/workflows/ci-sidecar.yml"
OBSERVABILITY_WORKFLOW = ROOT / ".github/workflows/ci-observability-contracts.yml"
PRODUCTION_WORKFLOW = ROOT / ".github/workflows/ci-production-readiness.yml"


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(1)


def read(path: pathlib.Path) -> str:
    if not path.is_file():
        fail(f"missing real-Citus fixture contract artifact: {path.relative_to(ROOT)}")
    text = path.read_text(encoding="utf-8")
    if not text.strip():
        fail(f"empty real-Citus fixture contract artifact: {path.relative_to(ROOT)}")
    return text


def require_once(text: str, token: str, context: str) -> None:
    if text.count(token) != 1:
        fail(f"{context} must contain exactly one occurrence of: {token}")


def validate_base_lock() -> None:
    rows = list(csv.DictReader(io.StringIO(read(BASE_LOCK)), delimiter="\t"))
    if not rows or list(rows[0]) != ["pg_major", "base_image"]:
        fail(
            "real-Citus fixture base lock must have its exact header and at least one row"
        )
    seen: set[str] = set()
    for row in rows:
        major = row["pg_major"]
        image = row["base_image"]
        if major not in {"16", "17", "18"} or major in seen:
            fail(
                "real-Citus fixture base lock has an unsupported or duplicate PG major"
            )
        if not re.fullmatch(
            rf"docker\.io/library/postgres:{major}-bookworm@sha256:[0-9a-f]{{64}}",
            image,
        ):
            fail(f"real-Citus fixture PG{major} base must be digest-pinned bookworm")
        seen.add(major)
    if seen != {"16", "17", "18"}:
        fail("real-Citus fixture base lock must cover PG16, PG17, and PG18 exactly")


def validate_http_package_lock() -> None:
    rows = list(csv.DictReader(io.StringIO(read(HTTP_PACKAGE_LOCK)), delimiter="\t"))
    header = list(rows[0]) if rows else []
    if header != ["pg_major", "package", "version"]:
        fail("real-Citus HTTP package lock must have its exact header")
    if rows != [
        {
            "pg_major": "17",
            "package": "postgresql-17-http",
            "version": "1.7.2-2.pgdg12+1",
        }
    ]:
        fail("real-Citus HTTP package lock must contain the reviewed PG17 package")


def validate_dockerfile(dockerfile: str) -> None:
    if any("install-all" in line and "||" in line for line in dockerfile.splitlines()):
        fail("real-Citus fixture Dockerfile must not fall back from install-all")
    if re.search(r"\bmake\s+install(?!-all)(?:\s|;|$)", dockerfile):
        fail(
            "real-Citus fixture Dockerfile must not omit downgrade SQL with make install"
        )
    false_direct_install_assertion = (
        'test -s "/out$(pg_config --sharedir)/extension/'
        'citus--${CITUS_EXTENSION_VERSION}.sql"'
    )
    if false_direct_install_assertion in dockerfile:
        fail(
            "real-Citus fixture must validate the generated SQL inventory, not assume "
            "a direct default-version install script"
        )
    for token in (
        "FROM ${BASE_IMAGE} AS citus-build",
        "FROM ${BASE_IMAGE} AS companion-test-fixture",
        'case "${PG_MAJOR}" in 16|17|18)',
        "ARG AI_BLAISE_SOURCE_GIT_SHA",
        "ARG AI_BLAISE_SOURCE_GIT_TREE",
        'ai-blaise.citus.test-fixture="true"',
        'ai-blaise.citus.test-fixture.scope="source-built-companion-test-only"',
        'ai-blaise.citus.test-fixture.release-target="false"',
        'ai-blaise.citus.test-fixture.citus-extension-version="${CITUS_EXTENSION_VERSION}"',
        'ai-blaise.citus.test-fixture.id="${AI_BLAISE_FIXTURE_ID}"',
        'ai-blaise.citus.source-content-sha256="${AI_BLAISE_SOURCE_CONTENT_SHA256}"',
        'ai-blaise.citus.source-tree-state="${AI_BLAISE_SOURCE_TREE_STATE}"',
        "COPY config ./config",
        "COPY src ./src",
        "COPY vendor ./vendor",
        "      clang \\",
        "      llvm-dev \\",
        '"postgresql-server-dev-${PG_MAJOR}=${PG_VERSION}"',
        'test -s "$(pg_config --includedir-server)/postgres.h";',
        "make install-all DESTDIR=/out;",
        'test -s "/out$(pg_config --pkglibdir)/citus.so";',
        'test -s "/out$(pg_config --pkglibdir)/citus_columnar.so";',
        "test \"$(sed -n \"s/^default_version = '\\([^']*\\)'$/\\1/p\" "
        '"${extension_dir}/citus.control")" = "${CITUS_EXTENSION_VERSION}";',
        "for sql_file in src/backend/distributed/build/sql/citus--*.sql "
        "src/backend/columnar/build/sql/citus_columnar--*.sql; do",
        'test -s "${sql_file}";',
        'cmp "${sql_file}" "${extension_dir}/$(basename "${sql_file}")";',
        'CMD ["postgres", "-c", "shared_preload_libraries=citus"]',
    ):
        require_once(dockerfile, token, "real-Citus fixture Dockerfile")
    for filename in (
        "ai_blaise_citus.control",
        "ai_blaise_citus--0.1.0.sql",
        "ai_blaise_citus--0.1.0--0.1.1.sql",
        "ai_blaise_citus--0.1.1--0.1.0.sql",
        "ai_blaise_citus--0.1.1--0.1.2.sql",
    ):
        require_once(
            dockerfile,
            f"COPY images/citus-pg-overlay/extensions/{filename}",
            "real-Citus fixture Dockerfile",
        )


def validate_http_dockerfile(dockerfile: str) -> None:
    context = "real-Citus HTTP fixture Dockerfile"
    for token in (
        "# FEATURE: A10 A11",
        "ARG REAL_CITUS_FIXTURE_PARENT",
        "FROM ${REAL_CITUS_FIXTURE_PARENT} AS companion-http-test-fixture",
        'ai-blaise.citus.test-fixture.http="true"',
        'ai-blaise.citus.test-fixture.scope="source-built-companion-http-test-only"',
        'ai-blaise.citus.test-fixture.release-target="false"',
        'ai-blaise.citus.test-fixture.http-package-version="${PG_HTTP_PACKAGE_VERSION}"',
        'ai-blaise.citus.test-fixture.http-parent-image-id="${REAL_CITUS_FIXTURE_IMAGE_ID}"',
        'ai-blaise.citus.test-fixture.http-parent-fixture-id="${REAL_CITUS_FIXTURE_ID}"',
        'test "${PG_MAJOR}" = "17";',
        'test "${PG_HTTP_PACKAGE}" = "postgresql-17-http";',
        'postgres_server_version="$(postgres --version)";',
        'apt-get install -y --no-install-recommends "${PG_HTTP_PACKAGE}=${PG_HTTP_PACKAGE_VERSION}";',
        'test "$(postgres --version)" = "${postgres_server_version}";',
        'test "$(dpkg-query -W -f=\'${Version}\' "${PG_HTTP_PACKAGE}")" = "${PG_HTTP_PACKAGE_VERSION}";',
        'test -s "/usr/share/postgresql/${PG_MAJOR}/extension/http.control";',
        "rm -rf /var/lib/apt/lists/*",
    ):
        require_once(dockerfile, token, context)
    if "postgres:" in dockerfile or re.search(
        r"apt-get install[^\n]*postgresql-17-http(?:\s|\")", dockerfile
    ):
        fail(f"{context} must use its immutable parent and exact package version")


def validate_context_builder(context_builder: str) -> None:
    for token in (
        '"config"',
        '"src"',
        '"vendor"',
        '"images/citus-test-fixture/Dockerfile"',
        '"images/citus-pg-overlay/extensions/ai_blaise_citus.control"',
        '"images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql"',
        '"images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0--0.1.1.sql"',
        '"images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.1--0.1.0.sql"',
        '"images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.1--0.1.2.sql"',
        'hashlib.sha256(b"ai-blaise/real-citus-test-fixture-context/v1\\0")',
        'struct.pack(">I", stat.S_IMODE(metadata.st_mode))',
        "resolved = path.resolve(strict=True)",
        '"ls-files"',
        '"--cached"',
        '"--others"',
        '"--exclude-standard"',
        "for relative in _source_inventory(source, selected_inputs):",
    ):
        require_once(context_builder, token, "real-Citus fixture context builder")
    if "copytree(" in context_builder:
        fail(
            "real-Citus fixture context must not recursively copy ignored build products"
        )


def validate_builder(builder: str) -> None:
    for token in (
        'python3 "${contract_check}" >&2',
        "rev-parse --verify 'HEAD^{commit}'",
        "rev-parse --verify 'HEAD^{tree}'",
        'python3 "${context_builder}"',
        '--source "${repo_root}"',
        '--destination "${fixture_context}"',
        'dockerfile="${fixture_context}/images/citus-test-fixture/Dockerfile"',
        "--target companion-test-fixture",
        '--build-arg "BASE_IMAGE=${base_image}"',
        '--build-arg "CITUS_EXTENSION_VERSION=${citus_extension_version}"',
        '--build-arg "AI_BLAISE_FIXTURE_ID=${fixture_identity}"',
        '--build-arg "AI_BLAISE_SOURCE_CONTENT_SHA256=${source_content_sha256}"',
        '--build-arg "AI_BLAISE_SOURCE_GIT_SHA=${source_git_sha}"',
        '--build-arg "AI_BLAISE_SOURCE_GIT_TREE=${source_git_tree}"',
        '--build-arg "AI_BLAISE_SOURCE_TREE_STATE=${source_tree_state}"',
        'verify_label "ai-blaise.citus.test-fixture.release-target" "false"',
        'verify_label "ai-blaise.citus.test-fixture.citus-extension-version" "${citus_extension_version}"',
        'verify_label "ai-blaise.citus.test-fixture.id" "${fixture_identity}"',
        'verify_label "ai-blaise.citus.source-content-sha256" "${source_content_sha256}"',
        'provenance_git_sha="$(read_label "ai-blaise.citus.source-git-sha")"',
        'provenance_git_tree="$(read_label "ai-blaise.citus.source-git-tree")"',
        'provenance_tree_state="$(read_label "ai-blaise.citus.source-tree-state")"',
        '[[ "${provenance_git_sha}" =~ ^[0-9a-f]{40}$ ]]',
        '[[ "${provenance_git_tree}" =~ ^[0-9a-f]{40}$ ]]',
        '[[ "${provenance_tree_state}" =~ ^(clean|dirty)$ ]]',
        '[[ "${provenance_revision}" == "${provenance_git_sha}" ]]',
        'image_id="$(docker image inspect --format \'{{.Id}}\' "${image}")"',
        "printf '%s\\n' \"${image_id}\"",
        'image="ai-blaise-citus-test-fixture:pg${pg_major}-${fixture_identity}"',
        '"${pg_major}" "${base_image}" "${citus_extension_version}" "${source_content_sha256}"',
    ):
        require_once(builder, token, "real-Citus fixture builder")
    if "--pull" in builder or "postgres:${pg_major}" in builder:
        fail("real-Citus fixture builder must use only the digest-pinned base lock")
    if '"${repo_root}" >&2' in builder:
        fail("real-Citus fixture builder must not send the mutable worktree as context")
    if "source_git_sha:0" in builder:
        fail("real-Citus fixture cache identity must not be derived from HEAD alone")
    read_label_body = builder.split("read_label() {", 1)[1].split("\n}", 1)[0]
    if '"${image_id}"' not in read_label_body or '"${image}"' in read_label_body:
        fail(
            "real-Citus fixture cache labels must be verified through immutable image ID"
        )
    for label in (
        "ai-blaise.citus.source-git-sha",
        "ai-blaise.citus.source-git-tree",
        "ai-blaise.citus.source-tree-state",
        "org.opencontainers.image.revision",
    ):
        if f'verify_label "{label}"' in builder:
            fail(
                "real-Citus fixture cache must preserve build-time Git provenance "
                "across metadata-only source transitions"
            )


def validate_http_builder(builder: str) -> None:
    context = "real-Citus HTTP fixture builder"
    for token in (
        "# FEATURE: A10 A11",
        'python3 "${contract_check}" >&2',
        'fixture_image_id="$("${base_builder}" --pg-major "${pg_major}")"',
        'fixture_id="$(docker image inspect --format \'{{ index .Config.Labels "ai-blaise.citus.test-fixture.id" }}\' "${fixture_image_id}")"',
        'fixture_tag="ai-blaise-citus-test-fixture:pg${pg_major}-${fixture_id}"',
        'fixture_parent="${fixture_tag}"',
        "verify_parent_tag() {",
        "verify_parent_ancestry() {",
        "verify_http_fixture() {",
        "docker image inspect --format '{{json .RootFS.Layers}}' \"${fixture_image_id}\"",
        "docker image inspect --format '{{json .RootFS.Layers}}' \"${image_id}\"",
        'verify_label "ai-blaise.citus.test-fixture.id" "${fixture_id}"',
        "child[: len(parent)] != parent",
        'install -m 0644 "${dockerfile}" "${build_root}/Dockerfile"',
        'dockerfile_sha256="$(python3 - "${build_root}/Dockerfile"',
        "hashlib.sha256(pathlib.Path(sys.argv[1]).read_bytes()).hexdigest()",
        'hashlib.sha256(b"ai-blaise/real-citus-http-test-fixture/v1\\0")',
        'image="ai-blaise-citus-http-test-fixture:pg${pg_major}-${http_fixture_id}"',
        "--pull=false",
        "--target companion-http-test-fixture",
        '--build-arg "REAL_CITUS_FIXTURE_PARENT=${fixture_parent}"',
        '--build-arg "PG_HTTP_PACKAGE=${package_name}"',
        '--build-arg "PG_HTTP_PACKAGE_VERSION=${package_version}"',
        '--build-arg "REAL_CITUS_FIXTURE_IMAGE_ID=${fixture_image_id}"',
        '--build-arg "REAL_CITUS_FIXTURE_ID=${fixture_id}"',
        "trap cleanup EXIT",
        "trap 'exit 129' HUP",
        "trap 'exit 130' INT",
        "trap 'exit 143' TERM",
    ):
        require_once(builder, token, context)
    if builder.count("verify_parent_tag") != 3:
        fail(f"{context} must verify its parent tag before and after use")
    if builder.count("verify_parent_ancestry") != 2:
        fail(f"{context} must verify immutable parent rootfs ancestry")
    if builder.count("verify_http_fixture") != 3:
        fail(f"{context} must verify both cached and newly built HTTP fixtures")
    if builder.count("printf '%s\\n' \"${image_id}\"") != 2:
        fail(f"{context} must return both cached and newly built immutable image IDs")
    if (
        "docker image tag" in builder
        or "@${fixture_image_id}" in builder
        or 'fixture_parent="${fixture_image_id}"' in builder
    ):
        fail(
            f"{context} must use only its locally verified content-derived parent tag"
        )
    if "postgres:" in builder:
        fail(f"{context} must not bypass the shared immutable base builder")
    if not (
        builder.index('install -m 0644 "${dockerfile}" "${build_root}/Dockerfile"')
        < builder.index('dockerfile_sha256="$(python3 - "${build_root}/Dockerfile"')
        < builder.index('if docker image inspect "${image}"')
        < builder.index("docker build \\")
    ):
        fail(f"{context} must snapshot before hashing, cache selection, and build")


def validate_ai_sql_smoke(smoke: str) -> None:
    for token in (
        'python3 "${fixture_contract}"',
        'fixture_image="$("${fixture_builder}" --pg-major "${pg_major}"',
        'docker run --name "${container}"',
        "--network none",
        '-d "${fixture_image}"',
        'docker rm --force --volumes "${container}"',
        "init_complete=0",
        "ready=0",
        "CREATE EXTENSION citus;",
        "CREATE EXTENSION pgcrypto;",
        "CREATE EXTENSION ai_blaise_citus;",
        "'pg_catalog.citus_add_node(text,integer,integer,noderole,name)'",
        "to_regclass('pg_catalog.pg_dist_node')",
    ):
        require_once(smoke, token, "AI SQL real-Citus smoke")
    if not (
        smoke.index("CREATE EXTENSION citus;")
        < smoke.index("CREATE EXTENSION pgcrypto;")
        < smoke.index("CREATE EXTENSION ai_blaise_citus;")
    ):
        fail("AI SQL smoke must create Citus before companion prerequisites")
    docker_run = smoke.split('docker run --name "${container}"', 1)[1].split(
        ">/dev/null", 1
    )[0]
    if (
        "postgres:17" in smoke
        or "/usr/share/postgresql/17/extension" in smoke
        or re.search(r"(^|\s)-v\s", docker_run)
        or re.search(r"(^|\s)-p\s", docker_run)
    ):
        fail("AI SQL smoke must not fall back to a stock PostgreSQL fixture")


def validate_a10_a11_live_smoke(smoke: str) -> None:
    context = "A10/A11 live real-Citus HTTP smoke"
    pinned_mock_image = (
        'mock_image="docker.io/library/python:3.12-slim@sha256:'
        '78387bc3881b8273120a12ebe6c1ab22b018ccc2c9adf565ae1ac9b536e184ea"'
    )
    for token in (
        'http_fixture_builder="${repo_root}/ci/ai-blaise/build-real-citus-http-test-fixture.sh"',
        'fixture_contract="${repo_root}/ci/ai-blaise/real-citus-test-fixture-contract.py"',
        pinned_mock_image,
        "pg_major=17",
        'python3 "${fixture_contract}"',
        'fixture_image="$("${http_fixture_builder}" --pg-major "${pg_major}")"',
        'docker rm --force --volumes "${pg_container}" "${mock_container}"',
        'docker network create "${network}"',
        '--network "${network}" --network-alias mock-llm',
        'docker run -d --name "${pg_container}" --network "${network}"',
        '-d "${fixture_image}"',
        "mock_ready=0",
        "postgres_init_complete=0",
        'grep -q "PostgreSQL init process complete"',
        "postgres_ready=0",
        "CREATE EXTENSION citus;",
        "CREATE EXTENSION pgcrypto;",
        "CREATE EXTENSION http;",
        "CREATE EXTENSION ai_blaise_citus;",
        "if ! observed_at=\"$(date -u '+%Y-%m-%dT%H:%M:%SZ')\"; then",
        'if [[ ! "${observed_at}" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T'
        "[0-9]{2}:[0-9]{2}:[0-9]{2}Z$ ]]; then",
        "if ! git_sha=\"$(git rev-parse --verify 'HEAD^{commit}')\"; then",
        'if [[ ! "${git_sha}" =~ ^[0-9a-f]{40}$ ]]; then',
        '"${observed_at}" "${git_sha}"',
    ):
        require_once(smoke, token, context)
    if smoke.count("python:3.12-slim") != 1 or "date -I" in smoke:
        fail(f"{context} must pin the mock image and capture portable UTC evidence")
    extension_sequence = "\n".join(
        (
            "CREATE EXTENSION citus;",
            "CREATE EXTENSION pgcrypto;",
            "CREATE EXTENSION http;",
            "CREATE EXTENSION ai_blaise_citus;",
        )
    )
    if extension_sequence not in smoke:
        fail(f"{context} must create Citus before HTTP and the companion")
    if not (
        smoke.index("mock_ready=0")
        < smoke.index("postgres_init_complete=0")
        < smoke.index("postgres_ready=0")
        < smoke.index("CREATE EXTENSION citus;")
    ):
        fail(f"{context} must complete mock and initdb readiness before SQL")
    database_run = smoke.split('docker run -d --name "${pg_container}"', 1)[1].split(
        ">/dev/null", 1
    )[0]
    if (
        "postgres:" in smoke
        or "/usr/share/postgresql/" in smoke
        or "apt-get" in smoke
        or re.search(r"(^|\s)-v\s", database_run)
        or re.search(r"(?m)^\s*-p\s", smoke)
    ):
        fail(f"{context} must use its immutable HTTP fixture on a private network")


def validate_sql_extension_matrix_smoke(smoke: str) -> None:
    context = "SQL extension PG16/PG17/PG18 real-Citus smoke"
    for token in (
        'fixture_builder="${repo_root}/ci/ai-blaise/build-real-citus-test-fixture.sh"',
        'fixture_contract="${repo_root}/ci/ai-blaise/real-citus-test-fixture-contract.py"',
        'pg_majors_default="16 17 18"',
        "SQL_EXTENSION_SMOKE_IMAGE is retired; use source-verified CITUS_TEST_FIXTURE_IMAGE",
        "CITUS_TEST_FIXTURE_IMAGE requires exactly one SQL_EXTENSION_SMOKE_PG_MAJORS major",
        'python3 "${fixture_contract}"',
        'docker rm --force --volumes "${active_container}"',
    ):
        require_once(smoke, token, context)
    fixture_path = smoke.split("run_smoke_for_pg_major()", 1)[1].split(
        "run_bundle1_source_build_smoke()", 1
    )[0]
    for token in (
        'fixture_image="$("${fixture_builder}" --pg-major "${pg_major}")"',
        "--network none",
        '-d "${fixture_image}"',
        'postgres_args=(-c "shared_preload_libraries=citus,pg_stat_statements")',
        "local init_complete=0",
        "PostgreSQL init process complete",
        "CREATE EXTENSION citus;",
        "CREATE EXTENSION pg_stat_statements;",
        "CREATE EXTENSION pgcrypto;",
        "CREATE EXTENSION ai_blaise_citus;",
    ):
        require_once(fixture_path, token, context)
    database_run = fixture_path.split("docker run \\", 1)[1].split(">/dev/null", 1)[0]
    if (
        "SQL_EXTENSION_SMOKE_IMAGE" in fixture_path
        or "postgres:" in fixture_path
        or "docker pull" in fixture_path
        or re.search(r"(^|\s)-v\s", database_run)
        or re.search(r"(^|\s)-p\s", database_run)
    ):
        fail(f"{context} must not use stock images, mounts, or published ports")
    if not (
        fixture_path.index("CREATE EXTENSION citus;")
        < fixture_path.index("CREATE EXTENSION pgcrypto;")
        < fixture_path.index("CREATE EXTENSION ai_blaise_citus;")
    ):
        fail(f"{context} must create Citus before companion prerequisites")
    if "CREATE FUNCTION create_distributed_table" in fixture_path:
        fail(f"{context} must not replace real Citus distribution with a stub")
    for token in (
        "FROM pg_catalog.pg_dist_partition",
        "FROM pg_catalog.pg_dist_shard",
        "apply_distribute_hypertable did not create exactly two real Citus shards",
        "real Citus bridge insert/readback failed",
        "GRANT EXECUTE ON FUNCTION companion_tenant_id_matches(text),",
        "companion_current_tenant_id(), companion_require_tenant_id()",
        "'companion_set_session_claims(text,text,text,text,boolean)', 'EXECUTE'",
        "Sec1 RLS runtime role must not receive claim mutation authority",
    ):
        if token not in fixture_path:
            fail(
                f"{context} must preserve the real distribution and RLS contract: {token}"
            )


def validate_canary_upgrade_smoke(smoke: str) -> None:
    context = "canary upgrade PG17/PG18 real-Citus smoke"
    for token in (
        'fixture_builder="${repo_root}/ci/ai-blaise/build-real-citus-test-fixture.sh"',
        'fixture_contract="${repo_root}/ci/ai-blaise/real-citus-test-fixture-contract.py"',
        "pg_majors=(17 18)",
        "CANARY_UPGRADE_IMAGE overrides are retired; use source-verified CITUS_TEST_FIXTURE_IMAGE",
        "CITUS_TEST_FIXTURE_IMAGE requires one explicit CANARY_UPGRADE_PG_MAJOR",
        'python3 "${fixture_contract}"',
        'fixture_image="$("${fixture_builder}" --pg-major "${pg_major}")"',
        "--network none",
        '-d "${fixture_image}"',
        "local init_complete=0",
        "PostgreSQL init process complete",
    ):
        require_once(smoke, token, context)
    if smoke.count('docker rm --force --volumes "${active_container}"') != 2:
        fail(f"{context} must clean container volumes on success and failure")
    extension_sequences = (
        "CREATE EXTENSION citus;\nCREATE EXTENSION pgcrypto;\n"
        "CREATE EXTENSION ai_blaise_citus VERSION '0.1.0';",
        "CREATE EXTENSION citus;\nCREATE EXTENSION pgcrypto;\n"
        "CREATE EXTENSION ai_blaise_citus;",
        "CREATE EXTENSION citus;\nCREATE EXTENSION pgcrypto;\n"
        "CREATE EXTENSION ai_blaise_citus VERSION '0.1.2';",
    )
    for sequence in extension_sequences:
        require_once(smoke, sequence, context)
    database_run = smoke.split("docker run \\", 1)[1].split(">/dev/null", 1)[0]
    if (
        "postgres:" in smoke
        or "docker pull" in smoke
        or re.search(r"(^|\s)-v\s", database_run)
        or re.search(r"(^|\s)-p\s", database_run)
    ):
        fail(f"{context} must use only the shared immutable fixture")


def validate_security_backup_smoke(smoke: str) -> None:
    context = "security backup/restore PG17/PG18 real-Citus smoke"
    for token in (
        'fixture_builder="${repo_root}/ci/ai-blaise/build-real-citus-test-fixture.sh"',
        'fixture_contract="${repo_root}/ci/ai-blaise/real-citus-test-fixture-contract.py"',
        "EXTENSION_SECURITY_IMAGE is retired; use source-verified CITUS_TEST_FIXTURE_IMAGE",
        'python3 "${fixture_contract}"',
        'fixture_image="$("${fixture_builder}" --pg-major "${pg_major}")"',
        'docker rm --force --volumes "${container}"',
        'docker run --network none -d -e POSTGRES_HOST_AUTH_METHOD=trust "${fixture_image}"',
        "init_complete=0",
        "PostgreSQL init process complete",
        "CREATE EXTENSION citus;\nCREATE EXTENSION pgcrypto;\n"
        "CREATE EXTENSION ai_blaise_citus VERSION '0.1.1';",
        "CREATE EXTENSION citus;\nCREATE EXTENSION pgcrypto;\n"
        "CREATE EXTENSION ai_blaise_citus VERSION '0.1.2';",
        "psql_db security_restore <<'SQL'\nCREATE EXTENSION citus;\n"
        "CREATE EXTENSION pgcrypto;",
    ):
        require_once(smoke, token, context)
    if (
        "postgres:" in smoke
        or "docker cp" in smoke
        or re.search(r"docker run[^\n]*\s-v\s", smoke)
        or re.search(r"docker run[^\n]*\s-p\s", smoke)
    ):
        fail(f"{context} must use only the shared immutable fixture")


def validate_observability_replication_smoke(smoke: str) -> None:
    context = "observability replication PG17 real-Citus smoke"
    for token in (
        'fixture_builder="${repo_root}/ci/ai-blaise/build-real-citus-test-fixture.sh"',
        'fixture_contract="${repo_root}/ci/ai-blaise/real-citus-test-fixture-contract.py"',
        "pg_major=17",
        "OBSERVABILITY_REPLICATION_SMOKE_IMAGE is retired; use source-verified CITUS_TEST_FIXTURE_IMAGE",
        'python3 "${fixture_contract}"',
        'fixture_image="$("${fixture_builder}" --pg-major "${pg_major}")"',
        'docker rm --force --volumes "${primary}" "${standby}"',
        'docker network create "${network}"',
        'docker run \\\n  --name "${primary}"',
        'docker run \\\n  --name "${standby}"',
        'grep -q "PostgreSQL init process complete"',
        "CREATE EXTENSION citus;",
        "CREATE EXTENSION pgcrypto;",
        "CREATE EXTENSION ai_blaise_citus;",
        'pg_basebackup -h \\"${primary}\\" -D \\"\\${PGDATA}\\" -U replicator -Fp -Xs -R --checkpoint=fast',
        'exec gosu postgres \\"\\$(pg_config --bindir)/postgres\\"',
        "pg_is_in_recovery()",
        "-c 'INSERT INTO observability_smoke VALUES (2);'",
        "replay_seen=0",
        "-c 'SELECT count(*) FROM observability_smoke WHERE value = 2;'",
        'if [[ "${replay_seen}" != "1" ]]; then',
    ):
        require_once(smoke, token, context)
    for token in (
        "companion_pg_stat_local_activity",
        "companion_pg_stat_distributed",
        "companion_pg_dist_replication_lag",
    ):
        if token not in smoke:
            fail(f"{context} must exercise {token}")
    if smoke.count('-d "${fixture_image}"') != 2:
        fail(f"{context} must run primary and standby from the same immutable fixture")
    if smoke.count("shared_preload_libraries=citus") != 2:
        fail(f"{context} must preload Citus in primary and standby")
    if re.search(r"rm\s+-rf\s+[^\n]*\$\{PGDATA\}", smoke):
        fail(f"{context} must let pg_basebackup reject a nonempty standby volume")
    if not (
        smoke.index("CREATE EXTENSION citus;")
        < smoke.index("CREATE EXTENSION pgcrypto;")
        < smoke.index("CREATE EXTENSION ai_blaise_citus;")
    ):
        fail(f"{context} must create Citus before companion prerequisites")
    if not (
        smoke.index("pg_basebackup")
        < smoke.index("pg_is_in_recovery()")
        < smoke.index("INSERT INTO observability_smoke VALUES (2)")
        < smoke.index("SELECT count(*) FROM observability_smoke WHERE value = 2")
    ):
        fail(f"{context} must prove a post-backup insert replays on the standby")
    primary_run = smoke.split('docker run \\\n  --name "${primary}"', 1)[1].split(
        ">/dev/null", 1
    )[0]
    standby_run = smoke.split('docker run \\\n  --name "${standby}"', 1)[1].split(
        ">/dev/null", 1
    )[0]
    if (
        re.search(
            r'(?m)^[A-Za-z_][A-Za-z0-9_]*="(?:docker\.io/library/)?postgres:[^"]+"$',
            smoke,
        )
        or '-d "postgres:' in smoke
        or "/usr/lib/postgresql/" in smoke
        or "/usr/share/postgresql/" in smoke
        or "docker cp" in smoke
        or re.search(r"(^|\s)-v\s", primary_run + standby_run)
        or re.search(r"(^|\s)-p\s", primary_run + standby_run)
    ):
        fail(f"{context} must use only the shared fixture on a private network")


def validate_shared_pg17_fixture_consumer(smoke: str, context: str) -> None:
    for token in (
        'fixture_builder="${repo_root}/ci/ai-blaise/build-real-citus-test-fixture.sh"',
        'fixture_contract="${repo_root}/ci/ai-blaise/real-citus-test-fixture-contract.py"',
        "pg_major=17",
        'python3 "${fixture_contract}"',
        'fixture_image="$("${fixture_builder}" --pg-major "${pg_major}")"',
        'docker rm --force --volumes "${container}"',
        "docker run \\",
        "--network none",
        '-d "${fixture_image}"',
        "init_complete=0",
        'grep -q "PostgreSQL init process complete"',
        "ready=0",
        "CREATE EXTENSION citus;",
        "CREATE EXTENSION pgcrypto;",
        "CREATE EXTENSION ai_blaise_citus;",
    ):
        require_once(smoke, token, context)
    if not (
        smoke.index("CREATE EXTENSION citus;")
        < smoke.index("CREATE EXTENSION pgcrypto;")
        < smoke.index("CREATE EXTENSION ai_blaise_citus;")
    ):
        fail(f"{context} must create Citus before companion prerequisites")
    if smoke.index("init_complete=0") > smoke.index("ready=0"):
        fail(f"{context} must wait for completed initdb before SQL readiness")
    docker_run = smoke.split("docker run \\", 1)[1].split(">/dev/null", 1)[0]
    if (
        "postgres:" in smoke
        or "/usr/share/postgresql/" in smoke
        or "MIGRATION_INVARIANTS_SMOKE_IMAGE" in smoke
        or "SQL_EXTENSION_SMOKE_IMAGE" in smoke
        or re.search(r"(^|\s)-v\s", docker_run)
        or re.search(r"(^|\s)-p\s", docker_run)
    ):
        fail(f"{context} must use only the shared immutable real-Citus fixture")


def validate_otel_smoke(smoke: str) -> None:
    context = "OTEL pool/sidecar real-Citus smoke"
    for token in (
        'fixture_builder="${repo_root}/ci/ai-blaise/build-real-citus-test-fixture.sh"',
        'fixture_contract="${repo_root}/ci/ai-blaise/real-citus-test-fixture-contract.py"',
        "pg_major=17",
        'python3 "${fixture_contract}"',
        'fixture_image="$("${fixture_builder}" --pg-major "${pg_major}")"',
        'docker rm --force --volumes "${container}"',
        '-p "127.0.0.1:${postgres_port}:5432"',
        'kind_fixture_image="ai-blaise-citus-test-fixture:kind-${kind_cluster}"',
        'docker image tag "${fixture_image}" "${kind_fixture_image}"',
        "docker image inspect --format '{{.Id}}' \"${kind_fixture_image}\"",
        'kind load docker-image "${kind_fixture_image}" --name "${kind_cluster}"',
        '--image="${kind_fixture_image}"',
        "--image-pull-policy=Never",
        'docker image rm "${kind_fixture_image}"',
        "kind_postgres_init_complete=0",
    ):
        require_once(smoke, token, context)
    if len(re.findall(r"(?m)^postgres_init_complete=0$", smoke)) != 1:
        fail(f"{context} must contain one default-mode initdb sentinel")
    if smoke.count('grep -q "PostgreSQL init process complete"') != 2:
        fail(f"{context} must await completed initdb in default and kind modes")
    extension_sequence = (
        "CREATE EXTENSION citus; CREATE EXTENSION pgcrypto; "
        "CREATE EXTENSION ai_blaise_citus;"
    )
    if smoke.count("CREATE EXTENSION citus;") != 2:
        fail(f"{context} must create Citus in both exercised databases")
    if smoke.count("CREATE EXTENSION pgcrypto;") != 2:
        fail(f"{context} must create pgcrypto in both exercised databases")
    if smoke.count("CREATE EXTENSION ai_blaise_citus;") != 2:
        fail(f"{context} must create the companion in both exercised databases")
    default_sequence = "\n".join(
        (
            "CREATE EXTENSION citus;",
            "CREATE EXTENSION pgcrypto;",
            "CREATE EXTENSION ai_blaise_citus;",
        )
    )
    if default_sequence not in smoke or extension_sequence not in smoke:
        fail(f"{context} must create Citus before companion prerequisites")
    database_run = smoke.split("docker run \\", 1)[1].split(">/dev/null", 1)[0]
    if (
        "postgres:" in smoke
        or "OTEL_SMOKE_POSTGRES_IMAGE" in smoke
        or "/usr/share/postgresql/" in smoke
        or re.search(r"(^|\s)-v\s", database_run)
        or re.search(r'(?m)^\s*-p\s+"(?!127\.0\.0\.1:)', smoke)
    ):
        fail(f"{context} must use only the immutable fixture and loopback publication")


def main() -> None:
    validate_base_lock()
    validate_http_package_lock()
    validate_dockerfile(read(DOCKERFILE))
    validate_http_dockerfile(read(HTTP_DOCKERFILE))
    validate_context_builder(read(CONTEXT_BUILDER))
    validate_builder(read(BUILDER))
    validate_http_builder(read(HTTP_BUILDER))
    validate_ai_sql_smoke(read(AI_SQL_SMOKE))
    validate_a10_a11_live_smoke(read(A10_A11_LIVE_SMOKE))
    validate_shared_pg17_fixture_consumer(
        read(MIGRATION_INVARIANTS_SMOKE), "migration invariants real-Citus smoke"
    )
    validate_shared_pg17_fixture_consumer(
        read(SCHEMA_JOB_SMOKE), "schema-job real-Citus smoke"
    )
    validate_otel_smoke(read(OTEL_SMOKE))
    validate_sql_extension_matrix_smoke(read(SQL_EXTENSION_SMOKE))
    validate_canary_upgrade_smoke(read(CANARY_UPGRADE_SMOKE))
    validate_security_backup_smoke(read(SECURITY_BACKUP_SMOKE))
    validate_observability_replication_smoke(read(OBSERVABILITY_REPLICATION_SMOKE))
    image_check = read(IMAGE_CHECK)
    image_workflow = read(IMAGE_WORKFLOW)
    sidecar_workflow = read(SIDECAR_WORKFLOW)
    observability_workflow = read(OBSERVABILITY_WORKFLOW)
    production_workflow = read(PRODUCTION_WORKFLOW)
    for token in (
        "real-citus-test-fixture-contract.py",
        "real-citus-test-fixture-contract_test.py",
        "materialize-real-citus-test-fixture.py",
        "build-real-citus-test-fixture.sh",
        "Dockerfile.http",
        "http-packages.lock.tsv",
        "build-real-citus-http-test-fixture.sh",
    ):
        if token not in image_check + image_workflow:
            fail(f"real-Citus fixture CI/static wiring missing: {token}")
    if (
        "REQUIRE_DOCKER=1 bash ci/ai-blaise/ai-sql-contract-smoke.sh"
        not in image_workflow
    ):
        fail("ci-image workflow must execute the migrated AI SQL smoke")
    if (
        "REQUIRE_DOCKER=1 bash ci/ai-blaise/a10-a11-ai-sql-live-smoke.sh"
        not in image_workflow
    ):
        fail("ci-image workflow must execute the A10/A11 real-Citus live smoke")
    if (
        "REQUIRE_DOCKER=1 bash ci/ai-blaise/migration-invariants-smoke.sh"
        not in image_workflow
    ):
        fail("ci-image workflow must execute migration invariants on real Citus")
    if (
        "REQUIRE_DOCKER=1 ci/ai-blaise/schema-job-f1-2vi-smoke.sh"
        not in sidecar_workflow
    ):
        fail("ci-sidecar workflow must execute schema-job SQL on real Citus")
    if (
        "bash ci/ai-blaise/otel-trace-propagation-smoke.sh"
        not in observability_workflow
    ):
        fail("observability workflow must execute OTEL propagation on real Citus")
    if (
        "REQUIRE_DOCKER=1 bash ci/ai-blaise/sql-extension-smoke.sh"
        not in image_workflow
    ):
        fail("ci-image workflow must execute the PG16/PG17/PG18 real-Citus matrix")
    if (
        "REQUIRE_DOCKER=1 bash ci/ai-blaise/observability-replication-smoke.sh"
        not in image_workflow
    ):
        fail("ci-image workflow must execute observability replication on real Citus")
    if "bash ci/ai-blaise/canary-upgrade-rollback-smoke.sh" not in production_workflow:
        fail("production workflow must execute the real-Citus canary matrix")
    if (
        "bash ci/ai-blaise/extension-security-backup-smoke.sh"
        not in production_workflow
        or "EXTENSION_SECURITY_IMAGE" in production_workflow
    ):
        fail(
            "production workflow must execute security restore through the fixture builder"
        )
    print("real-citus-test-fixture-contract passed")


if __name__ == "__main__":
    main()
