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
- `handle_mcp_sidecar_http_bytes()`
- `cargo run -p ai_blaise_citus_sidecar_mcp -- run-canonical`
- `cargo run -p ai_blaise_citus_sidecar_mcp -- serve`
- `cargo run -p ai_blaise_citus_sidecar_mcp -- serve-stdio`
- `ci/ai-blaise/mcp-sidecar-stdio-smoke.sh`
- `ci/ai-blaise/mcp-sidecar-http-smoke.sh`

The `serve-stdio` mode validates the canonical sidecar MCP policy before
serving the same safe-mode and tenant-scope JSON-RPC tool contract exposed by
`tools/citus-mcp`. The `serve` mode keeps the standard sidecar `/healthz`,
`/readyz`, and `/metrics` endpoints and adds `POST /mcp` for HTTP JSON-RPC.
Both modes identify as `ai-blaise-citus-mcp-sidecar` and are guarded in CI by
real process traffic; the Kubernetes production smoke additionally sends
`POST /mcp` through a port-forward to the deployed exhaustive-profile sidecar
pod. Authentication integration, live database/Kubernetes tool execution, and
production deployment of this sidecar remain alpha.

These surfaces cover `FEATURE: MCP1`, `FEATURE: MCP2`, `FEATURE: MCP3`, and
`FEATURE: D11`.
