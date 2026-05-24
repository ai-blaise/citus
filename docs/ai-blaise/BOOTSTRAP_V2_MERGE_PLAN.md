# Bootstrap V2 Merge Plan

`docs/ai-blaise/bootstrap-v2-merge-plan.json` is the machine-readable merge
order for the open `bootstrap-v2` PR train starting at PR #70. It records the
current PR metadata snapshot, merge order, dependency hints, draft blockers,
shared conflict paths, and rules for when the expensive Citus matrix is worth
running.

The plan intentionally treats `origin/bootstrap-v2-intermediate` as reference
only. Merge-order work must target `bootstrap-v2`.

## Validation

Run the metadata-only validator from the repository root:

```bash
python3 ci/ai-blaise/bootstrap-v2-merge-plan-check.py --offline
python3 ci/ai-blaise/bootstrap-v2-merge-plan-check.py --live
```

`--offline` validates the JSON schema, order, dependency direction, batch
coverage, and matrix-policy references. `--live` also queries GitHub with
`gh pr list`; planned PRs that close, retarget, rename, rehead, or change draft
state fail the check. Newly opened PRs outside the snapshot are warnings by
default and become failures with `--strict-open-set`.

Before final release promotion, run:

```bash
python3 ci/ai-blaise/bootstrap-v2-merge-plan-check.py --live --strict-open-set --require-no-drafts
```

That command is expected to fail while any draft blocker remains or while any
open PR #70+ is missing from the manifest.

## Current Snapshot

Captured from GitHub on `2026-05-24T05:58:00Z` for the post-runtime-durability manifest:

- Open manifest: 7 PRs against `bootstrap-v2`; PRs #70, #71, #72, #73, #74, #75, #76, #77, #78, #79, #80, #81, #82, #83, #84, #85, #86, #87, #88, #89, #90, #91, #92, #93, #94, #95, #96, #98, #101, #104, #106, #107, and #108 are recorded as landed by direct `bootstrap-v2` merge batches and are no longer open-plan work.
- Draft blockers: PR #109.
- Non-draft merge-order blockers: PR #97.
- PRs #87, #96, #98, #101, and #104 were integrated by the direct runtime durability/upgrade/pool batch after focused VM verification and the Timescale 2.27 Docker cohabitation smoke passed; Timescale 2.28 remains a truthful pass-with-note until `timescale/timescaledb:2.28.0-pg17` exists.
- CI state is volatile and is not used as a merge-order drift signal. The plan requires title, head branch, base branch, and draft state to match live GitHub metadata.

## Merge Order

| Order | PR | Status | Batch | Dependency hints |
| --- | --- | --- | --- | --- |
| 1 | #99 | ready | draft-release-and-patch-audit | none |
| 2 | #100 | ready | draft-release-and-patch-audit | none |
| 3 | #102 | ready | draft-release-and-patch-audit | #99, #100 |
| 4 | #105 | ready | draft-release-and-patch-audit | #100 |
| 5 | #103 | ready | draft-release-and-patch-audit | #102 |
| 6 | #109 | draft-blocked | draft-release-and-patch-audit | #99, #100, #102, #103, #105 |
| 7 | #97 | blocked | draft-release-and-patch-audit | #100, #102, #103, #105, #109 |

Use the JSON as canonical if this table drifts.

## Known Blockers

- PR #109 (draft-blocked): Draft as of the post-runtime-durability snapshot. Canonical integration PR overlaps the production gap audit, performance evidence, sidecar runtime smoke, runbook command, patch audit, and release preflight lanes; review as a reconciliation branch, not a small standalone slice. Run the focused gates it wires together first, then use the draft-release-and-patch-audit matrix boundary if the integration remains the chosen landing path.
- PR #97 (blocked): Ready for review as of 2026-05-24T02:42:19Z, but held behind final release-hardening PRs in this merge plan. Release publishability should land last after environment, performance, gap-audit, runbook, upgrade, image, deployment, release-monitor, and integration-reconciliation gates are integrated.

## Expensive Matrix Rules

Batch meaningful work before a full Citus matrix run. Run focused PR smokes
first, then run the expensive matrix at the batch boundaries marked
`run_expensive_citus_matrix_after=true` in the JSON.

Keep the matrix monitor in a parallel worker while non-overlapping implementation continues on the next PR or batch.

Run the full matrix immediately if a merge or conflict resolution edits:

- `src/backend/distributed/**`, `src/include/distributed/**`,
  `src/test/regress/**`, `src/test/cdc/**`, or `patches/series`
- `images/citus-pg-overlay/extensions/**`, SQL upgrade manifests, or
  installable extension SQL
- `.github/workflows/build_and_test.yml`, `.github/workflows/run_tests.yml`,
  `packaging-test-pipelines.yml`, or Citus matrix fanout logic

Do not run the full matrix for merge-plan-only changes, metadata-only drift, or
a no-semantic-diff rebase when focused checks have already passed on identical
patch content.

Before promotion, the final integrated `bootstrap-v2` tip must pass the full
Citus matrix plus the ai-blaise release gates listed in `docs/ai-blaise/RELEASING.md`.
