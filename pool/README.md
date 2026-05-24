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

The proxy keeps the data plane byte-transparent after startup admission; when
Auth3 is enabled it consumes and strips pool-only credential startup parameters
before forwarding the sanitized startup packet upstream. Readiness checks
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

- `SettingsBucketPoolMap` and opaque settings fingerprints
- `PlacementSubscriber`, `ShardMap`, `PlanCache`, and `PreparedStatementCache`
- `ExtendedPipelineBuffer` for extended-query protocol batching
- `TenantMirrorPolicy` and HTAP `QueryFeatures` classifier
- `AuthVerificationCache`, live auth-sidecar introspection, startup-token
  stripping, and verified-claim revocation handling
- `ClosestReplicaTable` GeoIP routing boundary
- `TicketKeyRing` TLS ticket rotation boundary
- `TenantQuotaTable` token-bucket admission
- `VirtualPidTable` cancel-request rewrite support
- `RealtimeHookQueue` CDC-to-realtime framing
- `AdminCommand`/`AdminState` pgcat-style admin command parser
- `PoolRuntimeContract`, `PoolExecutionReport`, `PoolProxyConfig`, and `PoolProxyState`

These types are the first local model for `FEATURE: T2` placement-generation
partial plan-cache invalidation and `FEATURE: T3` single-shard route
selection.
`PoolRuntimeContract` adds validation for `FEATURE: T1`, `FEATURE: T3`,
`FEATURE: T9`, `FEATURE: T12`, `FEATURE: T15`, `FEATURE: R10`,
`FEATURE: Sec12`, `FEATURE: Auth3`, and `FEATURE: MR5`.
`cargo run -p ai_blaise_citus_pool -- run-canonical` emits the deterministic
execution summary for the pool runtime and shard-map contracts used by CI.
`ci/ai-blaise/pool-proxy-smoke.sh` starts PostgreSQL, runs `serve`, sends SQL
through the pool listener, proves CIDR-allowed and CIDR-denied data-port
traffic, exercises active-connection overload, tenant-quota fail-closed denial,
Auth3 startup-token admission against the real auth sidecar, revoked-token
fail-closed denial, upstream-unreachable fail-closed routing, and asserts
readiness plus Prometheus counters.
