# citus-mcp

Model Context Protocol command-line server for safe cluster operations.

Current implemented surface:

- `McpToolRequest`
- `McpTool`
- `TenantScope`
- `SafeMode`

These contracts cover `FEATURE: MCP1`, `FEATURE: MCP2`, `FEATURE: MCP3`, and
the `FEATURE: D11` MCP developer workflow.
Use `cargo run -p ai_blaise_citus_mcp -- run-canonical` to emit the
deterministic tool policy TSV report.
