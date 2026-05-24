# Production Gap Audit Integration Plan

This branch is a disposable integration simulation for making PR #103's
machine-derived production-gap audit canonical while preserving the
feature-specific gates from PRs #99, #102, #104, and #105. It does not edit the
active PR branches.

## Merge Order Tested

1. #103 `codex/production-gap-inventory-audit`
2. #99 `codex/patch-production-integration-audit-20260524020118`
3. #102 `codex/slo-performance-evidence-hardening-20260524015908`
4. #104 `codex/sidecar-runtime-contract-smoke-20260524`
5. #105 `codex/runbook-command-checks-20260524-022745`

#103 is the correct first merge because it replaces hand-maintained production
readiness inventory prose with the canonical machine-derived contract. The
remaining branches then layer feature-specific gates around that contract.

## Conflict Resolutions Required

### #99 after #103

Conflict: `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`.

Resolution: keep #103's machine-derived inventory block and discard #99's stale
hard-coded `273/125/148` count prose. Preserve #99's Citus patch production
integration audit paragraph and the new `citus-patch-production-audit` gate.
Also keep the target in `.PHONY`; it is an integration gate, not just a helper.

### #102 after #99

Conflicts: `Makefile.ai-blaise` and
`docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`.

Resolution: keep #103's machine-derived inventory block again. In the Makefile,
keep #102's `performance-evidence-check`, `performance-evidence-release-check`,
and `performance-evidence-smoke` targets, while unioning `gate-close` so it also
keeps #99's `citus-patch-production-audit` dependency. The
`ci/ai-blaise/production-gap-audit.sh` performance threshold guards merge cleanly
and must remain.

### #104 after #102

Conflicts: `Makefile.ai-blaise`, `ci/ai-blaise/production-gap-audit.sh`, and
`docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`.

Resolution: keep #103's machine-derived inventory block in the audit doc. In the
Makefile, union `.PHONY` and `gate-close` so the merged gate keeps #99's patch
audit, #102's performance evidence targets, and #104's
`sidecar-api-runtime-smoke` target. In `production-gap-audit.sh`, keep the
machine-derived inventory and #102 performance threshold checks, then add #104's
sidecar API runtime smoke guards (`run-bun-runtime-canonical`, `/drain`, invalid
listen address, bad command rejection, accepting-new-work metric, Makefile
wiring, and `ci-sidecar` workflow wiring).

### #105 after #104

Conflict: `Makefile.ai-blaise`.

Resolution: union `.PHONY`, the target block, and `gate-close` so
`runbook-command-check` is added without dropping #99's patch audit, #102's
performance evidence targets, or #104's sidecar runtime smoke gate.

## Canonical Contract Preserved

`docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md` must not restate mutable inventory
counts. The canonical runtime fields are emitted by:

- `ci/ai-blaise/production-readiness-check.sh`
- `ci/ai-blaise/production-gap-audit.sh`

The production gap audit currently emits:

```text
production_gap_audit source_feature_ids=273 doc_feature_headings=273 feature_headings=273 production_ready=125 alpha_headings=148 inventory_contract=machine_derived source_only_alpha=0 v2_acceptance=model_only production_release_blocked=true live_sql_guards=true chart_folded_to_command_center=2026-05-22
```

Those numeric values are evidence output, not doc prose.

## Verification Run On VM

Passed on `/home/spencer/wt/integration-gap-audit-canonical-20260524T024726Z`:

- `bash -n ci/ai-blaise/production-gap-audit.sh ci/ai-blaise/performance-evidence-check.sh ci/ai-blaise/sidecar-api-runtime-smoke.sh ci/ai-blaise/runbook-command-check.sh ci/ai-blaise/citus-patch-production-audit.sh`
- `ci/ai-blaise/production-gap-audit.sh`
- `ci/ai-blaise/production-readiness-check.sh`
- `ci/ai-blaise/citus-patch-production-audit.sh`
- `bash ci/ai-blaise/runbook-command-check.sh`
- `BASE_SHA=origin/bootstrap-v2 HEAD_SHA=HEAD ci/ai-blaise/features-doc-check.sh`
- `ci/ai-blaise/slop-scan.sh`
- `git diff --check origin/bootstrap-v2...HEAD`
- `make -f Makefile.ai-blaise performance-evidence-smoke`
- `make -f Makefile.ai-blaise sidecar-api-runtime-smoke` after loading `$HOME/.cargo/env`

Fail-closed behavior verified:

- `make -f Makefile.ai-blaise performance-evidence-release-check` exits nonzero
  without release artifacts, reporting missing release evidence for core
  harnesses and the microbench aggregate.

## Integration Guidance

Use this branch as the concrete reconciliation model if these PRs are folded
before their individual branches are rebased. If rebasing individually instead,
apply the same resolutions in this order:

1. Land or rebase #103 first.
2. Rebase #99, keeping patch audit gates but dropping manual inventory counts.
3. Rebase #102, unioning Makefile gates and preserving machine-derived audit
   prose.
4. Rebase #104, adding sidecar runtime smoke guards into #103's audit script
   instead of replacing the generated inventory logic.
5. Rebase #105 last, adding `runbook-command-check` to `.PHONY`, the target
   list, CI, and `gate-close` without dropping earlier gates.
