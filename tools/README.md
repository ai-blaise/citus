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

`citus-lsp` now has a file-backed diagnostic CLI for supported distributed SQL
migration statements: Citus colocation checks, distribution-column safety,
tenant-filter hints, Timescale hypertable bridge diagnostics, and quick-fix
planning. `ci/ai-blaise/citus-lsp-smoke.sh` guards the promoted CLI surface;
JSON-RPC editor transport and full PostgreSQL grammar coverage remain alpha.

`citus-watch` now has a snapshot-backed dashboard frame runtime for the
unified operator view over companion metadata, Prometheus metrics, and
pool-side signals.
`cargo run -p ai_blaise_citus_watch -- run-canonical` emits the deterministic
watch dashboard summary used by CI, and
`cargo run -p ai_blaise_citus_watch -- render-frame --snapshot <snapshot.tsv>`
renders the terminal dashboard frame guarded by
`ci/ai-blaise/tools-ui-runtime-smoke.sh`.

`citus-schema-designer` now has a snapshot-backed SVG renderer for
distribution, hypertable, search-index, webhook, and operator shard-map
overlays.
`cargo run -p ai_blaise_citus_schema_designer -- run-canonical` emits the
deterministic overlay-layer summary used by CI, and
`cargo run -p ai_blaise_citus_schema_designer -- render-svg --snapshot <snapshot.tsv>`
renders the guarded SVG output.

`citus-tui` now has a snapshot-backed terminal frame runtime for the
rainfrog-based shell panels and guarded operator action previews.
`cargo run -p ai_blaise_citus_tui -- run-canonical` emits the deterministic
TUI session summary used by CI, and
`cargo run -p ai_blaise_citus_tui -- render-frame --snapshot <snapshot.tsv> --panel shards`
renders a concrete frame from measured snapshot data.

`citus-admin` now has a snapshot-backed HTML route renderer and fail-closed
action validator for the WhoDB-based web administration surface.
`cargo run -p ai_blaise_citus_admin -- run-canonical` emits the deterministic
admin route/action summary used by CI, and
`cargo run -p ai_blaise_citus_admin -- render --snapshot <snapshot.tsv> --route /cluster/shards`
renders a concrete route from the same runtime snapshot.

The shared snapshot parser and validation model lives in
`tools/citus-tool-runtime`.

`citus-mcp` now has a deterministic CLI policy runner in addition to the
sidecar MCP runner: `cargo run -p ai_blaise_citus_mcp -- run-canonical`.
