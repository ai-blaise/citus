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

Captured from GitHub on `2026-05-24T03:05:29Z`:

- Open series: PR #70 through PR #109 against `bootstrap-v2`.
- Draft blockers: PR #87, PR #109.
- Non-draft merge-order blockers: PR #70, PR #71, PR #97.
- PR #103, PR #104, PR #105, PR #106, PR #107, and PR #108 are no longer
  draft blockers in this plan. PR #109 is tracked as the canonical integration
  reconciliation branch for overlapping release-hardening lanes.
- CI state is volatile and is not used as a merge-order drift signal. The plan
  requires title, head branch, base branch, and draft state to match live
  GitHub metadata.

## Merge Order

| Order | PR | Status | Batch | Dependency hints |
| --- | --- | --- | --- | --- |
| 1 | #107 | ready | merge-plan-governance | none |
| 2 | #108 | ready | source-patch-foundation | none |
| 3 | #78 | ready | source-patch-foundation | none |
| 4 | #77 | ready | source-patch-foundation | #78 |
| 5 | #70 | blocked | ready-runtime-foundation | #108 |
| 6 | #71 | blocked | ready-runtime-foundation | #108 |
| 7 | #72 | ready | ready-runtime-foundation | none |
| 8 | #73 | ready | ready-runtime-foundation | none |
| 9 | #74 | ready | ready-runtime-foundation | #73 |
| 10 | #75 | ready | ready-runtime-foundation | none |
| 11 | #76 | ready | ready-runtime-foundation | none |
| 12 | #79 | ready | ready-operator-k8s | none |
| 13 | #80 | ready | ready-operator-k8s | #79 |
| 14 | #81 | ready | source-patch-foundation | #78, #77 |
| 15 | #82 | ready | ready-runtime-foundation | none |
| 16 | #83 | ready | ready-operator-k8s | #79, #80 |
| 17 | #84 | ready | ready-runtime-foundation | none |
| 18 | #85 | ready | ready-runtime-foundation | none |
| 19 | #86 | ready | ready-runtime-foundation | #82 |
| 20 | #89 | ready | ready-operator-k8s | #79, #80, #83 |
| 21 | #90 | ready | ready-operator-k8s | #84, #89 |
| 22 | #93 | ready | ready-operator-k8s | #89 |
| 23 | #91 | ready | ready-operator-k8s | #89, #90, #93 |
| 24 | #92 | ready | ready-operator-k8s | #91 |
| 25 | #88 | ready | ready-release-evidence | #70, #71, #72, #73, #74, #75, #76, #79, #80, #81, #82, #83, #84, #85, #86, #89, #90, #91, #92, #93 |
| 26 | #94 | ready | ready-release-evidence | #70, #71, #72, #73, #74, #75, #76, #82, #84, #86, #89, #90, #91, #92, #93 |
| 27 | #95 | ready | ready-release-evidence | #70 |
| 28 | #87 | draft-blocked | draft-runtime-durability-upgrade-pool | #77, #81 |
| 29 | #104 | ready | draft-runtime-durability-upgrade-pool | #74 |
| 30 | #101 | ready | draft-runtime-durability-upgrade-pool | #76, #83, #89 |
| 31 | #96 | ready | draft-runtime-durability-upgrade-pool | #76, #81, #101 |
| 32 | #98 | ready | draft-runtime-durability-upgrade-pool | #72 |
| 33 | #99 | ready | draft-release-and-patch-audit | #78, #77 |
| 34 | #100 | ready | draft-release-and-patch-audit | #88 |
| 35 | #102 | ready | draft-release-and-patch-audit | #88, #99, #100 |
| 36 | #105 | ready | draft-release-and-patch-audit | #95, #96, #100 |
| 37 | #103 | ready | draft-release-and-patch-audit | #88, #102, #104 |
| 38 | #106 | ready | draft-release-and-patch-audit | none |
| 39 | #109 | draft-blocked | draft-release-and-patch-audit | #99, #100, #102, #103, #104, #105, #106 |
| 40 | #97 | blocked | draft-release-and-patch-audit | #81, #88, #91, #96, #100, #102, #103, #105, #106, #109 |

Use the JSON as canonical if this table drifts.

## Known Blockers

- PR #70 (blocked): Hold behind PR #108 until the PG18 CDC catchup timeout is resolved or conclusively isolated from the branch.
- PR #71 (blocked): Hold behind PR #108 until the PG18 CDC schema-change/move catchup behavior is proved on PG18.
- PR #87 (draft-blocked): Draft as of 2026-05-24T02:53:14Z. PR body states Timescale 2.28 evidence depends on the published timescale/timescaledb:2.28.0-pg17 image tag; keep the skip-with-note truthful until upstream publication is verified. Full Citus matrix jobs were still pending when metadata was refreshed.
- PR #109 (draft-blocked): Draft as of 2026-05-24T03:05:29Z. Canonical integration PR overlaps the production gap audit, performance evidence, sidecar runtime smoke, runbook command, patch audit, and release preflight lanes; review as a reconciliation branch, not a small standalone slice. Run the focused gates it wires together first, then use the draft-release-and-patch-audit matrix boundary if the integration remains the chosen landing path.
- PR #97 (blocked): Ready for review as of 2026-05-24T02:42:19Z, but held behind final release-hardening PRs in this merge plan. Release publishability should land last after environment, performance, gap-audit, runbook, agentmemory recovery, upgrade, image, deployment, and release-monitor gates are integrated.

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
