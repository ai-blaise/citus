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
- `cargo run -p ai_blaise_citus_sidecar_mcp -- run-canonical`

These contracts cover `FEATURE: MCP1`, `FEATURE: MCP2`, and `FEATURE: MCP3`.
