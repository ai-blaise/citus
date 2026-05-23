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

The proxy keeps the data plane byte-transparent while the shard-map router and
plan-cache logic mature behind the same binary. Readiness checks connect to the
configured upstream, so Kubernetes does not route traffic to a pool pod that
cannot reach Postgres.
`FEATURE: Sec13` is enforced in the live proxy: clients outside the configured
CIDR allowlist are rejected before an upstream connection is opened, and
`ai_blaise_citus_pool_rejected_connections_total` records those denials.

Current implemented surface:

- `SettingsBucketPoolMap` and opaque settings fingerprints
- `PlacementSubscriber`, `ShardMap`, `PlanCache`, and `PreparedStatementCache`
- `ExtendedPipelineBuffer` for extended-query protocol batching
- `TenantMirrorPolicy` and HTAP `QueryFeatures` classifier
- `AuthVerificationCache` and verified-claim revocation handling
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
traffic, and asserts readiness plus Prometheus counters.
