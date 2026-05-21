# citus-mcp

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Model Context Protocol command-line server for safe cluster operations.

Current implemented surface:

- `McpToolRequest`
- `McpTool`
- `TenantScope`
- `SafeMode`
- `serve-stdio` line-delimited JSON-RPC handler for `initialize`,
  `tools/list`, and guarded `tools/call` requests

These contracts cover `FEATURE: MCP1`, `FEATURE: MCP2`, `FEATURE: MCP3`, and
the `FEATURE: D11` MCP developer workflow.
Use `cargo run -p ai_blaise_citus_mcp -- run-canonical` to emit the
deterministic tool policy TSV report.
Use `cargo run -p ai_blaise_citus_mcp -- serve-stdio` for the production-ready
stdio policy surface that is smoke-tested by
`ci/ai-blaise/mcp-stdio-smoke.sh`. Live database/Kubernetes tool execution and
the `sidecar/mcp` deployment remain alpha.
