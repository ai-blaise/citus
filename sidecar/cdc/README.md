# sidecar/cdc

CDC runtimes for ai-blaise Citus overlays.

The runtime accepts wal2json frames over `serve-runtime` (`POST /ingest`), exposes `/healthz`, `/readyz`, `/metrics`, `/state`, `/streams`, and `/dlq`, applies PII anonymization before encoding sink frames, parses configured DDL stream-table rows into `DdlStreamEvent` records, advances checkpoint/ack state after dispatch, and can bridge events to realtime over TCP or `unix:/path.sock`.

Implemented surfaces:

- `source`: `LogicalReplicationClient`, `ReplicationFrame`, wal2json decoder, pgoutput logical-view decoder, schema-capture DDL stream parsing, checkpoint/ack tracking.
- `live`: runtime ingest, sink dispatch, DLQ routing, realtime bridge.
- `sinks`: deterministic webhook, realtime, NATS core PUB, Pub/Sub, Kafka, Kinesis, HTTP/2, and analytical mirror wire frames.
- `anon` and `dlq`: defense-in-depth anonymization and durable dead-letter records.

Verification:

- `cargo test -p ai_blaise_citus_sidecar_cdc`
- `cargo run -q -p ai_blaise_citus_sidecar_cdc -- run-live-canonical`
- `REQUIRE_DOCKER=1 bash ci/ai-blaise/sidecar-cdc-smoke.sh` for the live PostgreSQL DDL capture harness
- `bash ci/ai-blaise/sidecar-cdc-smoke.sh`

External managed brokers remain explicit integration boundaries unless their feature entry in `docs/ai-blaise/NEW_FEATURES.md` says otherwise.

The separate `serve` entry point is not a working logical-replication stream:
its consumer currently verifies slot/publication setup and returns after
`SELECT 1`, without a frame pump. Its `/readyz` therefore returns 503 and its
ready metric stays zero, including probe-only operation when targets or NATS
are unavailable. `/healthz` remains a liveness endpoint; metrics and drain
remain available. A NATS connection or successful setup query must not be
interpreted as replication, durable delivery, or recovery readiness.

The custom-component observability smoke verifies this probe-only state explicitly:
CDC readiness is 503 with `ready=false` and the inactive-stream reason, health
and metrics remain 200, and the ready gauge is zero. Other component readiness
expectations are unchanged; a dead CDC process or an unrelated 503 does not pass.
