#!/usr/bin/env python3
"""Mutation regressions for the source-built real-Citus fixture contract."""

from __future__ import annotations

import contextlib
import csv
import hashlib
import importlib.util
import io
import json
import os
import struct
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
CHECK = ROOT / "ci/ai-blaise/real-citus-test-fixture-contract.py"
CONTEXT_BUILDER = ROOT / "ci/ai-blaise/materialize-real-citus-test-fixture.py"
BUILDER = ROOT / "ci/ai-blaise/build-real-citus-test-fixture.sh"
HTTP_BUILDER = ROOT / "ci/ai-blaise/build-real-citus-http-test-fixture.sh"
BASE_LOCK = ROOT / "images/citus-test-fixture/base-images.lock.tsv"


def load_materializer():
    specification = importlib.util.spec_from_file_location(
        "real_citus_fixture_materializer", CONTEXT_BUILDER
    )
    if specification is None or specification.loader is None:
        raise RuntimeError("real-Citus fixture materializer could not be imported")
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


class RealCitusFixtureContractTests(unittest.TestCase):
    def run_check(self, mutation=None):
        script = CHECK.read_text(encoding="utf-8")
        read_text = Path.read_text

        def fixture_read(path, *args, **kwargs):
            text = read_text(path, *args, **kwargs)
            if mutation and path.resolve() == (ROOT / mutation[0]).resolve():
                return text.replace(mutation[1], mutation[2])
            return text

        stdout, stderr = io.StringIO(), io.StringIO()
        previous = Path.cwd()
        try:
            os.chdir(ROOT)
            with (
                patch.object(Path, "read_text", fixture_read),
                contextlib.redirect_stdout(stdout),
                contextlib.redirect_stderr(stderr),
            ):
                try:
                    exec(
                        compile(script, str(CHECK), "exec"),
                        {"__name__": "__main__", "__file__": str(CHECK)},
                    )
                except SystemExit as error:
                    return error.code, stdout.getvalue(), stderr.getvalue()
        finally:
            os.chdir(previous)
        return 0, stdout.getvalue(), stderr.getvalue()

    def assert_mutation_fails(self, mutation, message):
        code, _, error = self.run_check(mutation)
        self.assertEqual(code, 1)
        self.assertIn(message, error)

    def test_current_source_contract(self):
        code, output, error = self.run_check()
        self.assertEqual(code, 0, error)
        self.assertIn("real-citus-test-fixture-contract passed", output)

    def test_builder_contract_only_command_boundary(self):
        result = subprocess.run(
            [
                "bash",
                "ci/ai-blaise/build-real-citus-test-fixture.sh",
                "--contract-only",
            ],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("real-citus-test-fixture-contract passed", result.stdout)

    def test_http_builder_contract_only_command_boundary(self):
        result = subprocess.run(
            ["bash", str(HTTP_BUILDER), "--contract-only"],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("real-citus-http-test-fixture-contract passed", result.stdout)

    def test_plain_citus_install_is_rejected(self):
        self.assert_mutation_fails(
            (
                "images/citus-test-fixture/Dockerfile",
                "make install-all DESTDIR=/out;",
                "make install DESTDIR=/out;",
            ),
            "must not omit downgrade SQL",
        )

    def test_install_fallback_is_rejected(self):
        self.assert_mutation_fails(
            (
                "images/citus-test-fixture/Dockerfile",
                "make install-all DESTDIR=/out;",
                "make install-all DESTDIR=/out || make install DESTDIR=/out;",
            ),
            "must not fall back",
        )

    def test_generated_sql_inventory_and_default_version_checks_are_required(self):
        mutations = (
            (
                "test \"$(sed -n \"s/^default_version = '\\([^']*\\)'$/\\1/p\" "
                '"${extension_dir}/citus.control")" = "${CITUS_EXTENSION_VERSION}";',
                'test -n "${CITUS_EXTENSION_VERSION}";',
            ),
            (
                "for sql_file in src/backend/distributed/build/sql/citus--*.sql "
                "src/backend/columnar/build/sql/citus_columnar--*.sql; do",
                "for sql_file in src/backend/distributed/build/sql/citus--*.sql; do",
            ),
            ('test -s "${sql_file}";', 'test -n "${sql_file}";'),
            (
                'cmp "${sql_file}" "${extension_dir}/$(basename "${sql_file}")";',
                'test -e "${extension_dir}/$(basename "${sql_file}")";',
            ),
        )
        for old, new in mutations:
            with self.subTest(old=old):
                self.assert_mutation_fails(
                    ("images/citus-test-fixture/Dockerfile", old, new),
                    "must contain exactly one occurrence",
                )

    def test_false_direct_default_version_install_assertion_is_rejected(self):
        self.assert_mutation_fails(
            (
                "images/citus-test-fixture/Dockerfile",
                'test -s "/out$(pg_config --sharedir)/extension/citus.control";',
                'test -s "/out$(pg_config --sharedir)/extension/citus.control"; '
                'test -s "/out$(pg_config --sharedir)/extension/'
                'citus--${CITUS_EXTENSION_VERSION}.sql";',
            ),
            "must validate the generated SQL inventory",
        )

    def test_mutable_base_image_is_rejected(self):
        self.assert_mutation_fails(
            (
                "images/citus-test-fixture/base-images.lock.tsv",
                "docker.io/library/postgres:17-bookworm@sha256:051f7b7b3abdd564d5d1bd1e8c4b9c1b6e77087d1dd22020ede611c096a272e0",
                "postgres:17-bookworm",
            ),
            "must be digest-pinned",
        )

    def test_fixture_base_lock_requires_all_three_postgresql_majors(self):
        for major, digest in (
            (
                "16",
                "bb3e1a57e5407e0a5280b4211980a5e537f4abd234a87014ac979849a78dd825",
            ),
            (
                "18",
                "1c59e2c3c818eaa0f0628f695b36e7c9e362d6b219b36a54a32df645cbd7e1af",
            ),
        ):
            with self.subTest(major=major):
                self.assert_mutation_fails(
                    (
                        "images/citus-test-fixture/base-images.lock.tsv",
                        f"{major}\tdocker.io/library/postgres:{major}-bookworm@sha256:{digest}\n",
                        "",
                    ),
                    "must cover PG16, PG17, and PG18 exactly",
                )

    def test_http_package_version_lock_is_exact(self):
        self.assert_mutation_fails(
            (
                "images/citus-test-fixture/http-packages.lock.tsv",
                "1.7.2-2.pgdg12+1",
                "1.7.2-latest",
            ),
            "must contain the reviewed PG17 package",
        )

    def test_http_wrapper_rejects_stock_parent_or_unpinned_package_install(self):
        for old, new in (
            (
                "FROM ${REAL_CITUS_FIXTURE_PARENT} AS companion-http-test-fixture",
                "FROM postgres:17 AS companion-http-test-fixture",
            ),
            (
                '"${PG_HTTP_PACKAGE}=${PG_HTTP_PACKAGE_VERSION}";',
                '"${PG_HTTP_PACKAGE}";',
            ),
        ):
            with self.subTest(new=new):
                self.assert_mutation_fails(
                    ("images/citus-test-fixture/Dockerfile.http", old, new),
                    "must contain exactly one occurrence",
                )

    def test_http_builder_rejects_floating_parent_tag_fallback(self):
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/build-real-citus-http-test-fixture.sh",
                'fixture_parent="docker.io/library/${fixture_tag}@${fixture_image_id}"',
                'fixture_parent="docker.io/library/${fixture_tag}@${fixture_image_id}"\n'
                'docker image tag "${fixture_image_id}" "${fixture_tag}"',
            ),
            "must not use a floating parent tag",
        )

    def test_http_builder_builds_the_prehashed_snapshot_after_checkout_drift(self):
        with tempfile.TemporaryDirectory(prefix="real-citus-http-snapshot-") as root:
            fixture_root = Path(root)
            (fixture_root / "ci/ai-blaise").mkdir(parents=True)
            (fixture_root / "images/citus-test-fixture").mkdir(parents=True)
            subprocess.run(["git", "init", "-q"], cwd=fixture_root, check=True)

            builder = (
                fixture_root / "ci/ai-blaise/build-real-citus-http-test-fixture.sh"
            )
            builder.write_bytes(HTTP_BUILDER.read_bytes())
            builder.chmod(0o755)
            contract = fixture_root / "ci/ai-blaise/real-citus-test-fixture-contract.py"
            contract.write_text("print('fixture contract passed')\n", encoding="utf-8")
            base_builder = (
                fixture_root / "ci/ai-blaise/build-real-citus-test-fixture.sh"
            )
            base_builder.write_text(
                "#!/usr/bin/env bash\nprintf '%s\\n' \"${FAKE_BASE_IMAGE_ID}\"\n",
                encoding="utf-8",
            )
            base_builder.chmod(0o755)

            dockerfile = fixture_root / "images/citus-test-fixture/Dockerfile.http"
            original_dockerfile = (
                ROOT / "images/citus-test-fixture/Dockerfile.http"
            ).read_bytes()
            dockerfile.write_bytes(original_dockerfile)
            package_lock = (
                fixture_root / "images/citus-test-fixture/http-packages.lock.tsv"
            )
            package_lock.write_bytes(
                (ROOT / "images/citus-test-fixture/http-packages.lock.tsv").read_bytes()
            )

            base_image_id = f"sha256:{'a' * 64}"
            fixture_id = "b" * 64
            http_image_id = f"sha256:{'c' * 64}"
            package_name = "postgresql-17-http"
            package_version = "1.7.2-2.pgdg12+1"
            dockerfile_sha256 = hashlib.sha256(original_dockerfile).hexdigest()
            digest = hashlib.sha256(b"ai-blaise/real-citus-http-test-fixture/v1\0")
            for value in (
                "17",
                base_image_id,
                fixture_id,
                package_name,
                package_version,
                dockerfile_sha256,
            ):
                encoded = value.encode("utf-8")
                digest.update(struct.pack(">Q", len(encoded)))
                digest.update(encoded)
            http_fixture_id = digest.hexdigest()
            http_labels = {
                "ai-blaise.citus.test-fixture.http": "true",
                "ai-blaise.citus.test-fixture.release-target": "false",
                "ai-blaise.citus.test-fixture.pg-major": "17",
                "ai-blaise.citus.test-fixture.http-package": package_name,
                "ai-blaise.citus.test-fixture.http-package-version": package_version,
                "ai-blaise.citus.test-fixture.http-parent-image-id": base_image_id,
                "ai-blaise.citus.test-fixture.http-parent-fixture-id": fixture_id,
                "ai-blaise.citus.test-fixture.http-id": http_fixture_id,
            }

            fake_bin = fixture_root / "bin"
            fake_bin.mkdir()
            docker = fake_bin / "docker"
            docker.write_text(
                r"""#!/usr/bin/env python3
import hashlib
import json
import os
from pathlib import Path
import re
import sys

args = sys.argv[1:]
base_id = os.environ["FAKE_BASE_IMAGE_ID"]
fixture_id = os.environ["FAKE_FIXTURE_ID"]
fixture_tag = f"ai-blaise-citus-test-fixture:pg17-{fixture_id}"
fixture_parent = f"docker.io/library/{fixture_tag}@{base_id}"
http_id = os.environ["FAKE_HTTP_IMAGE_ID"]
built = Path(os.environ["FAKE_BUILT_MARKER"])
checkout = Path(os.environ["FAKE_CHECKOUT_DOCKERFILE"])

if args[:2] == ["image", "inspect"]:
    target = args[-1]
    if "--format" not in args:
        if target.startswith("ai-blaise-citus-http-test-fixture:"):
            checkout.write_bytes(checkout.read_bytes() + b"# concurrent checkout drift\n")
            raise SystemExit(0 if built.exists() else 1)
        raise SystemExit(0)
    template = args[args.index("--format") + 1]
    if template == "{{.Id}}":
        if target in {base_id, fixture_tag, fixture_parent}:
            print(base_id)
            raise SystemExit(0)
        if target.startswith("ai-blaise-citus-http-test-fixture:") and built.exists():
            print(http_id)
            raise SystemExit(0)
        raise SystemExit(40)
    match = re.fullmatch(r'{{ index \.Config\.Labels "([^"]+)" }}', template)
    if match is None:
        raise SystemExit(41)
    label = match.group(1)
    if target == base_id and label == "ai-blaise.citus.test-fixture.id":
        print(fixture_id)
        raise SystemExit(0)
    if target == http_id:
        print(json.loads(os.environ["FAKE_HTTP_LABELS"])[label])
        raise SystemExit(0)
    raise SystemExit(42)

if args and args[0] == "build":
    dockerfile = Path(args[args.index("-f") + 1])
    observed_sha = hashlib.sha256(dockerfile.read_bytes()).hexdigest()
    if observed_sha != os.environ["FAKE_ORIGINAL_DOCKERFILE_SHA"]:
        raise SystemExit(50)
    build_args = {}
    for index, value in enumerate(args):
        if value == "--build-arg":
            key, argument = args[index + 1].split("=", 1)
            build_args[key] = argument
    if build_args.get("AI_BLAISE_HTTP_FIXTURE_ID") != os.environ["FAKE_HTTP_FIXTURE_ID"]:
        raise SystemExit(51)
    if build_args.get("REAL_CITUS_FIXTURE_PARENT") != fixture_parent:
        raise SystemExit(52)
    Path(os.environ["FAKE_PROOF_MARKER"]).write_text("snapshot-bound\n", encoding="utf-8")
    built.write_text("built\n", encoding="utf-8")
    raise SystemExit(0)

raise SystemExit(60)
""",
                encoding="utf-8",
            )
            docker.chmod(0o755)

            built_marker = fixture_root / "built"
            proof_marker = fixture_root / "proof"
            tmpdir = fixture_root / "tmp"
            tmpdir.mkdir()
            environment = os.environ.copy()
            environment.update(
                {
                    "PATH": f"{fake_bin}{os.pathsep}{environment['PATH']}",
                    "TMPDIR": str(tmpdir),
                    "FAKE_BASE_IMAGE_ID": base_image_id,
                    "FAKE_FIXTURE_ID": fixture_id,
                    "FAKE_HTTP_IMAGE_ID": http_image_id,
                    "FAKE_HTTP_FIXTURE_ID": http_fixture_id,
                    "FAKE_HTTP_LABELS": json.dumps(http_labels, sort_keys=True),
                    "FAKE_ORIGINAL_DOCKERFILE_SHA": dockerfile_sha256,
                    "FAKE_CHECKOUT_DOCKERFILE": str(dockerfile),
                    "FAKE_BUILT_MARKER": str(built_marker),
                    "FAKE_PROOF_MARKER": str(proof_marker),
                }
            )
            result = subprocess.run(
                ["bash", str(builder), "--pg-major", "17"],
                cwd=fixture_root,
                env=environment,
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout, f"{http_image_id}\n")
            self.assertEqual(
                proof_marker.read_text(encoding="utf-8"), "snapshot-bound\n"
            )
            self.assertIn(b"concurrent checkout drift", dockerfile.read_bytes())
            self.assertEqual(list(tmpdir.iterdir()), [])

    def test_source_content_fingerprint_is_required(self):
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/build-real-citus-test-fixture.sh",
                'python3 "${context_builder}"',
                'printf "not-a-source-fingerprint"',
            ),
            "must contain exactly one occurrence",
        )

    def test_contract_output_cannot_contaminate_immutable_image_id(self):
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/build-real-citus-test-fixture.sh",
                'python3 "${contract_check}" >&2',
                'python3 "${contract_check}"',
            ),
            "must contain exactly one occurrence",
        )

    def test_cache_labels_are_not_verified_through_mutable_tag(self):
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/build-real-citus-test-fixture.sh",
                '"{{ index .Config.Labels \\"${label}\\" }}" "${image_id}"',
                '"{{ index .Config.Labels \\"${label}\\" }}" "${image}"',
            ),
            "must be verified through immutable image ID",
        )

    def test_cache_hit_preserves_well_formed_build_provenance_after_metadata_change(
        self,
    ):
        materializer = load_materializer()
        with tempfile.TemporaryDirectory(prefix="real-citus-cache-boundary-") as root:
            fixture_root = Path(root)
            context = fixture_root / "context"
            context.mkdir()
            source_content_sha256 = materializer.materialize(ROOT, context)

            with BASE_LOCK.open(encoding="utf-8", newline="") as handle:
                base_rows = list(csv.DictReader(handle, delimiter="\t"))
            base_image = next(
                row["base_image"] for row in base_rows if row["pg_major"] == "17"
            )
            control = (context / "src/backend/distributed/citus.control").read_text(
                encoding="utf-8"
            )
            version_lines = [
                line
                for line in control.splitlines()
                if line.startswith("default_version = ")
            ]
            self.assertEqual(len(version_lines), 1)
            citus_version = version_lines[0].split("'", 2)[1]
            fixture_identity = hashlib.sha256(
                "\0".join(
                    ("17", base_image, citus_version, source_content_sha256)
                ).encode()
            ).hexdigest()

            image_id = f"sha256:{'a' * 64}"
            prior_git_sha = "1" * 40
            labels = {
                "ai-blaise.citus.test-fixture": "true",
                "ai-blaise.citus.test-fixture.scope": (
                    "source-built-companion-test-only"
                ),
                "ai-blaise.citus.test-fixture.release-target": "false",
                "ai-blaise.citus.test-fixture.pg-major": "17",
                "ai-blaise.citus.test-fixture.base-image": base_image,
                "ai-blaise.citus.test-fixture.citus-extension-version": citus_version,
                "ai-blaise.citus.test-fixture.id": fixture_identity,
                "ai-blaise.citus.source-content-sha256": source_content_sha256,
                "ai-blaise.citus.source-git-sha": prior_git_sha,
                "ai-blaise.citus.source-git-tree": "2" * 40,
                "ai-blaise.citus.source-tree-state": "clean",
                "org.opencontainers.image.revision": prior_git_sha,
            }

            fake_bin = fixture_root / "bin"
            fake_bin.mkdir()
            docker = fake_bin / "docker"
            docker.write_text(
                r"""#!/usr/bin/env python3
import json
import os
import re
import sys

args = sys.argv[1:]
if args[:2] == ["image", "inspect"]:
    if "--format" not in args:
        print("{}")
        raise SystemExit(0)
    template = args[args.index("--format") + 1]
    target = args[-1]
    if template == "{{.Id}}":
        if target != os.environ["FAKE_IMAGE_REF"]:
            raise SystemExit(91)
        print(os.environ["FAKE_IMAGE_ID"])
        raise SystemExit(0)
    if target != os.environ["FAKE_IMAGE_ID"]:
        raise SystemExit(92)
    match = re.fullmatch(r'{{ index \.Config\.Labels "([^"]+)" }}', template)
    if match is None:
        raise SystemExit(93)
    print(json.loads(os.environ["FAKE_LABELS"])[match.group(1)])
    raise SystemExit(0)
if args and args[0] == "build":
    open(os.environ["FAKE_BUILD_MARKER"], "w", encoding="utf-8").close()
    raise SystemExit(94)
raise SystemExit(95)
""",
                encoding="utf-8",
            )
            docker.chmod(0o755)
            build_marker = fixture_root / "unexpected-build"
            temporary_parent = fixture_root / "tmp"
            temporary_parent.mkdir()
            image_ref = "ai-blaise-fixture-cache-regression:test"
            environment = os.environ.copy()
            environment.update(
                {
                    "PATH": f"{fake_bin}{os.pathsep}{environment['PATH']}",
                    "TMPDIR": str(temporary_parent),
                    "CITUS_TEST_FIXTURE_IMAGE": image_ref,
                    "FAKE_IMAGE_REF": image_ref,
                    "FAKE_IMAGE_ID": image_id,
                    "FAKE_LABELS": json.dumps(labels, sort_keys=True),
                    "FAKE_BUILD_MARKER": str(build_marker),
                }
            )
            result = subprocess.run(
                ["bash", str(BUILDER), "--pg-major", "17"],
                cwd=ROOT,
                env=environment,
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout, f"{image_id}\n")
            self.assertFalse(build_marker.exists())

    def test_materialized_context_is_required(self):
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/build-real-citus-test-fixture.sh",
                '"${fixture_context}" >&2',
                '"${repo_root}" >&2',
            ),
            "must not send the mutable worktree",
        )

    def test_stock_postgres_fallback_is_rejected(self):
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/ai-sql-contract-smoke.sh",
                'fixture_image="$("${fixture_builder}" --pg-major "${pg_major}"',
                'fixture_image="postgres:17" # ',
            ),
            "must contain exactly one occurrence",
        )

    def test_host_mount_or_published_port_is_rejected(self):
        for injected in (
            '  -v "${control_file}:/tmp/control:ro" \\\n  -d "${fixture_image}"',
            '  -p "0.0.0.0:5432:5432" \\\n  -d "${fixture_image}"',
        ):
            with self.subTest(injected=injected):
                self.assert_mutation_fails(
                    (
                        "ci/ai-blaise/ai-sql-contract-smoke.sh",
                        '  -d "${fixture_image}"',
                        injected,
                    ),
                    "must not fall back to a stock PostgreSQL fixture",
                )

    def test_additional_shared_fixture_consumers_reject_stock_fallbacks(self):
        for path in (
            "ci/ai-blaise/migration-invariants-smoke.sh",
            "ci/ai-blaise/schema-job-f1-2vi-smoke.sh",
        ):
            with self.subTest(path=path):
                self.assert_mutation_fails(
                    (
                        path,
                        'fixture_image="$("${fixture_builder}" --pg-major "${pg_major}")"',
                        'fixture_image="$("${fixture_builder}" --pg-major "${pg_major}")"\nstock_fallback="postgres:17"',
                    ),
                    "must use only the shared immutable real-Citus fixture",
                )

    def test_additional_shared_fixture_consumers_reject_host_mounts(self):
        for path in (
            "ci/ai-blaise/migration-invariants-smoke.sh",
            "ci/ai-blaise/schema-job-f1-2vi-smoke.sh",
        ):
            with self.subTest(path=path):
                self.assert_mutation_fails(
                    (
                        path,
                        '  -d "${fixture_image}"',
                        '  -v "${repo_root}:/untrusted:ro" \\\n  -d "${fixture_image}"',
                    ),
                    "must use only the shared immutable real-Citus fixture",
                )

    def test_additional_shared_fixture_consumers_require_citus_first(self):
        expected = "\n".join(
            (
                "CREATE EXTENSION citus;",
                "CREATE EXTENSION pgcrypto;",
                "CREATE EXTENSION ai_blaise_citus;",
            )
        )
        reversed_order = "\n".join(
            (
                "CREATE EXTENSION ai_blaise_citus;",
                "CREATE EXTENSION pgcrypto;",
                "CREATE EXTENSION citus;",
            )
        )
        for path in (
            "ci/ai-blaise/migration-invariants-smoke.sh",
            "ci/ai-blaise/schema-job-f1-2vi-smoke.sh",
        ):
            with self.subTest(path=path):
                self.assert_mutation_fails(
                    (path, expected, reversed_order),
                    "must create Citus before companion prerequisites",
                )

    def test_additional_shared_fixture_consumers_wait_for_completed_initdb(self):
        for path in (
            "ci/ai-blaise/migration-invariants-smoke.sh",
            "ci/ai-blaise/schema-job-f1-2vi-smoke.sh",
        ):
            with self.subTest(path=path):
                self.assert_mutation_fails(
                    (path, "init_complete=0", "init_finished=0"),
                    "must contain exactly one occurrence",
                )

    def test_additional_shared_fixture_workflow_wiring_is_required(self):
        for mutation, message in (
            (
                (
                    ".github/workflows/ci-image.yml",
                    "REQUIRE_DOCKER=1 bash ci/ai-blaise/migration-invariants-smoke.sh",
                    "echo migration-invariants-smoke-disabled",
                ),
                "ci-image workflow must execute migration invariants on real Citus",
            ),
            (
                (
                    ".github/workflows/ci-sidecar.yml",
                    "REQUIRE_DOCKER=1 ci/ai-blaise/schema-job-f1-2vi-smoke.sh",
                    "echo schema-job-f1-2vi-smoke-disabled",
                ),
                "ci-sidecar workflow must execute schema-job SQL on real Citus",
            ),
        ):
            with self.subTest(path=mutation[0]):
                self.assert_mutation_fails(mutation, message)

    def test_a10_live_smoke_rejects_stock_mounts_runtime_install_and_publish(self):
        continuation = "\\"
        mutations = (
            (
                'fixture_image="$("${http_fixture_builder}" --pg-major "${pg_major}")"',
                'fixture_image="$("${http_fixture_builder}" --pg-major "${pg_major}")"\n'
                'stock_fallback="postgres:17"',
            ),
            (
                '  -d "${fixture_image}" >/dev/null',
                '  -v "${repo_root}:/untrusted:ro" '
                + continuation
                + '\n  -d "${fixture_image}" >/dev/null',
            ),
            (
                'log "booting immutable real-Citus PG17 HTTP fixture"',
                'log "booting immutable real-Citus PG17 HTTP fixture"\n'
                "apt-get install postgresql-17-http",
            ),
            (
                "  -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=app " + continuation,
                "  -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=app "
                + continuation
                + '\n  -p "0.0.0.0:5432:5432" '
                + continuation,
            ),
        )
        for old, new in mutations:
            with self.subTest(new=new):
                self.assert_mutation_fails(
                    ("ci/ai-blaise/a10-a11-ai-sql-live-smoke.sh", old, new),
                    "must use its immutable HTTP fixture",
                )

    def test_a10_live_smoke_requires_order_and_completed_initdb(self):
        extension_sequence = "\n".join(
            (
                "CREATE EXTENSION citus;",
                "CREATE EXTENSION pgcrypto;",
                "CREATE EXTENSION http;",
                "CREATE EXTENSION ai_blaise_citus;",
            )
        )
        companion_first = "\n".join(
            (
                "CREATE EXTENSION ai_blaise_citus;",
                "CREATE EXTENSION pgcrypto;",
                "CREATE EXTENSION http;",
                "CREATE EXTENSION citus;",
            )
        )
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/a10-a11-ai-sql-live-smoke.sh",
                extension_sequence,
                companion_first,
            ),
            "must create Citus before HTTP and the companion",
        )
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/a10-a11-ai-sql-live-smoke.sh",
                "postgres_init_complete=0",
                "postgres_init_finished=0",
            ),
            "must contain exactly one occurrence",
        )

    def test_a10_live_smoke_requires_digest_pinned_mock_image(self):
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/a10-a11-ai-sql-live-smoke.sh",
                "docker.io/library/python:3.12-slim@sha256:"
                "78387bc3881b8273120a12ebe6c1ab22b018ccc2c9adf565ae1ac9b536e184ea",
                "python:3.12-slim",
            ),
            "must contain exactly one occurrence",
        )

    def test_a10_live_smoke_requires_fail_closed_portable_evidence_identity(self):
        mutations = (
            (
                "if ! observed_at=\"$(date -u '+%Y-%m-%dT%H:%M:%SZ')\"; then",
                'observed_at="$(date -Is)"',
            ),
            (
                "if ! git_sha=\"$(git rev-parse --verify 'HEAD^{commit}')\"; then",
                'git_sha="$(git rev-parse HEAD)"',
            ),
            (
                'if [[ ! "${observed_at}" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T'
                "[0-9]{2}:[0-9]{2}:[0-9]{2}Z$ ]]; then",
                'if [[ -z "${observed_at}" ]]; then',
            ),
        )
        for old, new in mutations:
            with self.subTest(new=new):
                self.assert_mutation_fails(
                    ("ci/ai-blaise/a10-a11-ai-sql-live-smoke.sh", old, new),
                    "must contain exactly one occurrence",
                )

    def test_a10_live_workflow_wiring_is_required(self):
        self.assert_mutation_fails(
            (
                ".github/workflows/ci-image.yml",
                "REQUIRE_DOCKER=1 bash ci/ai-blaise/a10-a11-ai-sql-live-smoke.sh",
                "echo a10-a11-real-citus-live-smoke-disabled",
            ),
            "ci-image workflow must execute the A10/A11 real-Citus live smoke",
        )

    def test_otel_smoke_rejects_stock_postgres_and_host_extension_mounts(self):
        for old, new in (
            (
                'fixture_image="$("${fixture_builder}" --pg-major "${pg_major}")"',
                'fixture_image="$("${fixture_builder}" --pg-major "${pg_major}")"\n'
                'stock_fallback="postgres:17"',
            ),
            (
                '  -d "${fixture_image}" >/dev/null',
                '  -v "${repo_root}:/untrusted:ro" \\\n'
                '  -d "${fixture_image}" >/dev/null',
            ),
        ):
            with self.subTest(new=new):
                self.assert_mutation_fails(
                    ("ci/ai-blaise/otel-trace-propagation-smoke.sh", old, new),
                    "must use only the immutable fixture",
                )

    def test_otel_smoke_rejects_wildcard_postgres_publication(self):
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/otel-trace-propagation-smoke.sh",
                '-p "127.0.0.1:${postgres_port}:5432"',
                '-p "0.0.0.0:${postgres_port}:5432"',
            ),
            "must contain exactly one occurrence",
        )

    def test_otel_smoke_requires_both_database_extension_orders(self):
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/otel-trace-propagation-smoke.sh",
                "CREATE EXTENSION citus; CREATE EXTENSION pgcrypto; "
                "CREATE EXTENSION ai_blaise_citus;",
                "CREATE EXTENSION ai_blaise_citus; CREATE EXTENSION pgcrypto; "
                "CREATE EXTENSION citus;",
            ),
            "must create Citus before companion prerequisites",
        )

    def test_otel_kind_mode_requires_exact_immutable_fixture_load(self):
        for old, new in (
            (
                'kind load docker-image "${kind_fixture_image}" --name "${kind_cluster}"',
                "echo fixture-load-skipped",
            ),
            ("--image-pull-policy=Never", "--image-pull-policy=IfNotPresent"),
        ):
            with self.subTest(old=old):
                self.assert_mutation_fails(
                    ("ci/ai-blaise/otel-trace-propagation-smoke.sh", old, new),
                    "must contain exactly one occurrence",
                )

    def test_otel_workflow_wiring_is_required(self):
        self.assert_mutation_fails(
            (
                ".github/workflows/ci-observability-contracts.yml",
                "bash ci/ai-blaise/otel-trace-propagation-smoke.sh",
                "echo otel-real-citus-smoke-disabled",
            ),
            "observability workflow must execute OTEL propagation on real Citus",
        )

    def test_observability_replication_rejects_stock_mounts_and_publication(self):
        mutations = (
            (
                'fixture_image="$("${fixture_builder}" --pg-major "${pg_major}")"',
                'fixture_image="$("${fixture_builder}" --pg-major "${pg_major}")"\n'
                'stock_fallback="postgres:17"',
            ),
            (
                '  -e POSTGRES_PASSWORD=postgres \\\n  -d "${fixture_image}"',
                "  -e POSTGRES_PASSWORD=postgres \\\n"
                '  -v "${repo_root}:/untrusted:ro" \\\n'
                '  -d "${fixture_image}"',
            ),
            (
                '  --network "${network}" \\\n  -e POSTGRES_PASSWORD=postgres',
                '  --network "${network}" \\\n'
                '  -p "0.0.0.0:5432:5432" \\\n'
                "  -e POSTGRES_PASSWORD=postgres",
            ),
        )
        for old, new in mutations:
            with self.subTest(new=new):
                self.assert_mutation_fails(
                    ("ci/ai-blaise/observability-replication-smoke.sh", old, new),
                    "must use only the shared fixture on a private network",
                )

    def test_observability_replication_requires_both_citus_preloads(self):
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/observability-replication-smoke.sh",
                '  -c shared_preload_libraries=citus\n" >/dev/null',
                '" >/dev/null',
            ),
            "must preload Citus in primary and standby",
        )

    def test_observability_replication_requires_post_backup_replay(self):
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/observability-replication-smoke.sh",
                "-R --checkpoint=fast",
                "-R",
            ),
            "must contain exactly one occurrence",
        )
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/observability-replication-smoke.sh",
                "-c 'SELECT count(*) FROM observability_smoke WHERE value = 2;'",
                "-c 'SELECT 1;'",
            ),
            "must contain exactly one occurrence",
        )
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/observability-replication-smoke.sh",
                'pg_basebackup -h \\"${primary}\\"',
                'rm -rf \\"\\${PGDATA}\\"/*\npg_basebackup -h \\"${primary}\\"',
            ),
            "must let pg_basebackup reject a nonempty standby volume",
        )

    def test_observability_replication_requires_safe_binary_and_extension_order(self):
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/observability-replication-smoke.sh",
                'exec gosu postgres \\"\\$(pg_config --bindir)/postgres\\"',
                "exec gosu postgres /usr/lib/postgresql/17/bin/postgres",
            ),
            "must contain exactly one occurrence",
        )
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/observability-replication-smoke.sh",
                "CREATE EXTENSION citus;\nCREATE EXTENSION pgcrypto;\n"
                "CREATE EXTENSION ai_blaise_citus;",
                "CREATE EXTENSION ai_blaise_citus;\nCREATE EXTENSION pgcrypto;\n"
                "CREATE EXTENSION citus;",
            ),
            "must create Citus before companion prerequisites",
        )

    def test_observability_replication_requires_volume_cleanup_and_workflow(self):
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/observability-replication-smoke.sh",
                'docker rm --force --volumes "${primary}" "${standby}"',
                'docker rm --force "${primary}" "${standby}"',
            ),
            "must contain exactly one occurrence",
        )
        self.assert_mutation_fails(
            (
                ".github/workflows/ci-image.yml",
                "REQUIRE_DOCKER=1 bash ci/ai-blaise/observability-replication-smoke.sh",
                "echo observability-replication-smoke-disabled",
            ),
            "ci-image workflow must execute observability replication on real Citus",
        )

    def test_sql_extension_matrix_requires_real_citus_for_all_majors(self):
        mutations = (
            (
                'pg_majors_default="16 17 18"',
                'pg_majors_default="17 18"',
            ),
            (
                'fixture_image="$("${fixture_builder}" --pg-major "${pg_major}")"',
                'fixture_image="postgres:${pg_major}"',
            ),
            (
                'postgres_args=(-c "shared_preload_libraries=citus,pg_stat_statements")',
                'postgres_args=(-c "shared_preload_libraries=pg_stat_statements")',
            ),
            ("    --network none \\\n", '    -v "${repo_root}:/untrusted:ro" \\\n'),
        )
        for old, new in mutations:
            with self.subTest(new=new):
                self.assert_mutation_fails(
                    ("ci/ai-blaise/sql-extension-smoke.sh", old, new),
                    "SQL extension PG16/PG17/PG18 real-Citus smoke",
                )

    def test_sql_extension_matrix_requires_citus_before_the_companion(self):
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/sql-extension-smoke.sh",
                "CREATE EXTENSION citus;\nCREATE EXTENSION pg_stat_statements;\n"
                "CREATE EXTENSION pgcrypto;",
                "CREATE EXTENSION pg_stat_statements;\nCREATE EXTENSION pgcrypto;\n"
                "CREATE EXTENSION citus;",
            ),
            "must create Citus before companion prerequisites",
        )

    def test_sql_extension_matrix_requires_real_distribution_not_a_citus_stub(self):
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/sql-extension-smoke.sh",
                "CREATE FUNCTION create_hypertable(",
                "CREATE FUNCTION create_distributed_table(regclass, text) "
                "RETURNS void LANGUAGE plpgsql AS $$ BEGIN END $$;\n\n"
                "CREATE FUNCTION create_hypertable(",
            ),
            "must not replace real Citus distribution with a stub",
        )
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/sql-extension-smoke.sh",
                "FROM pg_catalog.pg_dist_shard",
                "FROM pg_catalog.pg_class",
            ),
            "must preserve the real distribution and RLS contract",
        )

    def test_sql_extension_matrix_preserves_narrow_rls_helper_grants(self):
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/sql-extension-smoke.sh",
                "GRANT EXECUTE ON FUNCTION companion_tenant_id_matches(text),",
                "-- runtime helper grant removed",
            ),
            "must preserve the real distribution and RLS contract",
        )
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/sql-extension-smoke.sh",
                "Sec1 RLS runtime role must not receive claim mutation authority",
                "claim mutation authority was not checked",
            ),
            "must preserve the real distribution and RLS contract",
        )

    def test_canary_matrix_rejects_stock_mounts_and_wrong_extension_order(self):
        extension_sequence = (
            "CREATE EXTENSION citus;\nCREATE EXTENSION pgcrypto;\n"
            "CREATE EXTENSION ai_blaise_citus VERSION '0.1.0';"
        )
        mutations = (
            (
                'fixture_image="$("${fixture_builder}" --pg-major "${pg_major}")"',
                'fixture_image="postgres:${pg_major}"',
            ),
            (
                "    --network none \\\n",
                '    -v "${repo_root}:/untrusted:ro" \\\n',
            ),
            (
                extension_sequence,
                "CREATE EXTENSION ai_blaise_citus VERSION '0.1.0';\n"
                "CREATE EXTENSION pgcrypto;\nCREATE EXTENSION citus;",
            ),
        )
        for old, new in mutations:
            with self.subTest(new=new):
                self.assert_mutation_fails(
                    ("ci/ai-blaise/canary-upgrade-rollback-smoke.sh", old, new),
                    "canary upgrade PG17/PG18 real-Citus smoke",
                )

    def test_security_restore_rejects_stock_copy_and_missing_citus(self):
        mutations = (
            (
                'fixture_image="$("${fixture_builder}" --pg-major "${pg_major}")"',
                'fixture_image="postgres:${pg_major}"',
            ),
            (
                'container="$(docker run --network none -d -e '
                'POSTGRES_HOST_AUTH_METHOD=trust "${fixture_image}")"',
                'docker cp "${repo_root}/untrusted.sql" "${container}:/tmp/untrusted.sql"\n'
                'container="$(docker run --network none -d -e '
                'POSTGRES_HOST_AUTH_METHOD=trust "${fixture_image}")"',
            ),
            (
                "psql_db security_restore <<'SQL'\nCREATE EXTENSION citus;\n"
                "CREATE EXTENSION pgcrypto;",
                "psql_db security_restore <<'SQL'\nCREATE EXTENSION pgcrypto;",
            ),
        )
        for old, new in mutations:
            with self.subTest(new=new):
                self.assert_mutation_fails(
                    ("ci/ai-blaise/extension-security-backup-smoke.sh", old, new),
                    "security backup/restore PG17/PG18 real-Citus smoke",
                )

    def test_security_restore_workflow_must_not_supply_a_stock_image(self):
        self.assert_mutation_fails(
            (
                ".github/workflows/ci-production-readiness.yml",
                "EXTENSION_SECURITY_PG_MAJOR: ${{ matrix.major }}",
                "EXTENSION_SECURITY_PG_MAJOR: ${{ matrix.major }}\n"
                "          EXTENSION_SECURITY_IMAGE: postgres:17",
            ),
            "production workflow must execute security restore through the fixture builder",
        )

    def test_retired_stock_image_overrides_fail_at_the_real_command_boundary(self):
        cases = (
            (
                "ci/ai-blaise/sql-extension-smoke.sh",
                {"SQL_EXTENSION_SMOKE_IMAGE": "postgres:17"},
                "SQL_EXTENSION_SMOKE_IMAGE is retired",
            ),
            (
                "ci/ai-blaise/canary-upgrade-rollback-smoke.sh",
                {"CANARY_UPGRADE_IMAGE": "postgres:17"},
                "CANARY_UPGRADE_IMAGE overrides are retired",
            ),
            (
                "ci/ai-blaise/extension-security-backup-smoke.sh",
                {
                    "EXTENSION_SECURITY_PG_MAJOR": "17",
                    "EXTENSION_SECURITY_IMAGE": "postgres:17",
                },
                "EXTENSION_SECURITY_IMAGE is retired",
            ),
            (
                "ci/ai-blaise/observability-replication-smoke.sh",
                {"OBSERVABILITY_REPLICATION_SMOKE_IMAGE": "postgres:17"},
                "OBSERVABILITY_REPLICATION_SMOKE_IMAGE is retired",
            ),
            (
                "ci/ai-blaise/sql-extension-smoke.sh",
                {"CITUS_TEST_FIXTURE_IMAGE": f"sha256:{'1' * 64}"},
                "CITUS_TEST_FIXTURE_IMAGE requires exactly one",
            ),
            (
                "ci/ai-blaise/canary-upgrade-rollback-smoke.sh",
                {"CITUS_TEST_FIXTURE_IMAGE": f"sha256:{'1' * 64}"},
                "CITUS_TEST_FIXTURE_IMAGE requires one explicit",
            ),
        )
        for script, additions, message in cases:
            with self.subTest(script=script):
                environment = os.environ.copy()
                environment.update(additions)
                result = subprocess.run(
                    ["bash", script],
                    cwd=ROOT,
                    env=environment,
                    check=False,
                    capture_output=True,
                    text=True,
                )
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(message, result.stderr)

    def test_retired_stock_image_override_rejections_are_mandatory(self):
        for path, old in (
            (
                "ci/ai-blaise/sql-extension-smoke.sh",
                "SQL_EXTENSION_SMOKE_IMAGE is retired; use source-verified CITUS_TEST_FIXTURE_IMAGE",
            ),
            (
                "ci/ai-blaise/canary-upgrade-rollback-smoke.sh",
                "CANARY_UPGRADE_IMAGE overrides are retired; use source-verified CITUS_TEST_FIXTURE_IMAGE",
            ),
            (
                "ci/ai-blaise/extension-security-backup-smoke.sh",
                "EXTENSION_SECURITY_IMAGE is retired; use source-verified CITUS_TEST_FIXTURE_IMAGE",
            ),
            (
                "ci/ai-blaise/observability-replication-smoke.sh",
                "OBSERVABILITY_REPLICATION_SMOKE_IMAGE is retired; use source-verified CITUS_TEST_FIXTURE_IMAGE",
            ),
        ):
            with self.subTest(path=path):
                self.assert_mutation_fails(
                    (path, old, "legacy stock override unexpectedly accepted"),
                    "must contain exactly one occurrence",
                )

    def test_context_fingerprint_binds_bytes_modes_and_symlinks(self):
        module = load_materializer()

        with tempfile.TemporaryDirectory(prefix="real-citus-context-test-") as root:
            fixture_root = Path(root)
            source = fixture_root / "source"
            source.mkdir()
            executable = source / "configure"
            executable.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
            executable.chmod(0o755)
            tree = source / "tree"
            tree.mkdir()
            value = tree / "value.txt"
            value.write_text("first\n", encoding="utf-8")
            (tree / "value-link").symlink_to("value.txt")
            local_source = tree / "local-source.c"
            local_source.write_text("int local_source;\n", encoding="utf-8")
            ignored_object = tree / "stale-build.o"
            ignored_object.write_bytes(b"stale native object one")
            (source / ".gitignore").write_text("*.o\n", encoding="utf-8")
            subprocess.run(["git", "init", "-q"], cwd=source, check=True)
            subprocess.run(
                [
                    "git",
                    "add",
                    ".gitignore",
                    "configure",
                    "tree/value.txt",
                    "tree/value-link",
                ],
                cwd=source,
                check=True,
            )

            def staged_identity(name):
                destination = fixture_root / name
                destination.mkdir()
                identity = module.materialize(
                    source, destination, inputs=("configure", "tree")
                )
                self.assertFalse((destination / "tree/stale-build.o").exists())
                self.assertTrue((destination / "tree/local-source.c").is_file())
                return identity

            first = staged_identity("first")
            self.assertEqual(first, staged_identity("same"))
            ignored_object.write_bytes(b"stale native object two")
            self.assertEqual(first, staged_identity("changed-ignored-object"))
            local_source.write_text("int changed_local_source;\n", encoding="utf-8")
            self.assertNotEqual(first, staged_identity("changed-untracked-source"))
            local_source.write_text("int local_source;\n", encoding="utf-8")
            value.write_text("second\n", encoding="utf-8")
            self.assertNotEqual(first, staged_identity("changed-bytes"))
            value.write_text("first\n", encoding="utf-8")
            executable.chmod(0o644)
            self.assertNotEqual(first, staged_identity("changed-mode"))

            outside = fixture_root / "outside"
            outside.write_text("outside\n", encoding="utf-8")
            (tree / "escaping-link").symlink_to(outside)
            destination = fixture_root / "escape"
            destination.mkdir()
            with self.assertRaises(module.MaterializationError):
                module.materialize(source, destination, inputs=("tree",))

    def test_ignored_build_products_cannot_reenter_context_copy(self):
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/materialize-real-citus-test-fixture.py",
                '"--exclude-standard"',
                '"--ignored"',
            ),
            "must contain exactly one occurrence",
        )

    def test_companion_before_citus_is_rejected(self):
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/ai-sql-contract-smoke.sh",
                "CREATE EXTENSION citus;",
                "CREATE EXTENSION ai_blaise_citus;",
            ),
            "must contain exactly one occurrence",
        )


if __name__ == "__main__":
    unittest.main()
