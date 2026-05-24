#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

python3 <<'PY'
import dataclasses
import pathlib
import re
import shlex
import subprocess
import sys
import textwrap
from typing import Iterable

ROOT = pathlib.Path(".")
RUNBOOK_DIR = ROOT / "docs/ai-blaise/RUNBOOKS"
SHELL_LANGS = {"bash", "sh", "shell"}
SQL_LANGS = {"sql", "psql"}
DRY_RUN_LANGS = SHELL_LANGS | SQL_LANGS | {"yaml", "yml"}
PLACEHOLDER_RE = re.compile(r"<[A-Za-z][A-Za-z0-9_.:-]*>")
SCRIPT_REF_RE = re.compile(
    r"(?<![A-Za-z0-9_./-])"
    r"((?:ci|scripts|benchmarks|tests)/[A-Za-z0-9_./-]+\.sh)"
)
ABS_SIDECAR_RE = re.compile(r"/usr/local/bin/citus-sidecar-([a-z0-9-]+)\b")
MAKE_RE = re.compile(r"\bmake\s+-f\s+Makefile\.ai-blaise\s+([A-Za-z0-9_.:-]+)\b")
CARGO_PACKAGE_RE = re.compile(r"^name\s*=\s*\"([^\"]+)\"", re.M)

SIDE_CAR_DIR_ALIASES = {
    "edge-functions": "edge_functions",
    "schema-job": "schema_job",
    "txn-status": "txn_status",
}


@dataclasses.dataclass(frozen=True)
class CodeBlock:
    path: pathlib.Path
    start_line: int
    end_line: int
    language: str
    text: str

    @property
    def location(self) -> str:
        return f"{self.path}:{self.start_line}"


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    sys.exit(1)


def read_text(path: pathlib.Path) -> str:
    return path.read_text(encoding="utf-8", errors="ignore")


def iter_runbooks() -> Iterable[pathlib.Path]:
    return sorted(RUNBOOK_DIR.glob("*.md"))


def iter_code_blocks(path: pathlib.Path) -> Iterable[CodeBlock]:
    lines = read_text(path).splitlines()
    in_block = False
    language = ""
    start_line = 0
    block_lines: list[str] = []

    for line_number, line in enumerate(lines, start=1):
        match = re.match(r"^\s*```\s*([^`]*)$", line)
        if not match:
            if in_block:
                block_lines.append(line)
            continue

        if not in_block:
            in_block = True
            language = match.group(1).strip().lower()
            start_line = line_number + 1
            block_lines = []
            continue

        text = textwrap.dedent("\n".join(block_lines)).strip("\n")
        yield CodeBlock(
            path=path,
            start_line=start_line,
            end_line=line_number - 1,
            language=language,
            text=text,
        )
        in_block = False
        language = ""
        start_line = 0
        block_lines = []

    if in_block:
        fail(f"unterminated fenced code block in {path}:{start_line - 1}")


def normalize_shell(text: str) -> str:
    normalized = PLACEHOLDER_RE.sub("PLACEHOLDER", text)
    # Keep examples parseable when operators paste a block with an unset release tag.
    normalized = normalized.replace('${RELEASE_TAG}', '${RELEASE_TAG:-release-candidate}')
    return normalized


def shell_syntax_errors(blocks: Iterable[CodeBlock]) -> list[str]:
    errors: list[str] = []
    for block in blocks:
        script = "set -euo pipefail\n" + normalize_shell(block.text) + "\n"
        result = subprocess.run(
            ["bash", "-n"],
            input=script,
            text=True,
            capture_output=True,
            check=False,
        )
        if result.returncode != 0:
            detail = (result.stderr or result.stdout).strip()
            errors.append(f"{block.location}: bash -n failed: {detail}")
    return errors


def sql_shape_errors(blocks: Iterable[CodeBlock]) -> list[str]:
    errors: list[str] = []
    for block in blocks:
        text = block.text.strip()
        if not text:
            errors.append(f"{block.location}: empty SQL block")
            continue
        if ";" not in text:
            errors.append(f"{block.location}: SQL block has no statement terminator")
    return errors


def package_roots() -> dict[str, pathlib.Path]:
    roots: dict[str, pathlib.Path] = {}
    for cargo_toml in ROOT.rglob("Cargo.toml"):
        if ".git" in cargo_toml.parts or "target" in cargo_toml.parts:
            continue
        match = CARGO_PACKAGE_RE.search(read_text(cargo_toml))
        if match:
            roots[match.group(1)] = cargo_toml.parent
    return roots


def make_targets() -> set[str]:
    makefile = ROOT / "Makefile.ai-blaise"
    if not makefile.exists():
        return set()
    targets: set[str] = set()
    for line in read_text(makefile).splitlines():
        if line.startswith("\t") or not line or line.startswith("#"):
            continue
        match = re.match(r"^([A-Za-z0-9_.:-]+)\s*:(?![=])", line)
        if match:
            targets.add(match.group(1))
    return targets


def shlex_tokens(text: str) -> list[str]:
    lexer = shlex.shlex(normalize_shell(text), posix=True, punctuation_chars=True)
    lexer.whitespace_split = True
    lexer.commenters = ""
    try:
        return list(lexer)
    except ValueError:
        # bash -n reports the primary syntax failure; keep token checks bounded.
        return []


