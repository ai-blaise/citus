# pool

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Shard-aware pooler. The production `serve` command runs a real PostgreSQL TCP
proxy with a separate admin health port:

- `AI_BLAISE_POOL_LISTEN_ADDR`: client PostgreSQL listener, default
  `0.0.0.0:5432`
- `AI_BLAISE_POOL_ADMIN_ADDR`: HTTP admin listener for `/healthz`, `/readyz`,
  and `/metrics`, default `0.0.0.0:8080`
- `AI_BLAISE_POOL_UPSTREAM_ADDR`: required PostgreSQL upstream target
- `AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST`: optional comma-separated CIDR list
  for PostgreSQL data-port clients; empty means allow all
- `AI_BLAISE_POOL_MAX_ACTIVE_CONNECTIONS`: optional active data-connection
  limit; when full, the pool waits up to `AI_BLAISE_POOL_ADMISSION_TIMEOUT_MS`
  and then rejects without opening an upstream connection
- `AI_BLAISE_POOL_ADMISSION_TIMEOUT_MS`: overload backpressure wait budget,
  default `0` for fail-fast admission
- `AI_BLAISE_POOL_STARTUP_TIMEOUT_MS`: maximum time to wait for a complete
  PostgreSQL startup envelope, default `2000` and minimum `500`
- `AI_BLAISE_POOL_QUOTA_TENANT_ID`, `AI_BLAISE_POOL_QUOTA_BURST`, and
  `AI_BLAISE_POOL_QUOTA_REFILL_PER_SECOND`: optional single-tenant token bucket
  for data-plane connection admission; when enabled, missing, unknown, or
  over-budget tenant startup packets fail closed before upstream routing
- `AI_BLAISE_POOL_AUTH_INTROSPECTION_URL`: optional auth-sidecar
  `/auth/introspect` URL; when set, the pool requires a JWT in `ai_blaise.jwt`,
  `jwt`, `token`, `access_token`, `options`, or `application_name`, validates
  it before opening an upstream socket, and strips pool-only auth parameters
  before forwarding startup bytes to PostgreSQL
- `AI_BLAISE_POOL_AUTH_TIMEOUT_MS`: auth introspection connect/read/write
  budget, default `750`
- `AI_BLAISE_POOL_AUTH_CACHE_TTL_MS`: optional verified-claim cache TTL;
  default `0` revalidates every startup so revocation is observed immediately
- `AI_BLAISE_POOL_AUTH_REQUIRE_TENANT_MATCH`: defaults to true and rejects a
  startup tenant that differs from the token tenant
- `AI_BLAISE_POOL_SETTINGS_BUCKET_GUCS`: optional comma-separated tracked GUC
  list; when set, the live proxy fingerprints startup parameters and libpq
  `options` assignments into settings buckets before opening the upstream
  backend. Any planner- or executor-affecting `citus.*` USERSET GUC belongs
  here; patch 0003 marks them all `GUC_REPORT`, so tracked entries stay
  observable through ParameterStatus. As of upstream 008b391a7 the list worth
  tracking beyond `citus.enable_repartition_joins` includes
  `citus.executor_batch_size`, `citus.executor_chunk_size`,
  `citus.enable_or_clause_arm_pruning`, and
  `citus.allow_unsafe_insert_select_pushdown`
- `AI_BLAISE_POOL_SETTINGS_BUCKET_NAME`: optional settings-bucket namespace,
  default `startup-gucs`
- `AI_BLAISE_POOL_SETTINGS_BUCKET_MAX_CONNECTIONS`: optional per-fingerprint
  accounting limit, default `1024`

The proxy keeps the data plane byte-transparent after startup admission; when
Auth3 is enabled it consumes and strips pool-only credential startup parameters
before forwarding the sanitized startup packet upstream. When settings buckets
are enabled, startup `options` such as
`-c citus.enable_repartition_joins=on` are parsed for fingerprint accounting
before the sanitized startup bytes are forwarded to PostgreSQL. Readiness checks
connect to the configured upstream, so Kubernetes does not route traffic to a
pool pod that cannot reach Postgres.
`FEATURE: Sec13` is enforced in the live proxy: clients outside the configured
CIDR allowlist are rejected before an upstream connection is opened, and
`ai_blaise_citus_pool_rejected_connections_total` records those denials.
`FEATURE: Sec12` is enforced for the narrow pool data-plane quota surface:
tenant IDs are read from the PostgreSQL startup envelope (`application_name`,
`options`, or explicit startup parameter), token-bucket denials return a
PostgreSQL startup error, and overload/quota/upstream fail-closed paths expose
Prometheus counters.

