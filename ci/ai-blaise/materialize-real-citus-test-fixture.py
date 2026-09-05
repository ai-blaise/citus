#!/usr/bin/env python3
"""Stage and fingerprint tracked plus nonignored real-Citus fixture inputs."""

from __future__ import annotations

import argparse
import hashlib
import os
import pathlib
import shutil
import stat
import struct
import subprocess
import sys
from collections.abc import Iterable

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
    "images/citus-test-fixture/Dockerfile",
    "images/citus-pg-overlay/extensions/ai_blaise_citus.control",
    "images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql",
    "images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0--0.1.1.sql",
    "images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.1--0.1.0.sql",
    "images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.1--0.1.2.sql",
)


class MaterializationError(ValueError):
    """The selected source cannot form a closed regular-file fixture context."""


def _is_within(path: pathlib.Path, root: pathlib.Path) -> bool:
    try:
        path.relative_to(root)
    except ValueError:
        return False
    return True


def _copy_input(source: pathlib.Path, destination: pathlib.Path, relative: str) -> None:
    source_path = source / relative
    destination_path = destination / relative
    try:
        metadata = source_path.lstat()
    except FileNotFoundError as error:
        raise MaterializationError(
            f"missing fixture source input: {relative}"
        ) from error
    if stat.S_ISLNK(metadata.st_mode):
        resolved = source_path.resolve(strict=True)
        if not _is_within(resolved, source):
            raise MaterializationError(
                f"fixture source symlink escapes repository: {relative}"
            )
        destination_path.parent.mkdir(parents=True, exist_ok=True)
        destination_path.symlink_to(os.readlink(source_path))
    elif stat.S_ISREG(metadata.st_mode):
        destination_path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source_path, destination_path, follow_symlinks=False)
    else:
        raise MaterializationError(f"unsupported fixture source input type: {relative}")


def _source_inventory(source: pathlib.Path, inputs: tuple[str, ...]) -> tuple[str, ...]:
    """Return tracked and nonignored worktree files selected by ``inputs``."""

    try:
        repository_root = subprocess.run(
            ["git", "-C", str(source), "rev-parse", "--show-toplevel"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError) as error:
        raise MaterializationError(
            "fixture source must be a readable Git worktree"
        ) from error
    if pathlib.Path(repository_root).resolve() != source:
        raise MaterializationError("fixture source must be the Git worktree root")

    try:
        inventory_bytes = subprocess.run(
            [
                "git",
                "-C",
                str(source),
                "ls-files",
                "--cached",
                "--others",
                "--exclude-standard",
                "-z",
                "--",
                *inputs,
            ],
            check=True,
            capture_output=True,
        ).stdout
        inventory_text = inventory_bytes.decode("utf-8")
    except (OSError, subprocess.CalledProcessError, UnicodeDecodeError) as error:
        raise MaterializationError(
            "fixture source inventory could not be read from Git"
        ) from error

    paths: list[str] = []
    for entry in inventory_text.split("\0"):
        if not entry:
            continue
        candidate = pathlib.PurePosixPath(entry)
        if candidate.is_absolute() or ".." in candidate.parts:
            raise MaterializationError("Git returned an invalid fixture source path")
        path = source / candidate
        if path.exists() or path.is_symlink():
            paths.append(candidate.as_posix())

    selected = tuple(sorted(set(paths)))
    for requested in inputs:
        prefix = f"{requested.rstrip('/')}/"
        if requested not in selected and not any(
            path.startswith(prefix) for path in selected
        ):
            raise MaterializationError(
                f"fixture source input has no tracked or nonignored files: {requested}"
            )
    return selected


def _entries(root: pathlib.Path) -> list[pathlib.Path]:
    entries: list[pathlib.Path] = []
    for current, directory_names, file_names in os.walk(root, followlinks=False):
        directory_names.sort()
        file_names.sort()
        current_path = pathlib.Path(current)
        for name in directory_names + file_names:
            entries.append(current_path / name)
    return sorted(entries, key=lambda path: path.relative_to(root).as_posix())


def fingerprint(root: pathlib.Path) -> str:
    """Hash path, type, permission bits, link target, and regular-file bytes."""

    digest = hashlib.sha256(b"ai-blaise/real-citus-test-fixture-context/v1\0")
    for path in _entries(root):
        relative = path.relative_to(root).as_posix().encode("utf-8")
        metadata = path.lstat()
        digest.update(struct.pack(">Q", len(relative)))
        digest.update(relative)
        if stat.S_ISDIR(metadata.st_mode):
            digest.update(b"d")
            digest.update(struct.pack(">I", 0))
        elif stat.S_ISLNK(metadata.st_mode):
            resolved = path.resolve(strict=True)
            if not _is_within(resolved, root):
                raise MaterializationError(
                    f"staged fixture symlink escapes context: {path.relative_to(root)}"
                )
            target = os.readlink(path).encode("utf-8")
            digest.update(b"l")
            digest.update(struct.pack(">I", 0))
            digest.update(struct.pack(">Q", len(target)))
            digest.update(target)
        elif stat.S_ISREG(metadata.st_mode):
            digest.update(b"f")
            digest.update(struct.pack(">I", stat.S_IMODE(metadata.st_mode)))
            digest.update(struct.pack(">Q", metadata.st_size))
            with path.open("rb") as handle:
                while chunk := handle.read(1024 * 1024):
                    digest.update(chunk)
        else:
            raise MaterializationError(
                f"unsupported staged fixture input type: {path.relative_to(root)}"
            )
    return digest.hexdigest()


def materialize(
    source: pathlib.Path,
    destination: pathlib.Path,
    inputs: Iterable[str] = SOURCE_INPUTS,
) -> str:
    source = source.resolve(strict=True)
    destination = destination.resolve(strict=True)
    if _is_within(destination, source) or _is_within(source, destination):
        raise MaterializationError("fixture source and destination must not overlap")
    if any(destination.iterdir()):
        raise MaterializationError("fixture destination must be empty")
    selected_inputs = tuple(inputs)
    for relative in selected_inputs:
        candidate = pathlib.PurePosixPath(relative)
        if candidate.is_absolute() or ".." in candidate.parts:
            raise MaterializationError(f"invalid fixture source input: {relative}")
    for relative in _source_inventory(source, selected_inputs):
        _copy_input(source, destination, relative)
    return fingerprint(destination)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=pathlib.Path, required=True)
    parser.add_argument("--destination", type=pathlib.Path, required=True)
    args = parser.parse_args()
    try:
        identity = materialize(args.source, args.destination)
    except (MaterializationError, OSError, UnicodeError) as error:
        print(f"real-Citus fixture context: {error}", file=sys.stderr)
        return 1
    print(identity)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
