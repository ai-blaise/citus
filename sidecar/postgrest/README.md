# sidecar/postgrest

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

PostgREST runtime wrapper for the auto-REST API surface.

Implemented surface:

- `PostgrestSidecarPlan`, `RestRoute`, `DistributedViewBinding`,
  `ApiAuthPolicy`, and `OpenApiPlan` validation.
- Generated `postgrest.conf` with secret-backed `db-uri` and `jwt-secret`
  references rather than embedded secret values.
- `PostgrestSupervisor` lifecycle tracking plus `spawn_child_at(...)`, which
  writes the generated config and launches the configured PostgREST binary.
- HTTP front door for `/healthz`, `/readyz`, `/metrics`, `/openapi.json`, and
  canonical `/api/<table>` route descriptors.
- `cargo run -p ai_blaise_citus_sidecar_postgrest -- run-canonical`.
- `cargo run -p ai_blaise_citus_sidecar_postgrest -- run-runtime-canonical`.
- `bash ci/ai-blaise/sidecar-api-runtime-smoke.sh` builds the binary and verifies probe/drain fail-closed behavior.
- `bash ci/ai-blaise/api-trio-runtime-smoke.sh` boots the service and verifies
  readiness, metrics, OpenAPI, and route behavior over real TCP.

These contracts cover `FEATURE: API1`, `FEATURE: API2`, `FEATURE: API5`, and
`FEATURE: API6`. They do not prove a production database-backed PostgREST
binary deployment until the operand image includes PostgREST and a live
Postgres-backed smoke records that evidence.
