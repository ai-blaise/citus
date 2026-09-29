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
- Centralized Auth3 token introspection before request planning. Production
  transport is HTTPS-only, verifies an explicitly mounted CA, requires a PEM
  client identity for mTLS, applies a bounded total timeout, and never accepts
  plaintext fallback.
- Exactly one `Authorization: Bearer` token is accepted. `jwt_claims` and
  `tenant_id` in the request body are rejected as untrusted identity input; the
  tenant and complete PostgreSQL claims JSON come only from the validated
  Auth3 response.
- SQL rendering through the `graphql.resolve(...)` pg_graphql boundary with
  Auth3-validated claims installed through a bound
  `pg_catalog.set_config('request.jwt.claims', $1, true)` parameter in the same
  database transaction. Query, variables, and operation name are also bound
  parameters rather than SQL-interpolated input.
- Live PostgreSQL-backed query execution when
  `AI_BLAISE_GRAPHQL_LIVE_EXECUTION=1` is set and
  `AI_BLAISE_GRAPHQL_DATABASE_URL` points at a database with `pg_graphql`
  installed. The process refuses to bind its listener unless all live database
  and Auth3 transport settings are present and the mTLS credentials parse.
- Deterministic persisted-plan and subscription registration state.
- HTTP front door for `/healthz`, `/readyz`, `/metrics`, GraphiQL, and
  `/graphql/v1`. `/graphql/ws` authenticates the bearer token but returns 501;
  there is no implemented WebSocket/subscription transport yet.
- `cargo run -p ai_blaise_citus_sidecar_graphql -- run-canonical`.
- `cargo run -p ai_blaise_citus_sidecar_graphql -- run-runtime-canonical`.
- `cargo run -p ai_blaise_citus_sidecar_graphql -- check-runtime-dependencies`.
- `bash ci/ai-blaise/sidecar-api-runtime-smoke.sh` builds the binary and verifies probe/drain fail-closed behavior.
- `bash ci/ai-blaise/graphql-postgrest-runtime-smoke.sh` verifies static plans,
  dependency reporting, HTTPS-only Auth3 configuration, and fail-closed
  startup when mounted mTLS credentials are absent.
- `bash ci/ai-blaise/graphql-pggraphql-live-smoke.sh` boots a live PostgreSQL
  `pg_graphql` data plane and the real Auth3 sidecar. Auth3 is bound only to
  loopback behind an ephemeral TLS endpoint that requires a client
  certificate. The smoke issues two real tokens, proves tenant-scoped
  `graphql.resolve(...)` results, body-claim rejection, revoked-token
  rejection, and failure of a TLS client without a certificate.
- `bash ci/ai-blaise/api-trio-runtime-smoke.sh` proves the GraphQL process
  refuses to start without its Auth3 transport while covering the other API
  processes independently.

Required GraphQL runtime settings are
`AI_BLAISE_GRAPHQL_DATABASE_URL`, `AI_BLAISE_GRAPHQL_LIVE_EXECUTION=1`,
`AI_BLAISE_GRAPHQL_AUTH_INTROSPECTION_URL` (an exact HTTPS
`/auth/introspect` URL), `AI_BLAISE_GRAPHQL_AUTH_CA_CERT_PATH`,
`AI_BLAISE_GRAPHQL_AUTH_CLIENT_IDENTITY_PATH`,
`AI_BLAISE_GRAPHQL_AUTH_EXPECTED_ISSUER`, and
`AI_BLAISE_GRAPHQL_AUTH_EXPECTED_AUDIENCE`. The optional
`AI_BLAISE_GRAPHQL_AUTH_TIMEOUT_MS` is bounded to at most five seconds.

These contracts cover `FEATURE: API3`, `FEATURE: API4`, and `FEATURE: API5`.
`FEATURE: API4` has separate SQL evidence. The API3 smoke is bounded
single-process evidence, not a high-availability or promotion receipt. Durable
Auth3 identity/session persistence, Auth3 replica/revocation consistency,
production certificate issuance and rotation, Kubernetes traffic policy,
multi-worker GraphQL planning, and the subscription transport remain unproven
or unimplemented and must not be described as production-ready.
