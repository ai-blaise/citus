# Upgrade Runbook

`FEATURE: D9`

Use this runbook for canary upgrades of the ai-blaise/citus overlay and operand
image.

## Inputs

- Target upstream Citus commit and ai-blaise release branch.
- Current TS6 source/patch-series compatibility output.
- Operand image digest with extension manifest and SBOM after `FEATURE:
  Bundle1` is promoted. While Bundle1 remains alpha, the operand-image contract
  must not be used as production release evidence without a real operand image
  build/initdb smoke.
- Helm values for the canary namespace.
- Rollback branch, backup checkpoint, and PITR timestamp.
- `images/citus-pg-overlay/extensions/ai_blaise_citus-upgrade-manifest.tsv`
  for every local companion SQL install, upgrade, and downgrade transition.
- Current and target `pg_extension.extversion` rows for `citus` and
  `ai_blaise_citus` from the canary database.

## Bounded Compatibility Gate

Run the static gate before any image promotion or canary apply:

```bash
make -f Makefile.ai-blaise upgrade-rollback-guardrails
REQUIRE_DOCKER=1 bash ci/ai-blaise/canary-upgrade-rollback-smoke.sh
```

The gate executes `ci/ai-blaise/upgrade-rollback-guardrails.sh`. It fails
closed when:

- the companion SQL control files disagree on `default_version`;
- an `ai_blaise_citus--*.sql` install or transition file exists without a
  manifest row;
- a transition row lacks a reverse SQL contract, rollback statement, or
  version-skew statement;
- the bounded upstream Citus edge from `14.0-1` to the current
  `src/backend/distributed/citus.control` default lacks both upgrade and
  downgrade SQL; or
- this runbook, release docs, image docs, Make target, or Dockerfile wiring stop
  referencing the gate.

The static guardrail is not production evidence for the full upstream Citus
upgrade matrix. It keeps local overlay transition contracts explicit. The
Docker-backed canary smoke above is the bounded D9 live evidence for the local
companion SQL extension upgrade and rollback path; the broader upstream Citus
upgrade matrix remains a separate release gate.

## Live Companion SQL Canary Drill

`ci/ai-blaise/canary-upgrade-rollback-smoke.sh` is the executable canary drill
for the local companion SQL extension. It mounts the shipped
`ai_blaise_citus.control`, `ai_blaise_citus--0.1.0.sql`,
`ai_blaise_citus--0.1.0--0.1.1.sql`, and
`ai_blaise_citus--0.1.1--0.1.0.sql` files into a real PostgreSQL extension
directory, creates the extension at `0.1.0`, upgrades to `0.1.1`, records a
canary event, downgrades to `0.1.0`, and asserts that the 0.1.1 event surface is
removed. Release evidence must retain the smoke output row and the
`pg_extension.extversion` observations before and after rollback.

This smoke is intentionally local to the ai-blaise companion SQL surface. It
does not replace upstream Citus `check-citus-upgrade`, mixed-version Citus
upgrade tests, or production canary traffic observation for a release
candidate.

## SQL Preflight

Capture the exact extension versions in the canary before changing manifests or
images:

```sql
SELECT extname, extversion
FROM pg_extension
WHERE extname IN ('citus', 'ai_blaise_citus')
ORDER BY extname;
```

If the target image contains a newer local companion SQL transition, run it in
the canary before deploying binaries that require new catalog objects:

```sql
ALTER EXTENSION ai_blaise_citus UPDATE;
SELECT extname, extversion
FROM pg_extension
WHERE extname = 'ai_blaise_citus';
```

For upstream Citus major/minor movement, verify the target edge is one of the
manifested bounded edges and then run the upstream extension transition per the
Citus release note for that edge. The current fast gate checks only
`14.0-1 -> 15.0-1` and the matching downgrade file.

## Version-Skew Contract

- Do not run mixed `ai_blaise_citus` SQL catalog versions inside one Citus
  cluster. Upgrade companion SQL in the canary first, then roll sidecar and
  operator binaries that depend on the new objects.
- Do not promote writes while coordinator and workers report different Citus
  extension major versions. Read-only canary traffic may be mirrored during the
  bounded observation window only when the pool routes writes to the old
  release.
- If a sidecar or operator binary requires a new SQL object, its deployment must
  be blocked until `pg_extension.extversion` and the manifest row both show the
  target version.

## Dry-run Command Checks

Before rendering or applying the canary namespace, run the static and modeled
gates that catch stale commands, patch drift, image contract drift, and release
model overclaims:

```bash
bash ci/ai-blaise/runbook-command-check.sh
bash ci/ai-blaise/upstream-merge-dry.sh
bash ci/ai-blaise/image-check.sh
bash ci/ai-blaise/v2-acceptance-check.sh
```

These dry-runs are prerequisites for the canary. They are not production
evidence for the live upgrade until the canary namespace is rendered, applied,
smoked, and recorded for the exact release candidate.

## Canary Flow

1. Fetch upstream Citus and re-run the patch series gate; integrated TS6
   patches should either apply cleanly to the upstream-like tree or reverse
   cleanly when already present.
2. Run `make -f Makefile.ai-blaise upgrade-rollback-guardrails` and
   `REQUIRE_DOCKER=1 bash ci/ai-blaise/canary-upgrade-rollback-smoke.sh`;
   treat any failure as a release blocker, not a warning.
3. If Bundle1 is promoted, run the promoted operand-image build/initdb smoke
   for bundled, optional, and hard-blocked extension validation. While Bundle1
   remains alpha, run the static image contract and SQL runtime smokes, but do
   not treat them as production evidence for the full operand image.
4. Record current `pg_extension` versions, run any companion SQL transition in
   the canary, and record target versions after `ALTER EXTENSION
   ai_blaise_citus UPDATE`.
5. Render Helm with production values and apply it to the canary namespace.
6. Run real Citus+TimescaleDB cohabitation, plan-cache, latency, branch,
   vectorizer, search, HTAP, multi-region, chaos, slop, feature-doc, license,
   and image gates.
7. Mirror read traffic through the pool with a capped sample.
8. Promote canary writes only after plan-freeze and regression checks pass and
   the version-skew window is closed.
9. Publish release notes with `NEW_FEATURES.md` deltas, image digests, extension
   versions, and the guardrail output row.

## Rollback

1. Disable mirrored writes and restore pool routing to the previous release.
2. If the failure happened before write promotion and the manifest includes a
   reverse local companion SQL transition, run the reverse transition in the
   canary and verify `pg_extension.extversion` returns to the previous version.
3. If no reverse transition exists, or if writes were promoted, use the recorded
   PITR timestamp and backup checkpoint. Do not claim in-place downgrade.
4. Reconcile branch, tenant, Migration CR, and schema-job status. Migration
   phases before `PUBLIC` may use the F1 rollback planner; after `PUBLIC`, the
   forward fix is a new Migration CR.
5. Keep the failed canary namespace for forensic inspection until logs,
   metrics, WAL replay notes, and extension-version observations are captured.

## Downgrade Safety

Every future `ai_blaise_citus--FROM--TO.sql` transition must be paired with a
manifested reverse `ai_blaise_citus--TO--FROM.sql` before it can enter the
release gate. If the safe rollback path is PITR-only, the transition must stay
out of the release gate until this runbook, manifest, and operator rollout plan
are updated to make that boundary explicit.
