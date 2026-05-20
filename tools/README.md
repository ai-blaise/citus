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
`cargo run -p ai_blaise_citus_watch -- run-canonical` emits the deterministic
watch dashboard summary used by CI.

`citus-schema-designer` now has the initial Rust visualization contract for
distribution, hypertable, search-index, webhook, and live shard-map overlays.
`cargo run -p ai_blaise_citus_schema_designer -- run-canonical` emits the
deterministic overlay-layer summary used by CI.

`citus-tui` now has the initial Rust session contract for the rainfrog-based
terminal shell panels and guarded operator actions.
`cargo run -p ai_blaise_citus_tui -- run-canonical` emits the deterministic
TUI session summary used by CI.

`citus-admin` now has the initial Rust route and action contract for the
WhoDB-based web administration surface.
`cargo run -p ai_blaise_citus_admin -- run-canonical` emits the deterministic
admin route/action summary used by CI.

`citus-mcp` now has a deterministic CLI policy runner in addition to the
sidecar MCP runner: `cargo run -p ai_blaise_citus_mcp -- run-canonical`.
