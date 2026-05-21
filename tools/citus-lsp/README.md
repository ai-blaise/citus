# citus-lsp

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Postgres language-server fork target with Citus and TimescaleDB diagnostics.

The Rust crate defines the analyzer contract and a runnable diagnostic surface:

- non-colocated join diagnostics
- unsafe distribution-column ALTER diagnostics
- missing tenant-filter diagnostics
- missing search-analyzer diagnostics
- Timescale hypertable bridge diagnostics
- quick-fix actions for adding distribution columns and bridge calls

`cargo run -p ai_blaise_citus_lsp -- analyze-canonical` emits a tab-separated
diagnostic stream for the canonical Citus/Timescale SQL scenario. CI checks the
binary and library targets together so the executable surface cannot drift from
the analyzer contract.

These contracts cover `FEATURE: D4`, `FEATURE: M5`, and `FEATURE: TS8`.
