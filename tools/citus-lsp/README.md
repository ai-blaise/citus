# citus-lsp

Postgres language-server fork target with Citus and TimescaleDB diagnostics.

The initial Rust crate defines the analyzer contract used by the future LSP
server:

- non-colocated join diagnostics
- unsafe distribution-column ALTER diagnostics
- missing tenant-filter diagnostics
- missing search-analyzer diagnostics
- Timescale hypertable bridge diagnostics
- quick-fix actions for adding distribution columns and bridge calls

These contracts cover `FEATURE: D4`, `FEATURE: M5`, and `FEATURE: TS8`.
