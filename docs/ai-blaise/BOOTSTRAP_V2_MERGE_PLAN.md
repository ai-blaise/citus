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

Captured from GitHub on `2026-05-24T03:26:26Z` for the post-landing manifest:

- Open manifest: 37 PRs against `bootstrap-v2`; PR #106, #107, #108 are recorded as landed by this direct `bootstrap-v2` merge batch and are no longer open-plan work.
- Draft blockers: PR #109.
- Non-draft merge-order blockers: PR #87, PR #97.
- PR #87 is no longer draft, but remains blocked until the Timescale 2.28 image/tag evidence is truthful and the branch is refreshed against the current `bootstrap-v2` tip.
- CI state is volatile and is not used as a merge-order drift signal. The plan requires title, head branch, base branch, and draft state to match live GitHub metadata.

## Merge Order

| Order | PR | Status | Batch | Dependency hints |
| --- | --- | --- | --- | --- |
| 1 | #78 | ready | source-patch-foundation | none |
| 2 | #77 | ready | source-patch-foundation | #78 |
| 3 | #70 | ready | ready-runtime-foundation | none |
| 4 | #71 | ready | ready-runtime-foundation | none |
| 5 | #72 | ready | ready-runtime-foundation | none |
| 6 | #73 | ready | ready-runtime-foundation | none |
| 7 | #74 | ready | ready-runtime-foundation | #73 |
| 8 | #75 | ready | ready-runtime-foundation | none |
| 9 | #76 | ready | ready-runtime-foundation | none |
| 10 | #79 | ready | ready-operator-k8s | none |
| 11 | #80 | ready | ready-operator-k8s | #79 |
| 12 | #81 | ready | source-patch-foundation | #78, #77 |
| 13 | #82 | ready | ready-runtime-foundation | none |
| 14 | #83 | ready | ready-operator-k8s | #79, #80 |
| 15 | #84 | ready | ready-runtime-foundation | none |
| 16 | #85 | ready | ready-runtime-foundation | none |
| 17 | #86 | ready | ready-runtime-foundation | #82 |
| 18 | #89 | ready | ready-operator-k8s | #79, #80, #83 |
| 19 | #90 | ready | ready-operator-k8s | #84, #89 |
| 20 | #93 | ready | ready-operator-k8s | #89 |
| 21 | #91 | ready | ready-operator-k8s | #89, #90, #93 |
| 22 | #92 | ready | ready-operator-k8s | #91 |
| 23 | #88 | ready | ready-release-evidence | #70, #71, #72, #73, #74, #75, #76, #79, #80, #81, #82, #83, #84, #85, #86, #89, #90, #91, #92, #93 |
| 24 | #94 | ready | ready-release-evidence | #70, #71, #72, #73, #74, #75, #76, #82, #84, #86, #89, #90, #91, #92, #93 |
| 25 | #95 | ready | ready-release-evidence | #70 |
| 26 | #87 | blocked | draft-runtime-durability-upgrade-pool | #77, #81 |
| 27 | #104 | ready | draft-runtime-durability-upgrade-pool | #74 |
| 28 | #101 | ready | draft-runtime-durability-upgrade-pool | #76, #83, #89 |
| 29 | #96 | ready | draft-runtime-durability-upgrade-pool | #76, #81, #101 |
| 30 | #98 | ready | draft-runtime-durability-upgrade-pool | #72 |
| 31 | #99 | ready | draft-release-and-patch-audit | #78, #77 |
| 32 | #100 | ready | draft-release-and-patch-audit | #88 |
| 33 | #102 | ready | draft-release-and-patch-audit | #88, #99, #100 |
| 34 | #105 | ready | draft-release-and-patch-audit | #95, #96, #100 |
| 35 | #103 | ready | draft-release-and-patch-audit | #88, #102, #104 |
| 36 | #109 | draft-blocked | draft-release-and-patch-audit | #99, #100, #102, #103, #104, #105 |
| 37 | #97 | blocked | draft-release-and-patch-audit | #81, #88, #91, #96, #100, #102, #103, #105, #109 |

Use the JSON as canonical if this table drifts.

## Known Blockers

- PR #87 (blocked): PR #87 is no longer draft, but remains blocked until the Timescale 2.28 image/tag evidence is truthful and the branch is refreshed against the current bootstrap-v2 tip. PR body states Timescale 2.28 evidence depends on the published timescale/timescaledb:2.28.0-pg17 image tag; keep the skip-with-note truthful until upstream publication is verified. Full Citus matrix jobs were still pending when metadata was refreshed, and live mergeStateStatus was DIRTY at the post-landing snapshot.
- PR #109 (draft-blocked): Draft as of the post-landing snapshot. Canonical integration PR overlaps the production gap audit, performance evidence, sidecar runtime smoke, runbook command, patch audit, and release preflight lanes; review as a reconciliation branch, not a small standalone slice. Run the focused gates it wires together first, then use the draft-release-and-patch-audit matrix boundary if the integration remains the chosen landing path.
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
