# sidecar/postgrest

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

PostgREST runtime wrapper for the auto-REST API surface.

Current implemented surface:

- `PostgrestSidecarPlan`
- `RestRoute`
- `DistributedViewBinding`
- `ApiAuthPolicy`
- `OpenApiPlan`
- `canonical_postgrest_execution_plan()`
- `cargo run -p ai_blaise_citus_sidecar_postgrest -- run-canonical`

These contracts cover `FEATURE: API1`, `FEATURE: API2`, `FEATURE: API5`, and
`FEATURE: API6`.
