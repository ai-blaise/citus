#!/usr/bin/env bash
# Fail-closed audit for custom Citus patch production claims.

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "${repo_root}"

python3 - <<'PY'
import json
import pathlib
import sys

root = pathlib.Path.cwd()
errors: list[str] = []


def fail(message: str) -> None:
    errors.append(message)


def read_text(path: pathlib.Path) -> str:
    try:
        return path.read_text()
    except FileNotFoundError:
        fail(f"missing required file: {path.relative_to(root)}")
        return ""


manifest_path = root / "benchmarks/citus-patches/production-gates.json"
try:
    manifest = json.loads(read_text(manifest_path))
except json.JSONDecodeError as exc:
    print(f"invalid JSON in {manifest_path.relative_to(root)}: {exc}", file=sys.stderr)
    sys.exit(1)

required_ids = {"0004", "0006", "0007", "0008"}
patches = manifest.get("patches", [])
ids = [entry.get("id") for entry in patches]
if set(ids) != required_ids or len(ids) != len(required_ids):
    fail(
        "production-gates.json must contain exactly patch ids "
        f"{sorted(required_ids)}, got {ids}"
    )

series_path = root / "patches/series"
series_entries: list[str] = []
for raw_line in read_text(series_path).splitlines():
    line = raw_line.split("#", 1)[0].strip()
    if line:
        series_entries.append(line)

for entry in patches:
    patch_id = str(entry.get("id", ""))
    expected_patch_path = str(entry.get("expected_patch_path", ""))
    status = str(entry.get("status", ""))
    production_ready = bool(entry.get("production_ready", False))
    required_series = str(entry.get("required_patch_series", ""))
    gate = entry.get("benchmark_gate", {})

    if patch_id not in required_ids:
        continue

    if required_series != "patches/series":
        fail(f"{patch_id}: required_patch_series must be patches/series")

    if not expected_patch_path.startswith("patches/"):
        fail(f"{patch_id}: expected_patch_path must live under patches/")

    patch_path = root / expected_patch_path
    artifact_exists = patch_path.is_file()
    listed_in_series = pathlib.Path(expected_patch_path).name in series_entries

    if listed_in_series and not artifact_exists:
        fail(f"{patch_id}: patches/series lists missing artifact {expected_patch_path}")

    if artifact_exists:
        patch_text = read_text(patch_path)
        for token in ("FEATURE:", "diff --git", "@@ "):
            if token not in patch_text:
                fail(f"{patch_id}: patch artifact missing {token!r} marker")

    if not artifact_exists or not listed_in_series:
        if status != "roster-only":
            fail(f"{patch_id}: missing/not-listed patches must be status roster-only")
        if production_ready:
            fail(f"{patch_id}: missing/not-listed patch cannot be production_ready=true")
    else:
        if status == "roster-only":
            fail(f"{patch_id}: landed patch artifacts must not remain status roster-only")


    if not isinstance(entry.get("required_evidence"), list) or len(entry["required_evidence"]) < 4:
        fail(f"{patch_id}: required_evidence must list patch, check, runtime, and audit evidence")

    if not isinstance(gate, dict):
        fail(f"{patch_id}: benchmark_gate must be an object")
        gate = {}

    gate_path_raw = str(gate.get("result_path", ""))
    if not gate_path_raw.startswith("benchmarks/citus-patches/results/"):
        fail(f"{patch_id}: benchmark_gate.result_path must live under benchmarks/citus-patches/results/")

    if gate.get("required_mode") != "measured":
        fail(f"{patch_id}: benchmark_gate.required_mode must be measured")

    if not gate.get("metric"):
        fail(f"{patch_id}: benchmark_gate.metric is required")

    threshold_keys = {
        "max_regression_pct",
        "min_improvement_pct",
        "max_value",
        "min_value",
        "required_value",
        "max_registration_conflicts",
        "min_cases",
        "min_sample_count",
    }
    if not any(key in gate for key in threshold_keys):
        fail(f"{patch_id}: benchmark_gate needs at least one fail-closed threshold")

    gate_path = root / gate_path_raw if gate_path_raw else None
    gate_result = None
    if gate_path is not None and gate_path.exists():
        try:
            gate_result = json.loads(gate_path.read_text())
        except json.JSONDecodeError as exc:
            fail(f"{patch_id}: invalid result JSON at {gate_path_raw}: {exc}")
        else:
            if gate_result.get("mode") != "measured":
                fail(f"{patch_id}: existing result {gate_path_raw} must be mode=measured")
            metric = gate.get("metric")
            if metric not in gate_result:
                fail(f"{patch_id}: existing result {gate_path_raw} missing metric {metric}")
            result_text = json.dumps(gate_result, sort_keys=True).lower()
            if "scaffold" in result_text or "placeholder" in result_text or "skipped" in result_text:
                fail(f"{patch_id}: existing result {gate_path_raw} looks scaffolded/skipped")

    if production_ready:
        if not artifact_exists:
            fail(f"{patch_id}: production_ready requires patch artifact {expected_patch_path}")
        if not listed_in_series:
            fail(f"{patch_id}: production_ready requires {expected_patch_path} in patches/series")
        if gate_path is None or not gate_path.exists():
            fail(f"{patch_id}: production_ready requires measured result {gate_path_raw}")
        if gate_result is None:
            fail(f"{patch_id}: production_ready requires parseable measured result {gate_path_raw}")


