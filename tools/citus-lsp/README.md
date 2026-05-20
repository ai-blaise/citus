# citus-lsp

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
