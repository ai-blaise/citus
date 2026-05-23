# sidecar/realtime

Production realtime runtime for CDC-backed Phoenix-style channels.

The runtime serves raw WebSocket clients at `/realtime/v1/websocket`, accepts CDC ingest over TCP or `unix:/path.sock`, exposes `/healthz`, `/readyz`, and `/metrics` on the WebSocket listener, and broadcasts `postgres_changes` plus presence diffs through an in-process tenant-aware hub.

Implemented surfaces:

- `ws`: RFC 6455 handshake and frame encode/decode.
- `phoenix`: Phoenix v2 array frame encode/decode.
- `hub`: subscriptions, tenant/topic filters, mailbox fan-out, presence state, metrics.
- `live`: WS listener, CDC ingest listener, UDS support, probe responses.

Verification:

- `cargo test -p ai_blaise_citus_sidecar_realtime`
- `bash ci/ai-blaise/sidecar-realtime-smoke.sh`

Client SDK compatibility beyond the Phoenix wire protocol remains a separate matrix concern.
