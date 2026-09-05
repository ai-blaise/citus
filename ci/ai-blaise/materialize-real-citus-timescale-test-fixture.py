#!/usr/bin/env python3
"""Stage the narrow source context for the real-Citus Timescale fixture."""

# FEATURE: TS6 TS18

from __future__ import annotations

import argparse
import importlib.util
import pathlib
import sys

BASE_MATERIALIZER = pathlib.Path(__file__).with_name(
    "materialize-real-citus-test-fixture.py"
)
SOURCE_INPUTS = (
    "Makefile",
    "Makefile.global.in",
    "aclocal.m4",
    "autogen.sh",
    "configure",
    "configure.ac",
    "prep_buildtree",
    "config",
    "src",
    "vendor",
    "images/citus-timescale-cohabitation/Dockerfile",
    "images/citus-timescale-cohabitation/base-image.lock.tsv",
    "images/citus-pg-overlay/extensions/ai_blaise_citus.control",
    "images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql",
    "images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0--0.1.1.sql",
    "images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.1--0.1.0.sql",
    "images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.1--0.1.2.sql",
    "images/citus-pg-overlay/upgrades/ai_blaise_citus--0.1.2.sql",
)


def load_base_materializer():
    """Load the reviewed path/type/mode-aware base materializer."""

    specification = importlib.util.spec_from_file_location(
        "real_citus_fixture_materializer", BASE_MATERIALIZER
    )
    if specification is None or specification.loader is None:
        raise RuntimeError("base real-Citus fixture materializer could not be loaded")
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=pathlib.Path, required=True)
    parser.add_argument("--destination", type=pathlib.Path, required=True)
    args = parser.parse_args()
    materializer = load_base_materializer()
    try:
        identity = materializer.materialize(
            args.source, args.destination, inputs=SOURCE_INPUTS
        )
    except (materializer.MaterializationError, OSError, UnicodeError) as error:
        print(f"real-Citus Timescale fixture context: {error}", file=sys.stderr)
        return 1
    print(identity)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
