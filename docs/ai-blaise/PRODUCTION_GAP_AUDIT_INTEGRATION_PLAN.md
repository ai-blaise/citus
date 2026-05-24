# Production Gap Audit Integration Record

This record captures the production-gap, release, performance, runbook, patch,
and sidecar-runtime reconciliation that has now been folded into
`bootstrap-v2`. It is not an unexecuted plan and should not be used as a reason
to keep separate PR branches alive after their heads are ancestors of the base.

## Landed PRs

The final release/patch integration batch preserves the contracts from:

- #99 `codex/patch-production-integration-audit-20260524020118`
- #100 `codex/release-env-preflight`
- #102 `codex/slo-performance-evidence-hardening-20260524015908`
- #103 `codex/production-gap-inventory-audit`
- #105 `codex/runbook-command-checks-20260524-022745`
- #109 `codex/integration-gap-audit-canonical-20260524T024726Z`
- #97 `codex/release-publishability-20260523-2357`

It also preserves the #104 sidecar API runtime smoke that had already landed in
the runtime durability batch.

## Resolved Contracts

- `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md` keeps the machine-derived
  inventory boundary and does not restate mutable source/heading/status counts
  in prose.
- `Makefile.ai-blaise` unions release preflight, release publishability, Citus
  patch production audit, runbook command validation, performance evidence,
  sidecar API runtime smoke, DR restore-depth, and release gate monitor targets
  into the release path without dropping prior gates.
- `ci/ai-blaise/production-gap-audit.sh` preserves machine-derived inventory,
  performance evidence fail-closed checks, sidecar API runtime smoke guards,
  runbook command wiring, DR restore-depth evidence, live Kubernetes harness
  boundaries, and Timescale 2.28 pass-with-note truth.
- The PostgREST, GraphQL, and edge-functions HTTP serve loops retain persistent
  runtime drain state; edge-functions also retains process-local function
  registration state across requests.
- The release gate monitor baseline is refreshed to the current 51-command V2
  domain-contract output.

## Verification On VM

The final compact replay passed on the VM worktree
`/home/spencer/wt/release-patch-final-integration-20260524T0553Z` with log
`/tmp/release-patch-final-compact-20260524T061138Z.log`:

- `ci/ai-blaise/env-preflight.sh release`
- `cargo metadata --locked --format-version=1`
- `cargo fmt --all -- --check`
- `bash ci/ai-blaise/sidecar-api-runtime-smoke.sh`
- `bash ci/ai-blaise/citus-patch-production-audit.sh`
- `bash ci/ai-blaise/runbook-command-check.sh`
- `bash ci/ai-blaise/performance-evidence-check.sh exploratory`
- `PERF_EVIDENCE_MODE=release BENCH_RESULT_TAG=release bash ci/ai-blaise/performance-evidence-check.sh release` verified fail-closed without release artifacts
- `bash ci/ai-blaise/release-publishability-check.sh`
- `REQUIRE_DOCKER=1 bash ci/ai-blaise/dr-restore-depth-check.sh`
- `RELEASE_GATE_MONITOR_STATIC=1 bash ci/ai-blaise/release-gate-monitor.sh --local-only`
- `bash ci/ai-blaise/production-gap-audit.sh`
- `bash ci/ai-blaise/production-readiness-check.sh audit`
- `bash ci/ai-blaise/v2-closure-check.sh`
- `bash ci/ai-blaise/v2-acceptance-check.sh`
- `bash ci/ai-blaise/features-doc-check.sh`
- `bash ci/ai-blaise/docs-evidence-boundary-check.sh`
- `bash ci/ai-blaise/deploy-check.sh`
- `bash ci/ai-blaise/image-check.sh`
- `git diff --cached --check`
- `git diff --check`

The release performance evidence gate intentionally remains fail-closed without
complete release benchmark artifacts. That behavior is the production boundary,
not a release signoff.

The production gap audit currently emits machine-derived counts similar to:

```text
production_gap_audit source_feature_ids=276 doc_feature_headings=276 feature_headings=276 production_ready=164 alpha_headings=112 inventory_contract=machine_derived source_only_alpha=0 v2_acceptance=model_only production_release_blocked=true live_sql_guards=true k8s_guardrail_contract=true live_k8s_e2e_harness=true chart_folded_to_command_center=2026-05-22
```

Those numeric values are evidence output from the scripts, not hand-maintained
doc prose.
