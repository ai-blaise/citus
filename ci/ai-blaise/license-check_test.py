#!/usr/bin/env python3
"""Regression tests for the license metadata command boundary."""

from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "ci" / "ai-blaise" / "license-check.sh"
VALID_METADATA = {
    "packages": [
        {
            "id": "path+file:///workspace#fixture@0.1.0",
            "name": "fixture",
            "version": "0.1.0",
            "license": "MIT",
        }
    ],
    "workspace_members": ["path+file:///workspace#fixture@0.1.0"],
    "workspace_root": "/workspace",
    "resolve": {
        "nodes": [{"id": "path+file:///workspace#fixture@0.1.0"}],
    },
}


class LicenseCheckTests(unittest.TestCase):
    """Exercise real and mocked Cargo metadata scans."""

    def run_check(
        self,
        *,
        path: str | None = None,
        release_mode: str = "0",
        extra_env: dict[str, str] | None = None,
    ) -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        env["AI_BLAISE_RELEASE_MODE"] = release_mode
        if path is not None:
            env["PATH"] = path
        if extra_env:
            env.update(extra_env)
        return subprocess.run(
            ["/bin/bash", str(SCRIPT)],
            cwd=REPO_ROOT,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )

    @staticmethod
    def write_command(directory: Path, name: str, body: str) -> Path:
        command = directory / name
        command.write_text("#!/bin/sh\nset -eu\n" + body, encoding="utf-8")
        command.chmod(0o755)
        return command

    def test_real_locked_workspace_metadata_scan_passes(self) -> None:
        result = self.run_check(release_mode="1")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("locked Cargo metadata scan passed", result.stdout)

    def test_cargo_failure_is_fatal_and_uses_locked_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            command_dir = Path(temporary)
            args_file = command_dir / "cargo-args"
            self.write_command(
                command_dir,
                "cargo",
                'printf "%s\\n" "$*" >"$LICENSE_TEST_CARGO_ARGS"\nexit 42\n',
            )
            result = self.run_check(
                path=f"{command_dir}{os.pathsep}{os.environ['PATH']}",
                extra_env={"LICENSE_TEST_CARGO_ARGS": str(args_file)},
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(
                args_file.read_text(encoding="utf-8").strip(),
                "metadata --locked --format-version 1",
            )
            self.assertIn("cargo metadata --locked did not complete", result.stderr)

    def test_empty_cargo_metadata_is_fatal(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            command_dir = Path(temporary)
            self.write_command(command_dir, "cargo", "exit 0\n")
            result = self.run_check(
                path=f"{command_dir}{os.pathsep}{os.environ['PATH']}"
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("returned empty output", result.stderr)

    def test_malformed_or_incomplete_metadata_is_fatal(self) -> None:
        malformed_documents = (
            "{not-json",
            json.dumps(
                {
                    "packages": [],
                    "workspace_members": [],
                    "workspace_root": "/workspace",
                    "resolve": {"nodes": []},
                }
            ),
            json.dumps(
                {
                    **VALID_METADATA,
                    "workspace_members": ["missing-package-id"],
                }
            ),
            f"{json.dumps(VALID_METADATA)}\n{json.dumps(VALID_METADATA)}",
            f"{{}}\n{json.dumps(VALID_METADATA)}",
        )
        for document in malformed_documents:
            with (
                self.subTest(document=document),
                tempfile.TemporaryDirectory() as temporary,
            ):
                command_dir = Path(temporary)
                self.write_command(
                    command_dir,
                    "cargo",
                    'printf "%s\\n" "$LICENSE_TEST_METADATA"\n',
                )
                result = self.run_check(
                    path=f"{command_dir}{os.pathsep}{os.environ['PATH']}",
                    extra_env={"LICENSE_TEST_METADATA": document},
                )

                self.assertNotEqual(result.returncode, 0)
                self.assertIn("malformed or incomplete cargo metadata", result.stderr)

    def test_jq_failure_is_fatal_after_scan_starts(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            command_dir = Path(temporary)
            self.write_command(
                command_dir,
                "cargo",
                'printf "%s\\n" "$LICENSE_TEST_METADATA"\n',
            )
            self.write_command(command_dir, "jq", "exit 43\n")
            result = self.run_check(
                path=f"{command_dir}{os.pathsep}{os.environ['PATH']}",
                extra_env={"LICENSE_TEST_METADATA": json.dumps(VALID_METADATA)},
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("jq rejected malformed or incomplete cargo metadata", result.stderr)

    def test_policy_filter_jq_failure_is_fatal(self) -> None:
        real_jq = shutil.which("jq")
        self.assertIsNotNone(real_jq)
        with tempfile.TemporaryDirectory() as temporary:
            command_dir = Path(temporary)
            marker = command_dir / "jq-validation-complete"
            self.write_command(
                command_dir,
                "cargo",
                'printf "%s\\n" "$LICENSE_TEST_METADATA"\n',
            )
            self.write_command(
                command_dir,
                "jq",
                'if [ ! -e "$LICENSE_TEST_JQ_MARKER" ]; then\n'
                '  : >"$LICENSE_TEST_JQ_MARKER"\n'
                '  exec "$LICENSE_TEST_REAL_JQ" "$@"\n'
                "fi\n"
                "exit 43\n",
            )
            result = self.run_check(
                path=f"{command_dir}{os.pathsep}{os.environ['PATH']}",
                extra_env={
                    "LICENSE_TEST_JQ_MARKER": str(marker),
                    "LICENSE_TEST_METADATA": json.dumps(VALID_METADATA),
                    "LICENSE_TEST_REAL_JQ": str(real_jq),
                },
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertTrue(marker.is_file())
            self.assertIn(
                "jq could not evaluate repository dependency policy", result.stderr
            )

    def test_term_terminates_scan_and_cleans_metadata_file(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            test_root = Path(temporary)
            command_dir = test_root / "bin"
            metadata_dir = test_root / "metadata"
            command_dir.mkdir()
            metadata_dir.mkdir()
            self.write_command(
                command_dir,
                "cargo",
                'kill -TERM "$PPID"\nexit 0\n',
            )
            result = self.run_check(
                path=f"{command_dir}{os.pathsep}{os.environ['PATH']}",
                extra_env={"TMPDIR": str(metadata_dir)},
            )

            self.assertEqual(result.returncode, 143, result.stderr)
            self.assertEqual(
                list(metadata_dir.glob("ai-blaise-license-metadata.*")), []
            )

    def test_existing_repository_dependency_policy_is_preserved(self) -> None:
        policy_cases = (
            ("GPL-3.0-only", False),
            ("GPL-3.0-only OR LGPL-3.0-only", True),
        )
        for license_expression, permitted in policy_cases:
            with (
                self.subTest(license_expression=license_expression),
                tempfile.TemporaryDirectory() as temporary,
            ):
                command_dir = Path(temporary)
                metadata = {
                    **VALID_METADATA,
                    "packages": [
                        {
                            **VALID_METADATA["packages"][0],
                            "license": license_expression,
                        }
                    ],
                }
                self.write_command(
                    command_dir,
                    "cargo",
                    'printf "%s\\n" "$LICENSE_TEST_METADATA"\n',
                )
                result = self.run_check(
                    path=f"{command_dir}{os.pathsep}{os.environ['PATH']}",
                    extra_env={"LICENSE_TEST_METADATA": json.dumps(metadata)},
                )

                if permitted:
                    self.assertEqual(result.returncode, 0, result.stderr)
                else:
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn(
                        "forbidden by repository dependency policy", result.stderr
                    )

    def test_missing_tools_are_only_an_explicit_exploratory_skip(self) -> None:
        grep = shutil.which("grep")
        self.assertIsNotNone(grep)
        with tempfile.TemporaryDirectory() as temporary:
            command_dir = Path(temporary)
            (command_dir / "grep").symlink_to(grep)

            exploratory = self.run_check(path=str(command_dir))
            self.assertEqual(exploratory.returncode, 0, exploratory.stderr)
            self.assertIn("exploratory-only metadata scan skipped", exploratory.stderr)

            release = self.run_check(path=str(command_dir), release_mode="1")
            self.assertNotEqual(release.returncode, 0)
            self.assertIn("requires cargo in release mode", release.stderr)
            self.assertIn("requires jq in release mode", release.stderr)


if __name__ == "__main__":
    unittest.main()
