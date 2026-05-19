# sidecar/vectorizer

Rust vectorizer worker that will implement the `pgai` vectorizer model without
the `plpython3u` runtime floor.

Current implemented surface:

- `VectorizerJob`
- `TenantTokenBudget`
- `TokenReservation`

This is the first local model for `FEATURE: A2` vectorizer execution and
`FEATURE: A4` per-tenant token budgets.
