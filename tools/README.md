# tools

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Developer and operator tools:

- `citusctl`
- `citus-tui`
- `citus-lsp`
- `citus-admin`
- `citus-schema-designer`
- `citus-mcp`
- `citus-watch`

`citusctl` now has a deterministic canonical command runner covering dev,
apply, inspect, time-travel, and WAL replay planning contracts.
`cargo run -p ai_blaise_citusctl -- run-canonical` emits the summary used by
CI.

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
