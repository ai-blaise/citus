#!/usr/bin/env python3
"""Exercise the production-gap audit's structural and source trust boundaries."""

import contextlib
import io
import json
import os
from pathlib import Path
import shlex
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
FEATURES = ROOT / "docs/ai-blaise/NEW_FEATURES.md"
AUDIT = ROOT / "ci/ai-blaise/production-gap-audit.sh"
SCRIPT = AUDIT.read_text().split("python3 <<'PY'\n", 1)[1].rsplit("\nPY", 1)[0]


class ProductionGapAuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        environment = os.environ.copy()
        environment.pop("DEVELOPER_DIR", None)
        build = subprocess.run(
            [
                "cargo",
                "build",
                "--locked",
                "--quiet",
                "-p",
                "ai_blaise_feature_register",
            ],
            cwd=ROOT,
            env=environment,
            check=False,
            capture_output=True,
            text=True,
        )
        if build.returncode != 0:
            raise RuntimeError(
                "could not build the feature-register test binary:\n"
                f"{build.stdout}{build.stderr}"
            )
        metadata = subprocess.run(
            ["cargo", "metadata", "--locked", "--format-version", "1", "--no-deps"],
            cwd=ROOT,
            env=environment,
            check=True,
            capture_output=True,
            text=True,
        )
        executable_name = "ai_blaise_feature_register"
        if os.name == "nt":
            executable_name += ".exe"
        cls.feature_register = (
            Path(json.loads(metadata.stdout)["target_directory"])
            / "debug"
            / executable_name
        )
        if not cls.feature_register.is_file():
            raise RuntimeError(
                f"feature-register test binary was not built at {cls.feature_register}"
            )

    def run_audit(
        self,
        transform=lambda text: text,
        source_mutation=None,
        *,
        legacy_present=True,
    ):
        read_text = Path.read_text
        is_file = Path.is_file

        def fixture_read(path, *args, **kwargs):
            text = read_text(path, *args, **kwargs)
            if path.resolve() == FEATURES:
                return transform(text)
            if source_mutation and path.resolve() == ROOT / source_mutation[0]:
                return text.replace(source_mutation[1], source_mutation[2])
            return text

        def fixture_is_file(path):
            if path.resolve() == FEATURES and not legacy_present:
                return False
            return is_file(path)

        stdout, stderr = io.StringIO(), io.StringIO()
        previous_directory = Path.cwd()
        try:
            os.chdir(ROOT)
            with (
                patch.object(Path, "read_text", fixture_read),
                patch.object(Path, "is_file", fixture_is_file),
                contextlib.redirect_stdout(stdout),
                contextlib.redirect_stderr(stderr),
            ):
                try:
                    exec(compile(SCRIPT, str(AUDIT), "exec"), {"__name__": "__main__"})
                except SystemExit as error:
                    return error.code, stdout.getvalue(), stderr.getvalue()
        finally:
            os.chdir(previous_directory)
        return 0, stdout.getvalue(), stderr.getvalue()

    def test_current_repository_reports_source_checks_without_live_execution_claims(
        self,
    ):
        code, output, error = self.run_audit()
        self.assertEqual(code, 0, error)
        self.assertIn("feature_registry=validated", output)
        self.assertIn("inventory_contract=identity_only", output)
        self.assertIn("production_release_blocked=true", output)
        self.assertIn("live_sql_contract_sources_checked=true", output)
        self.assertIn("live_k8s_e2e_harness_source_checked=true", output)
        self.assertNotIn("production_ready=", output)
        self.assertNotIn("live_sql_guards=true", output)

    def test_missing_legacy_catalog_does_not_break_the_source_audit(self):
        code, output, error = self.run_audit(legacy_present=False)
        self.assertEqual(code, 0, error)
        self.assertIn("source_feature_coverage=validated", output)

    def test_obsolete_or_invented_legacy_prose_is_not_inventory_authority(self):
        invented_legacy_catalog = """\
# Archived compatibility notes

### Fictional99: prose that is not in the machine register

**Status**: invented

This deliberately has no source, receipt, or maturity semantics.
"""
        code, output, error = self.run_audit(lambda _: invented_legacy_catalog)
        self.assertEqual(code, 0, error)
        self.assertIn("inventory_contract=identity_only", output)

    def test_optional_legacy_catalog_cannot_make_a_global_release_overclaim(self):
        code, _, error = self.run_audit(
            lambda text: text + "\nThe full plan is production-ready.\n"
        )
        self.assertEqual(code, 1)
        self.assertIn(
            "contains overclaiming wording: full plan is production-ready", error
        )

    def test_malformed_machine_register_is_rejected_by_the_real_validator(self):
        with tempfile.TemporaryDirectory(
            prefix="production-gap-register-"
        ) as directory:
            fixture_root = Path(directory)
            (fixture_root / "docs").mkdir(parents=True)
            fixture_audit = fixture_root / "ci/ai-blaise/production-gap-audit.sh"
            fixture_audit.parent.mkdir(parents=True)
            fixture_audit.write_text(AUDIT.read_text(), encoding="utf-8")
            (fixture_root / "docs/features.tsv").write_text(
                "id\ttitle\nD10\tmalformed register\n", encoding="utf-8"
            )
            fixture_bin = fixture_root / "test-bin"
            fixture_bin.mkdir()
            cargo_wrapper = fixture_bin / "cargo"
            cargo_wrapper.write_text(
                "#!/bin/sh\n"
                'while [ "$#" -gt 0 ] && [ "$1" != "--" ]; do shift; done\n'
                '[ "$#" -gt 0 ] || exit 64\n'
                "shift\n"
                f'exec {shlex.quote(str(self.feature_register))} "$@"\n',
                encoding="utf-8",
            )
            cargo_wrapper.chmod(0o755)
            subprocess.run(
                ["git", "init", "--quiet"],
                cwd=fixture_root,
                check=True,
                capture_output=True,
                text=True,
            )
            environment = os.environ.copy()
            environment.pop("DEVELOPER_DIR", None)
            environment["PATH"] = f"{fixture_bin}{os.pathsep}{environment['PATH']}"
            result = subprocess.run(
                ["bash", str(fixture_audit)],
                cwd=fixture_root,
                env=environment,
                check=False,
                capture_output=True,
                text=True,
            )

        self.assertEqual(result.returncode, 1)
        self.assertIn("feature-register:", result.stderr)
        self.assertIn("header mismatch", result.stderr)
        self.assertIn(
            "feature register source coverage validation failed", result.stderr
        )

    def test_source_contract_removal_is_rejected(self):
        code, _, error = self.run_audit(
            source_mutation=(
                "ci/ai-blaise/graphql-pggraphql-live-smoke.sh",
                "revoked_token_rejected=true",
                "removed_revocation_assertion",
            )
        )
        self.assertEqual(code, 1)
        self.assertIn("revoked_token_rejected=true", error)

    def test_release_path_cannot_be_replaced_with_structural_success(self):
        code, _, error = self.run_audit(
            source_mutation=(
                "ci/ai-blaise/production-readiness-check.sh",
                "-- release-gaps",
                "-- summary",
            )
        )
        self.assertEqual(code, 1)
        self.assertIn(
            "production-release must use the Rust release-gaps rejection path", error
        )


if __name__ == "__main__":
    unittest.main()
