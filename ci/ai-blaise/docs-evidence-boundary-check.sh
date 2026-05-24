#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

python3 <<'PY'
import datetime as dt
import pathlib
import re
import sys

ROOT = pathlib.Path(".")
FEATURES = ROOT / "docs/ai-blaise/NEW_FEATURES.md"
DEPLOY_README = ROOT / "deploy/README.md"
UPSTREAM_SYNC = ROOT / "docs/ai-blaise/UPSTREAM_SYNC.md"
IMAGE_OVERVIEW = ROOT / "images/README.ai-blaise.md"
PG_OVERLAY_README = ROOT / "images/citus-pg-overlay/README.md"
AUDIT = ROOT / "docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md"
RUNBOOKS = ROOT / "docs/ai-blaise/RUNBOOKS"

PRODUCTION_STATUSES = {
    "ga",
    "production",
    "production-ready",
    "production ready",
    "stable",
}


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

features_text = read(FEATURES)
heading_re = re.compile(r"^###\s+([A-Za-z][A-Za-z0-9]*):\s+(.+)$", re.M)
status_re = re.compile(r"^\*\*Status\*\*:\s*(.+)$", re.M)
production_evidence_re = re.compile(r"^Production evidence:", re.M)
headings = list(heading_re.finditer(features_text))

for index, heading in enumerate(headings):
    body_start = heading.start()
    body_end = headings[index + 1].start() if index + 1 < len(headings) else len(features_text)
    body = features_text[body_start:body_end]
    status_match = status_re.search(body)
    if not status_match:
        continue
    feature_id = heading.group(1)
    status = status_match.group(1).strip()
    normalized_status = status.lower()
    production_evidence_match = production_evidence_re.search(body)
    if normalized_status in PRODUCTION_STATUSES:
        if not production_evidence_match:
            add_failure(
                FEATURES,
                f"{feature_id} is {status!r} but lacks a Production evidence field",
                line_for(features_text, heading.start()),
            )
    elif production_evidence_match:
        add_failure(
            FEATURES,
            f"{feature_id} is {status!r} but uses the Production evidence label; use Evidence boundary until the status is promoted",
            line_for(features_text, body_start + production_evidence_match.start()),
        )

if not headings:
    add_failure(FEATURES, "no feature headings found in NEW_FEATURES.md")

docs_paths = []
for root in (ROOT / "docs/ai-blaise", ROOT / "deploy"):
    if root.exists():
        docs_paths.extend(path for path in root.rglob("*.md") if path.is_file())
for path in (IMAGE_OVERVIEW, PG_OVERLAY_README):
    if path.exists():
        docs_paths.append(path)

docs_paths = sorted(set(docs_paths))

blocked_outside_audit = {
    "production-verified": "use Status: production-ready plus measured evidence, or describe a contract/smoke boundary",
    "production certified by v2-acceptance": "V2 acceptance is modeled release gating, not production certification",
    "v2 acceptance proves production": "V2 acceptance must not be cited as production evidence",
    "full plan is production-ready": "the whole overlay is not production-ready while alpha features remain",
    "entire plan is production-ready": "the whole overlay is not production-ready while alpha features remain",
    "all custom features are production-ready": "feature status must remain per-entry and evidence-backed",
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

if failures:
    for path, line, message in failures:
        print(f"{path}:{line}: {message}", file=sys.stderr)
    sys.exit(1)

print(
    "docs_evidence_boundary_check\t"
    f"feature_headings={len(headings)}\t"
    f"docs_scanned={len(docs_paths)}\t"
    f"upstream_snapshot={snapshot_match.group(1) if snapshot_match else 'missing'}\t"
    "alpha_production_labels=0\t"
    "deploy_digest_boundary=true"
)
PY
