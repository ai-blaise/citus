# sidecar/vectorizer

Rust vectorizer sidecar for the ai-blaise Citus fork. It preserves the
pgai-compatible SQL shape from the companion extension while running the worker
runtime as a long-lived Rust service instead of relying on `plpython3u`.

Production-ready surface (bounded to the local Rust sidecar runtime verified by unit tests and the Docker/PostgreSQL smoke):

- PostgreSQL-backed queue polling from `ai.vectorizer_queue` with `FOR UPDATE
  SKIP LOCKED`, stale in-flight reclamation, worker ownership, and success or
  failure state transitions.
- Config-validated provider registry and provider-mode policy. The verified
  runtime uses deterministic mock providers; live/mixed network provider modes
  require `AI_BLAISE_VECTORIZER_ALLOW_LIVE_PROVIDERS=1`; `mixed` also requires
  at least one configured live provider. External provider operation is not
  claimed as verified by this crate smoke.
- Error classification plus bounded retry/backoff for transient provider
  transport, rate-limit, and server errors.
- Per-tenant token budgets in `ai.tenant_budget`, including reservation,
  refund, overrun rejection, and provider-billed token reconciliation.
- Durable cost accounting in `ai.usage_log`, using a PostgreSQL table that is
  TimescaleDB-hypertable-compatible on `recorded_at`.
- HTTP health, readiness, drain, Prometheus metrics, manual `/vectorize`, and
  `/queue/status` endpoints.
- Optional `FEATURE: A8` CRD-derived runtime contract that binds a sidecar
  instance to one provider, model, and destination dimension. When configured,
  manual requests and queue rows with another provider/model fail before budget
  reservation, and provider embeddings with the wrong dimension are rejected
  before queue success or usage accounting.

Commands:

```bash
cargo run -p ai_blaise_citus_sidecar_vectorizer -- run-canonical
AI_BLAISE_VECTORIZER_DATABASE_URL=postgres://postgres@127.0.0.1/postgres   cargo run -p ai_blaise_citus_sidecar_vectorizer -- serve
bash ci/ai-blaise/sidecar-vectorizer-smoke.sh
```

`run-canonical` is the deterministic local report used by CI. The smoke script
builds the real binary, starts PostgreSQL 17 in Docker, launches `serve` on a
fresh ephemeral loopback port, waits for `/readyz`, enqueues 100 rows, verifies
succeeded queue rows, checks `ai.usage_log` rows, budget decrementing, metrics,
manual `/vectorize` success, fail-closed invalid `/vectorize` requests,
CRD contract provider/model mismatch rejection, startup rejection for mismatched
mock dimensions, and `/queue/status`.

Environment:

- `AI_BLAISE_VECTORIZER_DATABASE_URL` is required in `serve` mode.
- `AI_BLAISE_LISTEN_ADDR` defaults to `0.0.0.0:8080`.
- `AI_BLAISE_VECTORIZER_PROVIDER_MODE` is `mock`, `live`, or `mixed`.
- `AI_BLAISE_VECTORIZER_CONTRACT_PROVIDER`,
  `AI_BLAISE_VECTORIZER_CONTRACT_MODEL`, and
  `AI_BLAISE_VECTORIZER_CONTRACT_DIMENSIONS` are the optional operator-rendered
  A8 contract. If any one is set, all three are required.
- `AI_BLAISE_VECTORIZER_BATCH_SIZE`, `AI_BLAISE_VECTORIZER_POLL_INTERVAL_MS`,
  `AI_BLAISE_VECTORIZER_VISIBILITY_TIMEOUT_SECONDS`,
  `AI_BLAISE_VECTORIZER_RETRY_INITIAL_BACKOFF_MS`, and
  `AI_BLAISE_VECTORIZER_PROVIDER_MAX_ATTEMPTS` tune queue and retry behavior.
- Live providers require `AI_BLAISE_VECTORIZER_ALLOW_LIVE_PROVIDERS=1` plus
  provider-specific config such as `OPENAI_API_KEY`, `AZURE_OPENAI_API_KEY`
  with `AZURE_OPENAI_BASE_URL`, `VOYAGE_API_KEY`, `COHERE_API_KEY`,
  `OLLAMA_BASE_URL` or `ENABLE_OLLAMA`, and `VLLM_BASE_URL`. The repository
  smoke does not claim successful external provider operation.

Feature markers: `FEATURE: A2`, `FEATURE: A3`, `FEATURE: A4`, `FEATURE: A5`,
`FEATURE: A6`, and `FEATURE: A8`.
