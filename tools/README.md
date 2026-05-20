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

`citus-watch` now has the initial Rust dashboard contract for the unified live
view over companion metadata, Prometheus metrics, and pool-side signals.

`citus-schema-designer` now has the initial Rust visualization contract for
distribution, hypertable, search-index, webhook, and live shard-map overlays.

`citus-tui` now has the initial Rust session contract for the rainfrog-based
terminal shell panels and guarded operator actions.

`citus-admin` now has the initial Rust route and action contract for the
WhoDB-based web administration surface.
