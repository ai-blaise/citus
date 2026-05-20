# E2E Acceptance Harness

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
