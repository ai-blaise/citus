# tools

Developer and operator tools:

- `citusctl`
- `citus-tui`
- `citus-lsp`
- `citus-admin`
- `citus-schema-designer`
- `citus-mcp`
- `citus-watch`

`citus-lsp` now has the initial Rust diagnostic contract for distributed SQL
editing: Citus colocation checks, distribution-column safety, tenant-filter
hints, Timescale hypertable bridge diagnostics, and quick-fix planning.
