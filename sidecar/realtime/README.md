# sidecar/realtime

Production realtime runtime for CDC-backed Phoenix-style channels.

The runtime serves raw WebSocket clients at `/realtime/v1/websocket`, accepts CDC ingest over TCP or `unix:/path.sock`, exposes `/healthz`, `/readyz`, and `/metrics` on the WebSocket listener, and broadcasts `postgres_changes` plus presence diffs through an in-process tenant-aware hub. The production boundary is deliberately narrow: single-node raw-socket Phoenix frames and CDC ingest are exercised, while browser client behavior, WebSocket extension negotiation, CDC tailing, multi-node pubsub, and Kubernetes traffic remain outside this proof.

Implemented surfaces:

- `ws`: RFC 6455 handshake and frame encode/decode.
- `phoenix`: Phoenix v2 array frame encode/decode.
- `hub`: subscriptions, tenant/topic filters, mailbox fan-out, presence state, metrics.
- `live`: WS listener, CDC ingest listener, UDS support, probe responses, fail-closed join validation, masked client-frame enforcement, and bounded text/CDC frame sizes.

Current caveats:

- `run-runtime-canonical` reports `runtime_boundary=single-node-raw-ws-cdc-ingest`, `browser_client_exercised=false`, `cdc_tailing_integrated=false`, `multi_node_pubsub=false`, and `kubernetes_traffic_exercised=false`.
- Clients that require `Sec-WebSocket-Extensions` are rejected; this is raw WebSocket protocol evidence, not browser-facing production readiness.
- Presence `online_at` validation requires a UTC-looking RFC3339 shape ending in `Z`; it is not a full calendar semantic parse.

Verification:

- `cargo test -p ai_blaise_citus_sidecar_realtime`
- `bash ci/ai-blaise/sidecar-realtime-smoke.sh`

Client SDK compatibility beyond the Phoenix wire protocol remains a separate matrix concern.
