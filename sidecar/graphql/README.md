# sidecar/graphql

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

GraphQL endpoint built on bundled `pg_graphql` and companion helper metadata.

Current implemented surface:

- `GraphqlSidecarPlan`
- `GraphqlSchemaBinding`
- `DistributedGraphqlBinding`
- `GraphqlAuthPolicy`
- `canonical_graphql_execution_plan()`
- `cargo run -p ai_blaise_citus_sidecar_graphql -- run-canonical`

These contracts cover `FEATURE: API3`, `FEATURE: API4`, and `FEATURE: API5`.
