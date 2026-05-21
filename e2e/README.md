# E2E Acceptance Harness

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

This crate holds executable acceptance contracts for ai-blaise critical paths.
The first harness models the Timescale-on-Citus path that is exercised by the
kind smoke script:

1. required preload configuration for Citus and TimescaleDB cohabitation
2. a `Hypertable` CRD spec
3. the guarded operator apply plan backed by companion planning primitives
4. the gates the real cluster test must prove

The model is intentionally pure Rust so it can run in every pull request, while
`tests/e2e/kind-timescale-citus-smoke.sh` consumes the same scenario shape for
contract and live cluster checks.

`release_gate_report` emits the canonical V2 release-gate TSV for the 15
continuous gates in the plan. The `v2-acceptance` CI workflow asserts that all
15 gates are represented and that the upstream-merge dry-run is pinned to the
current release branch.
