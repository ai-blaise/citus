# E2E Acceptance Harness

This crate holds executable acceptance contracts for ai-blaise critical paths.
The first harness models the Timescale-on-Citus path before the kind-based
database runner is wired:

1. required preload configuration for Citus and TimescaleDB cohabitation
2. a `Hypertable` CRD spec
3. the operator reconcile plan backed by companion planning primitives
4. the gates the real cluster test must prove

The model is intentionally pure Rust so it can run in every pull request. The
future cluster runner should consume the same scenario shape instead of inventing
a parallel acceptance contract.
