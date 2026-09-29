# Feature register

[`features.tsv`](features.tsv) is the structured feature inventory being
migrated from the legacy [prose register](ai-blaise/NEW_FEATURES.md). It retains
every historical ID, including aliases, components, benchmark artifacts, and
retired entry points. Consolidation changes catalog enumeration; it does not
delete SQL objects, APIs, compatibility records, or tests.

This migration is not complete. The legacy prose remains temporarily for the
explicit historical-ID coverage adapter and documentation references that have
not yet been retired. Its `production-ready` labels are preserved only in the
`claimed_status` column; they are not evidence-derived implementation status
and must not authorize a release.

## Read the dimensions separately

`kind` describes the artifact: product, integration, capability, tool,
operator, runbook, guard, or benchmark. An active capability is not necessarily
a separately deployable service.

`disposition` describes ownership and enumeration:

- `active`: an independently tracked requirement pending its own acceptance.
- `alias`: a historical identity whose behavior is retained by its canonical
  IDs. A composite alias can name more than one owner.
- `component-of`: distinct behavior and tests retained inside another feature.
- `evidence-only`: a test or benchmark artifact, not another product.
- `external-owned`: implementation belongs to another repository, identified
  in the scope; local markers do not establish implementation.
- `tombstone`: an intentionally retired entry point retained for compatibility.
- `deferred`: a positive capability intentionally not available yet.

`maturity` describes implementation review, not deployment readiness:

- `unreviewed`: the imported title-level behavior has not been independently
  reviewed in full. Source references do not make it implemented.
- `unimplemented`: the advertised positive capability is absent. A safe
  unavailable response is not an implementation of that capability.
- `model-only`: only a model or plan implements the claimed behavior.
- `partial`: a bounded part exists, with the missing behavior recorded in
  `scope` and `blockers`.
- `implemented`: the stated bounded behavior is implemented; this alone says
  nothing about production qualification, operational coverage, or performance.

There is deliberately no `production-ready` maturity value. A machine-readable
copy of an unsupported label would repeat the old failure. Production
qualification requires current-source, full-scope functional, security,
recovery, upgrade, and comparative performance evidence, plus the required
licensing and trusted release provenance. A green structural register check
does not supply that evidence.

## TSV contract

The file has one UTF-8 header and one row per stable ID, sorted by bytewise ID
order (`MB1`, `MB10`, ..., `MB2`). Fields may not contain tabs, line breaks, or
other control characters. List fields use semicolons; `-` means an empty list.

| Column | Meaning |
| --- | --- |
| `id`, `title` | Stable identity and human-readable title. |
| `kind`, `disposition` | Artifact type and reviewed ownership/enumeration. |
| `canonical_ids` | Existing successor or owner IDs; self-links and cycles are invalid. |
| `maturity` | Implementation review state, limited to the vocabulary above. |
| `source_paths` | Referenced in-repository source or packaging files; not an exhaustive dependency closure. |
| `check_paths` | Existing test, smoke, model, benchmark, or CI definitions. Their presence is not a passing run. |
| `evidence_paths` | Existing retained observation records. Read each record's source identity and scope; development evidence cannot certify production. |
| `claimed_status` | Historical prose label preserved during migration, never a release decision. |
| `scope`, `blockers` | Bounded behavior and missing requirements, including cross-repository ownership when applicable. |

Every local path must be normalized, resolve inside this repository, and name
an existing file. Missing `artifacts/...` outputs are not inserted as evidence.
External implementation ownership is described in `scope`; it must not be
represented by a path traversal to a neighboring checkout.

## Seed decisions

The reviewed consolidation set distinguishes aliases from retained components.
For example, S14 is a composite alias for tenant movement and quotas, while
TS8 remains a distinct hypertable-diagnostics component of the LSP. Edge1 is
kept separate because its region/replica eligibility checks differ from the
follower-read boundary. Shared code alone is not grounds for deleting a
requirement.

Microbenchmark IDs remain as evidence-only artifacts. Their existing
single-run/literature comparisons do not meet the rewrite's M3 acceptance
contract. Scaffold output is not a measurement, and total elapsed time copied
into percentile fields is not a latency distribution. These rows are partial
apparatus to replace, not performance claims to inherit.

O6 and O10 are externally owned dashboard and alert-rule implementations in
Command Center. D7 is its chart installation boundary. D8 is an implemented
exit-64 compatibility tombstone, not a functioning deployment wrapper.
Sto3 remains unimplemented for positive URL signing even though unavailable
responses are now safe. The retained Auth3/GraphQL development receipt proves
only its stated mTLS, tenant-isolation, identity-rejection, and revocation scope.

