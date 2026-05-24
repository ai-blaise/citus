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
- HTTP front door for `/healthz`, `/readyz`, `/metrics`, `/openapi.json`,
  canonical `/api/<table>` route descriptors, and opt-in proxying to an upstream
  PostgREST instance via `AI_BLAISE_POSTGREST_UPSTREAM`.
- `cargo run -p ai_blaise_citus_sidecar_postgrest -- run-canonical`.
- `cargo run -p ai_blaise_citus_sidecar_postgrest -- run-runtime-canonical`.
- `cargo run -p ai_blaise_citus_sidecar_postgrest -- check-runtime-dependencies`.
- `cargo run -p ai_blaise_citus_sidecar_postgrest -- run-live-postgrest`
  launches the configured PostgREST binary with resolved `PGRST_*` env values
  while keeping `postgrest.conf` secret-free.
- `bash ci/ai-blaise/sidecar-api-runtime-smoke.sh` builds the binary and verifies probe/drain fail-closed behavior.
- `bash ci/ai-blaise/graphql-postgrest-runtime-smoke.sh` boots the service and verifies live TCP probes, OpenAPI/config/route behavior, malformed input handling, method rejection, and fail-closed PostgREST database/JWT/binary dependency validation.
- `bash ci/ai-blaise/postgrest-live-data-plane-smoke.sh` runs a Citus-capable
  PostgreSQL container, the official PostgREST binary, the sidecar supervisor,
  and the sidecar proxy to verify table-backed REST traffic, `api.orders` schema
  profile routing, tenant RLS, and secret non-disclosure.
- `bash ci/ai-blaise/api-trio-runtime-smoke.sh` boots the service and verifies
  readiness, metrics, OpenAPI, and route behavior over real TCP.

These contracts cover `FEATURE: API1`, `FEATURE: API2`, `FEATURE: API5`, and
`FEATURE: API6` for the PostgREST REST path. They do not prove live
`pg_graphql` execution, edge-function execution, Kubernetes operator rollout, or
multi-worker rebalance orchestration; those remain with their respective alpha
feature boundaries until separately proven.
