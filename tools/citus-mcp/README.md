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
- `FEATURE: MCP4` read-only database execution when
  `AI_BLAISE_MCP_DATABASE_URL` is set, using the maintained PostgreSQL client
  with native TLS support, `BEGIN READ ONLY`, `SET LOCAL statement_timeout`,
  row and timeout ceilings, `EXPLAIN ANALYZE` rejection, tenant schema denial,
  and destructive-tool denial

These contracts cover `FEATURE: MCP1`, `FEATURE: MCP2`, `FEATURE: MCP3`,
`FEATURE: MCP4`, and the `FEATURE: D11` MCP developer workflow.
Use `cargo run -p ai_blaise_citus_mcp -- run-canonical` to emit the
deterministic tool policy TSV report.
Use `cargo run -p ai_blaise_citus_mcp -- serve-stdio` for the validation-only
stdio policy surface that is smoke-tested by
`ci/ai-blaise/mcp-stdio-smoke.sh`; set `AI_BLAISE_MCP_DATABASE_URL` and run
`ci/ai-blaise/mcp-db-smoke.sh` to exercise the MCP4 database path against a
real PostgreSQL container. Authentication, mutating database execution,
Kubernetes tool execution, and production deployment of the `sidecar/mcp`
service remain alpha.
