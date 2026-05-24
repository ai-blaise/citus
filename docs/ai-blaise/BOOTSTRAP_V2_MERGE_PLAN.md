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

Captured from GitHub on `2026-05-24T03:02:39Z`:

- Open series: PR #70 through PR #108 against `bootstrap-v2`.
- Draft blockers: PR #108, PR #87, PR #104, PR #105, PR #103.
- Non-draft merge-order blockers: PR #70, PR #71, PR #97.
- PR #106 and PR #107 were validated and are no longer draft blockers in this
  plan; PR #108 was added after it opened as the PG18 CDC catchup-diagnostics
  follow-up.
- CI state is volatile and is not used as a merge-order drift signal. The plan
  requires title, head branch, base branch, and draft state to match live
  GitHub metadata.

## Merge Order

| Order | PR | Status | Batch | Dependency hints |
| --- | --- | --- | --- | --- |
| 1 | #107 | ready | merge-plan-governance | none |
| 2 | #108 | draft-blocked | source-patch-foundation | none |
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
| 29 | #104 | draft-blocked | draft-runtime-durability-upgrade-pool | #74 |
| 30 | #101 | ready | draft-runtime-durability-upgrade-pool | #76, #83, #89 |
| 31 | #96 | ready | draft-runtime-durability-upgrade-pool | #76, #81, #101 |
| 32 | #98 | ready | draft-runtime-durability-upgrade-pool | #72 |
| 33 | #99 | ready | draft-release-and-patch-audit | #78, #77 |
| 34 | #100 | ready | draft-release-and-patch-audit | #88 |
| 35 | #102 | ready | draft-release-and-patch-audit | #88, #99, #100 |
| 36 | #105 | draft-blocked | draft-release-and-patch-audit | #95, #96, #100 |
| 37 | #103 | draft-blocked | draft-release-and-patch-audit | #88, #102, #104 |
| 38 | #106 | ready | draft-release-and-patch-audit | none |
| 39 | #97 | blocked | draft-release-and-patch-audit | #81, #88, #91, #96, #100, #102, #103, #105, #106 |

Use the JSON as canonical if this table drifts.

## Known Blockers

- PR #108 (draft-blocked): Draft as of 2026-05-24T03:02:39Z. Review and prove PG18 CDC schema-change/move catchup behavior before relying on PR #70/#71 CDC matrix results. Touches src/test/cdc/** and shared Actions log capture; run focused PG18 CDC installcheck evidence plus the source-patch foundation matrix boundary.
- PR #70 (blocked): Hold behind PR #108 until the PG18 CDC catchup timeout is resolved or conclusively isolated from the branch.
- PR #71 (blocked): Hold behind PR #108 until the PG18 CDC schema-change/move catchup behavior is proved on PG18.
- PR #87 (draft-blocked): Draft as of 2026-05-24T02:53:14Z. PR body states Timescale 2.28 evidence depends on the published timescale/timescaledb:2.28.0-pg17 image tag; keep the skip-with-note truthful until upstream publication is verified. Full Citus matrix jobs were still pending when metadata was refreshed.
- PR #104 (draft-blocked): Draft as of 2026-05-24T02:53:14Z. Sidecar API runtime smoke should land after PR #74 so PostgREST, GraphQL, and edge-function runtime checks build on the API trio surface. Touches production-gap-audit and shared runtime code; run focused sidecar API smoke before the batch matrix.
- PR #105 (draft-blocked): Draft as of 2026-05-24T02:53:14Z. Runbook command validation should land after DR restore, upgrade rollback, and release environment preflight checks so documented operational commands match the integrated gates. Touches production-readiness workflow and operational runbooks; keep in the final release-hardening tranche.
- PR #103 (draft-blocked): Draft as of 2026-05-24T02:53:14Z. Machine-derived production gap inventory should land after sidecar API runtime and performance evidence changes so the final audit vocabulary reflects all release evidence gates. Touches production-gap-audit and production readiness prose; keep in the final release-hardening tranche to avoid stale count churn.
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
