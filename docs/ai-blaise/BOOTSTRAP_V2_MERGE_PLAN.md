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

Captured from GitHub on `2026-05-24T02:53:14Z`:

- Open series: PR #70 through PR #107 against `bootstrap-v2`.
- Draft blockers: PR #87, PR #103, PR #104, PR #105, PR #106, PR #107.
- PR #94 through PR #102 were promoted out of draft before this refresh. PR #97
  still has merge-order blockers in this plan.
- Every listed PR had `mergeStateStatus=UNSTABLE` at capture time, mostly due
  to pending or failing CI. The plan does not treat `UNSTABLE` as a merge-order
  failure because CI state is volatile; it does require draft state to match.

## Merge Order

| Order | PR | Status | Batch | Dependency hints |
| --- | --- | --- | --- | --- |
| 1 | #107 | draft-blocked | merge-plan-governance | none |
| 2 | #78 | ready | source-patch-foundation | none |
| 3 | #77 | ready | source-patch-foundation | #78 |
| 4 | #70 | ready | ready-runtime-foundation | none |
| 5 | #71 | ready | ready-runtime-foundation | none |
| 6 | #72 | ready | ready-runtime-foundation | none |
| 7 | #73 | ready | ready-runtime-foundation | none |
| 8 | #74 | ready | ready-runtime-foundation | #73 |
| 9 | #75 | ready | ready-runtime-foundation | none |
| 10 | #76 | ready | ready-runtime-foundation | none |
| 11 | #79 | ready | ready-operator-k8s | none |
| 12 | #80 | ready | ready-operator-k8s | #79 |
| 13 | #81 | ready | source-patch-foundation | #78, #77 |
| 14 | #82 | ready | ready-runtime-foundation | none |
| 15 | #83 | ready | ready-operator-k8s | #79, #80 |
| 16 | #84 | ready | ready-runtime-foundation | none |
| 17 | #85 | ready | ready-runtime-foundation | none |
| 18 | #86 | ready | ready-runtime-foundation | #82 |
| 19 | #89 | ready | ready-operator-k8s | #79, #80, #83 |
| 20 | #90 | ready | ready-operator-k8s | #84, #89 |
| 21 | #93 | ready | ready-operator-k8s | #89 |
| 22 | #91 | ready | ready-operator-k8s | #89, #90, #93 |
| 23 | #92 | ready | ready-operator-k8s | #91 |
| 24 | #88 | ready | ready-release-evidence | #70, #71, #72, #73, #74, #75, #76, #79, #80, #81, #82, #83, #84, #85, #86, #89, #90, #91, #92, #93 |
| 25 | #94 | ready | ready-release-evidence | #70, #71, #72, #73, #74, #75, #76, #82, #84, #86, #89, #90, #91, #92, #93 |
| 26 | #95 | ready | ready-release-evidence | #70 |
| 27 | #87 | draft-blocked | draft-runtime-durability-upgrade-pool | #77, #81 |
| 28 | #104 | draft-blocked | draft-runtime-durability-upgrade-pool | #74 |
| 29 | #101 | ready | draft-runtime-durability-upgrade-pool | #76, #83, #89 |
| 30 | #96 | ready | draft-runtime-durability-upgrade-pool | #76, #81, #101 |
| 31 | #98 | ready | draft-runtime-durability-upgrade-pool | #72 |
| 32 | #99 | ready | draft-release-and-patch-audit | #78, #77 |
| 33 | #100 | ready | draft-release-and-patch-audit | #88 |
| 34 | #102 | ready | draft-release-and-patch-audit | #88, #99, #100 |
| 35 | #105 | draft-blocked | draft-release-and-patch-audit | #95, #96, #100 |
| 36 | #103 | draft-blocked | draft-release-and-patch-audit | #88, #102, #104 |
| 37 | #106 | draft-blocked | draft-release-and-patch-audit | #103 |
| 38 | #97 | blocked | draft-release-and-patch-audit | #81, #88, #91, #96, #100, #102, #103, #105, #106 |

Use the JSON as canonical if this table drifts.

## Known Blockers

- PR #107 (draft-blocked): This metadata-only plan PR should be reviewed and merged first so subsequent PRs can validate against the canonical order.
- PR #87 (draft-blocked): PR body states Timescale 2.28 evidence depends on the published timescale/timescaledb:2.28.0-pg17 image tag; keep the skip-with-note truthful until upstream publication is verified.
- PR #104 (draft-blocked): Sidecar API runtime smoke should land after PR #74 so PostgREST, GraphQL, and edge-function runtime checks build on the API trio surface.
- PR #105 (draft-blocked): Runbook command validation should land after DR restore, upgrade rollback, and release environment preflight checks so documented operational commands match the integrated gates.
- PR #103 (draft-blocked): Machine-derived production gap inventory should land after sidecar API runtime and performance evidence changes so the final audit vocabulary reflects all release evidence gates.
- PR #106 (draft-blocked): Agentmemory recovery docs reference the production inventory audit gap covered by PR #103, so keep them behind the machine-derived audit update.
- PR #97 (blocked): Release publishability should land last after environment, performance, gap-audit, runbook, agentmemory recovery, upgrade, image, deployment, and release-monitor gates are integrated.

## Expensive Matrix Rules

Batch meaningful work before a full Citus matrix run. Run focused PR smokes
first, then run the expensive matrix at the batch boundaries marked
`run_expensive_citus_matrix_after=true` in the JSON.

Run the full matrix immediately if a merge or conflict resolution edits:

- `src/backend/distributed/**`, `src/include/distributed/**`,
  `src/test/regress/**`, or `patches/series`
- `images/citus-pg-overlay/extensions/**`, SQL upgrade manifests, or
  installable extension SQL
- `.github/workflows/build_and_test.yml`, `.github/workflows/run_tests.yml`,
  `packaging-test-pipelines.yml`, or Citus matrix fanout logic

Do not run the full matrix for merge-plan-only changes, metadata-only drift, or
a no-semantic-diff rebase when focused checks have already passed on identical
patch content.

Before promotion, the final integrated `bootstrap-v2` tip must pass the full
Citus matrix plus the ai-blaise release gates listed in `docs/ai-blaise/RELEASING.md`.
