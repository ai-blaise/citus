#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

cargo run --locked --quiet -p ai_blaise_feature_register -- check

python3 <<'PY'
import datetime as dt
import pathlib
import re
import sys

ROOT = pathlib.Path(".")
DEPLOY_README = ROOT / "deploy/README.md"
UPSTREAM_SYNC = ROOT / "docs/ai-blaise/UPSTREAM_SYNC.md"
IMAGE_OVERVIEW = ROOT / "images/README.ai-blaise.md"
PG_OVERLAY_README = ROOT / "images/citus-pg-overlay/README.md"
AUDIT = ROOT / "docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md"
RUNBOOKS = ROOT / "docs/ai-blaise/RUNBOOKS"


def read(path: pathlib.Path) -> str:
    if not path.exists():
        failures.append((path, 1, f"missing required docs input: {path}"))
        return ""
    return path.read_text(encoding="utf-8", errors="ignore")


def line_for(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def add_failure(path: pathlib.Path, message: str, line: int = 1) -> None:
    failures.append((path, line, message))


def compact(text: str) -> str:
    return " ".join(text.split()).lower()


failures = []

docs_paths = []
for root in (ROOT / "docs/ai-blaise", ROOT / "deploy"):
    if root.exists():
        docs_paths.extend(path for path in root.rglob("*.md") if path.is_file())
for path in (IMAGE_OVERVIEW, PG_OVERLAY_README):
    if path.exists():
        docs_paths.append(path)

docs_paths = sorted(set(docs_paths))

blocked_outside_audit = {
    "production-verified": "describe the exact source-bound evidence and its scope; a status label cannot certify production",
    "production certified by v2-acceptance": "V2 acceptance is modeled release gating, not production certification",
    "v2 acceptance proves production": "V2 acceptance must not be cited as production evidence",
    "full plan is production-ready": "whole-overlay qualification is not established by source, model, or inventory checks",
    "entire plan is production-ready": "whole-overlay qualification is not established by source, model, or inventory checks",
    "all custom features are production-ready": "each claim needs current-source full-scope qualification, not a label",
}

for path in docs_paths:
    text = read(path)
    lower = text.lower()
    if path != AUDIT:
        for phrase, guidance in blocked_outside_audit.items():
            for match in re.finditer(re.escape(phrase), lower):
                add_failure(path, f"blocked overclaiming phrase {phrase!r}: {guidance}", line_for(text, match.start()))

    if "still published from this repository" in lower:
        add_failure(
            path,
            "avoid implying image publication already happened; require release push plus digest manifest evidence",
            line_for(text, lower.index("still published from this repository")),
        )

sidecar_values_prod_re = re.compile(
    r"`sidecar/[^`]+`\s+is enabled in\s+`values-prod\.yaml`",
    re.I,
)
if RUNBOOKS.exists():
    for path in sorted(RUNBOOKS.glob("*.md")):
        text = read(path)
        for match in sidecar_values_prod_re.finditer(text):
            add_failure(
                path,
                "runbooks must not assume alpha sidecars are enabled by values-prod.yaml; require an explicit promoted release overlay",
                line_for(text, match.start()),
            )

deploy_text = read(DEPLOY_README)
deploy_required = {
    "ai-blaise/command-center": "canonical chart handoff",
    "artifacts/ai-blaise-image-digests.tsv": "image digest manifest prerequisite",
    "OPERATOR_IMAGE_DIGEST": "operator digest handoff",
    "POOL_IMAGE_DIGEST": "pool digest handoff",
    "sha256:": "immutable digest requirement",
    "not proof of publication": "source-vs-publication boundary",
}
deploy_compact = compact(deploy_text)
for phrase, purpose in deploy_required.items():
    if phrase.lower() not in deploy_compact:
        add_failure(DEPLOY_README, f"deploy README must document {purpose}: {phrase}")

image_text = read(IMAGE_OVERVIEW)
for phrase in (
    "scripts/citus-scale/build-app-images.sh",
    "artifacts/ai-blaise-image-digests.tsv",
    "immutable repo digest",
):
    if phrase.lower() not in compact(image_text):
        add_failure(IMAGE_OVERVIEW, f"image overview must preserve release image evidence boundary: {phrase}")

upstream_text = read(UPSTREAM_SYNC)
snapshot_match = re.search(r"Status snapshot:\s*(\d{4}-\d{2}-\d{2})", upstream_text)
if not snapshot_match:
    add_failure(UPSTREAM_SYNC, "UPSTREAM_SYNC.md must include a Status snapshot date for PR/branch state")
else:
    snapshot = dt.date.fromisoformat(snapshot_match.group(1))
    today = dt.date.today()
    max_age_days = 45
    if snapshot > today:
        add_failure(UPSTREAM_SYNC, f"Status snapshot {snapshot} is in the future")
    elif (today - snapshot).days > max_age_days:
        add_failure(
            UPSTREAM_SYNC,
            f"Status snapshot {snapshot} is older than {max_age_days} days; refresh stale PR/branch state",
            line_for(upstream_text, snapshot_match.start()),
        )

if "not live pr evidence" not in compact(upstream_text):
    add_failure(UPSTREAM_SYNC, "UPSTREAM_SYNC.md must state that the snapshot is not live PR evidence")

audit_text = read(AUDIT)
if "Whole-Repo Production Readiness Audit" not in audit_text:
    add_failure(AUDIT, "missing whole-repo audit section")

if failures:
    for path, line, message in failures:
        print(f"{path}:{line}: {message}", file=sys.stderr)
    sys.exit(1)

print(
    "docs_evidence_boundary_check\t"
    f"docs_scanned={len(docs_paths)}\t"
    f"upstream_snapshot={snapshot_match.group(1) if snapshot_match else 'missing'}\t"
    "authority=docs-and-inventory-only\t"
    "deploy_digest_boundary=true"
)
PY