Current implemented surface:

- Live proxy settings-bucket startup parsing, borrow/release metrics, and raw
  PostgreSQL smoke evidence for tracked-GUC backend-state isolation
- `SettingsBucketPoolMap` and opaque settings fingerprints
- `PlacementSubscriber`, `ShardMap`, `PlanCache`, and `PreparedStatementCache`
- `ExtendedPipelineBuffer` and the `pool/wire` PostgreSQL v3 codec
  (`ai_blaise_citus_pool_wire`, ported from jackc/pgx `pgproto3` MIT). The
  production-ready `FEATURE: T7` boundary now covers raw simple-query frame
  pipelining AND typed extended-query `Parse`/`Bind`/`Describe`/`Execute`/
  `Sync`/`Flush` frame parsing on the live `serve` data plane via the new
  `forward_client_to_upstream` function in `pool/src/proxy.rs`. Every client
  -> upstream frame is decoded by the codec, accounted into per-pool atomic
  counters exposed at `/metrics`
  (`ai_blaise_citus_pool_ext_query_frames_total{frame="Parse|Bind|Describe|Execute|Sync|Flush|Close|Query|CopyData|Terminate|Other"}`),
  and forwarded byte-for-byte to the upstream. Shard-aware routing of an
  extended-query pipeline and transaction-aware batching across multiple
  `Sync` boundaries remain alpha-deferred under the same T7 contract.
- `TenantMirrorPolicy` fail-closed parser/report and HTAP `QueryFeatures` parser/classifier
- `AuthVerificationCache`, live auth-sidecar introspection, startup-token
  stripping, and verified-claim revocation handling
- `ClosestReplicaTable` GeoIP routing parser/report boundary
- `TicketKeyRing` TLS ticket rotation report boundary with redacted fingerprints
- `TenantQuotaTable` token-bucket admission
- `VirtualPidTable` cancel-request rewrite support
- `RealtimeHookQueue` CDC-to-realtime framing
- `AdminCommand`/`AdminState` pgcat-style admin command parser
- `PoolRuntimeContract`, `PoolExecutionReport`, `PoolProxyConfig`, and `PoolProxyState`

These types are the first local model for `FEATURE: T2` placement-generation
partial plan-cache invalidation and `FEATURE: T3` single-shard route
selection.
`PoolRuntimeContract` adds validation for `FEATURE: T1`, `FEATURE: T3`,
`FEATURE: T7`, `FEATURE: T9`, `FEATURE: T12`, `FEATURE: T15`, `FEATURE: R10`,
`FEATURE: Sec12`, `FEATURE: Auth3`, and `FEATURE: MR5`.
`cargo run -p ai_blaise_citus_pool -- run-canonical` emits the deterministic
execution summary for the pool runtime and shard-map contracts used by CI.
`ci/ai-blaise/pool-routing-security-smoke.sh` runs that real binary path and
asserts the T9/T12/R10 routing/security evidence columns and alpha MR5
parser/report evidence without claiming
live canary mirroring, managed GeoIP, rustls integration, or analytical query
execution.
`ci/ai-blaise/pool-proxy-smoke.sh` starts PostgreSQL, runs `serve`, sends SQL
through the pool listener, sends two raw PostgreSQL simple-query frames before
reading either result to prove the `FEATURE: T7` data-plane pipelining
boundary, proves CIDR-allowed and CIDR-denied data-port traffic, exercises
active-connection overload, tenant-quota fail-closed denial,
Auth3 startup-token admission against the real auth sidecar, revoked-token
fail-closed denial, upstream-unreachable fail-closed routing, proves tracked
`citus.enable_repartition_joins` startup values do not bleed across simultaneous
backend sessions, and asserts readiness plus Prometheus counters including
settings-bucket unique-fingerprint, borrow, release, and release-error metrics.
