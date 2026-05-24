# Bootstrap V2 Merge Plan

`docs/ai-blaise/bootstrap-v2-merge-plan.json` is the machine-readable merge
record for the `bootstrap-v2` PR train starting at PR #70. It now records that
the open PR train has been folded into `bootstrap-v2`; future PRs must refresh
this manifest before reusing it as an active merge-order plan.

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

After the final release/patch integration batch is pushed, the closed-loop check
is:

```bash
python3 ci/ai-blaise/bootstrap-v2-merge-plan-check.py --live --strict-open-set --require-no-drafts
```

That command should pass when GitHub has marked the batch PR heads as merged and
no new PR #70+ is open against `bootstrap-v2`.

## Current Snapshot

Captured from GitHub bookkeeping on `2026-05-24T06:14:25Z` for the final release/patch manifest:

- Open manifest: 0 PRs against `bootstrap-v2` are planned in this file.
- Draft blockers: none expected after the direct final release/patch integration batch closes draft PR #109 by ancestry.
- Non-draft merge-order blockers: none expected after the direct final release/patch integration batch closes PR #97 by ancestry.
- PRs #97, #99, #100, #102, #103, #105, and #109 are recorded as landed by the direct final release/patch integration batch after focused VM verification.
- PRs #87, #96, #98, #101, and #104 were integrated by the direct runtime durability/upgrade/pool batch after focused VM verification and the Timescale 2.27 Docker cohabitation smoke passed; Timescale 2.28 remains a truthful pass-with-note until `timescale/timescaledb-ha:pg17-ts2.28` exists.
- Earlier direct bootstrap-v2 batches recorded PRs #70-#86, #88-#95, and #106-#108 as landed.
- CI state is volatile and is not used as a merge-order drift signal. The plan requires title, head branch, base branch, and draft state to match live GitHub metadata when active PRs are present.

## Merge Order

No open PRs remain in the current manifest. Use the JSON as canonical if a new
PR train opens and this table is regenerated.

## Known Blockers

None in the current manifest.

## Expensive Matrix Rules

Batch meaningful work before a full Citus matrix run. Run focused PR smokes
first, then run the expensive matrix at meaningful integration boundaries. The
final release/patch integration batch is such a boundary; keep the matrix
monitor in a parallel worker while non-overlapping implementation continues on
the next PR or batch.

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
