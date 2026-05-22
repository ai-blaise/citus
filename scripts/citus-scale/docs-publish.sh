#!/usr/bin/env bash
# FEATURE: D14
#
# Local equivalent of `.github/workflows/ci-docs-publish.yml`. Renders the
# mkdocs Material site and pushes a versioned subtree to the `gh-pages`
# branch via `mike`. CI is the canonical publisher; this script exists so a
# release engineer can reproduce the push locally, or repair the gh-pages
# branch when CI is unavailable.
#
# Usage:
#   scripts/citus-scale/docs-publish.sh [<mike-version-alias>]
#
# Default alias is the current `main` SHA; the script also moves the `latest`
# alias to point at the freshly deployed version.
#
# Requirements: python3, pip, git checkout of ai-blaise/citus with `gh-pages`
# fetchable. Installs mkdocs-material + mike into a venv at .venv-docs/ if
# the packages are not already on PATH.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "${ROOT}"

VERSION_ALIAS="${1:-$(git rev-parse --short HEAD)}"

if ! command -v mkdocs >/dev/null 2>&1 || ! command -v mike >/dev/null 2>&1; then
  echo "[docs-publish] mkdocs/mike not on PATH; installing into .venv-docs/" >&2
  python3 -m venv .venv-docs
  # shellcheck source=/dev/null
  source .venv-docs/bin/activate
  pip install --quiet --upgrade pip
  pip install --quiet mkdocs-material==9.5.39 mike==2.1.3
fi

echo "[docs-publish] building site with --strict"
mkdocs build --strict

echo "[docs-publish] deploying version=${VERSION_ALIAS} (also updates latest)"
mike deploy --push --update-aliases "${VERSION_ALIAS}" latest
mike set-default --push latest

echo "[docs-publish] published https://ai-blaise.github.io/citus/ (version=${VERSION_ALIAS})"