upstream_sync = read_text(root / "docs/ai-blaise/UPSTREAM_SYNC.md")
benchmarks_doc = read_text(root / "docs/ai-blaise/BENCHMARKS.md")
readiness_doc = read_text(root / "docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md")

for entry in patches:
    patch_id = str(entry.get("id", ""))
    if patch_id not in required_ids:
        continue

    expected_patch_path = str(entry.get("expected_patch_path", ""))
    patch_path = root / expected_patch_path
    listed_in_series = pathlib.Path(expected_patch_path).name in series_entries
    artifact_exists = patch_path.is_file()

    row = next(
        (line for line in upstream_sync.splitlines() if line.startswith(f"| {patch_id} |")),
        "",
    )
    if not row:
        fail(f"UPSTREAM_SYNC.md missing table row for patch {patch_id}")
        continue

    if not artifact_exists or not listed_in_series:
        for phrase in ("roster-only", "not production-ready", "no `patches/*.patch` artifact"):
            if phrase not in row:
                fail(f"{patch_id}: UPSTREAM_SYNC.md row must include {phrase!r} while artifact is absent")

    for doc_path, doc_text in (
        ("docs/ai-blaise/UPSTREAM_SYNC.md", upstream_sync),
        ("docs/ai-blaise/BENCHMARKS.md", benchmarks_doc),
        ("docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md", readiness_doc),
    ):
        for line_no, line in enumerate(doc_text.splitlines(), start=1):
            lowered = line.lower()
            if patch_id in line and "production-ready" in lowered:
                allowed = (
                    "not production-ready" in lowered
                    or "non-production" in lowered
                    or "production-ready claim" in lowered
                    or "production-ready until" in lowered
                )
                if not allowed:
                    fail(f"{doc_path}:{line_no}: patch {patch_id} overclaims production-ready status")

if "benchmarks/citus-patches/production-gates.json" not in benchmarks_doc:
    fail("BENCHMARKS.md must cite benchmarks/citus-patches/production-gates.json")

for patch_id in sorted(required_ids):
    if patch_id not in benchmarks_doc:
        fail(f"BENCHMARKS.md must mention patch {patch_id}")

if "citus-patch-production-audit" not in readiness_doc:
    fail("PRODUCTION_READINESS_AUDIT.md must cite citus-patch-production-audit")

if errors:
    print("citus patch production audit failed:", file=sys.stderr)
    for error in errors:
        print(f"  - {error}", file=sys.stderr)
    sys.exit(1)

print("citus patch production audit ok: measured gates present where production_ready=true; missing gates remain fail-closed")
PY
