# sidecar/vectorizer

Rust vectorizer sidecar for the ai-blaise Citus fork. It preserves the
pgai-compatible SQL shape from the companion extension while running the worker
runtime as a long-lived Rust service instead of relying on `plpython3u`.

Production-ready surface:

- PostgreSQL-backed queue polling from `ai.vectorizer_queue` with `FOR UPDATE
  SKIP LOCKED`, stale in-flight reclamation, worker ownership, and success or
  failure state transitions.
- Config-validated provider registry with OpenAI, Azure OpenAI-compatible,
  Voyage, Cohere, Ollama, vLLM-compatible, and deterministic mock providers.
- Error classification plus bounded retry/backoff for transient provider
  transport, rate-limit, and server errors.
- Per-tenant token budgets in `ai.tenant_budget`, including reservation,
  refund, overrun rejection, and provider-billed token reconciliation.
- Durable cost accounting in `ai.usage_log`, using a PostgreSQL table that is
  TimescaleDB-hypertable-compatible on `recorded_at`.
- HTTP health, readiness, drain, Prometheus metrics, manual `/vectorize`, and
  `/queue/status` endpoints.

Commands:

```bash
cargo run -p ai_blaise_citus_sidecar_vectorizer -- run-canonical
AI_BLAISE_VECTORIZER_DATABASE_URL=postgres://postgres@127.0.0.1/postgres   cargo run -p ai_blaise_citus_sidecar_vectorizer -- serve
bash ci/ai-blaise/sidecar-vectorizer-smoke.sh
```

`run-canonical` is the deterministic local report used by CI. The smoke script
builds the real binary, starts PostgreSQL 17 in Docker, launches `serve`, waits
for `/readyz`, enqueues 100 rows, verifies succeeded queue rows, checks
`ai.usage_log` rows and budget decrementing, then exercises `/vectorize` and
`/queue/status`.

Environment:

- `AI_BLAISE_VECTORIZER_DATABASE_URL` is required in `serve` mode.
- `AI_BLAISE_LISTEN_ADDR` defaults to `0.0.0.0:8080`.
- `AI_BLAISE_VECTORIZER_PROVIDER_MODE` is `mock`, `live`, or `mixed`.
- `AI_BLAISE_VECTORIZER_BATCH_SIZE`, `AI_BLAISE_VECTORIZER_POLL_INTERVAL_MS`,
  `AI_BLAISE_VECTORIZER_VISIBILITY_TIMEOUT_SECONDS`,
  `AI_BLAISE_VECTORIZER_RETRY_INITIAL_BACKOFF_MS`, and
  `AI_BLAISE_VECTORIZER_PROVIDER_MAX_ATTEMPTS` tune queue and retry behavior.
- Live providers are enabled with `OPENAI_API_KEY`, `AZURE_OPENAI_API_KEY` plus
  `AZURE_OPENAI_BASE_URL`, `VOYAGE_API_KEY`, `COHERE_API_KEY`,
  `OLLAMA_BASE_URL` or `ENABLE_OLLAMA`, and `VLLM_BASE_URL`.

Feature markers: `FEATURE: A2`, `FEATURE: A3`, `FEATURE: A4`, `FEATURE: A5`,
and `FEATURE: A6`.
