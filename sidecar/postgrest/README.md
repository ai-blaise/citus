# sidecar/postgrest

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
