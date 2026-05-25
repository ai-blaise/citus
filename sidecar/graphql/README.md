# sidecar/graphql

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

GraphQL endpoint boundary for bundled `pg_graphql` and companion distributed
metadata.

Implemented surface:

- `GraphqlSidecarPlan`, `GraphqlSchemaBinding`, `DistributedGraphqlBinding`,
  and `GraphqlAuthPolicy` validation.
- RLS/JWT tenant-claim enforcement before request planning.
- SQL rendering through the `graphql.resolve(...)` pg_graphql boundary with
  request JWT claims installed through `set_config('request.jwt.claims', ...)`.
- Live PostgreSQL-backed query execution when
  `AI_BLAISE_GRAPHQL_LIVE_EXECUTION=1` is set and
  `AI_BLAISE_GRAPHQL_DATABASE_URL` points at a database with `pg_graphql`
  installed.
- Deterministic persisted-plan and subscription registration state.
- HTTP front door for `/healthz`, `/readyz`, `/metrics`, GraphiQL, `/graphql/v1`,
  and the `/graphql/ws` subscription transport boundary.
- `cargo run -p ai_blaise_citus_sidecar_graphql -- run-canonical`.
- `cargo run -p ai_blaise_citus_sidecar_graphql -- run-runtime-canonical`.
- `cargo run -p ai_blaise_citus_sidecar_graphql -- check-runtime-dependencies`.
- `bash ci/ai-blaise/sidecar-api-runtime-smoke.sh` builds the binary and verifies probe/drain fail-closed behavior.
- `bash ci/ai-blaise/graphql-postgrest-runtime-smoke.sh` boots the service and verifies live TCP probes, query/subscription boundary responses, malformed input handling, and fail-closed database/JWT dependency validation.
- `bash ci/ai-blaise/graphql-pggraphql-live-smoke.sh` boots a live PostgreSQL
  `pg_graphql` data plane, starts the sidecar in live execution mode, and
  verifies tenant-scoped `graphql.resolve(...)` results through `/graphql/v1`.
- `bash ci/ai-blaise/api-trio-runtime-smoke.sh` boots the service and verifies
  readiness, query handling, and subscription-boundary registration over real
  TCP.

These contracts cover `FEATURE: API3`, `FEATURE: API4`, and `FEATURE: API5`.
`FEATURE: API4` has separate production-ready SQL evidence. The API3
production-ready boundary covers live query execution and tenant RLS; durable
subscription fan-out, multi-worker GraphQL planning, and Kubernetes traffic are
outside this sidecar proof.
