#!/usr/bin/env python3
"""Run metadata-change checks against real Git fixtures and the Rust validator."""

import os
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "ci/ai-blaise/features-doc-check.sh"
VALIDATOR = Path(os.environ.get("FEATURE_REGISTER_BIN", ROOT / "target/debug/ai_blaise_feature_register"))
HEADER = (ROOT / "docs/features.tsv").read_text().splitlines()[0]


class FeatureDocChangeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="feature-doc-change-")
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        self.source = self.repo / "tools/example/src/lib.rs"
        self.source.parent.mkdir(parents=True)
        self.source.write_text("// FEATURE: A\npub fn initial() {}\n")
        self.inventory = self.repo / "docs/features.tsv"
        self.inventory.parent.mkdir()
        self.inventory.write_text(
            HEADER + "\nA\tExample\tcapability\tactive\t-\tpartial\t"
            "tools/example/src/lib.rs\t-\t-\tnone\tInitial scope\tNo release evidence\n"
        )
        self.git("init", "--quiet")
        self.git("config", "user.name", "Feature fixture")
        self.git("config", "user.email", "feature-fixture@example.invalid")
        self.git("config", "commit.gpgsign", "false")
        hooks = self.repo / ".git/fixture-hooks"
        hooks.mkdir()
        self.git("config", "core.hooksPath", str(hooks))
        self.base = self.commit()

    def git(self, *args):
        return subprocess.check_output(["git", "-C", str(self.repo), *args], text=True).strip()

    def commit(self):
        self.git("add", "--all")
        self.git("commit", "--quiet", "-m", "fixture")
        return self.git("rev-parse", "HEAD")

    def run_check(self, *, head=None, base=None):
        env = os.environ.copy()
        env["BASE_SHA"] = self.base if base is None else base
        env.pop("HEAD_SHA", None)
        if head is not None:
            env["HEAD_SHA"] = head
        return subprocess.run(
            ["bash", str(SCRIPT), "--repo", str(self.repo),
             "--feature-register-bin", str(VALIDATOR)],
            capture_output=True, text=True, env=env, check=False,
        )

    def test_committed_source_change_needs_machine_inventory_not_prose(self):
        self.source.write_text(self.source.read_text() + "pub fn added() {}\n")
        prose = self.repo / "docs/ai-blaise/NEW_FEATURES.md"
        prose.parent.mkdir()
        prose.write_text("### A: A historical prose update\n")
        result = self.run_check(head=self.commit())
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("changed without updating docs/features.tsv", result.stderr)

    def test_committed_source_and_inventory_change_pass(self):
        self.source.write_text(self.source.read_text() + "pub fn added() {}\n")
        self.inventory.write_text(self.inventory.read_text().replace("Initial scope", "Extended scope"))
        result = self.run_check(head=self.commit())
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("feature_doc_change_check\tpassed", result.stdout)

    def test_local_untracked_source_is_checked_with_nul_safe_filenames(self):
        extra = self.source.parent / "new\nsource.rs"
        extra.write_text("// FEATURE: A\npub fn added() {}\n")
        result = self.run_check()
        self.assertEqual(result.returncode, 1, result.stderr)
        self.inventory.write_text(self.inventory.read_text().replace("Initial scope", "Extended scope"))
        result = self.run_check()
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_unknown_source_identity_cannot_pass_with_a_prose_update(self):
        self.source.write_text("// FEATURE: Z9\npub fn added() {}\n")
        result = self.run_check()
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("Z9 (tools/example/src/lib.rs:1)", result.stderr)

    def test_bad_revision_is_rejected(self):
        result = self.run_check(base="--not-a-revision")
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("feature_doc_change_check\tpassed", result.stdout)


if __name__ == "__main__":
    unittest.main()