Unreviewed entries must be investigated individually. Do not force the
historical audit's numeric categories, convert referenced scripts into
receipts, or treat this initial import as completion of B3 or P7.

## Validation

Run the Rust validator from this repository:

```sh
cargo run --locked -p ai_blaise_feature_register -- check
cargo run --locked -p ai_blaise_feature_register -- summary
cargo run --locked -p ai_blaise_feature_register -- check-legacy-coverage
cargo run --locked -p ai_blaise_feature_register -- check-source-coverage
cargo run --locked -p ai_blaise_feature_register -- release-gaps
```

All commands validate before reporting. Counts are derived from rows, not
hard-coded acceptance targets. During the transition, CI must also preserve
exact ID coverage between this inventory and the legacy headings. Once all
legacy parsers have been replaced, historical identity coverage must remain
enforced without keeping the old prose authoritative.

`release-gaps` is an interim rejection/reporting path, not a completed release
verifier. It emits every row's identity, ownership, maturity, scope, and
blockers, and exits 1 because the trusted current-source release evidence
verifier is unimplemented. Invalid inputs exit 2. Historical claims,
implemented maturity, and local evidence references cannot make this command
authorize a release. The production-readiness wrapper uses this path. Its
audit mode now checks machine identities and documentation boundaries without
parsing legacy statuses.

The V2 inventory workflow, release monitor, production-readiness audit,
production-gap source audit, documentation-boundary checker, and feature-doc
change checker no longer derive identity or readiness from the prose catalog.
The feature-doc path requires `docs/features.tsv` updates
with feature-bearing source changes, including local untracked files; a prose
update alone cannot satisfy it. Git paths use NUL delimiters so unusual file
names cannot evade the change check. CI checks explicit commit ranges; local
runs also include uncommitted and nonignored new files. Source marker checks
cover identity only, allow libraries, and do not infer maturity.

The production-gap audit preserves direct source, SQL, smoke, and workflow
assertions. Its success output explicitly describes checked source contracts,
not executed live tests. The old catalog is optional and is examined only for
overclaims, like other documentation. Bundle1 uses its canonical image and
extension documentation, placement-generation uses the production audit's
bounded description, and restore-depth points to the PITR runbook's explicit
model-versus-live distinction. Their source checks remain in place.

D10 also owns the repository's locked Cargo metadata dependency-policy scan.
[`ci/ai-blaise/license-check.sh`](../ci/ai-blaise/license-check.sh) requires
one nonempty, structurally complete `cargo metadata --locked` document before
filtering licenses. Once that scan starts, Cargo, empty-output, malformed-data,
and jq failures are fatal in every mode. The focused regression is
[`ci/ai-blaise/license-check_test.py`](../ci/ai-blaise/license-check_test.py),
and [the license workflow](../.github/workflows/ci-license.yml) runs both the
failure-boundary suite and the actual locked workspace scan. This enforces the
existing repository dependency policy; it is not a legal compatibility
determination, an executed release receipt, or production qualification.

## Remaining legacy retirement

The release rejection path is repaired, but the following work remains before
deleting `NEW_FEATURES.md`:

| Consumer | Required change | Behavior to retain |
| --- | --- | --- |
| `check-legacy-coverage` in the feature-register tool and its CI/Make callers | Replace the transitional heading adapter with a committed historical-identity retention contract once this first machine register is reviewed. | Every historical ID, including aliases, components, and tombstones; do not substitute a fixed count. |
| Documentation and agent guidance still linking the legacy catalog | Retarget useful behavior descriptions and references; remove status-as-promotion language. | User-visible requirements, compatibility obligations, and accurate source/image/evidence boundaries. The separate patch-manifest audit is not a feature-status parser. |

The production-gap regressions now cover absent or invented legacy prose,
malformed machine metadata through the real Rust validator, source-removal,
and release-path substitution. The Rust validator also tests enum rejection
and that claimed status, implementation maturity, and arbitrary evidence paths
cannot qualify a release.

The [script wiring audit](ai-blaise/SCRIPT_WIRING_AUDIT.md) separately records
current execution-unwired programs and their required adoption or retirement
decisions. Source text reads, existence checks, documentation links, and TSV
references are not execution edges. Neither the historical feature-category
counts nor the historical dead-script count is a target to force into the new
inventory.
