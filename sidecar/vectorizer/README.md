# sidecar/vectorizer

Rust vectorizer worker that will implement the `pgai` vectorizer model without
the `plpython3u` runtime floor.

Current implemented surface:

- `VectorizerJob`
- `TenantTokenBudget`
- `TokenReservation`
- `ProviderRoute`
- `QueuePollPlan`
- `DistributedVectorizePlan`
- `VectorizerWorker`
- `VectorizerExecutionReport`
- `EmbeddingProviderClient`
- `UsageLogRecord`

`cargo run -p ai_blaise_citus_sidecar_vectorizer -- run-canonical` executes a
deterministic canonical batch and emits usage records as TSV. This is the
executable local model for `FEATURE: A2` vectorizer execution and `FEATURE: A4`
per-tenant token budgets, with provider routing for `FEATURE: A3`, usage
accounting for `FEATURE: A5`, and shard-local execution planning for
`FEATURE: A6`.
