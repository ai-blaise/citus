#!/usr/bin/env bash
set -euo pipefail

mode="${1:-audit}"
docs_file="docs/ai-blaise/NEW_FEATURES.md"
audit_file="docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md"

if [[ "${mode}" != "audit" && "${mode}" != "production-release" ]]; then
  echo "usage: $0 [audit|production-release]" >&2
  exit 2
fi

python3 - "${mode}" "${docs_file}" "${audit_file}" <<'PY'
import pathlib
import re
import sys

mode, docs_path, audit_path = sys.argv[1:4]
docs_file = pathlib.Path(docs_path)
audit_file = pathlib.Path(audit_path)
source_roots = [
    "companion",
    "sidecar",
    "pool",
    "operator",
    "e2e",
    "tools",
    "patches",
    "deploy",
    "images",
    "scripts",
]

feature_re = re.compile(r"FEATURE:\s+([A-Za-z][A-Za-z0-9]*)")
heading_re = re.compile(r"^###\s+([A-Za-z][A-Za-z0-9]*):\s+(.+)$", re.M)
status_re = re.compile(r"^\*\*Status\*\*:\s*(.+)$", re.M)
table_status_re = re.compile(
    r"^\|\s*([A-Za-z][A-Za-z0-9]*)\s*\|[^|]*\|[^|]*\|\s*([^|]+?)\s*\|",
    re.M,
)
explicit_evidence_re = re.compile(
    r"^-\s+(Executable|CI|Acceptance|SQL runtime|SQL extension):\s+`", re.M
)
contract_re = re.compile(r"\bcontract(s)?\b", re.I)


def read_text(path: pathlib.Path) -> str:
    return path.read_text(encoding="utf-8", errors="ignore")


def source_text() -> str:
    chunks = []
    for root in source_roots:
        root_path = pathlib.Path(root)
        if not root_path.exists():
            continue
        for path in root_path.rglob("*"):
            if not path.is_file():
                continue
            if ".git" in path.parts or "target" in path.parts:
                continue
            chunks.append(read_text(path))
    return "\n".join(chunks)


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    sys.exit(1)


docs = read_text(docs_file)
audit = read_text(audit_file)
sources = source_text()
docs_words = " ".join(docs.split())
audit_words = " ".join(audit.split())

source_ids = set(feature_re.findall(sources))
doc_ids = set(feature_re.findall(docs))
heading_matches = list(heading_re.finditer(docs))
heading_ids = [match.group(1) for match in heading_matches]
heading_id_set = set(heading_ids)

if not source_ids:
    fail("production readiness audit found no source FEATURE markers")

missing_docs = sorted(source_ids - doc_ids)
if missing_docs:
    fail("source FEATURE markers missing from NEW_FEATURES.md: " + ", ".join(missing_docs))

missing_sources = sorted(doc_ids - source_ids)
if missing_sources:
    fail("NEW_FEATURES.md references FEATURE ids missing from source: " + ", ".join(missing_sources))

duplicates = sorted({feature_id for feature_id in heading_ids if heading_ids.count(feature_id) > 1})
if duplicates:
    fail("duplicate feature headings in NEW_FEATURES.md: " + ", ".join(duplicates))

heading_without_source = sorted(heading_id_set - source_ids)
if heading_without_source:
    fail("feature headings missing source FEATURE markers: " + ", ".join(heading_without_source))

entries = []
for index, match in enumerate(heading_matches):
    end = heading_matches[index + 1].start() if index + 1 < len(heading_matches) else len(docs)
    body = docs[match.start():end]
    status_match = status_re.search(body)
    status = status_match.group(1).strip() if status_match else ""
    entries.append(
        {
            "id": match.group(1),
            "title": match.group(2).strip(),
            "status": status,
            "body": body,
            "has_explicit_evidence": bool(explicit_evidence_re.search(body)),
            "contract_mentions": len(contract_re.findall(body)),
        }
    )

missing_status = [entry["id"] for entry in entries if not entry["status"]]
if missing_status:
    fail("feature headings missing Status fields: " + ", ".join(sorted(missing_status)))

production_like_statuses = {"ga", "stable", "production", "production-ready", "production ready"}
production_like_entries = [
    entry for entry in entries if entry["status"].lower() in production_like_statuses
]
table_statuses = {
    match.group(1): match.group(2).strip()
    for match in table_status_re.finditer(docs)
    if match.group(1) != "ID"
}

missing_production_evidence = [
    entry["id"]
    for entry in production_like_entries
    if "Production evidence:" not in entry["body"]
]
if missing_production_evidence:
    fail(
        "production-like feature statuses lack Production evidence fields: "
        + ", ".join(sorted(missing_production_evidence))
    )

if "Whole-Repo Production Readiness Audit" not in audit:
    fail("PRODUCTION_READINESS_AUDIT.md is missing the whole-repo audit section")

if "not production-ready as a whole" not in audit_words:
    fail("PRODUCTION_READINESS_AUDIT.md must state that the overlay is not production-ready as a whole")

if "alpha means not production-ready" not in docs_words:
    fail("NEW_FEATURES.md must define alpha as not production-ready")

if mode == "production-release":
    non_production = [
        entry["id"]
        for entry in entries
        if entry["status"].lower() not in production_like_statuses
    ]
    non_production.extend(
        sorted(
            feature_id
            for feature_id in source_ids - heading_id_set
            if table_statuses.get(feature_id, "").lower() not in production_like_statuses
        )
    )
    if non_production:
        fail(
            "production release blocked: non-production feature statuses remain: "
            + ", ".join(sorted(non_production))
        )

status_counts = {}
for entry in entries:
    status_counts[entry["status"]] = status_counts.get(entry["status"], 0) + 1

heading_without_explicit_evidence = sorted(
    entry["id"] for entry in entries if not entry["has_explicit_evidence"]
)
contract_entries = sorted(entry["id"] for entry in entries if entry["contract_mentions"])
source_only_doc_refs = sorted(source_ids - heading_id_set)

print(
    "production_readiness_audit\t"
    f"mode={mode}\t"
    f"source_feature_ids={len(source_ids)}\t"
    f"doc_feature_refs={len(doc_ids)}\t"
    f"feature_headings={len(entries)}\t"
    f"status_counts={status_counts}\t"
    f"production_like_headings={len(production_like_entries)}\t"
    f"source_only_doc_refs={len(source_only_doc_refs)}\t"
    f"headings_without_explicit_evidence={len(heading_without_explicit_evidence)}\t"
    f"contract_headings={len(contract_entries)}"
)

if heading_without_explicit_evidence:
    print("headings_without_explicit_evidence=" + ",".join(heading_without_explicit_evidence))
if source_only_doc_refs:
    print("source_only_doc_refs=" + ",".join(source_only_doc_refs))
PY
