# Feature register validator

`ai_blaise_feature_register` validates the machine-readable feature
register at `docs/features.tsv`.

```sh
cargo run -p ai_blaise_feature_register -- check
cargo run -p ai_blaise_feature_register -- summary
cargo run -p ai_blaise_feature_register -- check-legacy-coverage
cargo run -p ai_blaise_feature_register -- check-source-coverage
cargo run -p ai_blaise_feature_register -- release-gaps
cargo run -p ai_blaise_feature_register -- check --repo /path/to/repository
```

The validator checks the exact schema, enum vocabulary, ID ordering and
references, successor graph, disposition invariants, and local file
references. `summary` prints deterministic structural counts only.

`check-source-coverage` compares source marker identities with the validated
inventory. It uses Git's tracked and nonignored untracked file enumeration
across the overlay roots, excluding build outputs. Every observed marker ID
must exist in the register, but externally owned or retired identities do not
need fabricated local markers. Missing Git metadata, an empty scan,
unregistered IDs, unreadable files, and source symlinks escaping the repository
fail the check. A marker is not proof that the stated behavior is implemented.
Library crates do not need executable targets.

A green check is **not** a production-readiness, release, promotion, or runtime
claim. In particular, source paths, test definitions, documentation, and
`FEATURE:` markers do not prove that code ran successfully or that an artifact
was built, signed, promoted, or operated in production. Release certification
requires separately governed, source-bound evidence.

`release-gaps` is an interim, fail-closed release report. A structurally valid
register produces these two leading records and exits 1:

```text
feature_release_gaps\tblocked
release_evidence_verifier\tunimplemented
```

The next record is a ten-field schema row:

```text
feature_release_gap_columns\tid\ttitle\tkind\tdisposition\tcanonical_ids\tmaturity\thistorical_claimed_status\tscope\tblockers
```

It is followed by exactly one ten-field `feature_release_gap` record for every
register row, in validated ID order. Empty canonical-ID lists render as `-`.
The report includes aliases, components, evidence-only rows, external-owned
rows, tombstones, and deferred rows so historical identities do not disappear.
It deliberately has no positive release path: changing implementation maturity
or historical claimed status, or adding local source, check, or evidence files,
cannot change exit 1. Invalid or missing register input exits 2. A future
positive path requires a separately governed verifier for trusted,
current-source, full-scope release receipts.

The legacy heading/marker coverage gates remain authoritative during the TSV
migration. This tool deliberately does not infer implementation or maturity
from markers and does not hard-code the current number of feature IDs.
`check-legacy-coverage` is a temporary migration adapter: after validating the
TSV, it requires exact ID-set agreement with the legacy `### ID: title`
headings. It does not compare titles or statuses and must be deliberately
removed once every legacy parser has been retargeted.
