#!/usr/bin/env python3
"""Exercise documentation boundaries without giving prose labels authority."""

import contextlib
import io
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
CHECK = ROOT / "ci/ai-blaise/docs-evidence-boundary-check.sh"
LEGACY = ROOT / "docs/ai-blaise/NEW_FEATURES.md"
SCRIPT = CHECK.read_text().split("python3 <<'PY'\n", 1)[1].rsplit("\nPY", 1)[0]


class DocumentationBoundaryTests(unittest.TestCase):
    def run_audit(self, mutation=None, *, legacy_absent=False):
        read_text, is_file = Path.read_text, Path.is_file

        def fixture_read(path, *args, **kwargs):
            if legacy_absent and path.resolve() == LEGACY:
                raise AssertionError(
                    "legacy catalog must not be a required audit input"
                )
            text = read_text(path, *args, **kwargs)
            if mutation and path.resolve() == ROOT / mutation[0]:
                return mutation[1](text)
            return text

        def fixture_is_file(path):
            if legacy_absent and path.resolve() == LEGACY:
                return False
            return is_file(path)

        stdout, stderr = io.StringIO(), io.StringIO()
        previous = Path.cwd()
        try:
            os.chdir(ROOT)
            with (
                patch.object(Path, "read_text", fixture_read),
                patch.object(Path, "is_file", fixture_is_file),
                contextlib.redirect_stdout(stdout),
                contextlib.redirect_stderr(stderr),
            ):
                try:
                    exec(compile(SCRIPT, str(CHECK), "exec"), {"__name__": "__main__"})
                except SystemExit as error:
                    return error.code, stdout.getvalue(), stderr.getvalue()
        finally:
            os.chdir(previous)
        return 0, stdout.getvalue(), stderr.getvalue()

    def test_current_repository(self):
        code, output, error = self.run_audit()
        self.assertEqual(code, 0, error)
        self.assertIn("authority=docs-and-inventory-only", output)

    def test_legacy_catalog_is_not_required(self):
        code, _, error = self.run_audit(legacy_absent=True)
        self.assertEqual(code, 0, error)

    def test_legacy_statuses_do_not_classify_evidence(self):
        code, _, error = self.run_audit(
            (
                "docs/ai-blaise/NEW_FEATURES.md",
                lambda text: text.replace(
                    "**Status**: production-ready", "**Status**: alpha"
                ),
            )
        )
        self.assertEqual(code, 0, error)

    def test_overclaim_in_a_runbook_is_rejected(self):
        code, _, error = self.run_audit(
            (
                "docs/ai-blaise/RUNBOOKS/production.md",
                lambda text: text + "\nAll custom features are production-ready.\n",
            )
        )
        self.assertEqual(code, 1)
        self.assertIn("blocked overclaiming phrase", error)

    def test_image_digest_requirement_cannot_be_removed(self):
        code, _, error = self.run_audit(
            (
                "deploy/README.md",
                lambda text: text.replace(
                    "OPERATOR_IMAGE_DIGEST", "REMOVED_IMAGE_IDENTITY"
                ),
            )
        )
        self.assertEqual(code, 1)
        self.assertIn("operator digest handoff", error)

    def test_stale_upstream_snapshot_is_rejected(self):
        code, _, error = self.run_audit(
            (
                "docs/ai-blaise/UPSTREAM_SYNC.md",
                lambda text: text.replace(
                    "Status snapshot: 2026-08-26", "Status snapshot: 2000-01-01"
                ),
            )
        )
        self.assertEqual(code, 1)
        self.assertIn("older than 45 days", error)


