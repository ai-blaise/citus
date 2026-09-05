#!/usr/bin/env python3
"""Mutation tests for the source-bound Citus + TimescaleDB fixture."""

# FEATURE: TS6 TS18

from __future__ import annotations

import contextlib
import importlib.util
import io
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from fixture_git_env import isolated_fixture_git_environment

ROOT = Path(__file__).resolve().parents[2]
CHECK = ROOT / "ci/ai-blaise/real-citus-timescale-test-fixture-contract.py"
BUILDER = ROOT / "ci/ai-blaise/build-real-citus-timescale-test-fixture.sh"
MATERIALIZER = ROOT / "ci/ai-blaise/materialize-real-citus-timescale-test-fixture.py"


def load_materializer():
    specification = importlib.util.spec_from_file_location(
        "timescale_fixture_materializer", MATERIALIZER
    )
    if specification is None or specification.loader is None:
        raise RuntimeError("Timescale fixture materializer could not be imported")
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


class RealCitusTimescaleFixtureContractTests(unittest.TestCase):
    def run_check(self, mutation=None):
        script = CHECK.read_text(encoding="utf-8")
        read_text = Path.read_text

        def fixture_read(path, *args, **kwargs):
            text = read_text(path, *args, **kwargs)
            if mutation and path.resolve() == (ROOT / mutation[0]).resolve():
                self.assertIn(mutation[1], text)
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
        self.assertIn("real-citus-timescale-test-fixture-contract passed", output)

    def test_builder_contract_only_command_boundary(self):
        result = subprocess.run(
            ["bash", str(BUILDER), "--contract-only"],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(
            "real-citus-timescale-test-fixture-contract passed", result.stdout
        )

    def test_base_lock_rejects_floating_or_changed_timescale_operand(self):
        for replacement in (
            "timescale/timescaledb-ha:pg17-ts2.27",
            "docker.io/timescale/timescaledb-ha:pg17-ts2.28@sha256:" + "1" * 64,
        ):
            with self.subTest(replacement=replacement):
                self.assert_mutation_fails(
                    (
                        "images/citus-timescale-cohabitation/base-image.lock.tsv",
                        "docker.io/timescale/timescaledb-ha:pg17-ts2.27@sha256:"
                        "4f61167e11c7c95bedf96433c720d671a53aa29ad7f52b142b529a6d0e9f0b20",
                        replacement,
                    ),
                    "must contain both exact reviewed PG17 rows",
                )

    def test_dockerfile_rejects_floating_base_and_broad_context(self):
        mutations = (
            (
                "ARG BASE_IMAGE\nFROM ${BASE_IMAGE}",
                "ARG BASE_IMAGE=timescale/timescaledb-ha:pg17-ts2.27\n"
                "FROM ${BASE_IMAGE}",
                "must not define a floating default base",
            ),
            (
                "COPY config ./config",
                "COPY . /build/citus",
                "must not copy a broad mutable checkout",
            ),
        )
        for old, new, message in mutations:
            with self.subTest(new=new):
                self.assert_mutation_fails(
                    ("images/citus-timescale-cohabitation/Dockerfile", old, new),
                    message,
                )

    def test_dockerfile_requires_install_all_and_complete_sql_inventory(self):
        mutations = (
            (
                'make install-all with_llvm="${WITH_LLVM}";',
                'make install with_llvm="${WITH_LLVM}";',
                "must not omit downgrade SQL",
            ),
            (
                'make install-all with_llvm="${WITH_LLVM}";',
                'make install-all with_llvm="${WITH_LLVM}" || make install;',
                "must not fall back from install-all",
            ),
            (
                'cmp "${sql_file}" "${extension_dir}/$(basename "${sql_file}")";',
                'test -s "${sql_file}";',
                "must contain exactly one occurrence",
            ),
            (
                "COPY images/citus-pg-overlay/upgrades/ai_blaise_citus--0.1.2.sql /build/companion/archive/",
                "# archived upgrade bytes omitted",
                "must contain exactly one occurrence",
            ),
        )
        for old, new, message in mutations:
            with self.subTest(new=new):
                self.assert_mutation_fails(
                    ("images/citus-timescale-cohabitation/Dockerfile", old, new),
                    message,
                )

    def test_dockerfile_pins_dev_headers_to_the_installed_server(self):
        self.assert_mutation_fails(
            (
                "images/citus-timescale-cohabitation/Dockerfile",
                '"postgresql-server-dev-${PG_MAJOR}=${postgresql_package_version}"',
                '"postgresql-server-dev-${PG_MAJOR}"',
            ),
            "must contain exactly one occurrence",
        )
        self.assert_mutation_fails(
            (
                "images/citus-timescale-cohabitation/Dockerfile",
                "sha256sum --check /build/vendor-baseline/timescaledb-libraries.sha256;",
                "true # vendor library integrity skipped;",
            ),
            "must verify timescaledb-libraries.sha256",
        )

    def test_materializer_requires_narrow_cohabitation_inputs(self):
        for old, new in (
            (
                '"images/citus-timescale-cohabitation/Dockerfile"',
                '"images/citus-test-fixture/Dockerfile"',
            ),
            (
                '"images/citus-pg-overlay/upgrades/ai_blaise_citus--0.1.2.sql"',
                '"images/citus-pg-overlay/extensions/ai_blaise_citus.control"',
            ),
        ):
            with self.subTest(new=new):
                self.assert_mutation_fails(
                    (
                        "ci/ai-blaise/materialize-real-citus-timescale-test-fixture.py",
                        old,
                        new,
                    ),
                    "Timescale fixture materializer",
                )

    def test_builder_binds_content_identity_and_verifies_both_image_paths(self):
        mutations = (
            (
                'hashlib.sha256(b"ai-blaise/real-citus-timescale-test-fixture/v1\\0")',
                'hashlib.sha256(b"unbound-fixture")',
                "must contain exactly one occurrence",
            ),
            (
                'verify_label "ai-blaise.citus.source-content-sha256" "${source_content_sha256}"',
                "true # content label ignored",
                "must contain exactly one occurrence",
            ),
            (
                "  verify_fixture\n  printf '%s\\n' \"${image_id}\"\n  exit 0",
                "  printf '%s\\n' \"${image_id}\"\n  exit 0",
                "must verify both cache-hit and newly built images",
            ),
        )
        for old, new, message in mutations:
            with self.subTest(new=new):
                self.assert_mutation_fails(
                    (
                        "ci/ai-blaise/build-real-citus-timescale-test-fixture.sh",
                        old,
                        new,
                    ),
                    message,
                )

    def test_ci_and_image_check_wiring_are_required(self):
        for mutation, message in (
            (
                (
                    "ci/ai-blaise/image-check.sh",
                    'python3 "${real_citus_timescale_fixture_contract}"',
                    "echo timescale-fixture-contract-disabled",
                ),
                "image-check must execute the Timescale fixture contract",
            ),
            (
                (
                    ".github/workflows/ci-image.yml",
                    "python3 ci/ai-blaise/real-citus-timescale-test-fixture-contract_test.py",
                    "echo timescale-fixture-mutations-disabled",
                ),
                "ci-image must execute the Timescale fixture contract and mutations",
            ),
            (
                (
                    ".github/workflows/ci-image.yml",
                    "for timescaledb_minor in 2.27 2.28; do",
                    "for timescaledb_minor in 2.27; do",
                ),
                "ci-image must execute both exact Timescale bridge minors",
            ),
            (
                (
                    "ci/ai-blaise/upgrade-rollback-guardrails.sh",
                    "REAL_CITUS_TIMESCALE_DEFAULT_VERSION_SMOKES",
                    "RETIRED_TIMESCALE_DEFAULT_VERSION_SMOKES",
                ),
                "upgrade guard is missing the source-bound Timescale bridge contract",
            ),
        ):
            with self.subTest(path=mutation[0]):
                self.assert_mutation_fails(mutation, message)

    def test_consumers_reject_retired_images_and_unknown_minors_before_docker(self):
        cases = (
            (
                "ci/ai-blaise/timescale-cohabitation-smoke.sh",
                {"TIMESCALE_COHABITATION_BASE_IMAGE": "retired:latest"},
                "TIMESCALE_COHABITATION_BASE_IMAGE/TAG are retired",
            ),
            (
                "ci/ai-blaise/timescale-bridge-smoke.sh",
                {"TIMESCALE_BRIDGE_SMOKE_IMAGE": "retired:latest"},
                "TIMESCALE_BRIDGE_SMOKE_IMAGE is retired",
            ),
            (
                "ci/ai-blaise/timescale-cohabitation-smoke.sh",
                {"TIMESCALE_COHABITATION_EXPECTED_TS_MINOR": "2.29"},
                "supports only the locked 2.27 and 2.28 lines",
            ),
            (
                "ci/ai-blaise/timescale-bridge-smoke.sh",
                {"TIMESCALE_BRIDGE_EXPECTED_TS_MINOR": "2.29"},
                "supports only the locked 2.27 and 2.28 lines",
            ),
            (
                "ci/ai-blaise/ts-version-matrix-smoke.sh",
                {"TS_VERSION_MATRIX": "2.27"},
                "required TS version was not selected: 2.28",
            ),
        )
        for relative, extra_env, message in cases:
            with self.subTest(relative=relative, extra_env=extra_env):
                env = os.environ.copy()
                env.update(extra_env)
                result = subprocess.run(
                    ["bash", str(ROOT / relative)],
                    cwd=ROOT,
                    env=env,
                    check=False,
                    capture_output=True,
                    text=True,
                )
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(message, result.stderr)

    def test_cohabitation_consumer_binds_selected_minor_and_real_citus(self):
        mutations = (
            (
                'builder_args=(--timescaledb-minor "${expected_ts_minor}")',
                'builder_args=(--timescaledb-minor "2.27")',
                "cohabitation smoke is missing",
            ),
            (
                "FROM pg_dist_partition",
                "FROM pg_class",
                "cohabitation smoke is missing",
            ),
            (
                "SELECT create_distributed_table('citus_smoke_events', 'tenant_id');",
                "CREATE FUNCTION create_distributed_table(regclass, text) RETURNS void LANGUAGE sql AS 'SELECT';",
                "must not stub the Citus distribution entrypoint",
            ),
            (
                'container_logs="$(docker logs --tail 200 "${container}" 2>&1 || true)"',
                'if docker logs "${container}" 2>&1 | grep -q "PostgreSQL init process complete"; then true; fi',
                "must use the bounded non-pipeline init wait",
            ),
        )
        for old, new, message in mutations:
            with self.subTest(new=new):
                self.assert_mutation_fails(
                    ("ci/ai-blaise/timescale-cohabitation-smoke.sh", old, new),
                    message,
                )

    def test_bridge_requires_distinct_negative_and_real_positive_databases(self):
        mutations = (
            (
                "CREATE DATABASE timescale_bridge_positive;",
                "SELECT 1;",
                "bridge smoke is missing",
            ),
            (
                "CREATE EXTENSION IF NOT EXISTS citus;\nCREATE EXTENSION IF NOT EXISTS timescaledb;",
                "CREATE EXTENSION IF NOT EXISTS timescaledb;\nCREATE EXTENSION IF NOT EXISTS citus;",
                "positive database is missing ordered token",
            ),
            (
                "FROM pg_dist_shard",
                "FROM pg_class",
                "bridge smoke is missing",
            ),
            (
                "INSERT INTO timescale_smoke_metrics(metric_time, tenant_id, value)",
                "CREATE FUNCTION create_distributed_table(regclass, text) RETURNS void LANGUAGE sql AS 'SELECT';\n"
                "INSERT INTO timescale_smoke_metrics(metric_time, tenant_id, value)",
                "must not stub the Citus distribution entrypoint",
            ),
            (
                'container_logs="$(docker logs --tail 200 "${container}" 2>&1 || true)"',
                'if docker logs "${container}" 2>&1 | grep -q "PostgreSQL init process complete"; then true; fi',
                "must use the bounded non-pipeline init wait",
            ),
        )
        for old, new, message in mutations:
            with self.subTest(new=new):
                self.assert_mutation_fails(
                    ("ci/ai-blaise/timescale-bridge-smoke.sh", old, new),
                    message,
                )

    def test_matrix_uses_both_exact_bases_and_the_same_verified_fixture(self):
        mutations = (
            (
                "tests/cohab-matrix/2.28/image-tag.txt",
                "docker.io/timescale/timescaledb-ha:pg17-ts2.28@sha256:"
                "bc9e09875460aa69fb536362fef7c8e92c51ad6aab3d13f91a2487d3547dc71a",
                "docker.io/timescale/timescaledb-ha:pg17-ts2.27@sha256:"
                "4f61167e11c7c95bedf96433c720d671a53aa29ad7f52b142b529a6d0e9f0b20",
                "2.28 image-tag must be its exact reviewed base",
            ),
            (
                "ci/ai-blaise/ts-version-matrix-smoke.sh",
                'fixture_image="$("${fixture_builder}" --timescaledb-minor "${ts_version}")"',
                'fixture_image="$("${fixture_builder}" --timescaledb-minor "2.27")"',
                "version matrix is missing",
            ),
            (
                "ci/ai-blaise/ts-version-matrix-smoke.sh",
                'TIMESCALE_COHABITATION_IMAGE="${fixture_image}"',
                'TIMESCALE_COHABITATION_IMAGE="mutable:latest"',
                "version matrix is missing",
            ),
            (
                "ci/ai-blaise/ts-version-matrix-smoke.sh",
                "      --network none \\\n",
                "",
                "version matrix is missing",
            ),
            (
                "ci/ai-blaise/ts-version-matrix-smoke.sh",
                'probe_logs="$(docker logs --tail 200 "${container}" 2>&1 || true)"',
                'if docker logs "${container}" 2>&1 | grep -q "PostgreSQL init process complete"; then true; fi',
                "must use the bounded non-pipeline init wait",
            ),
        )
        for mutation in mutations:
            with self.subTest(path=mutation[0], replacement=mutation[2]):
                self.assert_mutation_fails(mutation[:3], mutation[3])

    def test_each_minor_is_bound_to_only_its_reviewed_base(self):
        self.assert_mutation_fails(
            (
                "ci/ai-blaise/timescale-bridge-smoke.sh",
                '2.28)\n    expected_base_image="docker.io/timescale/timescaledb-ha:pg17-ts2.28@sha256:',
                '2.28)\n    expected_base_image="docker.io/timescale/timescaledb-ha:pg17-ts2.27@sha256:',
            ),
            "does not bind TimescaleDB 2.28 to its exact base",
        )

    def test_static_hook_claims_are_not_misrepresented_as_runtime_measurement(self):
        code, output, error = self.run_check(
            (
                "tests/cohab-matrix/2.27/expected-hook-claims.tsv",
                "ProcessUtility_hook\tclaimed\t",
                "ProcessUtility_hook\tnot_claimed\t",
            )
        )
        self.assertEqual(code, 0, error)
        self.assertIn("real-citus-timescale-test-fixture-contract passed", output)
        self.assert_mutation_fails(
            (
                "tests/cohab-matrix/compare-hook-claims.sh",
                "hook_runtime_comparison=unavailable",
                "hook_runtime_comparison=verified",
            ),
            "must state its non-runtime hook boundary",
        )

    @isolated_fixture_git_environment()
    def test_materializer_excludes_ignored_objects_and_binds_local_source(self):
        module = load_materializer()
        with tempfile.TemporaryDirectory(prefix="timescale-fixture-context-") as root:
            fixture_root = Path(root)
            source = fixture_root / "source"
            destination = fixture_root / "context"
            source.mkdir()
            destination.mkdir()
            (source / ".gitignore").write_text("*.o\n", encoding="utf-8")
            for relative in module.SOURCE_INPUTS:
                path = source / relative
                if relative in {"config", "src", "vendor"}:
                    path.mkdir(parents=True, exist_ok=True)
                    (path / "tracked.txt").write_text(relative, encoding="utf-8")
                else:
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text(relative, encoding="utf-8")
            local_source = source / "src/local-source.c"
            local_source.write_text("int local_source;\n", encoding="utf-8")
            ignored_object = source / "src/stale.o"
            ignored_object.write_bytes(b"stale native object")
            subprocess.run(["git", "init", "-q"], cwd=source, check=True)
            subprocess.run(["git", "add", "."], cwd=source, check=True)

            base = module.load_base_materializer()
            identity = base.materialize(
                source, destination, inputs=module.SOURCE_INPUTS
            )
            self.assertRegex(identity, r"^[0-9a-f]{64}$")
            self.assertTrue((destination / "src/local-source.c").is_file())
            self.assertFalse((destination / "src/stale.o").exists())


if __name__ == "__main__":
    unittest.main()
