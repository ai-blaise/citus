# sidecar/coldtier

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Pageserver-lite storage for cold shards and backup-as-data-source paths.

Current implemented surface:

- `ColdTierPlan`
- `TierPolicy`
- `ColdShard`
- `LayerFile`
- `SearchColdTierPlan`
- `TierMovePlan`
- `ColdTierRuntime`
- `ColdTierRuntimeReport`
- `canonical_move_plans()`
- `canonical_cold_tier_runtime_report()`
- `cargo run -p ai_blaise_citus_sidecar_coldtier -- run-canonical`
- `cargo run -p ai_blaise_citus_sidecar_coldtier -- run-runtime-canonical`

These contracts cover `FEATURE: R1`, `FEATURE: R5`, `FEATURE: R9`, and
`FEATURE: Search8`. The hardened alpha boundary is the deterministic local
`file://` runtime model: it validates layer/object placement, materializes
pageserver-lite layer bytes, refreshes cross-tier planner-route reports, counts
simulated cold-route reads, and publishes typed Tantivy/LanceDB search index artifacts.
Live S3/GCS/Azure writes, Kubernetes pageserver deployment, Citus route changes,
and Tantivy/LanceDB query serving remain alpha.

Focused smoke:

- `ci/ai-blaise/sidecar-coldtier-runtime-smoke.sh`
