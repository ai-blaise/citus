#!/usr/bin/env python3
"""Regression proof that disposable Git fixtures cannot mutate their parent repo."""

from __future__ import annotations

import importlib.util
import io
import os
from pathlib import Path
import stat
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from fixture_git_env import fixture_git_environment, isolated_fixture_git_environment


HERE = Path(__file__).resolve().parent


def load_fixture_tests(filename: str):
    specification = importlib.util.spec_from_file_location(
        filename.removesuffix(".py").replace("-", "_"), HERE / filename
    )
    if specification is None or specification.loader is None:
        raise RuntimeError("fixture regression module could not be imported")
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


def snapshot_tree(root: Path) -> dict[str, tuple[int, bytes]]:
    """Capture config, index, refs, objects, and source, without invoking Git."""

    return {
        path.relative_to(root).as_posix(): (
            stat.S_IMODE(path.stat().st_mode),
            path.read_bytes(),
        )
        for path in sorted(root.rglob("*"))
        if path.is_file()
    }


class FixtureGitEnvironmentTests(unittest.TestCase):
    def test_every_inherited_git_override_is_removed_without_mutating_input(self):
        inherited = {
            "PATH": "/test/bin",
            "LC_ALL": "C",
            "TMPDIR": "/test/tmp",
            "GIT_DIR": "/parent/.git",
            "GIT_WORK_TREE": "/parent",
            "GIT_COMMON_DIR": "/parent/.git",
            "GIT_INDEX_FILE": "/parent/.git/index",
            "GIT_OBJECT_DIRECTORY": "/parent/.git/objects",
            "GIT_ALTERNATE_OBJECT_DIRECTORIES": "/parent/other-objects",
            "GIT_NAMESPACE": "parent-namespace",
            "GIT_CONFIG": "/parent/.git/config",
            "GIT_CONFIG_COUNT": "1",
            "GIT_CONFIG_KEY_0": "core.worktree",
            "GIT_CONFIG_VALUE_0": "/parent",
            "GIT_CONFIG_PARAMETERS": "'core.bare=false'",
            "GIT_CONFIG_GLOBAL": "/parent/global-config",
            "GIT_CONFIG_SYSTEM": "/parent/system-config",
            "GIT_TEMPLATE_DIR": "/parent/template",
            "GIT_TRACE": "/parent/trace",
            "GIT_FUTURE_ROUTING_OVERRIDE": "/parent/future",
        }
        original = inherited.copy()
        isolated = fixture_git_environment(inherited)
        self.assertEqual(inherited, original)
        self.assertEqual(
            isolated,
            {
                "PATH": "/test/bin",
                "LC_ALL": "C",
                "TMPDIR": "/test/tmp",
                "GIT_CONFIG_NOSYSTEM": "1",
                "GIT_CONFIG_GLOBAL": os.devnull,
            },
        )

    def test_fixture_scope_restores_environment_even_after_failure(self):
        with patch.dict(os.environ, {"GIT_DIR": "/parent/.git", "KEEP_SETTING": "yes"}):
            inherited = dict(os.environ)
            with self.assertRaisesRegex(RuntimeError, "fixture failure"):
                with isolated_fixture_git_environment():
                    self.assertNotIn("GIT_DIR", os.environ)
                    self.assertEqual(os.environ["KEEP_SETTING"], "yes")
                    raise RuntimeError("fixture failure")
            self.assertEqual(dict(os.environ), inherited)

    def test_real_fixture_tests_preserve_parent_git_and_source_under_overrides(self):
        citus = load_fixture_tests("real-citus-test-fixture-contract_test.py")
        timescale = load_fixture_tests("real-citus-timescale-test-fixture-contract_test.py")
        cases = (
            citus.RealCitusFixtureContractTests(
                "test_http_builder_builds_the_prehashed_snapshot_after_checkout_drift"
            ),
            citus.RealCitusFixtureContractTests(
                "test_context_fingerprint_binds_bytes_modes_and_symlinks"
            ),
            timescale.RealCitusTimescaleFixtureContractTests(
                "test_materializer_excludes_ignored_objects_and_binds_local_source"
            ),
        )
        # Never point the deliberately hostile overrides at the live checkout. The parent
        # sentinel is itself disposable, but contains a real config, index, refs, and objects.
        with tempfile.TemporaryDirectory(prefix="fixture-git-parent-regression-") as directory:
            parent = Path(directory)
            environment = fixture_git_environment()

            def parent_git(*arguments: str) -> None:
                subprocess.run(
                    ["git", "-C", str(parent), *arguments],
                    check=True,
                    capture_output=True,
                    text=True,
                    env=environment,
                )

            parent_git("init", "--quiet")
            parent_git("config", "user.name", "Fixture isolation regression")
            parent_git("config", "user.email", "fixture-isolation@example.invalid")
            parent_git("config", "commit.gpgsign", "false")
            (parent / "source.txt").write_text("committed parent source\n", encoding="utf-8")
            parent_git("add", "source.txt")
            parent_git("commit", "--quiet", "-m", "parent sentinel")
            (parent / "source.txt").write_text("unstaged parent source\n", encoding="utf-8")
            (parent / "staged.txt").write_text("staged parent source\n", encoding="utf-8")
            parent_git("add", "staged.txt")
            (parent / "untracked.txt").write_text("untracked parent source\n", encoding="utf-8")
            before = snapshot_tree(parent)
            self.assertIn(".git/config", before)
            self.assertIn(".git/index", before)
            self.assertTrue(any(path.startswith(".git/refs/") for path in before))

            overrides = {
                "GIT_DIR": str(parent / ".git"),
                "GIT_WORK_TREE": str(parent),
                "GIT_COMMON_DIR": str(parent / ".git"),
                "GIT_INDEX_FILE": str(parent / ".git/index"),
                "GIT_OBJECT_DIRECTORY": str(parent / ".git/objects"),
                "GIT_NAMESPACE": "fixture-parent",
                "GIT_CONFIG": str(parent / ".git/config"),
                "GIT_CONFIG_COUNT": "1",
                "GIT_CONFIG_KEY_0": "core.worktree",
                "GIT_CONFIG_VALUE_0": str(parent),
                "GIT_CONFIG_PARAMETERS": "'core.bare=false'",
            }
            with patch.dict(os.environ, overrides):
                inherited = dict(os.environ)
                for case in cases:
                    with self.subTest(case=case.id()):
                        output = io.StringIO()
                        result = unittest.TextTestRunner(stream=output).run(case)
                        self.assertTrue(result.wasSuccessful(), output.getvalue())
                        self.assertEqual(dict(os.environ), inherited)
                        self.assertEqual(snapshot_tree(parent), before)


if __name__ == "__main__":
    unittest.main()