def cargo_errors(blocks: Iterable[CodeBlock], packages: dict[str, pathlib.Path]) -> list[str]:
    errors: list[str] = []
    for block in blocks:
        tokens = shlex_tokens(block.text)
        for index, token in enumerate(tokens[:-1]):
            if token != "cargo" or tokens[index + 1] != "run":
                continue
            command_tokens: list[str] = []
            for value in tokens[index + 2 :]:
                if value in {"|", ";", "&&", "||"}:
                    break
                command_tokens.append(value)

            package = None
            binary = None
            for i, value in enumerate(command_tokens):
                if value in {"-p", "--package"} and i + 1 < len(command_tokens):
                    package = command_tokens[i + 1]
                elif value.startswith("--package="):
                    package = value.split("=", 1)[1]
                elif value == "--bin" and i + 1 < len(command_tokens):
                    binary = command_tokens[i + 1]
                elif value.startswith("--bin="):
                    binary = value.split("=", 1)[1]

            if package is None:
                errors.append(f"{block.location}: cargo run is missing -p/--package")
                continue
            package_root = packages.get(package)
            if package_root is None:
                errors.append(f"{block.location}: cargo package not found: {package}")
                continue
            if binary is not None and not (package_root / "src/bin" / f"{binary}.rs").exists():
                errors.append(
                    f"{block.location}: cargo --bin {binary} missing under {package_root}/src/bin"
                )
            if binary is None and not (package_root / "src/main.rs").exists():
                errors.append(
                    f"{block.location}: cargo package {package} has no default binary src/main.rs"
                )
    return errors


def script_ref_errors(blocks: Iterable[CodeBlock]) -> list[str]:
    errors: list[str] = []
    for block in blocks:
        for match in SCRIPT_REF_RE.finditer(block.text):
            ref = match.group(1).rstrip(".,:;)")
            path = ROOT / ref
            if not path.exists():
                errors.append(f"{block.location}: referenced script is missing: {ref}")
            elif not path.is_file():
                errors.append(f"{block.location}: referenced script is not a file: {ref}")
    return errors


def sidecar_binary_errors(blocks: Iterable[CodeBlock]) -> list[str]:
    errors: list[str] = []
    for block in blocks:
        for match in ABS_SIDECAR_RE.finditer(block.text):
            sidecar = match.group(1)
            directory = SIDE_CAR_DIR_ALIASES.get(sidecar, sidecar.replace("-", "_"))
            main_rs = ROOT / "sidecar" / directory / "src/main.rs"
            cargo_toml = ROOT / "sidecar" / directory / "Cargo.toml"
            if not main_rs.exists() or not cargo_toml.exists():
                errors.append(
                    f"{block.location}: /usr/local/bin/citus-sidecar-{sidecar} "
                    f"has no matching sidecar/{directory} binary source"
                )
    return errors


def make_target_errors(blocks: Iterable[CodeBlock], targets: set[str]) -> list[str]:
    errors: list[str] = []
    for block in blocks:
        for match in MAKE_RE.finditer(normalize_shell(block.text)):
            target = match.group(1)
            if target not in targets:
                errors.append(f"{block.location}: Makefile.ai-blaise target not found: {target}")
    return errors


def coverage_errors(runbooks: Iterable[pathlib.Path], blocks: list[CodeBlock]) -> list[str]:
    by_path: dict[pathlib.Path, list[CodeBlock]] = {}
    for block in blocks:
        by_path.setdefault(block.path, []).append(block)

    errors: list[str] = []
    for runbook in runbooks:
        text = read_text(runbook)
        has_operational_section = bool(
            re.search(
                r"^##\s+(Recovery procedure|Restore Drill|Restore procedure|Regional Failover Drill|Canary Flow|Rollback|Switching traffic)",
                text,
                re.M,
            )
        )
        if not has_operational_section:
            continue
        machine_checked = [
            block
            for block in by_path.get(runbook, [])
            if block.language in DRY_RUN_LANGS and block.text.strip()
        ]
        if not machine_checked:
            errors.append(
                f"{runbook}: operational runbook lacks fenced bash/sql/yaml command examples"
            )
    return errors


def language_errors(blocks: Iterable[CodeBlock]) -> list[str]:
    errors: list[str] = []
    for block in blocks:
        if block.language == "":
            errors.append(f"{block.location}: fenced code block is missing a language tag")
    return errors


runbooks = list(iter_runbooks())
if not runbooks:
    fail(f"no runbooks found under {RUNBOOK_DIR}")

blocks = [block for runbook in runbooks for block in iter_code_blocks(runbook)]
shell_blocks = [block for block in blocks if block.language in SHELL_LANGS]
sql_blocks = [block for block in blocks if block.language in SQL_LANGS]
packages = package_roots()
targets = make_targets()

errors: list[str] = []
errors.extend(language_errors(blocks))
errors.extend(coverage_errors(runbooks, blocks))
errors.extend(shell_syntax_errors(shell_blocks))
errors.extend(sql_shape_errors(sql_blocks))
errors.extend(cargo_errors(shell_blocks, packages))
errors.extend(script_ref_errors(shell_blocks))
errors.extend(sidecar_binary_errors(shell_blocks))
errors.extend(make_target_errors(shell_blocks, targets))

if errors:
    for error in errors:
        print(error, file=sys.stderr)
    sys.exit(1)

print(
    "runbook_command_check\t"
    f"runbooks={len(runbooks)}\t"
    f"code_blocks={len(blocks)}\t"
    f"shell_blocks={len(shell_blocks)}\t"
    f"sql_blocks={len(sql_blocks)}\t"
    f"cargo_packages={len(packages)}\t"
    f"make_targets={len(targets)}"
)
PY
