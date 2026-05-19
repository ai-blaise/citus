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
- `EmbeddingProviderClient`
- `UsageLogRecord`

This is the first local model for `FEATURE: A2` vectorizer execution and
`FEATURE: A4` per-tenant token budgets, with provider routing for
`FEATURE: A3`, usage accounting for `FEATURE: A5`, and shard-local execution
planning for `FEATURE: A6`.
