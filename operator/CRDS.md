# CRD Catalog

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Implemented CRD surface from the V2 plan. `operator/src/main.rs` validates the
canonical catalog and emits the CI-gated `run-canonical` TSV summary. The
controller boundary runner also emits typed Conditions for the alpha mutation
edge: direct SQL execution and `.status` updates remain non-executable unless a
separate implementation moves them out of `AlphaNotImplemented` and updates the
smoke expectations.

- `CitusCluster` (`FEATURE: S4`, canonical Rust spec implemented)
- `ShardGroup` (`FEATURE: S2`, canonical Rust spec implemented)
- `Hypertable` (`FEATURE: TS7`, Rust spec and guarded apply plan implemented)
- `Branch` (`FEATURE: R2`, `FEATURE: C6`, `FEATURE: C7`, `FEATURE: C8`,
  canonical Rust spec implemented)
- `Vectorizer` (`FEATURE: A8`, canonical Rust spec implemented)
- `Sidecar` (`FEATURE: O5`, canonical Rust spec implemented)
- `Migration` (`FEATURE: C9`, `FEATURE: M3`, canonical Rust spec implemented)
- `ConflictPolicy` (`FEATURE: C4`, `FEATURE: C5`, canonical Rust spec
  implemented and included in the operator runner)
- `Tenant` (`FEATURE: S10`, `FEATURE: TO1`, `FEATURE: TO2`, `FEATURE: TO5`,
  canonical Rust spec implemented)
- `Region` (`FEATURE: MR1`, `FEATURE: MR4`, `FEATURE: MR8`, canonical Rust
  spec implemented)
- `SurvivalGoal` (`FEATURE: S11`, `FEATURE: MR2`, canonical Rust spec
  implemented and included in the operator runner)
- `Backup` (`FEATURE: B2`, `FEATURE: B6`, canonical Rust spec implemented)
- `Federation` (`FEATURE: F1`, canonical Rust spec implemented)
- `SearchIndex` (`FEATURE: Search2`, `FEATURE: Search7`, canonical Rust spec
  implemented and included in the operator runner)
- `Webhook` (`FEATURE: WH1`, canonical Rust spec implemented)
- `Function` (`FEATURE: EF3`, canonical Rust spec implemented)
- `ScheduledRepack` (`FEATURE: R7`, canonical Rust spec implemented)
