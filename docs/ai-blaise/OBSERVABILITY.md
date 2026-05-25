# Observability — Trace Propagation and Structured Logs

This document is the canonical reference for two observability surfaces
introduced in `ai-blaise/citus`:

- **OTEL trace-context propagation** (FEATURE: O14) — how the W3C
  `traceparent` flows from pool → PostgreSQL → companion → sidecars.
- **Per-sidecar structured-log schemas** (FEATURE: O15) — the JSON shape
  every sidecar emits to stdout, and how companion materializes typed SQL
  views on top.

Per-sidecar Grafana dashboards live in the `ai-blaise/command-center` repo
under `platform/citus/observability/dashboards/`.

## Trace propagation

We use the W3C trace-context format from
[`https://www.w3.org/TR/trace-context/`](https://www.w3.org/TR/trace-context/).
A single `traceparent` value is 55 ASCII bytes:

```
00-<32-hex trace-id>-<16-hex span-id>-<2-hex flags>
```

The propagator is `ai_blaise_citus_sidecar_shared::otel`. It exposes a
`TraceContext` trait with `extract` and `inject` operations and three
in-crate carriers: `HeaderMap` (HTTP), `MetadataMap` (gRPC), and
`SetLocalBuilder` (PostgreSQL `SET LOCAL`).

### Pool proxy

The pool proxy reads the first PostgreSQL startup envelope from each client
and inspects three locations for an inbound traceparent, in priority order:

1. A dedicated libpq startup parameter named `traceparent` (PostgreSQL 18+).
2. The libpq `options` parameter when it contains
   `-c trace.parent=<value>` (works on every supported PostgreSQL version
   and is the recommended form for production traffic).
3. The libpq `application_name` parameter using the wire format
   `application=<original>;traceparent=<value>;tracestate=<value>`. This is
   the legacy form for clients that cannot set custom GUCs.

The pool does **not** modify the byte stream. It records the parsed
traceparent in two counters,
`ai_blaise_citus_pool_traceparent_tapped_total` and
`ai_blaise_citus_pool_traceparent_absent_total`, emits a single stderr line
of the form `ai-blaise pool trace_tap=present ... traceparent=<value> ...`,
and forwards every byte to upstream PostgreSQL unchanged. PostgreSQL
processes the `options` parameter natively, setting the runtime GUC
`trace.parent` for the session.

Worked example with `psql`:

```
PGOPTIONS="-c trace.parent=00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01" \
  psql -h pool.host -p 5432 -U app -d app -c \
  "SELECT current_setting('trace.parent', true);"
```

PostgreSQL returns the inbound traceparent verbatim from
`current_setting('trace.parent', true)`. Companion pgrx code reads it back
the same way (see below).

### Companion extension

`companion::trace_context` defines the deterministic plan that companion
pgrx wiring follows on the server side:

| SQL function                                            | Behavior                                                        |
| ------------------------------------------------------- | --------------------------------------------------------------- |
| `companion.current_traceparent()`                       | Returns the active session traceparent, or NULL when absent.    |
| `companion.current_tracestate()`                        | Returns the active session tracestate, or NULL when absent.     |
| `companion.project_traceparent_from_application_name(text)` | Re-parses a raw startup `application_name` string, emits `SET LOCAL trace.parent`. |

The session GUCs we read from are `trace.parent` and `trace.state`. The
production path sets those GUCs through libpq `PGOPTIONS`; PostgreSQL truncates
server-side `application_name`, so the projection helper accepts an explicit raw
startup string when callers need to recover the legacy application-name wire
format. Spans emitted from inside a PostgreSQL backend can chain to the inbound
traceparent via the OpenTelemetry SDK's `WithSpanContext` constructor.

### Sidecars

Sidecar HTTP probe requests (`HttpProbeRequest`) carry a parsed `HeaderMap`
so any axum, hyper, or tonic handler that wraps its inbound request in
`HttpProbeRequest::with_headers` can call `extract_trace_context()` to
recover a `(TraceParent, TraceState)` pair. The shared runtime also exposes
`GET /tracez`, which returns the parsed inbound `traceparent`/`tracestate` for
live probe verification. The same helper is mirrored on `MetadataMap` for gRPC
handlers.

When a sidecar makes outbound calls (HTTP via reqwest, gRPC via tonic, or
PostgreSQL via libpq) it constructs the appropriate carrier, calls
`inject(&parent, &state)`, and lets the carrier serialize the value
according to its native encoding.

### Smoke test

`ci/ai-blaise/otel-trace-propagation-smoke.sh` verifies:

- The traceparent embedded via libpq `options` is recovered by the pool tap.
- PostgreSQL and `companion.current_traceparent()` observe the same
  `trace.parent` value.
- `companion.project_traceparent_from_application_name(text)` projects a raw
  startup string and fails closed for an invalid traceparent.
- The pool's `traceparent_tapped_total` counter increments by one.
- A follow-up connection without a traceparent increments
  `traceparent_absent_total`.
- The live shared sidecar `GET /tracez` endpoint returns the inbound HTTP
  trace headers and reports `valid=false` when they are absent.
- When `REQUIRE_KIND=1`, a kind-cluster scenario boots Jaeger, verifies the
  in-cluster PostgreSQL `trace.parent` GUC path, sends a synthetic OTLP span
  keyed to the accepted trace ID, and asserts that trace is queryable from
  Jaeger. This proves the correlation harness, not automatic pool, companion,
  or sidecar span export.

## Structured-log schemas

Every sidecar emits single-line JSON log records. The canonical schema is
defined in `ai_blaise_citus_sidecar_shared::log_schema` as the deterministic
catalog `canonical_sidecar_log_schemas()`. The catalog has 17 entries — one
per sidecar.

### Common fields

Common fields are emitted by every sidecar. Required fields anchor the JSON
shape; companion's typed SQL view rejects records that lack any required
common field.

| Field        | Type        | Required | Description                                                               |
| ------------ | ----------- | -------- | ------------------------------------------------------------------------- |
| `timestamp`  | timestamptz | yes      | RFC3339 wall-clock timestamp at emission, with timezone offset.           |
| `level`      | text        | yes      | One of `trace`, `debug`, `info`, `warn`, `error`, `critical`.             |
| `sidecar`    | text        | yes      | Component name (e.g. `vectorizer`, `cdc`, `postgrest`).                   |
| `message`    | text        | yes      | Human-readable single-line message.                                       |
| `traceparent`| text        | no       | W3C trace-context traceparent from the active inbound or outbound RPC.    |
| `tenant_id`  | text        | no       | Tenant identifier when the operation is tenant-scoped.                    |
| `request_id` | text        | no       | Application-level request identifier; correlates with the access log.     |
| `version`    | text        | no       | Sidecar binary version (semver), useful when rolling out.                 |
| `error`      | text        | no       | Top-level error message; present only when `level` is `error`/`critical`. |
| `fields`     | jsonb       | no       | Sidecar-specific structured fields (see per-sidecar schema below).        |

### Per-sidecar extensions

Each sidecar may add typed fields under the top-level `fields` JSON object.
The companion log view projects the named fields into typed SQL columns.

- **analytical**: `query_queue_depth` (bigint), `iceberg_snapshot_seconds` (bigint), `mirror_stream` (text).
- **auth**: `issuer` (text), `subject` (text), `denial_reason` (text).
- **backup**: `wal_archive_lag_seconds` (bigint), `last_backup_age_seconds` (bigint), `archive_uri` (text).
- **cdc**: `slot_name` (text), `sink` (text), `lag_seconds` (bigint), `delivered_count` (bigint).
- **coldtier**: `object_count` (bigint), `bytes_demoted` (bigint).
- **edge_functions**: `runtime` (text), `function` (text), `invocation_id` (text), `language_error` (text).
- **graphql**: `operation_name` (text), `operation_kind` (text), `language_error` (text).
- **hlc**: `logical_time` (bigint), `clock_skew_ms` (bigint).
- **mcp**: `tool` (text), `denial_kind` (text).
- **postgrest**: `route` (text), `method` (text), `status_code` (bigint), `language_error` (text).
- **raft**: `term` (bigint), `leader_id` (text), `follower_lag_index` (bigint).
- **realtime**: `topic` (text), `ws_connections` (bigint), `broadcast_fanout` (bigint).
- **repack**: `target` (text), `strategy` (text), `bytes_compacted` (bigint).
- **schema_job**: `job_name` (text), `attempt` (bigint), `dialect` (text).
- **storage**: `bucket` (text), `operation` (text), `object_bytes` (bigint).
- **txn_status**: `xid` (bigint), `commit_state` (text).
- **vectorizer**: `provider` (text), `model` (text), `embedding_count` (bigint), `tokens` (bigint), `cost_usd` (double precision).

### Companion log view

`companion::log_view::render_all_views` renders 17 deterministic
`CREATE OR REPLACE VIEW` statements:

```sql
CREATE OR REPLACE VIEW "companion"."sidecar_vectorizer_log" AS
SELECT
    (line ->> 'timestamp')::timestamptz AS "timestamp",
    line ->> 'level' AS "level",
    line ->> 'sidecar' AS "sidecar",
    line ->> 'message' AS "message",
    line ->> 'traceparent' AS "traceparent",
    line ->> 'tenant_id' AS "tenant_id",
    line ->> 'request_id' AS "request_id",
    line ->> 'version' AS "version",
    line ->> 'error' AS "error",
    line -> 'fields' AS "fields",
    line -> 'fields' ->> 'provider' AS "provider",
    line -> 'fields' ->> 'model' AS "model",
    (line -> 'fields' ->> 'embedding_count')::bigint AS "embedding_count",
    (line -> 'fields' ->> 'tokens')::bigint AS "tokens",
    (line -> 'fields' ->> 'cost_usd')::double precision AS "cost_usd"
FROM companion.sidecar_log_raw
WHERE line ->> 'sidecar' = 'vectorizer';
```

`companion.sidecar_log_raw(line jsonb, captured_at timestamptz)` is the
default ingestion table that Vector and fluent-bit populate from sidecar
stdout. Operators can override the source table with the second positional
argument to `companion::log_view::LogViewPlan::from_schema`.

### Emitting records

Sidecar code constructs a `LogRecord` from `ai_blaise_citus_sidecar_shared::log_schema`,
populating `traceparent` from whatever `TraceContext::extract` returned on
the inbound carrier. The `to_json_line()` method renders the record as
single-line JSON suitable for stdout; it escapes embedded quotes, newlines,
and control characters according to RFC 8259.

## Contract Smoke

`ci/ai-blaise/observability-contracts-check.sh` is the no-Kubernetes contract
smoke for custom component observability. It builds the operator, pool, shared
runtime, and every sidecar, starts each `serve` process on an ephemeral
loopback port, and asserts real JSON `/healthz` and `/readyz` payloads plus
Prometheus `/metrics` exposition. The same smoke starts the pool against a
dummy TCP upstream to validate the admin readiness and metrics labels, then
runs `sidecar-shared log-schema-canonical` to ensure every runtime sidecar has
a structured-log schema row.

`ci/ai-blaise/structured-log-ingestion-smoke.sh` is the PostgreSQL-backed O15
runtime proof. It renders the companion log-view SQL, starts `postgres:17`,
creates `companion.sidecar_log_raw`, ingests all canonical sidecar log records
as `jsonb`, and queries every generated typed view. The smoke intentionally
stops at the database ingestion/view boundary; live Vector/fluent-bit shipping,
Loki persistence, and dashboard correlation are separate deployment concerns.

## Cross-references

- Plan source of truth: `docs/ai-blaise/NEW_FEATURES.md` (`FEATURE: O14`,
  `FEATURE: O15`).
- Library: `ai_blaise_citus_sidecar_shared::{otel, log_schema}`.
- Companion: `ai_blaise_citus_companion::{log_view, trace_context}`.
- Pool tap: `ai_blaise_citus_pool::trace_tap`.
- Smokes: `ci/ai-blaise/otel-trace-propagation-smoke.sh`,
  `ci/ai-blaise/observability-contracts-check.sh`, and
  `ci/ai-blaise/structured-log-ingestion-smoke.sh`.
- Grafana dashboards: `ai-blaise/command-center` repo,
  `platform/citus/observability/dashboards/`.
