# sidecar/mcp

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Model Context Protocol service for cluster operations and AI-agent access.

Current implemented surface:

- `McpSidecarPlan`
- `McpAuthPlan`
- `McpSessionPolicy`
- `canonical_mcp_execution_plan()`
- `handle_mcp_sidecar_stdio_request()`
- `cargo run -p ai_blaise_citus_sidecar_mcp -- run-canonical`
- `cargo run -p ai_blaise_citus_sidecar_mcp -- serve-stdio`
- `ci/ai-blaise/mcp-sidecar-stdio-smoke.sh`

The `serve-stdio` mode validates the canonical sidecar MCP policy before
serving the same safe-mode and tenant-scope JSON-RPC tool contract exposed by
`tools/citus-mcp`. It identifies itself as `ai-blaise-citus-mcp-sidecar` and
is guarded in CI by real stdin/stdout JSON-RPC requests.

These surfaces cover `FEATURE: MCP1`, `FEATURE: MCP2`, `FEATURE: MCP3`, and
`FEATURE: D11`.
