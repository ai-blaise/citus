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
`FEATURE: Search8`. The runtime flow validates layer/object-store placement,
materializes pageserver-lite layer bytes, refreshes cross-tier planner routes,
counts cold-tier reads, and publishes Tantivy/LanceDB search index artifacts.
