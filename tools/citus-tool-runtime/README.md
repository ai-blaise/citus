# citus-tool-runtime

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Shared snapshot parser and renderer helpers for the Citus UI tools.

The crate validates a tab-separated cluster snapshot used by `citus-admin`,
`citus-schema-designer`, `citus-tui`, and `citus-watch`. Use
`cargo run -p ai_blaise_citus_tool_runtime -- run-canonical` to emit the
deterministic canonical snapshot summary used by executable-target gates.
