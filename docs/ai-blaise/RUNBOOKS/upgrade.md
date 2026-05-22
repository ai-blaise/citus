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

## Canary Flow

1. Fetch upstream Citus and re-run the patch series gate; integrated TS6
   patches should either apply cleanly to the upstream-like tree or reverse
   cleanly when already present.
2. If Bundle1 is promoted, run the promoted operand-image build/initdb smoke
   for bundled, optional, and hard-blocked extension validation. While Bundle1
   remains alpha, run the static image contract and SQL runtime smokes, but do
   not treat them as production evidence for the full operand image.
3. Render Helm with production values and apply it to the canary namespace.
4. Run real Citus+TimescaleDB cohabitation, plan-cache, latency, branch,
   vectorizer, search, HTAP, multi-region, chaos, slop, feature-doc, license,
   and image gates.
5. Mirror read traffic through the pool with a capped sample.
6. Promote canary writes only after plan-freeze and regression checks pass.
7. Publish release notes with `NEW_FEATURES.md` deltas and image digests.

## Rollback

1. Disable mirrored writes.
2. Restore pool routing to the previous release.
3. Reconcile branch and tenant status.
4. Keep the failed canary namespace for forensic inspection until logs,
   metrics, and WAL replay notes are captured.
