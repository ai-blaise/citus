# sidecar/cdc Modifications

## 2026-05-22 — Logical-replication consumer + async-nats sink + axum probe

Added under `serve`:

- `src/replication.rs` — tokio-postgres consumer that opens connections to the
  coordinator and every worker, ensures the logical replication slot under the
  `command_center_cdc` publication exists, and exposes a `PgoutputRowChange`
  surface decoded into `CdcEventEnvelope`. Env knobs:
  - `CITUS_CDC_COORDINATOR_URL` — coordinator DSN.
  - `CITUS_CDC_WORKER_URLS` — comma-separated worker DSNs.
  - `CITUS_CDC_SLOT_NAME` — defaults to `ai_blaise_cdc`.
  - `CITUS_CDC_PUBLICATION` — defaults to `command_center_cdc`.
- `src/nats_sink.rs` — `async-nats` publisher. Subjects formatted as
  `citus.cdc.<schema>.<table>`; headers include `Ai-Blaise-Cdc-Lsn`,
  `Ai-Blaise-Cdc-Tx-Xid`, `Ai-Blaise-Cdc-Op`, `Ai-Blaise-Cdc-Source`.
  Configured via `CITUS_CDC_NATS_URL` (default `nats://127.0.0.1:4222`).
- `src/runtime.rs` — `serve()` builds an `axum` router that delegates
  `/healthz`, `/readyz`, `/metrics`, and `/drain` to the shared
  `SidecarRuntime::handle_http_request` surface, then runs the consumer pool
  alongside it. SIGTERM / Ctrl-C triggers a graceful drain and aborts the
  worker tasks.
- `src/main.rs` — `cdc serve` now boots a multi-thread tokio runtime running
  the above; `run-canonical` and `run-runtime-canonical` paths are unchanged.
- `tests/cdc_replay.rs` — `#[ignore]`-marked integration test pinned to the
  testcontainers Docker harness; full Postgres+NATS spin lives in the
  bootstrap-v2 cluster e2e harness.

Regression coverage: unit tests in `replication::tests` cover env target
parsing, `PgoutputRowChange::into_event` validation, and the header set the
sink emits. Existing canonical contract tests in `lib.rs` are untouched.