class FeatureCatalogConsumerTests(unittest.TestCase):
    def run_bundle_check(self, mutation=None):
        check = ROOT / "ci/ai-blaise/bundle1-contract-check.py"
        script = check.read_text()
        read_text = Path.read_text

        def fixture_read(path, *args, **kwargs):
            if path.resolve() == LEGACY:
                raise AssertionError("Bundle1 must not require the legacy catalog")
            text = read_text(path, *args, **kwargs)
            if mutation and path.resolve() == ROOT / mutation[0]:
                return text.replace(mutation[1], mutation[2])
            return text

        stdout, stderr = io.StringIO(), io.StringIO()
        with (
            patch.object(Path, "read_text", fixture_read),
            contextlib.redirect_stdout(stdout),
            contextlib.redirect_stderr(stderr),
        ):
            try:
                exec(
                    compile(script, str(check), "exec"),
                    {
                        "__name__": "__main__",
                        "__file__": str(check),
                    },
                )
            except SystemExit as error:
                return error.code, stdout.getvalue(), stderr.getvalue()
        return 0, stdout.getvalue(), stderr.getvalue()

    def test_bundle_contract_does_not_read_the_legacy_catalog(self):
        code, output, error = self.run_bundle_check()
        self.assertEqual(code, 0, error)
        self.assertIn("bundle1-contract-check passed", output)

    def test_bundle_source_scope_removal_still_fails(self):
        code, _, error = self.run_bundle_check(
            (
                "images/citus-pg-overlay/Dockerfile",
                'ai-blaise.citus.bundle1.release-target="false"',
                'ai-blaise.citus.bundle1.release-target="true"',
            )
        )
        self.assertEqual(code, 1)
        self.assertIn("Bundle1 light target labels", error)

    def test_bundle_citus_install_without_downgrade_sql_fails(self):
        code, _, error = self.run_bundle_check(
            (
                "images/citus-pg-overlay/Dockerfile",
                "make install-all DESTDIR=/out;",
                "make install DESTDIR=/out;",
            )
        )
        self.assertEqual(code, 1)
        self.assertIn("install-all packaging", error)
        self.assertIn("downgrade SQL", error)

    def test_citus_ci_install_fallback_fails(self):
        code, _, error = self.run_bundle_check(
            (
                "ci/build-citus.sh",
                'make DESTDIR="${installdir}" install-all',
                'make DESTDIR="${installdir}" install-all || '
                'make DESTDIR="${installdir}" install',
            )
        )
        self.assertEqual(code, 1)
        self.assertIn("must not fall back", error)

    def test_timescale_cohabitation_plain_install_cannot_hide_behind_text(self):
        code, _, error = self.run_bundle_check(
            (
                "images/citus-timescale-cohabitation/Dockerfile",
                'make install-all with_llvm="${WITH_LLVM}";',
                "echo 'make install-all with_llvm=\"${WITH_LLVM}\";'; \\\n"
                '    make install with_llvm="${WITH_LLVM}";',
            )
        )
        self.assertEqual(code, 1)
        self.assertIn("Timescale cohabitation Citus image", error)
        self.assertIn("execute exact install-all packaging", error)

    def test_every_custom_citus_image_rejects_plain_install(self):
        cases = (
            (
                "images/citus-timescale-cohabitation/Dockerfile",
                'make install-all with_llvm="${WITH_LLVM}";',
                'make install with_llvm="${WITH_LLVM}";',
                "Timescale cohabitation Citus image",
            ),
            (
                "images/citus-pg-cron-cohabitation/Dockerfile",
                "make install-all;",
                "make install;",
                "pg_cron cohabitation Citus image",
            ),
            (
                "images/citus-pg-overlay/Dockerfile.pgcore-patches",
                "make install-all;",
                "make install;",
                "patched PostgreSQL core Citus image",
            ),
        )
        for path, install_all, install, context in cases:
            with self.subTest(path=path):
                code, _, error = self.run_bundle_check((path, install_all, install))
                self.assertEqual(code, 1)
                self.assertIn(context, error)
                self.assertIn("execute exact install-all packaging", error)

    def test_bundle_trusted_preload_rejects_citus_first(self):
        code, _, error = self.run_bundle_check(
            (
                "images/citus-pg-overlay/shared-preload-libraries.conf",
                "timescaledb,pgaudit,pgauditlogtofile,pgsodium,pg_cron,age,pg_failover_slots,pgnodemx,citus",
                "citus,timescaledb,pgaudit,pgauditlogtofile,pgsodium,pg_cron,age,pg_failover_slots,pgnodemx",
            )
        )
        self.assertEqual(code, 1)
        self.assertIn("trusted coextensions must load before a final Citus", error)

    def test_bundle_default_boot_must_call_zero_argument_order_check(self):
        code, _, error = self.run_bundle_check(
            (
                "ci/ai-blaise/bundle1-default-boot-smoke.sh",
                "SELECT companion_internal.assert_citus_cohabit_extension_order();",
                "SELECT 1; -- removed order assertion",
            )
        )
        self.assertEqual(code, 1)
        self.assertIn("Bundle1 default-boot smoke", error)
        self.assertIn("assert_citus_cohabit_extension_order", error)

    def test_bundle_citus_build_cannot_skip_supported_version_check(self):
        code, _, error = self.run_bundle_check(
            (
                "images/citus-pg-overlay/Dockerfile",
                "./configure PG_CONFIG=/usr/bin/pg_config;",
                "./configure PG_CONFIG=/usr/bin/pg_config --without-pg-version-check;",
            )
        )
        self.assertEqual(code, 1)
        self.assertIn("must not bypass", error)

    def run_placement_check(self, *, remove_c_symbol=False):
        paths = (
            "src/backend/distributed/metadata/metadata_cache.c",
            "src/include/distributed/metadata_cache.h",
            "src/backend/distributed/sql/citus--8.0-1.sql",
            "src/backend/distributed/sql/citus--14.0-1--15.0-1.sql",
            "src/backend/distributed/sql/udfs/citus_placement_generation/latest.sql",
            "src/backend/distributed/sql/udfs/citus_placement_generation/15.0-1.sql",
            "src/backend/distributed/sql/udfs/citus_placement_generation/14.0-1.sql",
            "companion/src/router_assist.rs",
            "patches/0005-placement-generation-counter.patch",
            "docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md",
            "ci/ai-blaise/pg-cron-cohabitation-smoke.sh",
        )
        with tempfile.TemporaryDirectory(prefix="placement-doc-consumer-") as directory:
            fixture = Path(directory)
            subprocess.run(["git", "init", "--quiet", str(fixture)], check=True)
            for relative in paths:
                destination = fixture / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(ROOT / relative, destination)
            self.assertFalse((fixture / "docs/ai-blaise/NEW_FEATURES.md").exists())
            if remove_c_symbol:
                metadata = fixture / paths[0]
                metadata.write_text(
                    metadata.read_text().replace(
                        "PG_FUNCTION_INFO_V1(citus_placement_generation)",
                        "REMOVED_PLACEMENT_SYMBOL",
                    )
                )
            return subprocess.run(
                [
                    "bash",
                    str(
                        ROOT / "ci/ai-blaise/placement-generation-udf-contract-smoke.sh"
                    ),
                ],
                cwd=fixture,
                text=True,
                capture_output=True,
                check=False,
            )

    def test_placement_contract_works_without_the_legacy_catalog(self):
        result = self.run_placement_check()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("placement_generation_udf_contract_smoke", result.stdout)

    def test_placement_c_symbol_removal_still_fails(self):
        result = self.run_placement_check(remove_c_symbol=True)
        self.assertEqual(result.returncode, 1)
        self.assertIn("PG_FUNCTION_INFO_V1(citus_placement_generation)", result.stderr)


if __name__ == "__main__":
    unittest.main()
