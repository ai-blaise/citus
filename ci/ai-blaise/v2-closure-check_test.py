#!/usr/bin/env python3
"""Integration tests for the inventory-only V2 compatibility gate."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


HEADER = (
    "id\ttitle\tkind\tdisposition\tcanonical_ids\tmaturity\tsource_paths\t"
    "check_paths\tevidence_paths\tclaimed_status\tscope\tblockers"
)
SCRIPT = Path(__file__).resolve().with_name("v2-closure-check.sh")
FEATURE_REGISTER_BIN: Path


class FixtureRepo:
    def __init__(self) -> None:
        self._temporary = tempfile.TemporaryDirectory(
            prefix="ai-blaise-v2-inventory-test-"
        )
        self.path = Path(self._temporary.name)
        subprocess.run(
            ["git", "init", "--quiet", str(self.path)],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        self.write_valid_inventory()

    def close(self) -> None:
        self._temporary.cleanup()

    def write_valid_inventory(self) -> None:
        library = self.path / "tools/example"
        (library / "src").mkdir(parents=True)
        (library / "Cargo.toml").write_text(
            "[package]\nname = \"inventory_fixture_library\"\nversion = \"0.0.0\"\n"
            "edition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n",
            encoding="utf-8",
        )
        marker = "// " + "FEATURE" + ": Lib1\n"
        (library / "src/lib.rs").write_text(
            marker + "pub fn library_only() {}\n", encoding="utf-8"
        )
        evidence = self.path / "docs/evidence/development-observation.md"
        evidence.parent.mkdir(parents=True)
        evidence.write_text("Development observation only.\n", encoding="utf-8")
        register = self.path / "docs/features.tsv"
        register.parent.mkdir(parents=True, exist_ok=True)
        register.write_text(
            HEADER
            + "\n"
            + "\t".join(
                [
                    "Lib1",
                    "Library-only fixture",
                    "capability",
                    "active",
                    "-",
                    "implemented",
                    "tools/example/src/lib.rs",
                    "-",
                    "docs/evidence/development-observation.md",
                    "production-ready",
                    "A legitimate library crate with no executable target.",
                    "No trusted current-source release evidence.",
                ]
            )
            + "\n",
            encoding="utf-8",
        )

    def run_gate(self) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                "bash",
                str(SCRIPT),
                "--repo",
                str(self.path),
                "--feature-register-bin",
                str(FEATURE_REGISTER_BIN),
            ],
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env={**os.environ, "LC_ALL": "C"},
        )


class InventoryGateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.fixture = FixtureRepo()

    def tearDown(self) -> None:
        self.fixture.close()

    def test_legitimate_library_crate_passes_without_an_executable(self) -> None:
        self.assertFalse((self.fixture.path / "tools/example/src/main.rs").exists())
        self.assertFalse((self.fixture.path / "tools/example/src/bin").exists())

        result = self.fixture.run_gate()

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, "")
        self.assertIn("v2_inventory_check\tpassed\n", result.stdout)
        self.assertIn("authority\tinventory-only\n", result.stdout)
        self.assertIn("inventory_rows\t1\n", result.stdout)
        self.assertIn("source_marker_ids\t1\n", result.stdout)

    def test_missing_inventory_is_rejected(self) -> None:
        (self.fixture.path / "docs/features.tsv").unlink()

        result = self.fixture.run_gate()

        self.assertEqual(result.returncode, 2)
        self.assertEqual(result.stdout, "")
        self.assertIn("cannot resolve docs/features.tsv", result.stderr)

    def test_malformed_inventory_is_rejected(self) -> None:
        register = self.fixture.path / "docs/features.tsv"
        contents = register.read_text(encoding="utf-8")
        register.write_text(
            contents.replace("id\ttitle", "identity\ttitle", 1), encoding="utf-8"
        )

        result = self.fixture.run_gate()

        self.assertEqual(result.returncode, 2)
        self.assertEqual(result.stdout, "")
        self.assertIn("header mismatch", result.stderr)

    def test_inventory_claims_cannot_imply_completed_release(self) -> None:
        result = self.fixture.run_gate()

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            result.stdout,
            "".join(
                [
                    "v2_inventory_check\tpassed\n",
                    "authority\tinventory-only\n",
                    "inventory_rows\t1\n",
                    "source_marker_ids\t1\n",
                    "runtime_closure\tunverified\n",
                    "release_qualification\tblocked\n",
                    "release_evidence_verifier\tunimplemented\n",
                ]
            ),
        )
        records = dict(line.split("\t", 1) for line in result.stdout.splitlines())
        self.assertEqual(records["runtime_closure"], "unverified")
        self.assertEqual(records["release_qualification"], "blocked")
        self.assertEqual(records["release_evidence_verifier"], "unimplemented")
        self.assertNotIn(records["runtime_closure"], {"passed", "ready", "complete"})
        self.assertNotIn(
            records["release_qualification"], {"passed", "ready", "complete"}
        )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--feature-register-bin", required=True, type=Path)
    return parser.parse_args()


if __name__ == "__main__":
    arguments = parse_args()
    FEATURE_REGISTER_BIN = arguments.feature_register_bin.resolve()
    if not FEATURE_REGISTER_BIN.is_file() or not os.access(FEATURE_REGISTER_BIN, os.X_OK):
        raise SystemExit(
            f"feature-register binary is not executable: {FEATURE_REGISTER_BIN}"
        )
    unittest.main(argv=[__file__])
