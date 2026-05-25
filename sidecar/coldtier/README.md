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
- `ColdTierMaterializationReport`
- `canonical_move_plans()`
- `canonical_cold_tier_runtime_report()`
- `materialize_file_artifacts()`
- `cargo run -p ai_blaise_citus_sidecar_coldtier -- run-canonical`
- `cargo run -p ai_blaise_citus_sidecar_coldtier -- run-runtime-canonical`
- `cargo run -p ai_blaise_citus_sidecar_coldtier -- run-local-file-materialization-canonical`

These contracts cover `FEATURE: R1`, `FEATURE: R5`, `FEATURE: R9`, and
`FEATURE: Search8`. The production-ready boundary is deterministic local
`file://` materialization: it validates layer/object placement, writes canonical
image, delta, Tantivy, and Lance artifact files under `/tmp/ai-blaise-coldtier`,
verifies artifact byte sizes, rejects non-file materialization, refreshes
cross-tier planner-route reports, and records `cold_tier_reads` as a sidecar
accounting counter. Live S3/GCS/Azure writes, Kubernetes pageserver deployment,
Citus route changes, Citus cold-read serving, distributed query planner
integration, and Tantivy/LanceDB query execution remain alpha.

Focused smoke:

- `ci/ai-blaise/sidecar-coldtier-runtime-smoke.sh`
