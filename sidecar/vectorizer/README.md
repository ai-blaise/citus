# sidecar/vectorizer

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

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
