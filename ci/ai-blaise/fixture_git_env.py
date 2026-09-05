"""Git isolation for tests that create and operate on disposable repositories.

Do not use this helper for source-provenance operations on the real checkout.
"""

from __future__ import annotations

from collections.abc import Iterator, Mapping
from contextlib import contextmanager
import os
from unittest.mock import patch


def fixture_git_environment(
    environment: Mapping[str, str] | None = None,
) -> dict[str, str]:
    """Preserve ordinary test settings, never inherited Git routing/configuration."""

    inherited = os.environ if environment is None else environment
    isolated = {
        key: value for key, value in inherited.items() if not key.startswith("GIT_")
    }
    # Ignoring only GIT_DIR/GIT_WORK_TREE is insufficient: index/object/common-dir,
    # config-injection, namespace, and template overrides can reach the parent too.
    # User/system Git config must not supply a worktree, hooks, or template either.
    isolated["GIT_CONFIG_NOSYSTEM"] = "1"
    isolated["GIT_CONFIG_GLOBAL"] = os.devnull
    return isolated


@contextmanager
def isolated_fixture_git_environment() -> Iterator[None]:
    """Isolate a serial fixture-test scope, including nested materializer Git calls."""

    with patch.dict(os.environ, fixture_git_environment(), clear=True):
        yield
