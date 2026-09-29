#!/usr/bin/env python3
"""Extract the exact product function, never a hand-copied implementation."""

import hashlib
import pathlib
import re
import sys

source = pathlib.Path(sys.argv[1]).read_bytes()
text = source.decode("utf-8")
tokens = re.compile(
    r'/\*.*?\*/|//[^\n]*|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|[{}]', re.S
)
parts = []
for name in (
    "GetTupleForTargetSchemaForCdc",
    "HasSchemaChanged",
    "TranslateChangesIfSchemaChanged",
    "TranslateAndPublishRelationForCDC",
):
    match = re.search(r"^static (?:void|bool|HeapTuple)\n" + name + r"\(", text, re.M)
    if match is None:
        raise SystemExit("product function definition missing: " + name)
    start = text.index("{", match.start())
    depth = 0
    end = None
    for token in tokens.finditer(text, start):
        if token.group() == "{":
            depth += 1
        elif token.group() == "}":
            depth -= 1
            if depth == 0:
                end = token.end()
                break
    if end is None:
        raise SystemExit("unbalanced product function: " + name)
    parts.append(text[match.start() : end] + "\n")
body = "\n".join(parts)
pathlib.Path(sys.argv[2]).write_text(body)
print("source_sha256=" + hashlib.sha256(source).hexdigest())
print("extracted_sha256=" + hashlib.sha256(body.encode()).hexdigest())
print("pg16_ownership_slots=" + str(int("translatedNewTuple" in body)))
