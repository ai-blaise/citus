# sidecar/graphql

GraphQL endpoint built on bundled `pg_graphql` and companion helper metadata.

Current implemented surface:

- `GraphqlSidecarPlan`
- `GraphqlSchemaBinding`
- `DistributedGraphqlBinding`
- `GraphqlAuthPolicy`
- `canonical_graphql_execution_plan()`
- `cargo run -p ai_blaise_citus_sidecar_graphql -- run-canonical`

These contracts cover `FEATURE: API3`, `FEATURE: API4`, and `FEATURE: API5`.
