# New Features Register

This is the canonical register of features that `ai-blaise/citus` adds beyond
vanilla Citus. Every feature-bearing PR updates this file.

Status semantics are intentionally conservative: alpha means not
production-ready, not feature-complete, and not eligible for production release
without separate measured evidence and an explicit status promotion. Contract,
model, catalog, SQL-plan, and runbook entries are implementation artifacts, not
proof that the end-to-end user-facing feature is fully integrated.

`ci/ai-blaise/v2-closure-check.sh` and the `v2-closure` workflow codify the
Rule 10 completion contract for the V2 plan: the old 79-item gap list must
remain present in implementation `FEATURE:` markers and this register, stale
completion wording is rejected, overlay crates must keep an executable target,
and the broad operator, companion, pool, and tool canonical runners must emit
their deterministic TSV summaries.
`e2e/src/release_gates.rs`, `ci/ai-blaise/v2-acceptance-check.sh`, and the
`v2-acceptance` workflow codify the 15 continuous release gates from the V2
plan, including the upstream-merge dry-run against `release-14.0`.
`ci/ai-blaise/production-readiness-check.sh` guards the register against
production-readiness overclaiming by verifying source/doc coverage, status
semantics, and the whole-repo audit record. `ci/ai-blaise/production-gap-audit.sh`
adds the stricter production path guard: V2 acceptance models and contract
runners must remain visible as prerequisites, not production evidence for
alpha functionality.

`operator/src/main.rs` and `e2e/src/operator_catalog.rs` are the pure-Rust
acceptance models for the V2 operator catalog. The operator runner validates
canonical `CitusCluster`, `ShardGroup`, `Hypertable`, hypertable apply-plan,
and catalog specs for `FEATURE: S2`, `FEATURE: S4`, `FEATURE: TS7`,
`FEATURE: A8`,
`FEATURE: B2`, `FEATURE: B6`, `FEATURE: C4`, `FEATURE: C5`, `FEATURE: C6`,
`FEATURE: C7`, `FEATURE: C8`, `FEATURE: C9`, `FEATURE: EF3`, `FEATURE: F1`,
`FEATURE: M3`, `FEATURE: MR1`, `FEATURE: MR2`, `FEATURE: MR4`, `FEATURE: MR8`,
`FEATURE: O5`, `FEATURE: R2`, `FEATURE: R7`, `FEATURE: S10`, `FEATURE: S11`,
`FEATURE: Search2`, `FEATURE: Search7`, `FEATURE: TO1`, `FEATURE: TO2`,
`FEATURE: TO5`, and `FEATURE: WH1`, then emits the deterministic TSV summary
with `cargo run -p ai_blaise_citus_operator -- run-canonical`.
`e2e/src/runtime_contracts.rs` validates canonical runtime contracts for
`FEATURE: Auth1`, `FEATURE: Auth3`, `FEATURE: B1`, `FEATURE: B3`,
`FEATURE: B4`, `FEATURE: C1`, `FEATURE: L8`, `FEATURE: MR5`, `FEATURE: R7`,
`FEATURE: R10`, `FEATURE: RT1`, `FEATURE: RT2`, `FEATURE: RT3`,
`FEATURE: RT4`, `FEATURE: Search8`, `FEATURE: Sec12`, `FEATURE: Sto1`,
`FEATURE: Sto3`, `FEATURE: Sto4`, `FEATURE: T1`, `FEATURE: T3`, `FEATURE: T9`,
`FEATURE: T12`, `FEATURE: T15`, and `FEATURE: WH3`.
`pool/src/main.rs` executes a real PostgreSQL TCP proxy in `serve` mode, with
upstream-aware admin readiness on a separate port; `ci/ai-blaise/pool-proxy-smoke.sh`
verifies live SQL, CIDR allow/deny behavior, and pipelined PostgreSQL
simple-query frames through that data port. The binary still emits the
deterministic pool runtime and shard-map summary for `FEATURE: Auth3`,
`FEATURE: MR5`, `FEATURE: R10`, `FEATURE: Sec12`, `FEATURE: T1`,
`FEATURE: T2`, `FEATURE: T3`, `FEATURE: T7`, `FEATURE: T9`, `FEATURE: T12`,
and `FEATURE: T15`.
`images/citus-pg-overlay/extension-manifest.tsv` and
`companion/src/extension_catalog.rs` validate the bundled, optional,
integration-target, and hard-blocked extension contracts for
`FEATURE: Bundle1`, `FEATURE: Search1`, `FEATURE: G1`, `FEATURE: JS1`,
`FEATURE: PM1`, `FEATURE: IA1`, `FEATURE: WF1`, and `FEATURE: F2`; the
companion catalog also emits a deterministic TSV summary through
`companion/src/bin/companion_contracts.rs`.
`images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql` installs the
`FEATURE: Auth2` SQL session-claim helper surface and `ci/ai-blaise/sql-extension-smoke.sh`
proves those helpers against a real `postgres:17` container.
`images/rust-runtime/Dockerfile` and
`scripts/citus-scale/build-app-images.sh` build the deployable Rust operator,
pool, sidecar, and tool images for `FEATURE: D13`; those binaries run the
shared TCP health/readiness/metrics server with `serve` so production
Kubernetes pods do not depend on placeholder responder images.
`ci/ai-blaise/kind-production-smoke.sh` installs those images into kind and
verifies live operator and sidecar `/healthz`, `/readyz`, and `/metrics`
responses from real pods, then verifies live SQL plus pool admin metrics
through the Helm chart, including aggregate pool request counters across
replicas.
`companion/src/advanced_planner.rs` executes a deterministic summary for the
broad V2 planner, tiering, regional, backup, federation, storage, and
research-guard feature contracts through
`companion/src/bin/companion_contracts.rs`.
`companion/src/ops_contracts.rs` executes a deterministic readiness summary
for install, deploy-wrapper, runbook, MCP, security, realtime-client,
io_uring, and protocol-pipeline gates through the same companion binary.
`sidecar/analytical/src/lib.rs` validates pg_lake/DataFusion/DuckDB,
lakehouse-read, Iceberg snapshot commit, federation, DuckDB extension, and
MotherDuck contracts for `FEATURE: L1`, `FEATURE: L2`, `FEATURE: L3`,
`FEATURE: L4`, `FEATURE: L5`, `FEATURE: L6`, `FEATURE: L8`,
`FEATURE: L12`, and `FEATURE: L13`.
`sidecar/analytical/src/lib.rs` also runs a deterministic analytical runtime
for those features, covering mirror materialization counters, lakehouse reads,
DataFusion pushdown shape, Iceberg snapshot commit reporting, federated
catalog publication, DuckDB extension loading, and MotherDuck session
accounting.
`sidecar/cdc/src/lib.rs` validates logical replication stream, DDL capture,
anonymization, reliable delivery, NATS, and Pub/Sub contracts for
`FEATURE: C1`, `FEATURE: C2`, `FEATURE: C3`, `FEATURE: C14`, `FEATURE: C15`,
`FEATURE: L8`, and `FEATURE: WH3`.
`sidecar/cdc/src/lib.rs` also applies canonical wal2json frames, fan-out
delivery plans, and replication ack/checkpoint state for the same CDC feature
surface.
`sidecar/coldtier/src/lib.rs` validates cold-tier layer files, tier movement,
and search-aware index contracts for `FEATURE: R1`, `FEATURE: R5`,
`FEATURE: R9`, and `FEATURE: Search8`.
`sidecar/coldtier/src/lib.rs` also runs a deterministic pageserver-lite
runtime for those features, covering layer object placement, bytes
materialized to object storage, cross-tier planner route refreshes, cold-tier
read accounting, and Tantivy/LanceDB search index publication.
`sidecar/edge_functions/src/lib.rs` validates Deno/Bun runtime launch, UDS
database callback, and triggered invocation contracts for `FEATURE: EF1`,
`FEATURE: EF2`, `FEATURE: EF4`, and `FEATURE: EF5`.
`sidecar/edge_functions/src/lib.rs` also executes a deterministic runtime host
for those features, covering trigger authorization, DB callback timeout bounds,
runtime command materialization, and invocation accounting.
`sidecar/graphql/src/lib.rs` validates pg_graphql endpoint, distributed table,
and RLS/JWT contracts for `FEATURE: API3`, `FEATURE: API4`, and
`FEATURE: API5`.
`sidecar/hlc/src/lib.rs` validates hybrid-logical-clock, closed timestamp, and
follower-read contracts for `FEATURE: S9`.
`sidecar/hlc/src/main.rs` emits the canonical closed-timestamp follower-read
runner for `FEATURE: S9`.
`sidecar/mcp/src/lib.rs` validates MCP service auth, session, safe-mode, and
tenant-scoped tool request policies for `FEATURE: MCP1`, `FEATURE: MCP2`, and
`FEATURE: MCP3`.
`tools/citus-mcp/src/lib.rs` also executes the production-ready read-only MCP
database runtime for `FEATURE: MCP4`, using `AI_BLAISE_MCP_DATABASE_URL`, the
maintained PostgreSQL client with native TLS support, read-only transactions,
statement timeouts, bounded JSON result materialization, tenant schema
validation, and destructive-tool denial.
`sidecar/mcp/src/main.rs` runs the sidecar MCP stdio and HTTP JSON-RPC policy
bridges for `FEATURE: MCP1`, `FEATURE: MCP2`, `FEATURE: MCP3`, and
`FEATURE: D11`.
`sidecar/postgrest/src/lib.rs` validates auto-REST route, distributed view,
RLS, JWT, and OpenAPI contracts for `FEATURE: API1`, `FEATURE: API2`,
`FEATURE: API5`, and `FEATURE: API6`.
`sidecar/raft/src/lib.rs` validates shard-group Raft membership, leader lease,
placement intent, quorum, and failover decisions for `FEATURE: S5`.
`sidecar/raft/src/main.rs` emits the canonical shard-group failover runner for
`FEATURE: S5`.
`sidecar/realtime/src/lib.rs` validates CDC-driven broadcast, tenant isolation,
filter, and presence contracts for `FEATURE: RT1`, `FEATURE: RT2`,
`FEATURE: RT3`, and `FEATURE: RT4`.
`sidecar/realtime/src/lib.rs` also models deterministic realtime runtime
fan-out for those features, covering active connections, filtered subscribers,
frame sizing, delivered message counts, and presence snapshot accounting.
`sidecar/repack/src/lib.rs` validates online repack command planning and
per-shard targets for `FEATURE: R7`.
`sidecar/repack/src/main.rs` emits the canonical online repack command runner
for `FEATURE: R7`.
`sidecar/schema_job/src/lib.rs` validates online-DDL worker leases, backfill,
safety, and gh-ost shadow-table contracts for `FEATURE: C10` and
`FEATURE: M2`.
`sidecar/schema_job/src/main.rs` emits the canonical online-DDL worker runner
for `FEATURE: C10` and `FEATURE: M2`.
`sidecar/storage/src/lib.rs` validates object metadata, presigned URL, bucket
ACL, and antivirus contracts for `FEATURE: Sto1`, `FEATURE: Sto3`,
`FEATURE: Sto4`, and `FEATURE: Sto5`.
`sidecar/storage/src/lib.rs` also runs a deterministic storage flow for those
features: presigned URL issuance, tenant bucket ACL checks, object size
enforcement, metadata persistence, and antivirus quarantine decisions.
`sidecar/txn_status/src/lib.rs` validates parallel-commit transaction status,
intent evidence, and 2PC fallback decisions for `FEATURE: T5`.
`sidecar/txn_status/src/main.rs` emits the canonical parallel-commit status
runner for `FEATURE: T5`.
The tool overlays expose deterministic canonical runners for their library
contracts: `tools/citus-mcp/src/main.rs`, `tools/citus-admin/src/main.rs`,
`tools/citus-schema-designer/src/main.rs`, `tools/citus-tui/src/main.rs`, and
`tools/citus-watch/src/main.rs`.
`ci/ai-blaise/citusctl-smoke.sh` exercises the real `citusctl` binary for the
`FEATURE: D2` plan-id guard.

## Operand Image

### Bundle1: Bundled Extension Image Contract

**Overlay**: `images/citus-pg-overlay`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: see `images/citus-pg-overlay/extension-manifest.tsv`

**Summary**: Defines the operand-image manifest, preload order, and required
extension initialization SQL for the V2 bundled-extension surface.

**Motivation**: The fork needs one machine-checkable contract for always-on,
optional, and hard-blocked extensions before image builds and Helm values can
be safely automated.

**Citus comparison**: Vanilla Citus does not ship an ai-blaise operand image
with TimescaleDB, search, graph, vector, storage, observability, security, and
federation extension policy.

**References**:

- Design: `docs/ai-blaise/BUNDLED_EXTENSIONS.md`
- CI: `ci/ai-blaise/image-check.sh`
- In-source: `FEATURE: Bundle1` in
  `images/citus-pg-overlay/extension-manifest.tsv`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical`

## Throughput

### T1: Settings-Bucket Connection Pool

**Overlay**: `pool/src/runtime.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the pool settings-bucket contract and versioned GUC
fingerprint for sharing worker connections across sessions with identical
tracked GUC state.

**Motivation**: Citus deployments need far more client sessions than worker
backends without losing session correctness.

**Citus comparison**: Vanilla Citus does not ship an external settings-bucket
pooler.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: T1` in `pool/src/runtime.rs`
- Executable: `cargo run -p ai_blaise_citus_pool -- run-canonical`
- Benchmark: `benchmarks/sysbench/run-suite.sh` (TPS / p95 per workload)
- Benchmark: `benchmarks/tpcc/run.sh` (tpmC, p99 latency, error rate)

### T2: Plan Cache Placement-Generation Invalidation

**Overlay**: `pool/src/shard_map.rs`, `companion/src/router_assist.rs`,
`patches/0003-guc-report-citus-userset.patch`,
`patches/0005-placement-generation-counter.patch`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Tracks shard placement generations and cached query fingerprints
so cached plans can be invalidated only when the placements they depend on
change. The Citus quilt patches add the in-process placement-generation
counter (`pg_catalog.citus_placement_generation()`) and tag every
USERSET `citus.*` GUC with `GUC_REPORT` so a transaction pooler sees
planner-affecting `SET` commands through ParameterStatus packets.

**Motivation**: Rebalance should not wipe the entire plan cache when only a
small subset of shard placements moved, and transaction pooling must not
silently inherit stale router/execution GUC state across multiplexed client
sessions.

**Citus comparison**: Vanilla Citus has plan invalidation behavior around shard
movement but does not ship the ai-blaise pool's generation-aware cache model.
Vanilla Citus also does not flag its USERSET GUCs with `GUC_REPORT`, which
makes correct transaction pooling impossible without these patches.

Executable evidence: `cargo test -p ai_blaise_citus_companion --lib
router_assist` runs the placement-generation subscriber contract end to end
(initial/unchanged/advanced/reset transitions, catalog SELECT shape,
sample validation). `cargo test -p ai_blaise_citus_pool --lib shard_map`
runs the pool-side plan-cache generation contract. The C-level counter is
upstream-PR candidate; full Citus-build evidence lands once the
`kind-smoke` overlay rebuilds the operand image with the quilt patches
applied.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: T2` in `pool/src/shard_map.rs`
- In-source: `FEATURE: T2` in `companion/src/router_assist.rs`
- In-source: `FEATURE: T2` in
  `src/backend/distributed/metadata/metadata_cache.c` (via
  `patches/0005-placement-generation-counter.patch`)
- In-source: `FEATURE: T2` in
  `src/backend/distributed/shared_library_init.c` (via
  `patches/0003-guc-report-citus-userset.patch`)
- Executable: `cargo run -p ai_blaise_citus_pool -- run-canonical`
- Executable: `cargo test -p ai_blaise_citus_companion --lib router_assist`
- Executable: `cargo test -p ai_blaise_citus_pool --lib shard_map`
- Patches: `patches/0003-guc-report-citus-userset.patch`,
  `patches/0005-placement-generation-counter.patch`

### T3: Fast-Path Single-Shard Router

**Overlay**: `pool/src/runtime.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Defines the pool routing contract and shard-map route selection
for sending eligible single-shard requests directly to the worker path with a
coordinator fallback.

**Motivation**: Coordinator-less topology needs a pool-level fast path before
query execution patches are wired in.

**Citus comparison**: Vanilla Citus plans single-shard queries but does not
ship this pool routing layer.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: T3` in `pool/src/runtime.rs`
- In-source: `FEATURE: T3` in `pool/src/shard_map.rs`
- In-source: `FEATURE: T3` in `pool/src/proxy.rs`
- Executable: `cargo run -p ai_blaise_citus_pool -- run-canonical`

### T5: Parallel Commit Transaction Status

**Overlay**: `sidecar/txn_status`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines Raft-backed transaction status records with staging
state, shard write intents, replication evidence, and finalize decisions.

**Motivation**: Multi-shard commits need a parallel-commit path that can
commit once every intent has durable replication evidence, while falling back
to classic 2PC when the sidecar path is unavailable or not staged.

**Citus comparison**: Vanilla Citus uses distributed 2PC but does not ship a
parallel-commit transaction-status sidecar.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: T5` in `sidecar/txn_status/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_txn_status -- run-canonical`
- Executable: `patches/postgres/0001-logical-commit-clock.patch` carries the
  PostgreSQL-core logical commit clock the parallel-commit path depends on for
  monotonic shard-finalize ordering. Runtime gate stays alpha until the
  txn_status sidecar lands; the patch is the upstream-quality diff that makes
  the gate compilable. Tracked under FEATURE: PGC1.
- Executable: `patches/postgres/0002-per-subtrans-commit-ts.patch` lets the
  coordinator attribute divergent per-shard commit timestamps inside a single
  umbrella transaction. Tracked under FEATURE: PGC2.

### T8: Toolkit Two-Step Aggregate Pushdown

**Overlay**: `companion/src/toolkit_distributed.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: `timescaledb_toolkit`

**Summary**: Provides an installable SQL Toolkit aggregate plan registry that
renders worker partial and coordinator finalize SQL for two-step aggregate
families.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies
`companion_internal.register_toolkit_aggregate_plan(...)` records
`companion_toolkit_aggregate_plans`, renders worker partial and coordinator
final SQL, and verifies unsupported aggregates fail closed. Actual
TimescaleDB Toolkit aggregate execution, planner hooks, worker pushdown
execution, and distributed result merging remain alpha.

**Motivation**: Toolkit aggregates should execute shard-local partials before
coordinator finalization so time-series rollups do not collapse back to a
single-node plan.

**Citus comparison**: Vanilla Citus can distribute many aggregates, but it
does not ship a Toolkit-specific two-step aggregate bridge.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: T8` in `companion/src/toolkit_distributed.rs`
- SQL runtime: `FEATURE: T8` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### T9: Mirroring For Canary Traffic

**Overlay**: `pool/src/runtime.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds a mirror-traffic policy with target and sample percentage.

**Motivation**: Planner, pool, and sidecar changes need low-risk A/B traffic
before they become default paths.

**Citus comparison**: Vanilla Citus does not mirror query traffic.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: T9` in `pool/src/runtime.rs`
- Executable: `cargo run -p ai_blaise_citus_pool -- run-canonical`

### T12: Pool HTAP Routing

**Overlay**: `pool/src/runtime.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines HTAP routing policy from the pool to the analytical
sidecar with staleness budget and predicate hints.

**Motivation**: Hot/warm/cold query routing needs a single contract before the
pool starts classifying real SQL.

**Citus comparison**: Vanilla Citus does not route HTAP queries to sidecars.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: T12` in `pool/src/runtime.rs`
- Executable: `cargo run -p ai_blaise_citus_pool -- run-canonical`

### T15: Transaction Pipelining In Pool

**Overlay**: `pool/src/runtime.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Proves the pool `serve` data plane preserves pipelined PostgreSQL
simple-query frames as a byte-transparent TCP proxy, while the broader
transaction-batching, shard-aware routing, and `FEATURE: T7` pipeline
contract remain alpha.

**Motivation**: Pool throughput work needs an explicit backpressure contract
and a measured wire-protocol baseline before transaction-level pipelining
reaches shard-aware routing.

**Citus comparison**: Vanilla Citus does not provide an external pool
pipelining contract.

Production evidence: `ci/ai-blaise/pool-proxy-smoke.sh` runs the real pool
against a `postgres:17` container, opens a raw PostgreSQL client through the
pool data port, sends two simple-query frames without waiting for the first
result, verifies ordered `pipeline_one` and `pipeline_two` rows from the real
backend, and keeps the existing live SQL plus pool admin metrics checks. The
Makefile `pool-proxy-smoke` target sets `REQUIRE_DOCKER=1`, and `gate-close`
depends on that target, so missing Docker cannot silently skip this evidence.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: T15` in `pool/src/runtime.rs`
- In-source: `FEATURE: T7` in `pool/src/proxy.rs`
- Executable: `cargo run -p ai_blaise_citus_pool -- run-canonical`
- CI: `ci/ai-blaise/pool-proxy-smoke.sh`
- Live SQL smoke: `ci/ai-blaise/pool-proxy-smoke.sh`
- Gate: `make -f Makefile.ai-blaise gate-close`
- Benchmark: `benchmarks/tpcc/run.sh`, `benchmarks/sysbench/run-suite.sh`
  (V2 gate 10 performance acceptance; alpha until full runs land in
  `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`)

## TimescaleDB Integration

### TS1: Distributed Hypertable Bridge

**Overlay**: `companion/citus_timescale`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: Provides the SQL surface that distributes a PostgreSQL
declarative-partitioned parent table through Citus while using TimescaleDB
hypertables for worker-local partitions. The `apply_distribute_hypertable`
SQL function executes the TimescaleDB and Citus calls when both extensions are
loaded, then records bridge state for operator/readiness inspection.

**Motivation**: Vanilla Citus does not understand TimescaleDB hypertables.
The bridge uses TimescaleDB's partitioned-hypertable seam without forking
TimescaleDB.

**SQL surface / API**:

```sql
SELECT companion.distribute_hypertable(
    'metrics'::regclass,
    dist_col => 'tenant_id',
    chunk_time_interval => INTERVAL '1 day',
    num_shards => 32
);
```

**Citus comparison**: Vanilla Citus can distribute ordinary tables and
partitions, but it has no distributed-hypertable orchestration.

**References**:

- Design: `docs/ai-blaise/COHABITATION.md`
- Acceptance: `e2e/src/timescale_on_citus.rs`
- In-source: `FEATURE: TS1` in `companion/src/citus_timescale.rs`
  and `e2e/src/timescale_on_citus.rs`
- SQL runtime: `FEATURE: TS18` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Benchmark: `benchmarks/timescale-ingest/ingest.py` (rows/s, compression
  ratio, queryable lag; alpha until full runs land)

### TS2: Distributed Compression Policy

**Overlay**: `companion/citus_timescale`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: Adds SQL-plan rendering, SQL apply execution, bridge-state
recording, and a `pg18`-gated pgrx surface for worker-fanned distributed
compression policy creation.

**Motivation**: Distributed hypertables need compression policies that are
declared once and applied consistently across worker-local hypertables.

**Citus comparison**: Vanilla Citus does not fan out TimescaleDB compression
policy setup.

**References**:

- Design: `docs/ai-blaise/COHABITATION.md`
- In-source: `FEATURE: TS2` in `companion/src/citus_timescale.rs`
- SQL runtime: `FEATURE: TS18` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`

### TS3: Distributed Continuous Aggregate Partials

**Overlay**: `companion/citus_timescale`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: Adds SQL-plan rendering, SQL apply execution, bridge-state
recording, and a `pg18`-gated pgrx surface for distributed continuous
aggregate definitions and refresh-policy arguments.

**Motivation**: Continuous aggregates must be coordinated through the same
bridge as distributed hypertables so worker partials and coordinator finals are
created predictably.

**Citus comparison**: Vanilla Citus does not orchestrate TimescaleDB continuous
aggregates across shards.

**References**:

- Design: `docs/ai-blaise/COHABITATION.md`
- In-source: `FEATURE: TS3` in `companion/src/citus_timescale.rs`
- SQL runtime: `FEATURE: TS18` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`

### TS4: Distributed Retention Policy

**Overlay**: `companion/citus_timescale`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: Adds SQL-plan rendering, SQL apply execution, bridge-state
recording, and a `pg18`-gated pgrx surface for cluster-wide retention policy
setup.

**Motivation**: Retention should drop old chunks across all worker-local
hypertables without requiring operator-authored per-worker SQL.

**Citus comparison**: Vanilla Citus does not provide TimescaleDB retention
policy fanout.

**References**:

- Design: `docs/ai-blaise/COHABITATION.md`
- In-source: `FEATURE: TS4` in `companion/src/citus_timescale.rs`
- SQL runtime: `FEATURE: TS18` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`

### TS5: Time-Range Shard Pruner

**Overlay**: `companion/citus_timescale`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: `timescaledb`

**Summary**: Adds planner support that combines Citus shard metadata with
TimescaleDB time dimensions to prune shards for time-bound predicates. The SQL
extension now records enabled pruner state through an executable
`apply_time_range_shard_pruner` surface.

**Motivation**: Distributed hypertables need shard pruning by tenant and time to
avoid scanning irrelevant worker-local hypertable chunks.

**SQL surface / API**:

```sql
SET companion.enable_time_range_shard_pruner = on;
SELECT time_range_shard_pruner('public.metrics', 'ts');
```

**Citus comparison**: Vanilla Citus prunes by distribution metadata, but it does
not consult TimescaleDB dimension slices.

**References**:

- Design: `docs/ai-blaise/COHABITATION.md`
- Acceptance: `e2e/src/timescale_on_citus.rs`
- SQL fallback: `time_range_shard_pruner()` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- In-source: `FEATURE: TS5` in `companion/src/citus_timescale.rs`
  and `e2e/src/timescale_on_citus.rs`
- SQL runtime: `FEATURE: TS18` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`

### TS6: Trusted Hook Coextensions

**Overlay**:

- `patches/0001-allow-trusted-hook-coextensions.patch`
- `patches/0002-preserve-trusted-hook-chain-state.patch`

**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Allows Citus to load after preexisting PostgreSQL hooks when the
operator explicitly configures trusted cohabiting extensions, then preserves
the captured planner, executor, and non-distributed EXPLAIN hook chain. The
TS6 source changes are now integrated into the fork, while the patch files
remain as rebase/reference artifacts for upstream review.

**Motivation**: Citus's upstream guard rejects any preexisting planner,
utility, executor, or explain hook. ai-blaise/citus needs a controlled,
operator-approved path for cohabiting extensions, starting with TimescaleDB.

**SQL surface / API**:

```conf
citus.cohabit_extensions = 'timescaledb'
```

The production allowlist currently recognizes only `timescaledb`; unsupported
names do not satisfy the trust check and Citus keeps its upstream first-hook
guard.

**Citus comparison**: Vanilla Citus errors if these hooks are already set at
load time. With TS6 enabled, ai-blaise/citus remains the outer Citus hook while
delegating to trusted preexisting hooks where the Citus path can safely do so.

Production evidence: `ci/ai-blaise/timescale-cohabitation-smoke.sh` builds a
real `timescale/timescaledb:latest-pg17` image with this Citus fork installed,
starts PostgreSQL with `shared_preload_libraries=timescaledb,citus` and
`citus.cohabit_extensions=timescaledb`, then creates `citus`, `timescaledb`,
and `ai_blaise_citus` in the same server. The VM run in the production audit
records the Git SHA, image identity, and command path, and the smoke is part of
`make -f Makefile.ai-blaise gate-close`.

**References**:

- Design: `docs/ai-blaise/COHABITATION.md`
- Executable: `ci/ai-blaise/timescale-cohabitation-smoke.sh`
- In-source marker after patch application:
  `FEATURE: TS6` in
  `src/backend/distributed/shared_library_init.c`,
  `src/backend/distributed/planner/distributed_planner.c`,
  `src/backend/distributed/executor/multi_executor.c`,
  `src/backend/distributed/planner/multi_explain.c`

### TS7: Hypertable CRD Reconciler

**Overlay**: `operator/src/crds/hypertable.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: Defines the Kubernetes `Hypertable` spec and typed guarded apply
plan that drive distributed hypertable creation, compression, retention,
continuous aggregate, and time-range shard-pruner reconciliation through
ordered companion SQL steps.

**Motivation**: The TimescaleDB bridge needs a declarative operator surface so
cluster state can be reconciled repeatedly instead of hand-applied.

**Citus comparison**: Vanilla Citus does not ship a Kubernetes CRD for
Timescale-aware distributed hypertables.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TS7` in `operator/src/crds/hypertable.rs`
- In-source: `FEATURE: TS7` in `operator/src/reconcile/hypertable.rs`
  (`HypertableApplyPlan` creates `ai_blaise_citus`, checks
  `companion_feature_status()`, validates the configured Timescale/Citus
  cohabitation precondition, then applies ordered companion SQL)
- Acceptance: `FEATURE: TS7` in `e2e/src/timescale_on_citus.rs`
  and canonical SQL emitter `e2e/src/bin/timescale_apply_plan.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### TS8: LSP Rules For Hypertable Invariants

**Overlay**: `tools/citus-lsp`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds edit-time diagnostics for creating Timescale hypertables on
distributed tables without the companion bridge, exposed through the
file-backed `citus-lsp analyze --metadata <metadata.tsv> --sql <migration.sql>`
CLI and the canonical diagnostic emitter.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/citus-lsp-smoke.sh`, which feeds a metadata TSV plus a real SQL
file into `citus-lsp analyze --metadata <metadata.tsv> --sql <migration.sql>`,
verifies the distributed-hypertable invariant diagnostic, then verifies that
`apply_distribute_hypertable(...)` suppresses that warning. Broader JSON-RPC
language-server protocol integration, editor transport, workspace indexing,
automatic file rewrites, and full PostgreSQL grammar coverage remain alpha.

**Motivation**: The required Timescale integration is subtle enough that users
need IDE feedback before invalid SQL reaches a migration or operator reconcile.

**Citus comparison**: Vanilla Citus has no LSP diagnostics for Timescale
hypertable invariants.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TS8` in `companion/src/lsp_metadata.rs`
- In-source: `FEATURE: TS8` in `tools/citus-lsp/src/lib.rs`
- Executable: `FEATURE: TS8` in `tools/citus-lsp/src/main.rs`

### TS9: Doctor Rules For Cohabitation

**Overlay**: `companion/src/db_doctor.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides installable SQL DB-doctor rule registration and violation
reporting for cohabitation and distributed-schema preflight checks.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies `companion_internal.get_violations(...)`
records `companion_db_doctor_rules`, emits missing-schema violations through
`companion_db_doctor_violations`, and verifies unsupported doctor rules fail
closed. Full pglinter rule execution, non-colocated-join SQL analysis,
Timescale catalog inspection, automatic remediation, and operator integration
remain alpha.

**Motivation**: Cohabiting extensions need a SQL-visible preflight and lint
surface so accidental violations are caught before migrations mutate schema.

**Citus comparison**: Vanilla Citus does not ship pglinter-style,
Timescale-aware cohabitation doctor rules.

**References**:

- Design: `docs/ai-blaise/COHABITATION.md`
- In-source: `FEATURE: TS9` in `companion/src/db_doctor.rs`
- SQL runtime: `FEATURE: TS9` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### TS12: Distributed Reorder Policy

**Overlay**: `companion/citus_timescale`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: Adds SQL-plan rendering, SQL apply execution, bridge-state
recording, and a `pg18`-gated pgrx surface for worker-fanned TimescaleDB
reorder policy setup.

**Motivation**: Reorder policies need to target worker-local hypertables while
remaining declarative at the coordinator/operator layer.

**Citus comparison**: Vanilla Citus does not orchestrate TimescaleDB reorder
policies across shards.

**References**:

- Design: `docs/ai-blaise/COHABITATION.md`
- In-source: `FEATURE: TS12` in `companion/src/citus_timescale.rs`
- SQL runtime: `FEATURE: TS18` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`

### TS18: Executable Timescale Bridge State

**Overlay**: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`, `citus`

**Summary**: Adds executable SQL apply functions and durable bridge-state
records for the distributed hypertable, compression, retention, continuous
aggregate, reorder, and time-range-pruner surfaces.

**Motivation**: The bridge must be testable as server-executable SQL instead
of only returning SQL text that references missing internal routines.

**Citus comparison**: Vanilla Citus does not expose a TimescaleDB bridge state
catalog or apply functions for Timescale policy fanout.

Production evidence: `ci/ai-blaise/sql-extension-smoke.sh` installs
`ai_blaise_citus` into a real `postgres:17` container, creates the bridge-state
catalog, exercises public apply entrypoints where plain PostgreSQL can safely
emulate dependency calls, requires durable `companion_timescale_bridge_state`
rows for all six bridge feature ids, and verifies that compression/CAGG apply
paths fail closed when TimescaleDB dependency functions are absent.
`ci/ai-blaise/timescale-bridge-smoke.sh` then installs the same extension into
a real `timescale/timescaledb:latest-pg17` container, stubs only the Citus
distribution entrypoint, and verifies real TimescaleDB hypertable,
compression, retention, reorder, continuous aggregate, and bridge-state
behavior. `ci/ai-blaise/timescale-cohabitation-smoke.sh` closes the previous
stub gap by building this Citus fork into a real TimescaleDB PG17 image,
loading `timescaledb,citus` with `citus.cohabit_extensions=timescaledb`,
creating real `citus`, `timescaledb`, and `ai_blaise_citus` extensions,
requiring real `create_distributed_table` rows in `pg_dist_partition`, and
then executing the TS1/TS2/TS3/TS4/TS5/TS12 apply functions against that live
cohabiting server without defining any Citus stub.

**References**:

- Design: `docs/ai-blaise/COHABITATION.md`
- SQL extension: `FEATURE: TS18` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- CI: `ci/ai-blaise/timescale-bridge-smoke.sh`
- CI: `ci/ai-blaise/timescale-cohabitation-smoke.sh`

### TS13: Distributed time_bucket_gapfill

**Overlay**: `companion/src/toolkit_distributed.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb_toolkit`

**Summary**: Provides an installable SQL gapfill aggregate plan helper that
records worker partial and coordinator `locf(interpolate(...))` finalization
SQL.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies
`companion_internal.register_toolkit_aggregate_plan(...)` records a TS13
`companion_toolkit_aggregate_plans` row for `time_bucket_gapfill`, renders
gapfill/finalizer SQL, and verifies missing `bucket_width` fails closed. Real
TimescaleDB gapfill execution, Toolkit state merging, planner integration,
and distributed query execution remain alpha.

**Motivation**: Time-series dashboards need gapfill across shards without
moving raw samples to the coordinator.

**Citus comparison**: Vanilla Citus does not provide a dedicated distributed
gapfill bridge.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TS13` in `companion/src/toolkit_distributed.rs`
- SQL runtime: `FEATURE: TS13` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### TS14: Distributed Metric Toolkit Aggregates

**Overlay**: `companion/src/toolkit_distributed.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb_toolkit`

**Summary**: Provides installable SQL plan registry support for counter,
gauge, and heartbeat Toolkit aggregate worker partials and coordinator
rollups.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies
`companion_internal.register_toolkit_aggregate_plan(...)` records a TS14
`counter_agg` plan in `companion_toolkit_aggregate_plans`, renders worker
partial SQL, and renders a
`rollup(partial_state)` coordinator finalizer. Real Toolkit metric aggregate
execution, worker/coordinator function availability checks, planner pushdown,
and distributed result merging remain alpha.

**Motivation**: Metric rollups should use Toolkit's partial/final model while
preserving Citus shard locality.

**Citus comparison**: Vanilla Citus does not ship first-class Toolkit metric
aggregate orchestration.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TS14` in `companion/src/toolkit_distributed.rs`
- SQL runtime: `FEATURE: TS14` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### TS15: Distributed Approximate Toolkit Aggregates

**Overlay**: `companion/src/toolkit_distributed.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: `timescaledb_toolkit`

**Summary**: Provides installable SQL plan registry support for percentile and
frequency Toolkit approximate aggregate worker partials and coordinator
rollups.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies
`companion_internal.register_toolkit_aggregate_plan(...)` records TS15
`percentile_agg` plan registration in `companion_toolkit_aggregate_plans` with
deterministic worker/coordinator SQL.
Real Toolkit approximate aggregate execution, sketch merge accuracy,
planner pushdown, and distributed result merging remain alpha.

**Motivation**: Approximate analytics should keep sketches shard-local until
the final coordinator merge.

**Citus comparison**: Vanilla Citus has aggregate pushdown, but not this
Toolkit-specific approximate aggregate catalog.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TS15` in `companion/src/toolkit_distributed.rs`
- SQL runtime: `FEATURE: TS15` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### TS16: Distributed Toolkit Downsamplers

**Overlay**: `companion/src/toolkit_distributed.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb_toolkit`

**Summary**: Provides installable SQL plan registry support for ASAP smoothing
and LTTB downsampler worker partials and coordinator finalizers.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies
`companion_internal.register_toolkit_aggregate_plan(...)` records TS16
`asap_smooth` plan registration in `companion_toolkit_aggregate_plans` and
verifies downsamplers without a `time_column` fail closed. Real Toolkit
downsampler execution, sampling-quality validation, planner pushdown, and
distributed result merging remain alpha.

**Motivation**: Downsampling needs to occur close to shard data before
coordinator rendering.

**Citus comparison**: Vanilla Citus does not provide Toolkit-aware
downsampling orchestration.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TS16` in `companion/src/toolkit_distributed.rs`
- SQL runtime: `FEATURE: TS16` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### TS17: Distributed Toolkit State Aggregates

**Overlay**: `companion/src/toolkit_distributed.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb_toolkit`

**Summary**: Provides installable SQL plan registry support for candlestick,
state, and range Toolkit aggregate worker partials and coordinator rollups.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies
`companion_internal.register_toolkit_aggregate_plan(...)` records TS17
`state_agg` plan registration in `companion_toolkit_aggregate_plans` with
deterministic worker/coordinator SQL.
Real Toolkit state aggregate execution, state/range merge semantics, planner
pushdown, and distributed result merging remain alpha.

**Motivation**: Finance, state-machine, and range analytics need the same
worker-partial/coordinator-final pattern as other Toolkit aggregates.

**Citus comparison**: Vanilla Citus does not bundle this Toolkit aggregate
surface.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TS17` in `companion/src/toolkit_distributed.rs`
- SQL runtime: `FEATURE: TS17` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

## AI / Vector

### A1: pgai-Compatible Vectorizer DSL

**Overlay**: `companion/src/vector.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgvector`, `timescaledb`

**Summary**: Provides an installable SQL vectorizer registry that validates a
pgai-compatible vectorizer definition, creates a shard-local queue table, and
records tenant token usage.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies `companion_internal.register_vectorizer(...)`
renders pgai-compatible `ai.create_vectorizer(...)` SQL, records
`companion_vectorizer_definitions`, creates a queue table, enqueues a document,
records `companion_vectorizer_usage_log`, and verifies missing source columns
and invalid chunk overlap fail closed. Actual pgai worker execution, embedding
provider calls, vector index creation, per-worker scheduling, and operator
reconciliation remain alpha.

**Motivation**: pgai's vectorizer DSL is the right user-facing shape, but its
archived Python worker is not a good runtime floor for this fork.

**Citus comparison**: Vanilla Citus has no AI vectorizer DSL or worker queue.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: A1` in `companion/src/vector.rs`
- SQL runtime: `FEATURE: A1` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### A2: Vectorizer Worker

**Overlay**: `sidecar/vectorizer`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds the vectorizer sidecar's embedding job model, health surface,
and deterministic canonical worker execution path.

**Motivation**: pgai's Python worker is archived and coordinator-oriented. The
fork needs a Rust worker model that can run per Citus worker.

**Citus comparison**: Vanilla Citus does not ship an embedding worker.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: A2` in `sidecar/vectorizer/src/lib.rs`
- Executable: `FEATURE: A2` in `sidecar/vectorizer/src/main.rs`

### A3: Vector Provider Routing

**Overlay**: `sidecar/vectorizer`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines provider/model/secret routing for OpenAI, Azure OpenAI,
Anthropic, Cohere, Voyage, Ollama, and Vertex AI embedding jobs.

**Motivation**: The vectorizer must validate provider routes before spending
tenant budget or dispatching requests.

**Citus comparison**: Vanilla Citus does not route embedding provider calls.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: A3` in `sidecar/vectorizer/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_vectorizer -- run-canonical`

### A4: Per-Tenant Token Budgets

**Overlay**: `sidecar/vectorizer`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds token reservation accounting so vectorization can reject work
that would exceed a tenant's embedding budget.

**Motivation**: Vectorization must be multi-tenant-safe before provider calls
are wired in.

**Citus comparison**: Vanilla Citus has no AI-provider budget accounting.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: A4` in `sidecar/vectorizer/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_vectorizer -- run-canonical`

### A5: Vectorizer Usage Accounting

**Overlay**: `sidecar/vectorizer`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: Adds usage records with tenant, provider, model, token, and
micro-cost accounting, emitted by the canonical vectorizer worker run.

**Motivation**: Cost dashboards and token budgets require a durable accounting
shape before provider calls are enabled for tenant workloads.

**Citus comparison**: Vanilla Citus does not account for embedding provider
usage.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: A5` in `sidecar/vectorizer/src/lib.rs`
- Executable: `FEATURE: A5` in `sidecar/vectorizer/src/main.rs`

### A6: Shard-Local Distributed Vectorize

**Overlay**: `sidecar/vectorizer`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines shard-local vectorizer queue polling and execution plans
so workers can process local shard jobs without coordinator row round trips.

**Motivation**: Distributed vectorization must preserve shard locality and
avoid pushing every embedding job through the coordinator.

**Citus comparison**: Vanilla Citus does not include shard-local embedding
workers.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: A6` in `sidecar/vectorizer/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_vectorizer -- run-canonical`

### A8: Vector Dimension Via CRD

**Overlay**: `operator/src/crds/vectorizer.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgvector`

**Summary**: Defines the `Vectorizer` operator spec for source columns,
embedding provider/model selection, destination vector dimensions, chunking,
scheduling, and secret binding.

**Motivation**: Vectorizer workers need a declarative contract before they can
fan embedding jobs across Citus workers safely.

**Citus comparison**: Vanilla Citus does not ship an AI vectorizer CRD.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: A8` in `operator/src/crds/vectorizer.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

## Topology

### S2: Topology-Aware Placement

**Overlay**: `operator/src/crds/shard_group.rs`, `operator/src/reconcile/shard_group.rs`, `operator/src/reconcile/citus_cluster.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Defines the `ShardGroup` placement policy surface and the
`ShardGroupReconcilePlan` plan-builder that renders the SQL apply plan
(`set_shard_count`, `set_shard_replication_factor`, `create_distributed_table`,
optional `update_distributed_table_colocation`, and a `pg_dist_shard`
post-condition guard) plus Kubernetes-style topology-spread constraints. The
`CitusClusterReconcilePlan` plan-builder renders the CloudNativePG cluster
manifest, pool Deployment intent, and one Deployment intent per declared
sidecar so the operator-owned reconcile contract is executable end-to-end.

Production evidence: Local and VM proof runs `cargo test -p
ai_blaise_citus_operator` (61 unit tests including reconcile-plan coverage for
coordinator-worker, coordinator-less, custom-sidecar, and colocation-free
shard-group cases) and `cargo run -p ai_blaise_citus_operator --
run-reconcile-plans`, which emits the canonical reconcile-plan TSV row
`ai-blaise-citus\t4\t4\ttrue\tfalse\t5\t1\t3\ttrue`. The matching SQL apply
plan and CloudNativePG cluster manifest are produced from the canonical
`CitusClusterSpec` and `ShardGroupSpec` without external Kubernetes
dependencies. Live in-cluster reconciliation (a Kubernetes controller loop
that watches the CRDs, applies the manifests, and updates `.status`) remains
gated behind the alpha `operator.controllerRbac.enabled` profile because the
operator runtime currently exposes only health/readiness/metrics and
plan-builder helpers.

**Motivation**: Placement decisions need an operator-owned policy before the
fork can prove zone-aware replication and survival-goal behavior.

**Citus comparison**: Vanilla Citus tracks placements but does not ship a
Kubernetes-native CRD for topology spread constraints.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: S2` in `operator/src/crds/shard_group.rs`,
  `operator/src/reconcile/shard_group.rs`,
  `operator/src/reconcile/citus_cluster.rs`
- Acceptance: `e2e/src/timescale_on_citus.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcile-plans`
- CI: `cargo test -p ai_blaise_citus_operator`

### S4: Coordinator-Less Topology Mode

**Overlay**: `operator/`, `pool/`, `e2e/`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Allows any node to serve as the entry point for single-shard
queries while multi-shard plans route to a chosen plan leader.

**Motivation**: The classic coordinator is a throughput and availability
bottleneck.

**Citus comparison**: Upstream Citus supports MX metadata on workers but does
not ship ai-blaise's pool/operator topology mode.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: S4` in `operator/src/crds/citus_cluster.rs`
- Acceptance: `e2e/src/timescale_on_citus.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### S5: Raft Per Shard Group

**Overlay**: `sidecar/raft`, `operator/`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Uses a sidecar Raft group per shard group to coordinate placement,
leases, and failover intent.

**Motivation**: The fork needs sub-five-second failover targets without baking
consensus logic into Postgres backends.

**Citus comparison**: Vanilla Citus relies on external PostgreSQL HA tooling.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: S5` in `sidecar/raft/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_raft -- run-canonical`

### S6: Per-Shard Placement Generation

**Overlay**: `companion/src/router_assist.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Provides installable SQL placement-generation helpers and
local-placement checks used by plan-cache invalidation and router fast paths.

**Motivation**: Pool and companion routing need versioned helper APIs before
placement-generation invalidation can move beyond the pool model.

**Citus comparison**: Vanilla Citus tracks shard placements but does not
expose these helper contracts as companion APIs.

Production evidence: `ci/ai-blaise/sql-extension-smoke.sh` installs
`ai_blaise_citus` into a real `postgres:17` container, requires
`companion_feature_status()` to mark `S6` as `sql-runtime`, calls
`companion_internal.bump_placement_generation(102008, 'worker-a')` twice,
verifies generation advancement through `companion_placement_generation(...)`,
verifies unknown shards return generation zero, checks
`companion_local_placement_matches(...)` for matching and non-matching workers,
and verifies shard zero fails closed. This status covers the local SQL
placement-generation state and local-placement helper surface only; actual
Citus metadata synchronization, pool cache invalidation, rebalance hooks,
planner invalidation, and operator-driven placement changes remain alpha.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: S6` in `companion/src/router_assist.rs`
- SQL runtime: `FEATURE: S6` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### S9: Closed-Timestamp Follower Reads

**Overlay**: `sidecar/hlc`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines hybrid logical clock timestamps, closed timestamp plans,
and follower-read safety checks.

**Motivation**: Bounded-staleness reads need a shared clock and closed
timestamp contract before replicas can serve `AS OF` queries.

**Citus comparison**: Vanilla Citus does not provide closed-timestamp follower
reads.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: S9` in `sidecar/hlc/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_hlc -- run-canonical`

### S10: Schema-Based Tenancy

**Overlay**: `operator/src/crds/tenant.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Defines the `Tenant` operator spec for one-tenant-per-schema
layouts on Citus schema-based sharding.

**Motivation**: SaaS tenancy needs a declarative lifecycle boundary before
tenant quotas, region affinity, migration, and archive jobs can reconcile.

**Citus comparison**: Vanilla Citus supports schema-based sharding but does not
ship a Kubernetes tenant CRD.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: S10` in `operator/src/crds/tenant.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### S11: Survival Goals

**Overlay**: `operator/src/crds/survival_goal.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines zone-failure and region-failure survival targets that the
operator can use to validate placement and replication intent.

**Motivation**: Replication factor alone is ambiguous; users need an explicit
failure domain goal for topology-aware reconciliation.

**Citus comparison**: Vanilla Citus does not expose a survival-goal API.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: S11` in `operator/src/crds/survival_goal.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### S13: Range-Based Dynamic Sharding

**Overlay**: `companion/src/router_assist.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Adds installable SQL hash and numeric range routing helpers so
companion and pool code can reason about target shard indexes through one API.

**Motivation**: Dynamic sharding needs a router contract before planner and
operator work can safely mix hash and range distribution.

**Citus comparison**: Vanilla Citus primarily exposes hash distribution
contracts and does not ship this range-routing helper surface.

Production evidence: `ci/ai-blaise/sql-extension-smoke.sh` installs
`ai_blaise_citus` into a real `postgres:17` container, requires
`companion_feature_status()` to mark `S13` as `sql-runtime`, verifies
`companion_hash_shard_index('tenant-a', 8)` is deterministic and bounded,
verifies `companion_range_shard_index(25, 0, 100, 4)` maps to shard index `1`,
and verifies zero-shard and out-of-bounds numeric range inputs fail closed.
This status covers the local SQL hash and numeric range routing helpers only;
actual dynamic shard creation, Citus router integration, operator rebalancing,
pool data-plane routing, and distributed range metadata propagation remain
alpha.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: S13` in `companion/src/router_assist.rs`
- SQL runtime: `FEATURE: S13` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### S14: Tenant Migration Online

**Overlay**: `companion/src/tenants.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Provides installable SQL tenant move and quota helper state for
online tenant migration planning.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies `companion_internal.plan_tenant_move(...)`
records `companion_tenant_moves`, `companion_internal.set_tenant_quota(...)`
records `companion_tenant_quotas`, and verifies same-worker moves and zero
connection quotas fail closed. Actual shard movement, pool draining, tenant
traffic migration, copy/backfill workers, and operator reconciliation remain
alpha.

**Motivation**: Tenant moves must be represented as validated plans before the
operator and companion coordinate online migration.

**Citus comparison**: Vanilla Citus can rebalance shards but does not expose a
tenant-level online migration plan.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: S14` in `companion/src/tenants.rs`
- SQL runtime: `FEATURE: S14` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

## Resource Efficiency

### R1: Cold Tier On Iceberg And Parquet

**Overlay**: `sidecar/coldtier`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_lake`, `pg_parquet`

**Summary**: Defines table-granular image and delta layer files for cold shard
storage on object stores, plus runnable canonical move-plan and runtime
emitters.

**Motivation**: Cold shard data needs a predictable object layout before
operators can evict low-temperature shards from the hot tier.

**Citus comparison**: Vanilla Citus does not provide an S3-backed cold shard
tier.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: R1` in `sidecar/coldtier/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_coldtier -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_coldtier -- run-runtime-canonical`

### R2: Scale-To-Zero Compute

**Overlay**: `operator/src/crds/branch.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds branch-level suspend intent so short-lived compute branches
can scale to zero while retaining their storage declaration.

**Motivation**: Development, analytics, and point-in-time investigation
branches should not burn compute while idle.

**Citus comparison**: Vanilla Citus does not provide branch lifecycle or
scale-to-zero semantics.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: R2` in `operator/src/crds/branch.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### R4: Idle-In-Transaction Detector

**Overlay**: `companion/src/observability.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines a guardrail plan and installable detection-only
`companion_idle_transactions(...)` SQL surface for sessions that sit idle in
transaction beyond a configured limit.

**Motivation**: Distributed transactions can hold locks and snapshots across
workers; stale idle transactions need predictable detection before any
cancel/terminate policy can be promoted.

**Citus comparison**: Vanilla Citus does not ship an idle-transaction detector
helper.

Production evidence: `ci/ai-blaise/sql-extension-smoke.sh` opens a real
PostgreSQL session, leaves it idle inside a transaction, and requires the
installable `companion_idle_transactions('100 milliseconds'::interval)` SQL
surface to detect that live backend from `pg_stat_activity`. The promoted
runtime scope is detection only; it does not cancel or terminate sessions. VM
verification for this promotion reran the smoke against a real `postgres:17`
container.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: R4` in `companion/src/observability.rs`
- SQL extension: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### R5: Hot/Warm/Cold Tier Policy Job

**Overlay**: `sidecar/coldtier`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines temperature-score thresholds and generated shard move
plans between hot, warm, and cold tiers, then accounts for canonical runtime
move execution.

**Motivation**: Tiering policy needs deterministic move plans before an
operator or sidecar starts relocating shard data.

**Citus comparison**: Vanilla Citus does not automate hot/warm/cold shard
movement.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: R5` in `sidecar/coldtier/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_coldtier -- run-runtime-canonical`

### R7: REPACK CONCURRENTLY Adoption

**Overlay**: `operator/src/crds/scheduled_repack.rs`, `sidecar/shared/src/contracts.rs`, `sidecar/repack`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: `pg_repack`

**Summary**: Defines the scheduled repack policy surface for online shard-table
maintenance, with strategy selection for `pg_repack` and the PostgreSQL 19
`REPACK CONCURRENTLY` path modeled by the repack sidecar.

**Motivation**: Repack cadence and target tables need to be auditable and
reconciled rather than run as one-off maintenance commands.

**Citus comparison**: Vanilla Citus can use external maintenance tooling but
does not provide a scheduled repack CRD.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: R7` in `operator/src/crds/scheduled_repack.rs`
- In-source: `FEATURE: R7` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: R7` in `sidecar/repack/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_repack -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### R9: Cross-Tier Query Planner Input

**Overlay**: `sidecar/coldtier`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Exposes cold-tier object URIs and shard table identity for
planner paths that span hot, warm, and cold storage, with runtime planner-route
refresh accounting.

**Motivation**: Cross-tier planning needs machine-readable cold-shard location
and format metadata before the companion planner can combine tiers.

**Citus comparison**: Vanilla Citus does not plan queries across object-store
cold shard layers.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: R9` in `sidecar/coldtier/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_coldtier -- run-runtime-canonical`

### R10: TLS Session Ticket Reuse In Pool

**Overlay**: `pool/src/runtime.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the pool TLS session-ticket reuse and rotation contract.

**Motivation**: Connection churn should not pay full TLS handshakes when
rotation and reuse can be controlled explicitly.

**Citus comparison**: Vanilla Citus does not include an external TLS pooler
contract.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: R10` in `pool/src/runtime.rs`
- Executable: `cargo run -p ai_blaise_citus_pool -- run-canonical`

## Change Data And Branching

### C4: Active-Active Conflict Policy

**Overlay**: `operator/src/crds/conflict_policy.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgactive`

**Summary**: Defines table-scoped conflict policy for active-active
reference-table replication contracts.

**Motivation**: Cross-region writes need explicit resolution rules before
replication can be enabled safely.

**Citus comparison**: Vanilla Citus does not ship active-active conflict
policy objects.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C4` in `operator/src/crds/conflict_policy.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### C5: Replication Conflict Taxonomy

**Overlay**: `operator/src/crds/conflict_policy.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `spock`

**Summary**: Carries the seven conflict classes used by replication-conflict
companion contracts and active-active reconcilers.

**Motivation**: Conflict resolution cannot be observable or testable if all
conflicts collapse into one undifferentiated failure state.

**Citus comparison**: Vanilla Citus does not expose a Spock-style conflict
classification contract.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C5` in `operator/src/crds/conflict_policy.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `patches/postgres/0001-logical-commit-clock.patch` and
  `patches/postgres/0002-per-subtrans-commit-ts.patch` provide the PG-core
  pieces the seven-class conflict resolver needs: monotonic commit timestamps
  to break last-update-wins ties deterministically, and per-subtransaction
  origin attribution so a forced delta apply keeps the remote node id instead
  of the apply worker's. Tracked under FEATURE: PGC1 and FEATURE: PGC2.

### PGC1: PostgreSQL Logical Commit Clock

**Overlay**: `patches/postgres/0001-logical-commit-clock.patch`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds a per-XLogCtl Lamport clock and an XLogReserveInsertHook so
commit timestamps are monotonically increasing in commit-LSN order, with a
per-backend remoteTransactionStopTimestamp that lets logical replication apply
workers bump the local clock forward when a remote transaction carries a
timestamp ahead of the local clock.

**Motivation**: Multi-master and parallel-commit deployments cannot resolve
conflicts deterministically when commit timestamps can move backwards inside a
single node's WAL. The hook closes that gap by running under the WAL-insert
lock so the commit time chosen by the hook is the same time that determines
LSN order. FEATURE: T5 (parallel commit transaction status) and FEATURE: C5
(replication conflict taxonomy) both depend on this clock.

**Citus comparison**: Vanilla PostgreSQL records `xactStopTimestamp` per
backend but does not enforce monotonic increase across the cluster; vanilla
Citus inherits that behaviour. The patch is the canonical pgEdge/Spock
contribution to pgsql-hackers, rebased to PostgreSQL 17.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- Upstream: `docs/ai-blaise/UPSTREAM_SYNC.md` (pgsql-hackers + pgEdge/spock
  links)
- In-source: `FEATURE: PGC1` in
  `patches/postgres/0001-logical-commit-clock.patch`
- In-source: `FEATURE: PGC1 PGC2` in `images/citus-pg-overlay/Dockerfile`
- Executable: `make -f Makefile.ai-blaise patches-check` validates the diff
  format and FEATURE markers. Runtime activation requires the custom-PG-compile
  pipeline; the patch stays alpha-with-placeholder until that ships, where
  alpha means not production-ready.

### PGC2: PostgreSQL Per-Subtransaction Commit Timestamps

**Overlay**: `patches/postgres/0002-per-subtrans-commit-ts.patch`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds `SubTransactionCommitTsEntry` so a single replication or
parallel-commit transaction can record a per-subxid commit time and origin
node id distinct from the umbrella transaction. The override is persisted via
a new `COMMIT_TS_SUBTRANS_TS` (`0x20`) WAL record under the existing
`RM_COMMIT_TS_ID` resource manager and replayed during recovery.

**Motivation**: Spock's delta-apply path forces a row update in a
subtransaction when last-update-wins would otherwise keep the local row; the
forced row must keep the remote commit timestamp and origin so a downstream
resolver can attribute the change correctly. FEATURE: T5 reuses the same
override for shard-level finalize timestamps inside an umbrella commit, and
FEATURE: C5 reuses it to attribute forced updates to the originating node.

**Citus comparison**: Vanilla PostgreSQL keeps one commit timestamp per top
xid; vanilla Citus does not extend that. The patch is the canonical
pgEdge/Spock contribution to pgsql-hackers, rebased to PostgreSQL 17.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- Upstream: `docs/ai-blaise/UPSTREAM_SYNC.md` (pgsql-hackers + pgEdge/spock
  links)
- In-source: `FEATURE: PGC2` in
  `patches/postgres/0002-per-subtrans-commit-ts.patch`
- In-source: `FEATURE: PGC1 PGC2` in `images/citus-pg-overlay/Dockerfile`
- Executable: `make -f Makefile.ai-blaise patches-check` validates the diff
  format and FEATURE markers. Runtime activation requires the custom-PG-compile
  pipeline; the patch stays alpha-with-placeholder until that ships, where
  alpha means not production-ready.

### C6: CSI Snapshot Branching

**Overlay**: `operator/src/crds/branch.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the branch source-cluster, storage, and branch-type
contract needed for snapshot-backed cluster branches.

**Motivation**: Branching needs an operator-owned API before CSI snapshot and
copy-on-write implementations can be reconciled safely.

**Citus comparison**: Vanilla Citus does not ship snapshot branch automation.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C6` in `operator/src/crds/branch.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### C7: Branch Suspend

**Overlay**: `operator/src/crds/branch.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Carries suspend intent on the branch spec so the operator can
coordinate scale-to-zero and resume workflows.

**Motivation**: Branch lifecycle must be declarative to avoid orphaned compute
or ad hoc suspend state.

**Citus comparison**: Vanilla Citus has no branch suspend/resume surface.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C7` in `operator/src/crds/branch.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### C8: Branch Promote

**Overlay**: `operator/src/crds/branch.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Establishes typed branch identity and source-cluster state for
atomic branch promotion workflows.

**Motivation**: Promote/cut-over workflows need the same branch object that
created and suspended the branch, so status and ownership stay consistent.

**Citus comparison**: Vanilla Citus does not provide branch promotion.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C8` in `operator/src/crds/branch.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### C9: Migration Framework

**Overlay**: `operator/src/crds/migration.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the migration CRD surface for pgroll-style and gh-ost
style online DDL workflows.

**Motivation**: Expand/contract schema changes need an operator-visible unit
that can coordinate validation, retries, and conflict handling.

**Citus comparison**: Vanilla Citus does not ship an online-migration CRD.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C9` in `operator/src/crds/migration.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### C10: Online DDL State Machine

**Overlay**: `companion/src/schema_jobs.rs`, `sidecar/schema_job`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides an installable SQL schema-job state machine for
`delete_only`, `write_only`, `backfill`, and `public` transitions with leased
job records.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies `companion_internal.schema_job_start(...)`
records `companion_schema_jobs`, `companion_internal.schema_job_advance(...)`
enforces valid forward transitions, and verifies invalid state transitions and
zero leases fail closed. Actual DDL execution workers, dual-write triggers,
backfill scheduling, lock orchestration, rollback, and operator reconciliation
remain alpha.

**Motivation**: Online schema changes need a validated state model before the
operator and schema-job sidecar can coordinate DDL safely.

**Citus comparison**: Vanilla Citus does not ship an F1-style schema-change
state machine.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C10` in `companion/src/schema_jobs.rs`
- In-source: `FEATURE: C10` in `sidecar/schema_job/src/lib.rs`
- SQL runtime: `FEATURE: C10` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_sidecar_schema_job -- run-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### C1: CDC Sidecar

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/cdc`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines logical-replication slot, publication, sink, and retry
contracts for the CDC sidecar, plus a runnable canonical delivery emitter.

**Motivation**: Realtime, webhooks, analytical mirrors, and external sinks all
need one validated CDC stream contract.

**Citus comparison**: Vanilla Citus does not ship an out-of-process CDC
sidecar.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C1` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: C1` in `sidecar/cdc/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_cdc -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_cdc -- run-runtime-canonical`

### C2: Schema-Aware CDC Sinks

**Overlay**: `sidecar/cdc`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds DDL stream-table and included-schema contracts for CDC sinks
that need schema changes alongside row events.

**Motivation**: Downstream mirrors and queues need a pgstream-style schema
timeline so consumers do not decode WAL against stale table metadata.

**Citus comparison**: Vanilla Citus does not ship schema-aware CDC sink
coordination.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C2` in `sidecar/cdc/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_cdc -- run-runtime-canonical`

### C3: CDC PII Anonymization

**Overlay**: `sidecar/cdc`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `anon`

**Summary**: Defines table/column anonymization rules that CDC delivery plans
must apply before routing events to external sinks.

**Motivation**: CDC frequently leaves the Postgres trust boundary; tagged PII
columns need a first-class redaction contract before external sink delivery.

**Citus comparison**: Vanilla Citus does not apply anonymization policy to
logical replication streams.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C3` in `sidecar/cdc/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_cdc -- run-runtime-canonical`

### C14: CDC NATS Sink

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/cdc`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds validated NATS subject and server URL routing for CDC event
delivery.

**Motivation**: Low-latency event consumers need a NATS route with the same
retry and dead-letter policy as webhook and realtime sinks.

**Citus comparison**: Vanilla Citus does not publish CDC events to NATS.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C14` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: C14` in `sidecar/cdc/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_cdc -- run-runtime-canonical`

### C15: CDC GCP Pub/Sub Sink

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/cdc`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds validated GCP Pub/Sub project and topic routing for CDC
event delivery.

**Motivation**: Managed cloud consumers need a Pub/Sub route without forking
the CDC sidecar delivery model.

**Citus comparison**: Vanilla Citus does not publish CDC events to GCP
Pub/Sub.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C15` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: C15` in `sidecar/cdc/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_cdc -- run-runtime-canonical`

## Migrations

### M1: pgroll-Style Expand-Contract

**Overlay**: `companion/src/migration.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides an installable SQL migration run registry and operation
renderer for expand/contract migrations with bounded lock timeout and backfill
batch settings.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies `companion_internal.migrate_start(...)`,
`companion_internal.migration_add_column(...)`,
`companion_internal.migrate_complete(...)`, and
`companion_migration_operations` record a completed migration with rendered
bounded expand DDL. The smoke verifies operations cannot run without an active
migration. Actual distributed DDL execution, schema-job workers, online
backfill, lock orchestration, rollback execution, and operator CRD
reconciliation remain alpha.

**Motivation**: Type changes, adds, drops, and renames need a reviewed
migration unit before schema-job workers and operator CRDs execute them.

**Citus comparison**: Vanilla Citus supports distributed DDL, but it does not
ship a pgroll-style expand/contract migration layer.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: M1` in `companion/src/migration.rs`
- SQL runtime: `FEATURE: M1` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_schema_job -- run-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- CI: `ci/ai-blaise/schema-job-f1-2vi-smoke.sh` (walks Migration through DELETE_ONLY/WRITE_ONLY/BACKFILL/PUBLIC with checkpointed phase log)

### M2: gh-ost-Style Online DDL

**Overlay**: `companion/src/schema_jobs.rs`, `sidecar/schema_job`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides installable SQL online-DDL operation rendering for
add-column, backfill, swap-column, and drop-column schema job steps.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies
`companion_internal.schema_job_add_operation(...)` records
`companion_schema_job_operations`, renders add-column and backfill SQL, and
`companion_internal.schema_job_render_plan(...)` returns the ordered operation
plan. Actual online DDL execution, trigger dual-writes, backfill workers,
cutover validation, rollback, and distributed-table orchestration remain
alpha.

**Motivation**: Online DDL needs explicit state transitions and lease
validation before a sidecar or companion UDF can execute it.

**Citus comparison**: Vanilla Citus has distributed DDL but does not provide
gh-ost-style online DDL state machinery.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: M2` in `companion/src/schema_jobs.rs`
- In-source: `FEATURE: M2` in `sidecar/schema_job/src/lib.rs`
- SQL runtime: `FEATURE: M2` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### M3: Migration CRD

**Overlay**: `operator/src/crds/migration.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds typed migration declarations with inline YAML DSL and
conflict handling mode.

**Motivation**: Migration runs need to be reviewed and reconciled as desired
state instead of launched imperatively.

**Citus comparison**: Vanilla Citus provides distributed DDL primitives but no
operator-owned migration object.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: M3` in `operator/src/crds/migration.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### M5: LSP Refactor Quick-Fixes

**Overlay**: `tools/citus-lsp`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds typed quick-fix actions for missing Citus distribution
columns and related colocation repairs, exposed through the file-backed
`citus-lsp analyze --metadata <metadata.tsv> --sql <migration.sql>` CLI and
the canonical diagnostic emitter.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/citus-lsp-smoke.sh`, which feeds a metadata TSV plus a real SQL
file into `citus-lsp analyze --metadata <metadata.tsv> --sql <migration.sql>`
and verifies quick-fix action emission for missing distribution columns,
non-colocated joins, missing tenant filters, missing search analyzers, and
distributed hypertable bridge usage. Broader JSON-RPC language-server protocol
integration, editor transport, workspace indexing, automatic file rewrites,
and full PostgreSQL grammar coverage remain alpha.

**Motivation**: Migrations should fail early in the editor with a concrete
fix plan before CI or the operator has to reject a schema change.

**Citus comparison**: Vanilla Citus does not provide IDE quick-fixes for
distributed schema authoring.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: M5` in `companion/src/lsp_metadata.rs`
- In-source: `FEATURE: M5` in `tools/citus-lsp/src/lib.rs`
- Executable: `FEATURE: M5` in `tools/citus-lsp/src/main.rs`

### M7: Pre-Flight Cohabit-Extension Check

**Overlay**: `companion/src/db_doctor.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides installable SQL preflight checks for required
`shared_preload_libraries` entries and trusted cohabiting extension order.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies
`companion_internal.assert_shared_preload_libraries(...)` and
`companion_internal.assert_citus_cohabit_extension_order(...)` accept a
Timescale-before-Citus preload list, reject missing Citus, and verify Citus
loaded before trusted cohabiting extensions fails closed. Runtime hook-chain
inspection, automatic server restart validation, operator remediation, and
multi-extension policy negotiation remain alpha.

**Motivation**: Operator and migration flows must refuse bad preload state
before they install Timescale or other hook-using extension surfaces.

**Citus comparison**: Vanilla Citus enforces its load-time hook guard, but it
does not provide this controlled cohabitation preflight.

**References**:

- Design: `docs/ai-blaise/COHABITATION.md`
- In-source: `FEATURE: M7` in `companion/src/db_doctor.rs`
- SQL runtime: `FEATURE: M7` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### M8: citusctl Plan / Apply

**Overlay**: `tools/citusctl`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the CLI plan/apply execution contract, including
rendered diffs, preflight checks, apply execution, and audit-record steps.

**Motivation**: Operator actions need a Terraform-style preview before
mutating clusters, tenants, branches, migrations, backups, or extension state.

**Citus comparison**: Vanilla Citus does not ship an operator CLI with
two-step plan/apply semantics.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: M8` in `tools/citusctl/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citusctl -- run-canonical`

### M9: Schema Visualization Output

**Overlay**: `tools/citus-schema-designer`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides snapshot-backed schema visualization output for
distribution, hypertable, search-index, webhook, and operator shard-placement
overlays.

Production evidence: The VM proof and `ci/ai-blaise/tools-ui-runtime-smoke.sh`
run the real `ai_blaise_citus_schema_designer` binary against a validated tools
snapshot TSV. The smoke requires `render-svg --snapshot <snapshot.tsv>` to emit
deterministic SVG containing the `D6 M9` feature marker, table overlays, and a
real shard-placement label, and it verifies malformed snapshot references fail
closed. Direct DrawDB front-end embedding, browser collaboration, and live
operator/companion watch streams remain alpha.

**Motivation**: Distributed schema design needs visual output that shows shard
and extension-specific state rather than only ordinary table relationships.

**Citus comparison**: Vanilla Citus does not ship a visual schema designer or
operator shard-map overlay model.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: M9` in `tools/citus-schema-designer/src/lib.rs`
- CI: `ci/ai-blaise/tools-ui-runtime-smoke.sh`
- Executable: `cargo run -p ai_blaise_citus_schema_designer -- run-canonical`

### M11: Online Column-Type Migration

**Overlay**: `companion/src/migration.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides an installable SQL online type-change helper that records
shadow-column DDL for companion migration plans.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies
`companion_internal.migration_online_type_change(...)` records shadow-column
DDL in `companion_migration_operations`. The smoke verifies identical source
and target types fail closed. Actual backfill workers, trigger-based dual
writes, cutover, validation scans, rollback, and distributed table rewrite
orchestration remain alpha.

**Motivation**: Large distributed tables need type migrations that can expand,
backfill, and contract without a long exclusive lock.

**Citus comparison**: Vanilla Citus can run distributed DDL, but it does not
ship an online column-type migration contract.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: M11` in `companion/src/migration.rs`
- SQL runtime: `FEATURE: M11` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_schema_job -- run-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- CI: `ci/ai-blaise/schema-job-f1-2vi-smoke.sh` (simulates mid-BACKFILL worker failure, verifies rollback restores DELETE_ONLY semantics, cleans partial backfill rows)

### M14: F1-Style Two-Version Invariant Controller

**Overlay**: `companion/src/schema_jobs/`, `sidecar/schema_job/src/controller.rs`,
`operator/src/reconcile/migration.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides the F1-style schema-change controller and the SQL
surface that drives Migration CRs through the
`delete_only -> write_only -> backfill -> public` phases while enforcing
the two-version invariant. Adds `companion.schema_job_phase_log`,
`companion.worker_schema_lease`, `companion_internal.schema_job_phase_log_insert`,
`companion_internal.worker_schema_lease_upsert`,
`companion_internal.schema_job_rollback_to`,
`companion_internal.schema_job_cleanup_backfill`, and
`companion_internal.schema_job_drop_added_column`.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/schema-job-f1-2vi-smoke.sh`, which installs `ai_blaise_citus`
into a real PostgreSQL server and walks a Migration through all four
phases, recording one phase-log row per transition, validating worker
lease acknowledgements, simulating a worker failure mid-BACKFILL,
triggering rollback and partial-backfill cleanup, and verifying every
forward-progress phase honors the two-version invariant. Distributed
backfill workers, kube-rs MigrationReconciler client, dual-write triggers,
and live planner-hook enforcement of the WRITE_ONLY/DELETE_ONLY
read/write invariants remain alpha.

**Motivation**: Citus distributes DDL but does not guarantee a bounded
number of in-flight schema versions or coordinate phase transitions
across workers. The F1 controller closes that gap.

**Citus comparison**: Vanilla Citus does not ship an F1-style controller,
phase log, worker lease, or rollback planner.

**References**:

- Design: `docs/ai-blaise/ADR/0008-f1-style-schema-change.md`
- Operator guide: `docs/ai-blaise/MIGRATIONS.md`
- In-source: `FEATURE: M14` in `companion/src/schema_jobs/mod.rs`
- In-source: `FEATURE: M14` in `companion/src/schema_jobs/controller.rs`
- In-source: `FEATURE: M14` in `companion/src/schema_jobs/worker_lease.rs`
- In-source: `FEATURE: M14` in `companion/src/schema_jobs/rollback.rs`
- In-source: `FEATURE: M14` in `sidecar/schema_job/src/controller.rs`
- In-source: `FEATURE: M14` in `operator/src/reconcile/migration.rs`
- SQL runtime: `FEATURE: M14` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_sidecar_schema_job -- run-canonical`
- CI: `ci/ai-blaise/schema-job-f1-2vi-smoke.sh`

### M15: Continuous Two-Version Invariant Verifier

**Overlay**: `companion/src/schema_jobs/mod.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_cron`

**Summary**: Adds `companion_internal.verify_two_version_invariant()`,
`companion.cluster_alarms`, and `companion_two_version_invariant_state`.
Returns a JSON report with the number of in-flight schema versions and the
list of jobs that exceed the limit; raises a critical
`two_version_invariant_violation` alarm row when the invariant is
breached. Designed to be scheduled by pg_cron every 60 seconds.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/schema-job-f1-2vi-smoke.sh`, which provokes a 3-version
violation, calls the verifier, asserts the JSON report records one
violation, and asserts a critical `companion.cluster_alarms` row exists.
The pg_cron schedule and the pager-routed alert wire-up to PagerDuty/Slack
remain alpha.

**Motivation**: F1's two-version invariant is the operational signal that
makes online schema change tractable. A continuous, in-database verifier
catches drift the moment it appears.

**Citus comparison**: Vanilla Citus does not track schema-version drift or
emit invariant alarms.

**References**:

- Design: `docs/ai-blaise/ADR/0008-f1-style-schema-change.md`
- Operator guide: `docs/ai-blaise/MIGRATIONS.md`
- In-source: `FEATURE: M15` in `companion/src/schema_jobs/mod.rs`
- SQL runtime: `FEATURE: M15` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/schema-job-f1-2vi-smoke.sh`

## Multi-Region

### MR1: Region CRD

**Overlay**: `operator/src/crds/region.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines named regions with Kubernetes zone and tablespace mapping.

**Motivation**: Multi-region placement and tenant affinity need stable region
objects rather than repeated stringly typed zone settings.

**Citus comparison**: Vanilla Citus has tablespaces and placements but no
region CRD.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: MR1` in `operator/src/crds/region.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### MR2: SurvivalGoal CRD

**Overlay**: `operator/src/crds/survival_goal.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Declares whether the cluster should survive zone or region
failure and how many replicas must remain available.

**Motivation**: The operator must be able to reject impossible survival goals
before it places shards.

**Citus comparison**: Vanilla Citus does not encode failure-domain objectives.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: MR2` in `operator/src/crds/survival_goal.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### MR4: Tablespaces By Region

**Overlay**: `operator/src/crds/region.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Adds a declarative region-to-tablespace mapping for region-affine
storage placement.

**Motivation**: Tablespaces are the PostgreSQL primitive, but the operator
needs a higher-level region policy to keep placements understandable.

**Citus comparison**: Vanilla Citus can use PostgreSQL tablespaces but does not
manage them as region objects.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: MR4` in `operator/src/crds/region.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### MR5: Pool GeoIP Routing

**Overlay**: `pool/src/runtime.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the pool region-routing contract from CIDR rules to
preferred regions.

**Motivation**: Multi-region reads need a pool-side routing contract before
GeoIP and edge-replica behavior can be enforced.

**Citus comparison**: Vanilla Citus does not provide GeoIP-aware pool routing.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: MR5` in `pool/src/runtime.rs`
- Executable: `cargo run -p ai_blaise_citus_pool -- run-canonical`

### MR8: Leader Pinning Per Region

**Overlay**: `operator/src/crds/region.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Carries leader-pinning intent on regions so HA reconcilers can
constrain primaries to chosen failure domains.

**Motivation**: Multi-region clusters need explicit write-leader placement to
control latency and failover behavior.

**Citus comparison**: Vanilla Citus leaves primary placement to external HA
tooling.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: MR8` in `operator/src/crds/region.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

## Backup / PITR

### B1: Backup Sidecar

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/backup`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines schedule and archive URI contracts for the backup sidecar,
plus runnable canonical backup/PITR emitters for plan and execution state.

**Motivation**: Backup execution needs a sidecar contract that matches the
operator CRD before WAL archive implementation begins.

**Citus comparison**: Vanilla Citus delegates backup sidecars to deployment
tooling.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: B1` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: B1` in `sidecar/backup/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_backup -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_backup -- run-runtime-canonical`

### B2: Backup CRD

**Overlay**: `operator/src/crds/backup.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines backup schedule, retention, object-store target, and
provider consumed by the backup sidecar reconciler and runtime contracts.

**Motivation**: PITR and backup-as-data-source workflows need an auditable
declarative schedule.

**Citus comparison**: Vanilla Citus does not ship a cluster backup CRD.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: B2` in `operator/src/crds/backup.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### B3: PITR Restore

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/backup`, `tools/citusctl`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds point-in-time restore target binding to the backup/restore
sidecar contract.

**Motivation**: PITR restore needs explicit target validation before `citusctl`
and sidecar code execute recovery.

**Citus comparison**: Vanilla Citus does not ship PITR restore orchestration.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: B3` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: B3` in `sidecar/backup/src/lib.rs`
- In-source: `FEATURE: B3` in `tools/citusctl/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_backup -- run-runtime-canonical`

### B4: Backup-As-Data-Source

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/backup`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Carries a queryable branch name for read-only branch creation from
backup archives.

**Motivation**: Time-travel and investigation workflows need backup archives
to become explicit read-only data sources.

**Citus comparison**: Vanilla Citus does not expose backup-as-branch behavior.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: B4` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: B4` in `sidecar/backup/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_backup -- run-runtime-canonical`

### B5: Time-Travel Query Intent

**Overlay**: `tools/citusctl`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds CLI validation for UTC time-travel targets before follower
read and backup-backed query paths execute.

**Motivation**: Time-travel operations need explicit timestamp validation at
the operator entrypoint before sidecars and companion GUCs consume the request.

**Citus comparison**: Vanilla Citus does not ship time-travel orchestration.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: B5` in `tools/citusctl/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citusctl -- run-canonical`

### B6: Encrypted Backups

**Overlay**: `operator/src/crds/backup.rs`, `sidecar/backup`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds a KMS key reference to backup policy so encrypted archives
are part of the reconciled contract.

**Motivation**: Backup encryption must be configured with the schedule, not
attached later by an external script.

**Citus comparison**: Vanilla Citus delegates backup encryption entirely to
deployment-specific tooling.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: B6` in `operator/src/crds/backup.rs`
- In-source: `FEATURE: B6` in `sidecar/backup/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_backup -- run-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

## Tenant Operations

### TO1: Tenant CRD

**Overlay**: `operator/src/crds/tenant.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Introduces the tenant lifecycle object used by tenant migration,
archive, quotas, and region-affinity workflows.

**Motivation**: Tenant operations require a first-class unit of ownership
rather than interpreting arbitrary schema names.

**Citus comparison**: Vanilla Citus does not ship tenant lifecycle objects.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TO1` in `operator/src/crds/tenant.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### TO2: Tenant Quotas

**Overlay**: `operator/src/crds/tenant.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds connection, QPS, and storage quotas to tenant declarations.

**Motivation**: Pool and sidecar enforcement need a typed quota source before
runtime admission control is wired in.

**Citus comparison**: Vanilla Citus has no per-tenant quota CRD.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TO2` in `operator/src/crds/tenant.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### TO3: Tenant Migration Online

**Overlay**: `companion/src/tenants.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Provides installable SQL tenant move planning with source worker,
target worker, optional region affinity, and queued move state.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies `companion_internal.plan_tenant_move(...)`
records `companion_tenant_moves` and verifies same-worker tenant moves fail
closed. Actual shard rebalancing, tenant traffic draining, data copy,
cutover, and operator reconciliation remain alpha.

**Motivation**: Tenant movement needs a typed plan that can be validated before
rebalance, pool draining, and schema routing are coordinated.

**Citus comparison**: Vanilla Citus rebalances shards, but does not expose a
tenant-level online move contract.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TO3` in `companion/src/tenants.rs`
- SQL runtime: `FEATURE: TO3` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### TO4: Tenant Archive

**Overlay**: `companion/src/tenants.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides installable SQL tenant archive planning with destination
URI, retention days, and queued archive state.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies
`companion_internal.plan_tenant_archive(...)` records
`companion_tenant_archives` and verifies zero retention fails closed. Actual
archive export, object-store writes, delete workflows, legal hold, and
operator reconciliation remain alpha.

**Motivation**: Tenant offboarding needs an auditable archive operation before
data removal can be automated.

**Citus comparison**: Vanilla Citus does not include tenant archive
automation.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TO4` in `companion/src/tenants.rs`
- SQL runtime: `FEATURE: TO4` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### TO5: Tenant Region Affinity

**Overlay**: `operator/src/crds/tenant.rs`, `companion/src/tenants.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides installable SQL tenant region-affinity metadata helpers
for placement and migration planning.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies
`companion_internal.set_tenant_region_affinity(...)` records
`companion_tenant_region_affinities` and verifies empty region affinity fails
closed. Actual placement enforcement, shard movement, regional failover
policy, scheduler integration, and operator reconciliation remain alpha.

**Motivation**: Region affinity needs to be part of tenant intent, not hidden
inside one-off placement annotations.

**Citus comparison**: Vanilla Citus does not model tenant-region affinity.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TO5` in `operator/src/crds/tenant.rs`
- In-source: `FEATURE: TO5` in `companion/src/tenants.rs`
- SQL runtime: `FEATURE: TO5` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

## Search

### Search2: Distributed BM25 Index

**Overlay**: `operator/src/crds/search_index.rs`, `companion/src/search_bridge.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_search`

**Summary**: Provides an installable SQL search index registry that validates
table, distribution-column, text-column, and optional vector-column metadata
and renders worker-local full-text index DDL.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies
`companion_internal.register_search_index(...)` records
`companion_search_worker_indexes`, renders deterministic GIN DDL, and verifies
a missing distribution column fails closed. Actual pg_search BM25 index
execution, worker index rollout, distributed DDL application, operator
reconciliation, and shard-aware query fanout remain alpha.

**Motivation**: Search indexes must be declared once and fanned out across
workers without losing table ownership or scorer semantics.

**Citus comparison**: Vanilla Citus does not ship a distributed BM25 search
index CRD.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Search2` in `operator/src/crds/search_index.rs`
- In-source: `FEATURE: Search2` in `companion/src/search_bridge.rs`
- SQL runtime: `FEATURE: Search2` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### Search3: Hybrid BM25 + Vector Ranking

**Overlay**: `companion/src/search_bridge.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_search`, `pgvector`

**Summary**: Provides an installable SQL hybrid ranking helper over the
companion search-document registry, combining PostgreSQL text rank with a
stored vector-score signal.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies `companion_internal.hybrid_rank(...)`
returns ranked rows from `companion_search_documents` and verifies a missing
vector column fails closed. Actual pgvector distance operators, ANN index
selection, model embeddings, and distributed query planning remain alpha.

**Motivation**: Hybrid search needs one coordinator-visible ranking contract
while BM25 and vector indexes remain worker-local.

**Citus comparison**: Vanilla Citus does not ship a hybrid search ranker.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Search3` in `companion/src/search_bridge.rs`
- SQL runtime: `FEATURE: Search3` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### Search7: Search Index CRD

**Overlay**: `operator/src/crds/search_index.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_search`

**Summary**: Adds the Kubernetes-facing `SearchIndex` object for declarative
text and hybrid search indexes.

**Motivation**: Search indexes need lifecycle and validation before companion
SQL and sidecar cold-tier integration can be reconciled.

**Citus comparison**: Vanilla Citus does not provide search-index lifecycle
objects.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Search7` in `operator/src/crds/search_index.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### Search8: Search-Aware Cold Tier

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/coldtier`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds search-index enablement to the analytical mirror contract so
cold-tier data can preserve search semantics, and materializes cold-tier
Tantivy/LanceDB search artifacts in the runtime report.

**Motivation**: Cold-tier movement should not discard full-text or hybrid
search availability.

**Citus comparison**: Vanilla Citus does not manage search-aware cold-tier
mirrors.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Search8` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: Search8` in `sidecar/coldtier/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_coldtier -- run-runtime-canonical`

### Search9: Search Reranker UDF Plan

**Overlay**: `companion/src/search_bridge.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides an installable SQL rerank request registry that records
provider/model intent for a relation of candidate search rows and emits the
deterministic input query for later sidecar execution.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies `companion_internal.rerank_search(...)`
records `companion_search_rerank_requests`, renders deterministic rerank SQL,
and verifies a missing rerank input relation fails closed. LLM/provider calls,
model serving, sidecar rerank execution, and distributed result hydration
remain alpha.

**Motivation**: Reranking should be explicit and auditable before LLM-provider
calls are wired into the search path.

**Citus comparison**: Vanilla Citus does not provide a search reranker UDF.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Search9` in `companion/src/search_bridge.rs`
- SQL runtime: `FEATURE: Search9` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

## HTAP

### L1: pg_lake Analytical Substrate

**Overlay**: `sidecar/analytical`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_lake`

**Summary**: Defines the analytical sidecar plan that binds a logical mirror
to a lakehouse read path, plus runnable canonical execution-plan and runtime
emitters.

**Motivation**: HTAP routing needs a concrete sidecar contract before pg_lake
or equivalent execution is wired into queries.

**Citus comparison**: Vanilla Citus does not ship a pg_lake-backed analytical
sidecar.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L1` in `sidecar/analytical/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_analytical -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_analytical -- run-runtime-canonical`

### L2: Rust Analytical Server

**Overlay**: `sidecar/analytical`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines Rust-native analytical engine selection for DataFusion,
DuckDB, or pg_lake-backed execution, plus deterministic runtime accounting.

**Motivation**: The analytical path should avoid a Python server in the hot
query path.

**Citus comparison**: Vanilla Citus does not include an out-of-process Rust
analytical server.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L2` in `sidecar/analytical/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_analytical -- run-runtime-canonical`

### L3: Iceberg, Parquet, and Delta Reads

**Overlay**: `sidecar/analytical`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_lake`, `pg_parquet`

**Summary**: Defines the lakehouse read plan for Iceberg, Parquet, and Delta
objects, then executes the canonical lakehouse read report.

**Motivation**: Warm and cold analytical storage needs one validated format and
object-URI contract before execution engines fan out reads.

**Citus comparison**: Vanilla Citus does not read Iceberg, Parquet, or Delta
tables through a sidecar.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L3` in `sidecar/analytical/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_analytical -- run-runtime-canonical`

### L4: DataFusion Pushdown

**Overlay**: `sidecar/analytical`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines projected-column, predicate, and limit pushdown contracts
for DataFusion execution and verifies their runtime shape.

**Motivation**: Analytical execution has to preserve pool and planner
predicate intent instead of scanning full object-store tables.

**Citus comparison**: Vanilla Citus does not push plans into DataFusion.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L4` in `sidecar/analytical/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_analytical -- run-runtime-canonical`

### L5: Iceberg Snapshot Commit At Prepare

**Overlay**: `sidecar/analytical`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines transaction, snapshot, prepare-LSN, and manifest URI
contracts for aligning Iceberg snapshot commits with distributed prepare, plus
canonical runtime commit reporting.

**Motivation**: Warm-tier visibility must line up with Citus distributed
transaction boundaries.

**Citus comparison**: Vanilla Citus has no Iceberg snapshot commit protocol.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L5` in `sidecar/analytical/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_analytical -- run-runtime-canonical`

### L6: Lakehouse Federation Catalogs

**Overlay**: `sidecar/analytical`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines external Iceberg catalog publication targets for
Snowflake, Trino, Spark, and Databricks, and reports canonical publication
counts.

**Motivation**: External analytical readers need a versioned federation contract
without learning Citus shard placement directly.

**Citus comparison**: Vanilla Citus does not publish lakehouse catalogs for
external engines.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L6` in `sidecar/analytical/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_analytical -- run-runtime-canonical`

### L8: Mooncake-Style Logical-Replication Mirror

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/cdc`, `sidecar/analytical`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the analytical mirror contract binding a CDC slot to
mirror name and object-storage URI, then accounts for deterministic mirror
materialization events in the analytical runtime.

**Motivation**: HTAP without dual-write requires a validated mirror stream
before analytical sidecars materialize warm columnar copies.

**Citus comparison**: Vanilla Citus does not ship a logical-replication
analytical mirror.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L8` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: L8` in `sidecar/cdc/src/lib.rs`
- In-source: `FEATURE: L8` in `sidecar/analytical/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_cdc -- run-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_analytical -- run-runtime-canonical`

### L9: Two-Step Aggregates Push To Workers

**Overlay**: `companion/src/toolkit_distributed.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: `timescaledb_toolkit`

**Summary**: Provides installable SQL worker-partial aggregate plan metadata so
Toolkit aggregate plans keep partial states worker-local before coordinator
finalization.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies `companion_internal.register_toolkit_aggregate_plan(...)`
records `companion_toolkit_aggregate_plans`, renders worker partial SQL, and
renders coordinator final SQL for mergeable partial states. Real Citus planner
pushdown, worker-local execution, network reduction measurement, and HTAP pool
routing remain alpha.

**Motivation**: HTAP rollups need to reduce network and coordinator CPU by
finalizing after worker partials.

**Citus comparison**: Vanilla Citus supports aggregate pushdown generally, but
not this explicit Toolkit/HTAP aggregate bridge.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L9` in `companion/src/toolkit_distributed.rs`
- SQL runtime: `FEATURE: L9` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### L12: DuckDB Extension Catalog

**Overlay**: `sidecar/analytical`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_duckdb`

**Summary**: Defines the allow-list of DuckDB extensions that analytical
sidecars may enable and counts canonical runtime extension loads.

**Motivation**: DuckDB extension use needs to be explicit before sidecars load
code from extension repositories.

**Citus comparison**: Vanilla Citus does not manage DuckDB extension catalogs.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L12` in `sidecar/analytical/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_analytical -- run-runtime-canonical`

### L13: MotherDuck Connector

**Overlay**: `sidecar/analytical`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_duckdb`

**Summary**: Defines MotherDuck database and token-secret binding for optional
cloud analytical routing, plus deterministic session accounting.

**Motivation**: MotherDuck connectivity should be an explicit opt-in secret
binding rather than an ambient runtime setting.

**Citus comparison**: Vanilla Citus does not include a MotherDuck connector.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L13` in `sidecar/analytical/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_analytical -- run-runtime-canonical`

## Auto API

### API1: PostgREST Sidecar

**Overlay**: `sidecar/postgrest`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `postgrest`

**Summary**: Defines schemas and REST routes exposed by the PostgREST
sidecar, plus a runnable canonical route emitter.

**Motivation**: Auto-REST needs a validated route surface before the sidecar
starts serving table-backed endpoints.

**Citus comparison**: Vanilla Citus does not ship a PostgREST sidecar.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: API1` in `sidecar/postgrest/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_postgrest -- run-canonical`

### API2: Distributed PostgREST Views

**Overlay**: `sidecar/postgrest`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `postgrest`

**Summary**: Binds REST routes to helper views with distribution column and
shard-count metadata.

**Motivation**: Auto-REST over distributed tables needs a versioned view contract
so requests route through Citus-aware helper views.

**Citus comparison**: Vanilla Citus does not generate PostgREST helper views.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: API2` in `sidecar/postgrest/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_postgrest -- run-canonical`

### API3: GraphQL Sidecar

**Overlay**: `sidecar/graphql`, `companion/src/graph_bridge.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_graphql`

**Summary**: Defines the GraphQL endpoint path, schema bindings, and exposed
tables for the GraphQL sidecar, plus a runnable canonical binding emitter.

**Motivation**: GraphQL routing needs a typed endpoint and schema-binding
contract before exposing pg_graphql to tenants.

**Citus comparison**: Vanilla Citus does not ship a GraphQL sidecar.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: API3` in `sidecar/graphql/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_graphql -- run-canonical`

### API4: Distributed GraphQL Tables

**Overlay**: `sidecar/graphql`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_graphql`

**Summary**: Provides installable SQL GraphQL distributed graph metadata that
binds a named graph to already-colocated vertex and edge tables.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies
`companion_internal.register_graphql_distributed_graph(...)` records
`companion_graphql_distributed_graphs` only after graph colocation metadata is
present, and verifies GraphQL graph registration requires graph colocation.
GraphQL server integration, auth policies, GraphQL query planning, and
operator integration remain alpha.

**Motivation**: GraphQL queries over distributed tables need explicit routing
metadata instead of relying on generic single-node table assumptions.

**Citus comparison**: Vanilla Citus does not provide GraphQL routing helpers
for distributed tables.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: API4` in `sidecar/graphql/src/lib.rs`
- In-source: `FEATURE: API4` in `companion/src/graph_bridge.rs`
- SQL runtime: `FEATURE: API4` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### API5: RLS-Aware Auto API

**Overlay**: `sidecar/postgrest`, `sidecar/graphql`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Requires RLS, JWT secret references, and tenant claims for
auto-API routes.

**Motivation**: Auto-generated APIs must preserve tenant isolation rather than
exposing raw distributed tables.

**Citus comparison**: Vanilla Citus does not enforce RLS-aware auto-API
policy.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: API5` in `sidecar/postgrest/src/lib.rs`
- In-source: `FEATURE: API5` in `sidecar/graphql/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_postgrest -- run-canonical`

### API6: Auto OpenAPI Document

**Overlay**: `sidecar/postgrest`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `postgrest`

**Summary**: Defines the OpenAPI path, title, and version served by the
PostgREST sidecar.

**Motivation**: Client generation and API inspection need a predictable
OpenAPI endpoint.

**Citus comparison**: Vanilla Citus does not serve OpenAPI documents.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: API6` in `sidecar/postgrest/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_postgrest -- run-canonical`

## Realtime

### RT1: Realtime Sidecar

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/realtime`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines realtime WebSocket topic contracts fed by CDC events,
plus a runnable canonical broadcast emitter.

**Motivation**: Realtime broadcasts need typed topic and tenant binding before
the WebSocket sidecar is implemented.

**Citus comparison**: Vanilla Citus does not ship realtime WebSocket
broadcasts.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: RT1` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: RT1` in `sidecar/realtime/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_realtime -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_realtime -- run-runtime-canonical`

### RT2: Per-Tenant Topic Isolation

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/realtime`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Requires tenant IDs on realtime subscriptions so topics can be
isolated per tenant.

**Motivation**: Realtime streams must not leak row changes across tenants.

**Citus comparison**: Vanilla Citus does not model realtime topic tenancy.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: RT2` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: RT2` in `sidecar/realtime/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_realtime -- run-runtime-canonical`

### RT3: Realtime Filter Expressions

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/realtime`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Carries server-side realtime filter expressions on subscription
contracts.

**Motivation**: Subscribers need filtered streams without pushing every CDC
event over the socket.

**Citus comparison**: Vanilla Citus does not ship realtime filters.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: RT3` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: RT3` in `sidecar/realtime/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_realtime -- run-runtime-canonical`

### RT4: Presence

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/realtime`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds presence enablement to realtime topic contracts.

**Motivation**: Presence needs to be declared with the channel so the realtime
sidecar can account for subscribers consistently.

**Citus comparison**: Vanilla Citus has no presence-channel surface.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: RT4` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: RT4` in `sidecar/realtime/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_realtime -- run-runtime-canonical`

## Edge Functions

### EF1: Deno Runtime Sidecar

**Overlay**: `sidecar/edge_functions`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `deno`

**Summary**: Defines Deno runtime launch plans for HTTP, scheduled, and
CDC-triggered edge functions, plus a runnable canonical launch emitter.

**Motivation**: Edge functions need a typed runtime contract before the
sidecar starts executing user code.

**Citus comparison**: Vanilla Citus does not ship a Deno edge-function
runtime.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: EF1` in `sidecar/edge_functions/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-runtime-canonical`

### EF2: Bun Runtime Alternative

**Overlay**: `sidecar/edge_functions`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `bun`

**Summary**: Adds Bun runtime launch planning for edge-function bundles.

**Motivation**: Some workloads prefer Bun startup and package compatibility;
the sidecar needs runtime selection without changing the CRD shape.

**Citus comparison**: Vanilla Citus does not ship a Bun edge-function runtime.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: EF2` in `sidecar/edge_functions/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-runtime-canonical`

### EF3: Function CRD

**Overlay**: `operator/src/crds/function.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines edge-function runtime, source, triggers, and secret
bindings for Deno and Bun deployments.

**Motivation**: Function deployment needs to be declarative so auth, pool, and
sidecar runtimes can share the same desired state.

**Citus comparison**: Vanilla Citus does not ship an edge-function CRD.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: EF3` in `operator/src/crds/function.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### EF4: Database Callback Over UDS

**Overlay**: `sidecar/edge_functions`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines Unix-domain-socket callback plans so edge functions can
call back into Postgres with a bounded statement timeout.

**Motivation**: Function runtimes need a local, explicit Postgres callback
contract rather than ad hoc TCP credentials in user code.

**Citus comparison**: Vanilla Citus does not expose an edge-function DB
callback path.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: EF4` in `sidecar/edge_functions/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-runtime-canonical`

### EF5: Triggered Edge Functions

**Overlay**: `sidecar/edge_functions`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines scheduled and CDC-event invocation contracts for edge
functions.

**Motivation**: Cron and event-driven functions need the same validation path
as HTTP functions before queue integration is wired in.

**Citus comparison**: Vanilla Citus does not invoke external edge functions
from schedules or CDC events.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: EF5` in `sidecar/edge_functions/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-runtime-canonical`

## Security / Auth

### Auth1: JWT-Issuing Service

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/auth`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines issuer, signing key reference, token TTL, and tenant claim
contract for the auth sidecar, plus a runnable canonical token-plan emitter.

**Motivation**: SQL helpers and the pool need the same token contract before
the auth sidecar starts issuing JWTs.

**Citus comparison**: Vanilla Citus does not ship a JWT issuer.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Auth1` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: Auth1` in `sidecar/auth/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_auth -- run-canonical`

### Auth3: Token Introspection Cache

**Overlay**: `pool/src/runtime.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines pool-side token introspection cache sizing and TTL.

**Motivation**: Auth verification must be fast enough for pooled connection
paths without repeatedly hitting the auth sidecar.

**Citus comparison**: Vanilla Citus does not include token introspection.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Auth3` in `pool/src/runtime.rs`
- Executable: `cargo run -p ai_blaise_citus_pool -- run-canonical`

### Sec1: RLS Helpers

**Overlay**: `companion/src/auth.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Provides installable SQL tenant RLS helper predicates that map
the active `Auth2` session tenant claim onto table tenant columns.

**Motivation**: Tenant-safe auto-API and pool integration need one validated
mapping from session claims to tenant columns.

**Citus comparison**: Vanilla Citus supports PostgreSQL RLS but does not ship
tenant-aware helper UDFs.

Production evidence: `ci/ai-blaise/sql-extension-smoke.sh` installs
`ai_blaise_citus` into a real `postgres:17` container, requires
`companion_feature_status()` to mark `Sec1` as `sql-runtime`, creates a real
PostgreSQL row-level security policy over `rls_smoke_orders` using
`companion_tenant_id_matches(tenant_id)`, switches into a non-superuser role,
verifies tenant-a and tenant-b sessions each see only their own rows, verifies
`WITH CHECK` rejects a cross-tenant insert, and verifies
`companion_require_tenant_id()` fails closed without a tenant claim. This
status covers the installable predicate helpers only; automatic policy
generation, pool authentication, and auto-API integration remain alpha until
independently proven. Sec2 JWT verification has its own evidence boundary and
does not expand the Sec1 RLS-helper claim.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Sec1` in `companion/src/auth.rs`
- SQL runtime: `FEATURE: Sec1` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### Sec2: JWT Verification UDF

**Overlay**: `companion/src/auth.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgcrypto`

**Summary**: Provides an installable SQL HS256 JWT verifier that returns
Auth2-compatible claims after signature and registered-claim validation.

**Motivation**: Auth sidecars and SQL helpers need the same verified claim
contract to avoid split-brain authorization behavior.

**Citus comparison**: Vanilla Citus does not provide JWT verification helpers.

Production evidence: `ci/ai-blaise/sql-extension-smoke.sh` installs
`pgcrypto` and `ai_blaise_citus` into a real `postgres:17` container, requires
`companion_feature_status()` to mark `Sec2` as `sql-runtime`, constructs a
signed HS256 JWT inside PostgreSQL, verifies it through
`companion_verify_jwt_hs256(...)`, checks issuer, array audience, expiration,
not-before, subject, role, tenant, and JWT ID claims, and feeds the verified
claims into the Auth2 session helper surface. The same smoke verifies bad
signatures, wrong audiences, expired tokens, and missing tenant claims fail
closed. This status covers the local SQL HS256 verifier only; JWKS/RSA/ECDSA
key discovery, Auth1 token issuance, pool authentication, token-cache
behavior, key rotation, and external secret resolution remain alpha.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Sec2` in `companion/src/auth.rs`
- SQL runtime: `FEATURE: Sec2` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### Sec5: Immutable Ledger

**Overlay**: `companion/src/ledger.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgcrypto`

**Summary**: Provides an installable append-only ledger table and transfer
function with SHA-256 hash-chain validation.

**Motivation**: Audit-heavy tenant operations need a tamper-evident record
before automated migrations, tenant moves, and privileged actions execute.

**Citus comparison**: Vanilla Citus does not ship an immutable ledger surface.

Production evidence: `ci/ai-blaise/sql-extension-smoke.sh` installs
`pgcrypto` and `ai_blaise_citus` into a real `postgres:17` container, requires
`companion_feature_status()` to mark `Sec5` as `sql-runtime`, appends two
ledger transfers with `companion_internal.ledger_transfer(...)`, verifies the
second transfer advances the hash chain, verifies `companion_ledger_chain_valid()`,
rejects a transfer with a missing previous hash, and verifies direct
`UPDATE` against `companion_internal.ledger_entries` fails with the
append-only trigger. This status covers the local SQL ledger runtime only;
multi-party accounting workflows, external ledger backends, tenant workflow
authorization, and migration/operator integration remain alpha.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Sec5` in `companion/src/ledger.rs`
- SQL runtime: `FEATURE: Sec5` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### Sec6: HMAC Tamper-Evidence On Ledger

**Overlay**: `companion/src/ledger.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgcrypto`

**Summary**: Provides an installable `companion_ledger_seal` function that
records append-only HMAC seals for ledger transfer hashes.

**Motivation**: Ledger rows need a separable integrity seal so compromised
database writes are detectable against an out-of-band secret.

**Citus comparison**: Vanilla Citus does not provide HMAC-sealed ledger
entries.

Production evidence: `ci/ai-blaise/sql-extension-smoke.sh` installs
`pgcrypto` and `ai_blaise_citus` into a real `postgres:17` container, requires
`companion_feature_status()` to mark `Sec6` as `sql-runtime`, seals a ledger
entry through `companion_ledger_seal('tr_001', 'ledger-secret',
'hmac-sha256')`, verifies the seal is visible through `companion_ledger_entries`,
verifies direct `DELETE` against `companion_internal.ledger_seals` fails with
the append-only trigger, and verifies unsupported HMAC algorithms fail closed.
This status covers the local SQL HMAC sealing runtime only; external secret
resolution, key rotation, hardware-backed signing, and privileged workflow
integration remain alpha.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Sec6` in `companion/src/ledger.rs`
- SQL runtime: `FEATURE: Sec6` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### Sec12: Per-Tenant Resource Quotas

**Overlay**: `pool/src/runtime.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines token-bucket admission policy for tenant-scoped pool
traffic.

**Motivation**: Tenant quotas need pool-side enforcement before noisy tenants
can be isolated reliably.

**Citus comparison**: Vanilla Citus does not enforce per-tenant pool quotas.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Sec12` in `pool/src/runtime.rs`
- Executable: `cargo run -p ai_blaise_citus_pool -- run-canonical`

### Sec13: Pool CIDR Access Control

**Overlay**: `pool/src/proxy.rs`, `ai-blaise/command-center: helm/charts/citus-cluster`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Enforces a comma-separated client CIDR allowlist on the pool
PostgreSQL data port through `AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST`, renders
that allowlist from Helm values, emits rejected-connection metrics, and renders
a matching Kubernetes `NetworkPolicy` for clusters with NetworkPolicy-capable
CNI enforcement.

**Motivation**: Production pool deployments need a fail-closed data-plane
boundary so accidental Service exposure cannot silently accept traffic outside
the intended client networks.

**Citus comparison**: Vanilla Citus does not ship an external pool with
application-level CIDR enforcement or a matching overlay NetworkPolicy.

Production evidence: the pool unit tests verify CIDR parsing, allow decisions,
invalid-prefix rejection, and pre-upstream rejection for denied clients.
`ci/ai-blaise/pool-proxy-smoke.sh` runs the real pool against `postgres:17`,
proves SQL traffic from `127.0.0.0/8` is allowed, restarts with
`192.0.2.0/24`, proves the same client is denied, and requires
`ai_blaise_citus_pool_rejected_connections_total` to record the rejection.
`ci/ai-blaise/kind-production-smoke.sh` renders the Helm allowlist into the
live pool deployment, proves allowed SQL traffic through the Service, upgrades
the release to a deny-only CIDR, proves SQL traffic is blocked in Kubernetes,
triggers application-level rejection through a port-forward to the live pool
data port, and verifies rejected-connection metrics from live pool pods. The
Helm deploy contract also renders `pool-networkpolicy.yaml` for the same
allowlist.

**References**:

- In-source: `FEATURE: Sec13` in `pool/src/proxy.rs`
- Helm: `FEATURE: Sec13` in
  `ai-blaise/command-center: helm/charts/citus-cluster/templates/pool-networkpolicy.yaml`
- Executable: `cargo test -p ai_blaise_citus_pool`
- CI: `ci/ai-blaise/pool-proxy-smoke.sh`
- CI: `ci/ai-blaise/kind-production-smoke.sh`
- Live SQL smoke: `ci/ai-blaise/pool-proxy-smoke.sh`
- Kubernetes smoke: `ci/ai-blaise/kind-production-smoke.sh`
- Gate: `make -f Makefile.ai-blaise gate-close`

### Auth2: Tenant-Aware Claims

**Overlay**: `companion/src/auth.rs`, `sidecar/auth`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides installable SQL session-claim helpers that set and read
`uid`, `role`, `tenant_id`, and optional JWT ID through ai-blaise custom GUCs.

**Motivation**: Pool, sidecar, and SQL helper code need one live claim surface
before JWT verification and token-cache behavior can build on the same names.

**Citus comparison**: Vanilla Citus does not model application tenant claims.

Production evidence: `ci/ai-blaise/sql-extension-smoke.sh` installs
`ai_blaise_citus` into a real `postgres:17` container, requires
`companion_feature_status()` to mark `Auth2` as `sql-runtime`, calls
`companion_set_session_claims('user-123', 'authenticated', 'tenant-a',
'jti-123')`, verifies `companion_current_session_claims()` and
`companion_current_tenant_id()` return the same values, and verifies empty
`uid` claims are rejected. Auth1 JWT issuance and Auth3 token caching remain
alpha until their own runtime evidence exists; Sec2 JWT verification has a
separate SQL-runtime evidence boundary.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Auth2` in `companion/src/auth.rs`
- In-source: `FEATURE: Auth2` in `sidecar/auth/src/lib.rs`
- SQL runtime: `FEATURE: Auth2` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_sidecar_auth -- run-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### Auth4: OAuth2 / OIDC Provider Contracts

**Overlay**: `sidecar/auth`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines OIDC provider configuration with issuer URL, secret refs,
and scopes for external identity integrations.

**Motivation**: Auth sidecars need an auditable provider contract before
Google, GitHub, Apple, Okta, Azure AD, or custom OIDC integrations are wired.

**Citus comparison**: Vanilla Citus does not ship OAuth2/OIDC auth services.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Auth4` in `sidecar/auth/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_auth -- run-canonical`

### Auth5: MFA Policy Contracts

**Overlay**: `sidecar/auth`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds MFA policy validation for TOTP and WebAuthn enablement plus
bounded retry attempts.

**Motivation**: MFA behavior needs a declarative sidecar contract before token
issuance can enforce step-up authentication.

**Citus comparison**: Vanilla Citus does not ship MFA policy management.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Auth5` in `sidecar/auth/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_auth -- run-canonical`

## Plan Management

### PM3: Plan Freeze Companion Module

**Overlay**: `companion/src/plan_freeze.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_hint_plan`, `sr_plan`

**Summary**: Provides an installable SQL plan-freeze registry that stores
query hashes, plan XML, hint-set names, and promotion thresholds.

**Motivation**: Planner changes in a distributed database need an explicit
escape hatch for latency-sensitive tenant queries before a regression reaches
users.

**Citus comparison**: Vanilla Citus does not ship a plan-freeze companion
module or auto-promotion policy.

Production evidence: `ci/ai-blaise/sql-extension-smoke.sh` installs
`ai_blaise_citus` into a real `postgres:17` container, requires
`companion_feature_status()` to mark `PM3` as `sql-runtime`, calls
`companion_internal.plan_freeze('query-hash-1', '<Plan><Node /></Plan>',
'orders_hint')`, attaches promotion thresholds with
`companion_internal.plan_auto_promote(...)`, verifies the frozen plan is
visible through `companion_plan_freezes`, and verifies an empty query hash
fails closed. This status covers the local SQL plan-freeze registry and
promotion-policy state only; actual planner enforcement, hint injection,
pg_hint_plan/sr_plan integration, auto-promotion workers, distributed plan
capture, and plan XML validation remain alpha.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: PM3` in `companion/src/plan_freeze.rs`
- SQL runtime: `FEATURE: PM3` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### PM4: Plan Regression Detection

**Overlay**: `companion/src/plan_freeze.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: `pg_hint_plan`, `sr_plan`

**Summary**: Adds installable SQL latency and cost regression policy
evaluation for frozen-plan candidates.

**Motivation**: Auto-promoted plans need a measurable guardrail that flags
candidate regressions before they replace a known-good plan.

**Citus comparison**: Vanilla Citus exposes plans and costs, but it does not
ship this persistent regression detector.

Production evidence: `ci/ai-blaise/sql-extension-smoke.sh` installs
`ai_blaise_citus` into a real `postgres:17` container, requires
`companion_feature_status()` to mark `PM4` as `sql-runtime`, attaches a
regression policy through `companion_internal.plan_regression_guard(...)`,
verifies `companion_plan_regression_violates(...)` flags a latency regression,
verifies an allowed candidate does not violate policy, verifies regression
samples are recorded, and verifies a missing frozen plan fails closed. This
status covers the local SQL regression-policy evaluator and sample log only;
automatic production-plan replacement, query capture, pg_hint_plan/sr_plan
enforcement, workload baselining, and distributed planner integration remain
alpha.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: PM4` in `companion/src/plan_freeze.rs`
- SQL runtime: `FEATURE: PM4` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

## Index Advisor

### IA3: Companion Advisor

**Overlay**: `companion/src/index_advisor.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `hypopg`, `pg_qualstats`

**Summary**: Provides an installable SQL index-advisor candidate registry and
ranking view that emits `CREATE INDEX CONCURRENTLY` scripts from cost deltas
and predicate counts.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies
`companion_internal.index_advisor_record_candidate(...)` records a ranked
candidate, `companion_index_advisor_ranked(...)` emits `CREATE INDEX
CONCURRENTLY` SQL, and verifies non-improving candidates fail closed. HypoPG and
pg_qualstats workload mining, automatic index creation, distributed index
rollout, and write-amplification governance remain alpha.

**Motivation**: Operators need reviewable index suggestions that rank real
workload benefit before applying changes to distributed tables.

**Citus comparison**: Vanilla Citus does not ship a HypoPG/pg_qualstats-backed
index advisor.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: IA3` in `companion/src/index_advisor.rs`
- SQL runtime: `FEATURE: IA3` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

## Webhooks

### WH1: Webhook CRD

**Overlay**: `operator/src/crds/webhook.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines outbound HTTP trigger declarations with events, URL,
header secret reference, retry policy, and payload template.

**Motivation**: Webhook delivery needs an operator-controlled contract before
CDC and queue sidecars can guarantee retry behavior.

**Citus comparison**: Vanilla Citus does not include webhook lifecycle
management.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: WH1` in `operator/src/crds/webhook.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### WH2: Companion Webhook Helpers

**Overlay**: `companion/src/webhooks.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides installable SQL webhook registration and trigger queue
helpers for `INSERT`, `UPDATE`, and `DELETE` events.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies
`companion_internal.webhook_register(...)`,
`companion_internal.install_webhook_trigger(...)`, and
`companion_webhook_events` register a webhook, install a table trigger, and
verifies INSERT and UPDATE rows are enqueued. The smoke also verifies non-http
webhook URLs fail closed. Outbound HTTP delivery, retry workers,
dead-letter queues, secret resolution, and operator webhook CRDs remain alpha.

**Motivation**: Declarative webhook CRDs need a companion SQL surface that
turns table/event/url configuration into queue-backed triggers.

**Citus comparison**: Vanilla Citus does not install outbound HTTP trigger
helpers.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: WH2` in `companion/src/webhooks.rs`
- SQL runtime: `FEATURE: WH2` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### WH3: Reliable Delivery

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/cdc`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines max-attempt and dead-letter queue policy for CDC-backed
webhook delivery.

**Motivation**: Webhooks need at-least-once retry and dead-letter behavior
before delivery sidecars can be trusted.

**Citus comparison**: Vanilla Citus does not include webhook retry contracts.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: WH3` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: WH3` in `sidecar/cdc/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_cdc -- run-runtime-canonical`

## Storage

### Sto1: Storage Sidecar

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/storage`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines bucket and metadata-table contracts for the storage
sidecar, plus a runnable canonical metadata/presign emitter.

**Motivation**: S3-compatible file storage needs a stable table and bucket
mapping before upload/download paths are implemented.

**Citus comparison**: Vanilla Citus does not ship an object storage sidecar.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Sto1` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: Sto1` in `sidecar/storage/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_storage -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_storage -- run-runtime-canonical`

### Sto3: Presigned URL Signing

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/storage`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines presigned upload URL TTL policy for the storage sidecar.

**Motivation**: Direct uploads need a bounded signing window to keep file
access auditable.

**Citus comparison**: Vanilla Citus does not generate presigned object-store
URLs.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Sto3` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: Sto3` in `sidecar/storage/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_storage -- run-runtime-canonical`

### Sto4: Bucket-Level ACLs

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/storage`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Carries tenant-column ACL binding for object metadata rows.

**Motivation**: Storage ACLs must line up with tenant RLS rather than existing
only in object-store policy.

**Citus comparison**: Vanilla Citus does not manage storage ACLs.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Sto4` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: Sto4` in `sidecar/storage/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_storage -- run-runtime-canonical`

### Sto5: Antivirus Scan Integration

**Overlay**: `sidecar/storage`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds antivirus scanner endpoint and quarantine-bucket validation
for object uploads.

**Motivation**: File attachments need a fail-closed malware scanning contract
before direct uploads are exposed to tenants.

**Citus comparison**: Vanilla Citus does not manage object-store antivirus
policy.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Sto5` in `sidecar/storage/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_storage -- run-runtime-canonical`

## MCP

### MCP1: citus-mcp Server

**Overlay**: `tools/citus-mcp`, `sidecar/mcp`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides the `tools/citus-mcp` line-delimited JSON-RPC stdio
server and the `sidecar/mcp` `serve-stdio` policy bridge for `initialize`,
`tools/list`, and validation-only guarded `tools/call` requests, including
deployed exhaustive-profile sidecar `POST /mcp` traffic.

Executable alpha evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/mcp-stdio-smoke.sh` and
`ci/ai-blaise/mcp-sidecar-stdio-smoke.sh`, which launch
`cargo run -q -p ai_blaise_citus_mcp -- serve-stdio` and
`cargo run -q -p ai_blaise_citus_sidecar_mcp -- serve-stdio`, send real
JSON-RPC stdin requests, verify MCP initialize capabilities, verify the tool
list contains shard/query/rebalance/archive validation tools, validate a
tenant-scoped `query_with_timeout` request, reject a cross-schema
tenant-scoped query, reject a destructive `tenant_archive` call while safe mode
is required, and reject a tenant-scoped query missing tenant scope.
`ci/ai-blaise/mcp-sidecar-http-smoke.sh` also launches
`cargo run -q -p ai_blaise_citus_sidecar_mcp -- serve` and verifies
`GET /readyz`, `GET /metrics`, and HTTP `POST /mcp` JSON-RPC behavior. The
Kubernetes production smoke sends `POST /mcp` through a port-forward to the
deployed exhaustive-profile MCP sidecar pod and verifies the same initialize,
tenant query validation, cross-schema denial, and destructive-denial behavior.
MCP4 covers read-only database execution for `tools/citus-mcp`;
authentication, mutating database execution, Kubernetes tool execution, and
production sidecar enablement remain alpha. Production values keep the MCP
sidecar disabled until the sidecar runtime contract is implemented and
live-gated.

**Motivation**: AI agents need a narrow, typed operation surface rather than
direct database or Kubernetes access.

**Citus comparison**: Vanilla Citus does not ship MCP tooling.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: MCP1` in `tools/citus-mcp/src/lib.rs`
- In-source: `FEATURE: MCP1` in `sidecar/mcp/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_mcp -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_mcp -- run-canonical`
- CI: `ci/ai-blaise/mcp-stdio-smoke.sh`
- CI: `ci/ai-blaise/mcp-sidecar-stdio-smoke.sh`
- CI: `ci/ai-blaise/mcp-sidecar-http-smoke.sh`
- CI: `ci/ai-blaise/kind-production-smoke.sh`

### MCP2: Safe-Mode Tools

**Overlay**: `tools/citus-mcp`, `sidecar/mcp`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds validation-only safe-mode checks that deny destructive MCP
tool requests by default.

Executable alpha evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/mcp-stdio-smoke.sh` and
`ci/ai-blaise/mcp-sidecar-stdio-smoke.sh`, which call the real tool and
sidecar stdio servers through JSON-RPC using `serve-stdio` and verify a
destructive `tenant_archive` tool call returns `isError: true` with the
safe-mode denial message while non-destructive tenant-scoped validation calls
are accepted. `ci/ai-blaise/mcp-sidecar-http-smoke.sh` and
`ci/ai-blaise/kind-production-smoke.sh` verify the same denial through the
sidecar HTTP `serve` path and the deployed Kubernetes sidecar. Disabling safe
mode for mutating production operations remains alpha. MCP4 covers read-only
database execution for `tools/citus-mcp`; authentication, mutating database
execution, Kubernetes tool execution, and production sidecar enablement remain
alpha.

**Motivation**: Agent operations should be inspect-first and dry-run-biased
unless explicitly allowed.

**Citus comparison**: Vanilla Citus does not provide safe-mode agent tooling.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: MCP2` in `tools/citus-mcp/src/lib.rs`
- In-source: `FEATURE: MCP2` in `sidecar/mcp/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_mcp -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_mcp -- run-canonical`
- CI: `ci/ai-blaise/mcp-stdio-smoke.sh`
- CI: `ci/ai-blaise/mcp-sidecar-stdio-smoke.sh`
- CI: `ci/ai-blaise/mcp-sidecar-http-smoke.sh`
- CI: `ci/ai-blaise/kind-production-smoke.sh`

### MCP3: Tenant-Scoped Tools

**Overlay**: `tools/citus-mcp`, `sidecar/mcp`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds tenant scope and allowed-schema validation to MCP tool
requests, including fail-closed rejection for obvious cross-schema SQL/table
references.

Executable alpha evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/mcp-stdio-smoke.sh` and
`ci/ai-blaise/mcp-sidecar-stdio-smoke.sh`, which send real JSON-RPC stdio
`tools/call` requests through the tool and sidecar `serve-stdio` processes
with `tenant_id` and `allowed_schemas`, verify accepted responses include the
tenant scope, verify a tenant-scoped query without tenant scope is rejected,
and verify `tenant_b` SQL is rejected when only `tenant_a` is allowed.
`ci/ai-blaise/mcp-sidecar-http-smoke.sh` and
`ci/ai-blaise/kind-production-smoke.sh` verify the same tenant-scope checks
through the sidecar HTTP `serve` path and the deployed Kubernetes sidecar.
Real database authorization, per-user auth, and sidecar session isolation
remain alpha. MCP4 covers read-only database execution for `tools/citus-mcp`;
authentication, mutating database execution, Kubernetes tool execution, and
production sidecar enablement remain alpha.

**Motivation**: Agent-visible tools must enforce tenant boundaries before
multi-tenant operator usage.

**Citus comparison**: Vanilla Citus has no tenant-scoped AI-agent tool layer.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: MCP3` in `tools/citus-mcp/src/lib.rs`
- In-source: `FEATURE: MCP3` in `sidecar/mcp/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_mcp -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_mcp -- run-canonical`
- CI: `ci/ai-blaise/mcp-stdio-smoke.sh`
- CI: `ci/ai-blaise/mcp-sidecar-stdio-smoke.sh`
- CI: `ci/ai-blaise/mcp-sidecar-http-smoke.sh`
- CI: `ci/ai-blaise/kind-production-smoke.sh`

### MCP4: Read-Only Database Tool Execution

**Overlay**: `tools/citus-mcp`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Executes the read-only MCP database tool subset from the real
`tools/citus-mcp` stdio server when `AI_BLAISE_MCP_DATABASE_URL` is set.

Production evidence: `ci/ai-blaise/mcp-db-smoke.sh` launches a real
`postgres:17` container, creates tenant-scoped data plus a `pg_dist_shard`
catalog fixture, starts `cargo run -q -p ai_blaise_citus_mcp -- serve-stdio`
with `AI_BLAISE_MCP_DATABASE_URL`, and drives JSON-RPC over stdin/stdout. The
smoke proves `query_with_timeout` returns live rows from `tenant_a.orders`,
`run_explain` returns a database-generated plan, `list_shards` reads catalog
rows including shard `102008` for `tenant_a.orders`, a `tenant_b` query is
denied before database execution with `schema tenant_b is outside
allowed_schemas`, and `tenant_archive` remains denied with `safe mode denied a
destructive tool`. The implementation uses the maintained PostgreSQL Rust
client with native TLS support rather than a toy protocol parser, wraps each
execution in `BEGIN READ ONLY`, applies `SET LOCAL statement_timeout`, limits
materialized rows with `AI_BLAISE_MCP_MAX_ROWS` capped at 1000 rows, caps
caller-supplied query timeouts at 300000 ms, rejects `EXPLAIN ANALYZE` so
`run_explain` cannot execute the explained statement, and returns JSON rows
through the MCP text response.

**Current boundary**: This production-ready claim is intentionally narrow:
read-only query, explain, catalog, replication-status, and index-inventory
execution through `tools/citus-mcp`. Authentication, mutating database
execution, Kubernetes tool execution, and production sidecar enablement remain
alpha and must stay disabled until separately implemented and live-gated.

**Motivation**: Agent-visible database reads need real execution evidence
without granting mutation or Kubernetes authority.

**Citus comparison**: Vanilla Citus does not ship MCP database tool execution.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: MCP4` in `tools/citus-mcp/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_mcp -- serve-stdio`
- CI: `ci/ai-blaise/mcp-db-smoke.sh`

## Operations / DX

### D1: citusctl dev up/down

**Overlay**: `tools/citusctl`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds the typed `dev up` and `dev down` command contract for local
cluster lifecycle operations.

**Motivation**: Contributors need a single CLI entrypoint for local end-to-end
clusters before the kind runner and image builder are wired.

**Citus comparison**: Vanilla Citus has development scripts, but not the
ai-blaise single-command local cluster contract.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: D1` in `tools/citusctl/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citusctl -- run-canonical`

### D2: citusctl apply

**Overlay**: `tools/citusctl`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Requires an explicit plan ID before apply-mode CLI execution and
fails closed when `citusctl apply` is invoked without one. This status applies
only to the real CLI parser/guard behavior; mutating cluster apply execution,
manifest reconciliation, migrations, backup restore, PITR, WAL replay, and dev
cluster lifecycle remain alpha until separately live-proven.

**Motivation**: Mutating operations should only run from a reviewed plan so
operator and CI behavior stay auditable.

**Citus comparison**: Vanilla Citus does not ship this plan-gated apply
workflow.

Production evidence: `ci/ai-blaise/citusctl-smoke.sh` runs the real
`ai_blaise_citusctl` binary locally, on the VM, and in the GitHub Actions
`tools` workflow. The smoke requires `citusctl apply` without a plan ID to fail
with `citusctl: plan_id must not be empty`, then verifies `plan inspect
cluster`, `plan apply ...`, and `apply plan-123 apply ...` emit the expected
non-mutating plan summaries and execute-step counts. Broader citusctl dev
cluster lifecycle, full plan/apply execution, migrations, backups, PITR, WAL
replay, and operator mutation workflows remain alpha.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: D2` in `tools/citusctl/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citusctl -- run-canonical`
- CI: `ci/ai-blaise/citusctl-smoke.sh`

### D3: citus-tui Interactive Shell

**Overlay**: `tools/citus-tui`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides a snapshot-backed terminal frame runtime for the
rainfrog-based shell panels with Citus-specific data and guarded operator
action previews.

Production evidence: The VM proof and `ci/ai-blaise/tools-ui-runtime-smoke.sh`
run the real `ai_blaise_citus_tui` binary against a validated tools snapshot
TSV. The smoke requires `render-frame --snapshot <snapshot.tsv> --panel shards`
to render concrete shard placement data, requires safe mode to reject a tenant
move without override, and requires the same action to succeed only with
`--unsafe-allow-mutation --confirm CONFIRM`. The broader interactive ratatui
event loop, direct database sessions, and live mutation execution remain alpha.

**Motivation**: Operators need an interactive terminal workflow that can inspect
cluster topology, shards, hypertables, search indexes, vectorizer backlog,
tenants, and branches while keeping mutating workflows behind explicit safety
gates.

**Citus comparison**: Vanilla Citus does not include an interactive terminal
administration shell.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: D3` in `tools/citus-tui/src/lib.rs`
- CI: `ci/ai-blaise/tools-ui-runtime-smoke.sh`
- Executable: `cargo run -p ai_blaise_citus_tui -- run-canonical`

### D4: citus-lsp IDE Diagnostics

**Overlay**: `tools/citus-lsp`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds the initial Citus-aware LSP analyzer contract for
non-colocated joins, unsafe distribution-column alters, missing tenant filters,
missing search analyzers, and the file-backed
`citus-lsp analyze --metadata <metadata.tsv> --sql <migration.sql>` CLI.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/citus-lsp-smoke.sh`, which feeds a metadata TSV plus a real SQL
file into `citus-lsp analyze --metadata <metadata.tsv> --sql <migration.sql>`,
verifies missing distribution column, non-colocated join,
distribution-column alter, hypertable invariant, missing tenant filter, and
missing search analyzer diagnostics, and verifies fail-closed behavior for bad
metadata or missing metadata. Broader JSON-RPC language-server protocol
integration, editor transport, workspace indexing, automatic file rewrites,
and full PostgreSQL grammar coverage remain alpha.

**Motivation**: Developers need edit-time errors for distributed SQL rules
rather than discovering them during deploy-time reconciliation.

**Citus comparison**: Vanilla Citus does not ship an IDE language server.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: D4` in `companion/src/lsp_metadata.rs`
- In-source: `FEATURE: D4` in `tools/citus-lsp/src/lib.rs`
- Executable: `FEATURE: D4` in `tools/citus-lsp/src/main.rs`

### D5: citus-admin Web UI

**Overlay**: `tools/citus-admin`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides a snapshot-backed HTML route renderer and fail-closed
action validator for the WhoDB-based web administration UI.

Production evidence: The VM proof and `ci/ai-blaise/tools-ui-runtime-smoke.sh`
run the real `ai_blaise_citus_admin` binary against a validated tools snapshot
TSV. The smoke requires `render --snapshot <snapshot.tsv> --route
/cluster/shards` to emit concrete HTML containing shard and worker data,
requires rebalance without `CONFIRM` to fail closed, and requires confirmed
rebalance to emit an accepted dry-run receipt. Full WhoDB front-end embedding,
browser sessions, live database writes, and Kubernetes-side admin deployment
remain alpha.

**Motivation**: Administrators need a browser UI for topology, shard,
Timescale, vectorizer, branch, tenant, backup, and realtime debugging
workflows, with mutating actions requiring exact confirmations.

**Citus comparison**: Vanilla Citus does not ship a web administration UI.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: D5` in `tools/citus-admin/src/lib.rs`
- CI: `ci/ai-blaise/tools-ui-runtime-smoke.sh`
- Executable: `cargo run -p ai_blaise_citus_admin -- run-canonical`

### D6: citus-schema-designer Visual

**Overlay**: `tools/citus-schema-designer`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides the DrawDB-targeted visual schema renderer for Citus
distribution, hypertable, search, webhook, and shard-placement overlays.

Production evidence: The VM proof and `ci/ai-blaise/tools-ui-runtime-smoke.sh`
run the real `ai_blaise_citus_schema_designer` binary against a validated tools
snapshot TSV. The smoke requires deterministic SVG output with the `D6 M9`
feature marker and real shard placement data, and requires invalid snapshot
references to fail closed. Full DrawDB front-end integration, collaborative
editing, and live operator/companion refresh remain alpha.

**Motivation**: Schema designers need a versioned model for distribution,
hypertable, search, webhook, and shard-placement layers before the UI reads
operator CRD or companion state.

**Citus comparison**: Vanilla Citus does not include a visual schema designer.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: D6` in `tools/citus-schema-designer/src/lib.rs`
- CI: `ci/ai-blaise/tools-ui-runtime-smoke.sh`
- Executable: `cargo run -p ai_blaise_citus_schema_designer -- run-canonical`

### D12: citus-watch Dashboard

**Overlay**: `tools/citus-watch`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides a snapshot-backed dashboard frame runtime for the
`citus-watch` operator view.

Production evidence: The VM proof and `ci/ai-blaise/tools-ui-runtime-smoke.sh`
run the real `ai_blaise_citus_watch` binary against a validated tools snapshot
TSV. The smoke requires `render-frame --snapshot <snapshot.tsv>` to emit pool,
vectorizer backlog, shard, tenant, and companion/Prometheus query-plan data.
Live Prometheus scraping, direct companion SQL sessions, and continuous terminal
refresh remain alpha.

**Motivation**: Operators need a single terminal dashboard that can read
companion metadata, Prometheus metrics, and pool signals without hand-built
queries.

**Citus comparison**: Vanilla Citus does not ship a unified TUI dashboard.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: D12` in `tools/citus-watch/src/lib.rs`
- CI: `ci/ai-blaise/tools-ui-runtime-smoke.sh`
- Executable: `cargo run -p ai_blaise_citus_watch -- run-canonical`

### D7: Helm One-Line Install

**Overlay**: `ai-blaise/command-center: helm/charts/citus-cluster`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides a production-safe direct Helm install surface for the
ai-blaise overlay. The chart defaults in `values.yaml` require immutable
operator/pool image digests and keep alpha sidecars, tools, and alpha
runtime/security intent disabled. They also omit controller-grade operator
RBAC while the operator production runtime only serves probes/metrics.
Non-production image-matrix coverage moved to the explicit
`values-exhaustive.yaml` profile, while `values-dev.yaml` remains the small
developer profile.

**Motivation**: A direct `helm upgrade --install` command should fail closed
unless production image identity is supplied, and it must not install the
exhaustive alpha profile by accident.

**Citus comparison**: Vanilla Citus does not ship the ai-blaise overlay chart
or its production default profile.

Production evidence: `ci/ai-blaise/deploy-check.sh` renders the default chart
and rejects missing immutable digests, alpha sidecar deployments, alpha tools,
and alpha runtime/security intent in the default profile. The same check keeps
`values-exhaustive.yaml` as the only direct Helm profile with all alpha
sidecars enabled. `ci/ai-blaise/kind-production-smoke.sh` now installs the
default chart profile with direct Helm against a live kind cluster, verifies
operator/pool replicas, rejects alpha workload deployments and controller-grade
operator RBAC, and runs live SQL plus operator admin traffic through the
installed release.

**References**:

- In-source: `FEATURE: D7` in `companion/src/ops_contracts.rs`
- Helm chart: `ai-blaise/command-center: helm/charts/citus-cluster`
- CI: `ci/ai-blaise/deploy-check.sh`
- Kubernetes smoke: `ci/ai-blaise/kind-production-smoke.sh`
- Gate: `make -f Makefile.ai-blaise gate-close`

### D8: Infrastructure Deploy Wrapper

**Overlay**: `scripts/citus-scale/deploy.sh`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides the production-safe human deploy wrapper for rendering
and installing the Helm overlay. The wrapper defaults to `values-prod.yaml`,
accepts release image tags and immutable operator/pool digests, refuses
non-production installs unless `ALLOW_ALPHA_INSTALL=1` is set explicitly, and
requires `ALLOW_MUTABLE_IMAGE_TAGS=1` before production rendering can bypass
digest pinning for local smoke images.

**Motivation**: Operators need one deploy entrypoint whose default behavior
matches GitOps and production values, so a direct install cannot accidentally
deploy the exhaustive alpha profile.

**Citus comparison**: Vanilla Citus does not ship the ai-blaise deploy
wrapper or its production-profile guardrails.

Production evidence: `ci/ai-blaise/kind-production-smoke.sh` runs
`scripts/citus-scale/deploy.sh` with `DEPLOY_PROFILE=prod` and `MODE=install`
against a live kind cluster, verifies the resulting `values-prod.yaml` release
has only operator/pool/PostgreSQL workloads, runs live SQL and pool admin
traffic through the installed release, rejects controller-grade operator RBAC
in the production profile, and proves the wrapper install path is part of
`make -f Makefile.ai-blaise gate-close`. `ci/ai-blaise/deploy-check.sh`
statically rejects regressions that remove the production default,
digest-inputs, mutable-tag escape hatch, controller RBAC boundary, or
non-production install refusal.

**References**:

- In-source: `FEATURE: D8` in `scripts/citus-scale/deploy.sh`
- CI: `ci/ai-blaise/deploy-check.sh`
- Kubernetes smoke: `ci/ai-blaise/kind-production-smoke.sh`
- Gate: `make -f Makefile.ai-blaise gate-close`

### D13: Production Runtime Image Matrix

**Overlay**: `images/rust-runtime`, `scripts/citus-scale`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Builds real Rust application images for the operator, pool,
sidecars, and `citusctl`, with deployed services defaulting to the long-running
`serve` command and the `citusctl` tool image defaulting to `plan inspect
cluster`. The pool image separates PostgreSQL TCP traffic from admin probes and
requires a configured upstream before readiness. The Kubernetes production smoke
also verifies live operator and sidecar health, readiness, and metrics over
port-forwarded pod traffic before accepting the deployment, runs the built
`citusctl` image as a Job, and aggregates pool request metrics across replicas
after the SQL smoke so service load balancing cannot hide a cold pool pod.
Controller-grade operator RBAC remains an alpha chart contract enabled only by
the exhaustive profile until the operator runs real Kubernetes watches and
reconciliation.

**Motivation**: Production Kubernetes verification must exercise the actual
app containers and PostgreSQL traffic path rather than synthetic responder
images.

**Citus comparison**: Vanilla Citus does not ship the ai-blaise operator,
pool, sidecar, or tool image matrix.

Production evidence: PR #11 head `f5f57f144` and merge commit `9110da454`
passed local and VM verification of the kind production smoke using the real
Rust image matrix, live operator and sidecar `/healthz`, `/readyz`, and
`/metrics` probes, real PostgreSQL traffic through the pool service, per-pod
pool request metric aggregation, and a separate `values-prod.yaml` profile
with alpha workloads disabled. Production values now require immutable
operator and pool image digests for release rendering; kind disables that
requirement only for locally loaded smoke images, so the smoke proves runtime
behavior but not release image-pinning evidence. Release image builds write
`artifacts/ai-blaise-image-digests.tsv` and fail if pushed images do not report
immutable repo digests. The deploy workflow and `gate-close` run
`ci/ai-blaise/kind-production-smoke.sh` as a live integration gate, while
the Makefile smoke targets set `REQUIRE_DOCKER=1` so missing Docker fails the
release gate instead of silently skipping live evidence. `gate-close` also runs
the image/deploy contract checks directly, with `REQUIRE_HELM=1` for rendered
chart checks so missing Helm fails the release gate instead of silently skipping
render evidence. The deploy wrapper install path is now live-gated by the
`values-prod.yaml` phase of the kind smoke through `MODE=install`, while the
optional tools Deployment remains dev-only and is not production evidence. The
kind smoke also runs the built `citusctl` image and
requires the `plan inspect cluster` output so tool images are executed, not
merely built or loaded. The
`ai-blaise/command-center: gitops/apps/13-citus-cluster.yaml` targets the `main` release branch and
`values-prod.yaml` for GitOps deployment with namespace creation and pruning
enabled; the Argo application is a GitOps render contract, not live controller
evidence.

**References**:

- Build script: `FEATURE: D13` in
  `scripts/citus-scale/build-app-images.sh`
- Runtime Dockerfile: `FEATURE: D13` in
  `images/rust-runtime/Dockerfile`
- Live SQL smoke: `ci/ai-blaise/pool-proxy-smoke.sh`
- Kubernetes smoke: `ci/ai-blaise/kind-production-smoke.sh`
- CI: `.github/workflows/ci-deploy.yml`
- Gate: `make -f Makefile.ai-blaise gate-close`
- CI: `ci/ai-blaise/image-check.sh`

### WF2: WAL Replay Debugger Command

**Overlay**: `tools/citusctl`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_walinspect`

**Summary**: Reserves CLI command planning and validation for WAL replay and
time-scoped investigation workflows.

**Motivation**: WAL forensics need to enter through plan/apply machinery so
replay and restore commands can share preflight and audit behavior.

**Citus comparison**: Vanilla Citus does not ship a WAL replay debugger CLI.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: WF2` in `tools/citusctl/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citusctl -- run-canonical`

## Federation

### F1: Federation CRD

**Overlay**: `operator/src/crds/federation.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `oracle_fdw`, `mysql_fdw`, `mongo_fdw`

**Summary**: Defines external federation links for warehouse, document, and
legacy database targets using secret-backed connection references.

**Motivation**: FDW and lakehouse federation need a typed source of desired
state before credentials and foreign schema creation are reconciled.

**Citus comparison**: Vanilla Citus can participate in FDW queries but does
not ship a federation CRD.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: F1` in `operator/src/crds/federation.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

## Graph

### G2: Distributed Graph Bridge

**Overlay**: `companion/src/graph_bridge.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `age`

**Summary**: Provides installable SQL graph colocation metadata that records
validated vertex-table, edge-table, vertex-key, and colocation-group bindings.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies
`companion_internal.ensure_graph_colocation(...)` records
`companion_graph_colocations` and verifies missing vertex keys fail closed.
Apache AGE graph execution, distributed graph traversal, and shard fanout
remain alpha.

**Motivation**: Graph queries need shard-local subgraphs before Cypher traffic
can safely run over distributed datasets.

**Citus comparison**: Vanilla Citus does not provide an Apache AGE
distributed-graph bridge.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: G2` in `companion/src/graph_bridge.rs`
- SQL runtime: `FEATURE: G2` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### G3: Graph Colocation Policy

**Overlay**: `companion/src/graph_bridge.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `age`

**Summary**: Provides an installable SQL graph colocation policy registry for
the vertex/edge placement metadata that graph and GraphQL bridge helpers share.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies
`companion_internal.ensure_graph_colocation(...)` records
`companion_graph_colocations` and verifies missing vertex keys fail closed.
Distributed graph placement enforcement, AGE catalog integration, traversal
routing, and operator reconciliation remain alpha.

**Motivation**: Traversals are only efficient when vertices and edges share
placement by tenant or graph key.

**Citus comparison**: Vanilla Citus has colocation groups, but no graph-aware
policy layer.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: G3` in `companion/src/graph_bridge.rs`
- SQL runtime: `FEATURE: G3` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

## JSON Schema

### JS2: Distributed JSON Schema Validation

**Overlay**: `companion/src/jsonschema_bridge.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_jsonschema`

**Summary**: Provides an installable SQL JSON schema registry and shard
validator with object-type and required-field checks.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies
`companion_internal.register_json_schema(...)` plus
`companion_internal.validate_jsonschema_shard(...)` report valid shard state,
and verifies non-object schemas fail closed. Full pg_jsonschema compatibility,
JSON Schema draft coverage, distributed validation workers, and operator
integration remain alpha.

**Motivation**: JSON validation must run on every shard, not only where a
coordinator migration happened to install a trigger.

**Citus comparison**: Vanilla Citus does not manage distributed
pg_jsonschema trigger fanout.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: JS2` in `companion/src/jsonschema_bridge.rs`
- SQL runtime: `FEATURE: JS2` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### M13: JSON Schema Validation On Insert

**Overlay**: `companion/src/jsonschema_bridge.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_jsonschema`

**Summary**: Provides an installable SQL JSON schema trigger helper that
installs table-level insert/update validation against registered schemas.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies
`companion_internal.install_jsonschema_trigger(...)` records
`companion_jsonschema_triggers`, accepts valid JSON documents, and verifies
documents missing required fields fail closed. Online migration orchestration,
backfill validation, trigger rollout orchestration, and operator integration
remain alpha.

**Motivation**: Migration and schema contracts need fail-fast JSON validation
before malformed tenant data is accepted.

**Citus comparison**: Vanilla Citus does not ship JSON Schema validation
helpers.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: M13` in `companion/src/jsonschema_bridge.rs`
- SQL runtime: `FEATURE: M13` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

## Geo

### Geo2: Geo-Aware Citus Distribution

**Overlay**: `companion/src/geo_distributed.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `postgis`

**Summary**: Provides an installable SQL geo bucket and distribution metadata
helper that adds a deterministic bucket column and records geo distribution
settings.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies `companion_geo_bucket(...)`,
`companion_internal.add_geohash_column(...)`, and
`companion_geo_distributions` work together, and verifies out-of-range latitude
fails closed. PostGIS geometry parsing, true geohash/S2/H3 indexes, distance
operators, and distributed spatial query planning remain alpha.

**Motivation**: Location-heavy workloads need spatially meaningful shard keys
so nearby data can be routed and rebalanced coherently.

**Citus comparison**: Vanilla Citus can distribute geometry tables but does
not create geo-aware distribution keys.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Geo2` in `companion/src/geo_distributed.rs`
- SQL runtime: `FEATURE: Geo2` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### Geo3: Geo Shard Pruning Planner Input

**Overlay**: `companion/src/geo_distributed.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `postgis`

**Summary**: Provides an installable SQL geo pruning metadata helper that
records table, geometry-column, and precision policy for later spatial-pruning
execution.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies
`companion_internal.enable_geo_shard_pruning(...)` records
`companion_geo_pruning_policies` and verifies out-of-range precision fails
closed. PostGIS planner hooks, shard exclusion, spatial selectivity
statistics, and operator integration remain alpha.

**Motivation**: Spatial queries should avoid scanning shards whose geohash
grid cells cannot intersect the requested bounding box.

**Citus comparison**: Vanilla Citus does not expose geo-shard pruning
metadata.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Geo3` in `companion/src/geo_distributed.rs`
- SQL runtime: `FEATURE: Geo3` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

## Observability

### O1: Query Percentile Views

**Overlay**: `companion/src/observability.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: `pg_stat_statements`

**Summary**: Adds the companion percentile contract and installable
`companion_pg_stat_statements_p95` SQL view over `pg_stat_statements` latency
data when the extension is present.

**Motivation**: Production operators need p95/p99/p99.9 query latency without
building one-off SQL at each installation.

**Citus comparison**: Vanilla Citus exposes distributed execution stats but
does not ship this percentile view contract.

Production evidence: `ci/ai-blaise/sql-extension-smoke.sh` starts a real
PostgreSQL 17 container with `shared_preload_libraries=pg_stat_statements`,
creates both `pg_stat_statements` and `ai_blaise_citus`, seeds a tracked SQL
statement, and requires the installable `companion_pg_stat_statements_p95`
view to report nonnegative percentile latency for that live statement. VM
verification for this promotion reran that smoke against `postgres:17`.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: O1` in `companion/src/observability.rs`
- SQL extension: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### O2: Local Activity Stats View

**Overlay**: `companion/src/observability.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Adds the local activity stats contract and installable
`companion_pg_stat_local_activity` SQL view for local node activity rollups.
The legacy `companion_pg_stat_distributed` view remains as a compatibility
alias for the same local-node data.

**Motivation**: Operators need a per-node view that can be installed on
coordinators and workers before a later multi-node aggregation layer is
promoted.

**Citus comparison**: Vanilla Citus exposes many stats views, but not this
single companion-owned local activity rollup contract.

Production evidence: `ci/ai-blaise/sql-extension-smoke.sh` installs
`ai_blaise_citus` into a real `postgres:17` container and requires
`companion_pg_stat_local_activity` and its compatibility alias
`companion_pg_stat_distributed` to report the local database node.
`ci/ai-blaise/observability-replication-smoke.sh` then starts a real
PostgreSQL primary, installs the extension, and requires the view to report
active local activity with nonnegative idle and wait counters.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: O2` in `companion/src/observability.rs`
- SQL extension: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- CI: `ci/ai-blaise/observability-replication-smoke.sh`

### O3: Distributed Replication Lag View

**Overlay**: `companion/src/observability.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Adds the replication-lag contract and installable
`companion_pg_dist_replication_lag` SQL view over `pg_stat_replication`.

**Motivation**: Multi-region and follower-read features need one companion
surface for lag budgets before HA gates can assert readiness.

**Citus comparison**: Vanilla Citus does not provide an ai-blaise regional lag
view contract.

Production evidence: `ci/ai-blaise/observability-replication-smoke.sh` starts
a real `postgres:17` primary and streaming standby on a Docker network, creates
a replication role, performs `pg_basebackup`, waits for the standby to enter
recovery, and requires the installable `companion_pg_dist_replication_lag`
view to report a streaming standby row with nonnegative lag bytes.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: O3` in `companion/src/observability.rs`
- SQL extension: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/observability-replication-smoke.sh`

### O4: Sidecar Health And Metrics Contract

**Overlay**: `sidecar/shared`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines shared sidecar health, readiness, drain state, HTTP probe
handling, Unix-socket probe serving, TCP probe serving for Kubernetes, and
Prometheus metrics emission.

**Motivation**: All ai-blaise sidecars need the same readiness semantics before
they can safely participate in Kubernetes rollout, drain, and chaos gates.

**Citus comparison**: Vanilla Citus does not ship out-of-process Rust sidecars
or a sidecar health contract.

Production evidence: PR #11 head `f5f57f144` and merge commit `9110da454`
passed local and VM verification of the kind production smoke that
port-forwarded into the live operator and every deployed sidecar and verified
`/healthz`, `/readyz`, and `/metrics` from the actual pods. Production values
still keep alpha feature sidecars disabled by default; this status applies
only to the shared probe/metrics runtime.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: O4` in `sidecar/shared/src/lib.rs`
- In-source: `FEATURE: O4` in `sidecar/shared/src/runtime.rs`
- Executable: `FEATURE: O4` in `sidecar/shared/src/main.rs`

### O5: Sidecar Deployment Contract

**Overlay**: `operator/src/crds/sidecar.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the operator-facing sidecar deployment contract for
replicas, resources, and type-specific configuration across the V2 sidecar
surface. The current implementation does not emit or export OpenTelemetry
traces; trace propagation remains unimplemented until real runtime code,
collector wiring, and measured VM/Kubernetes evidence are added.

**Motivation**: Rollout behavior is only useful if every sidecar is declared
and reconciled through a consistent resource contract.

**Citus comparison**: Vanilla Citus does not ship out-of-process sidecar
deployment objects.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: O5` in `operator/src/crds/sidecar.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### O6: Grafana Dashboards As ConfigMaps

**Overlay**: `ai-blaise/command-center: helm/charts/citus-cluster/templates/observability-dashboards.yaml`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds Helm-rendered Grafana dashboard ConfigMaps for Citus query
latency, distributed replication lag, vectorizer backlog, sidecar readiness,
and pool error rate.

**Motivation**: Operators need installable dashboards with the chart instead
of hand-maintained JSON pasted into each cluster.

**Citus comparison**: Vanilla Citus does not ship ai-blaise dashboard
ConfigMaps.

Production evidence: the kind production smoke in
`ci/ai-blaise/kind-production-smoke.sh` installs the default `values.yaml`,
`values-prod.yaml`, and explicit exhaustive Helm profiles into a real kind
cluster, then requires the live
`configmap/ai-blaise-citus-dashboards` resource to contain both dashboard JSON
payloads plus the emitted `ai_blaise_sidecar_ready` metric and the guarded pool
error-rate expression. `ci/ai-blaise/deploy-check.sh` parses the embedded
Grafana JSON, requires the exact dashboard files, panel titles, and PromQL
target expressions, renders the production profiles with Helm, and rejects
unguarded pool request-rate division.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: O6` in
  `ai-blaise/command-center: helm/charts/citus-cluster/templates/observability-dashboards.yaml`
- CI: `ci/ai-blaise/kind-production-smoke.sh`
- CI: `ci/ai-blaise/deploy-check.sh`

### O10: Alert Rules For Top Pains

**Overlay**: `ai-blaise/command-center: helm/charts/citus-cluster/templates/observability-prometheusrules.yaml`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds optional `PrometheusRule` alerts for replication lag,
sidecar readiness, vectorizer backlog, and pool error rate.

**Motivation**: The V2 chaos and production gates need chart-owned alert rules
for the failure modes most likely to hurt users first.

**Citus comparison**: Vanilla Citus does not ship these ai-blaise alert rules.

Production evidence: the kind production smoke in
`ci/ai-blaise/kind-production-smoke.sh` installs the monitoring CRDs into a
real kind cluster before Helm install, applies the default `values.yaml`,
`values-prod.yaml`, and explicit exhaustive chart profiles, then requires the
live
`prometheusrules.monitoring.coreos.com/ai-blaise-citus-alerts` resource to
contain the replication-lag, sidecar-readiness, vectorizer-backlog, and
pool-error-rate alerts. The live check also requires the pool error-rate alert
to use the guarded request-rate denominator and a positive-traffic predicate.
`ci/ai-blaise/deploy-check.sh` renders the same production profiles with Helm,
statically guards the alert names, and rejects unguarded pool request-rate
division.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: O10` in
  `ai-blaise/command-center: helm/charts/citus-cluster/templates/observability-prometheusrules.yaml`
- CI: `ci/ai-blaise/kind-production-smoke.sh`
- CI: `ci/ai-blaise/deploy-check.sh`

### O13: citus-watch TUI

**Overlay**: `tools/citus-watch`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides the snapshot-backed `citus-watch` unified operator frame
across cluster topology, shards, hypertables, EXPLAIN, rebalance, vectorizer
backlog, search indexes, tenants, and branches.

Production evidence: The VM proof and `ci/ai-blaise/tools-ui-runtime-smoke.sh`
run the real `ai_blaise_citus_watch` binary against a validated tools snapshot
TSV. The smoke requires the rendered frame to include pool readiness,
vectorizer backlog signals, and the companion shard-placement query plan.
Long-running terminal event handling, live Prometheus polling, and direct
companion database reads remain alpha.

**Motivation**: Runtime operations need a compact, terminal-native view that
tracks the same companion and metrics surfaces used by dashboards and alerts.

**Citus comparison**: Vanilla Citus does not ship a dedicated runtime
operations TUI.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: O13` in `tools/citus-watch/src/lib.rs`
- CI: `ci/ai-blaise/tools-ui-runtime-smoke.sh`
- Executable: `cargo run -p ai_blaise_citus_watch -- run-canonical`

### O14: W3C Trace-Context Propagation

**Overlay**: `sidecar/shared/src/otel.rs`, `pool/src/trace_tap.rs`,
`companion/src/trace_context.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Threads a W3C `traceparent` end-to-end from pool to companion
to sidecars. The shared `otel` module exposes a `TraceContext` extract /
inject trait with three carriers — `HeaderMap` (HTTP), `MetadataMap` (gRPC),
and `SetLocalBuilder` (PostgreSQL `SET LOCAL`). The pool proxy taps the
PostgreSQL startup envelope for an embedded traceparent in three places
(custom `traceparent` startup parameter, `options=-c trace.parent=`, and a
backwards-compatible `application_name` wire format) and records counters
for tapped versus absent connections without modifying the byte stream.
Companion's `trace_context` plan documents the canonical pgrx functions
`companion.current_traceparent`, `companion.current_tracestate`, and
`companion.project_traceparent_from_application_name`, which let
companion-side spans chain to the inbound trace via the
`current_setting('trace.parent', true)` GUC.

**Motivation**: Distributed-database observability needs a single
trace-id that survives the libpq wire so per-sidecar spans, companion
spans, and operator spans can be correlated in Jaeger or Tempo without
sampling drift.

**Citus comparison**: Vanilla Citus does not propagate W3C trace-context
through libpq.

Production evidence: `ci/ai-blaise/otel-trace-propagation-smoke.sh` boots a
real `postgres:17` container, runs the pool proxy against it, sends a
traceparent via libpq `PGOPTIONS`, and asserts that the pool's `trace_tap`
log line reports the exact traceparent and that
`ai_blaise_citus_pool_traceparent_tapped_total` increments. A follow-up
connection without a traceparent increments
`ai_blaise_citus_pool_traceparent_absent_total`. With `REQUIRE_KIND=1` the
script additionally boots a 3-node kind cluster with Jaeger and asserts the
trace lands at Jaeger.

**References**:

- Design: `docs/ai-blaise/OBSERVABILITY.md`
- In-source: `FEATURE: O14` in `sidecar/shared/src/otel.rs`
- In-source: `FEATURE: O14` in `pool/src/proxy.rs`
- In-source: `FEATURE: O14` in `pool/src/trace_tap.rs`
- In-source: `FEATURE: O14` in `companion/src/trace_context.rs`
- CI: `ci/ai-blaise/otel-trace-propagation-smoke.sh`

### O15: Per-Sidecar Structured-Log Schema

**Overlay**: `sidecar/shared/src/log_schema.rs`, `companion/src/log_view.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Declares the canonical JSON log shape for every ai-blaise
sidecar: nine common fields (timestamp, level, sidecar, message,
traceparent, tenant_id, request_id, version, error, fields) plus typed
per-sidecar extensions under `fields`. Companion's `log_view` module
renders 17 deterministic `CREATE OR REPLACE VIEW` statements, one per
sidecar, that project the JSON column from `companion.sidecar_log_raw`
into typed SQL columns; Vector or fluent-bit feed that raw table from
sidecar stdout.

**Motivation**: Operator tooling, the citus-watch TUI, and the Grafana
dashboards in `ai-blaise/command-center` all need a typed contract for log
ingestion. Without it the per-sidecar shape drifts and downstream consumers
cannot plan against the JSON column.

**Citus comparison**: Vanilla Citus emits unstructured Postgres log lines;
no per-sidecar JSON schema exists.

Production evidence: `ai_blaise_citus_sidecar_shared::log_schema` unit tests
validate every canonical schema, prove no extension field shadows a common
field, and confirm the schema catalog covers all 17 sidecars. Companion's
`log_view` tests render the deterministic SQL bundle and assert per-sidecar
projections cast each extension field to its declared SQL type.

**References**:

- Design: `docs/ai-blaise/OBSERVABILITY.md`
- In-source: `FEATURE: O15` in `sidecar/shared/src/lib.rs`
- In-source: `FEATURE: O15` in `sidecar/shared/src/log_schema.rs`
- In-source: `FEATURE: O15` in `companion/src/log_view.rs`
- Acceptance: `cargo test -p ai_blaise_citus_companion --lib log_view`

## Extension Catalog SQL Runtime

### A7: pgvector Cohabitation Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgvector`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the pgvector cohabitation contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not pin a bundled vector-extension
catalog contract.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: A7` in `companion/src/extension_catalog.rs`

### A12: vchord Alternate Vector Index Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `vchord`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the vchord alternate vector-index contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not track optional vector-index
alternatives in a catalog runtime.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: A12` in `companion/src/extension_catalog.rs`

### C11: pgl_ddl_deploy Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgl_ddl_deploy`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the DDL replication extension contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not bundle cross-region DDL
replication policy.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: C11` in `companion/src/extension_catalog.rs`

### C12: Replication-Slot Failover Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_failover_slots`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the logical replication slot failover contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not require logical slot failover
packaging.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: C12` in `companion/src/extension_catalog.rs`

### C13: Subscription Failover Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_subscription_pg_failover`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the logical subscription failover contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not package subscription failover
contracts.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: C13` in `companion/src/extension_catalog.rs`

### EF6: UDF Substrate Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `plrust`, `plv8`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the JavaScript and Rust in-database UDF substrate contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not bundle plv8/plrust as a platform
contract.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: EF6` in `companion/src/extension_catalog.rs`

### F2: Foreign Data Wrapper Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `oracle_fdw`, `mysql_fdw`, `mongo_fdw`, `tds_fdw`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the foreign data wrapper bundle contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not bundle the overlay FDW catalog
policy.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: F2` in `companion/src/extension_catalog.rs`

### F5: Outbound HTTP Extension Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgsql-http`, `pg_net`, `omnigres`

**Summary**: Provides an installable SQL extension catalog runtime entry for
outbound HTTP extension and integration-target policy.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not package pgsql-http or pg_net
policy.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: F5` in `companion/src/extension_catalog.rs`

### G1: Apache AGE Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `age`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the Apache AGE graph substrate contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not require Apache AGE in every
operand image.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: G1` in `companion/src/extension_catalog.rs`

### Geo1: PostGIS Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `postgis`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the PostGIS geospatial substrate contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not require PostGIS in every operand
image.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: Geo1` in `companion/src/extension_catalog.rs`

### IA1: HypoPG Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `hypopg`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the hypothetical-index advisor input contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not bundle hypothetical-index advisor
inputs.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: IA1` in `companion/src/extension_catalog.rs`

### IA2: pg_qualstats Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_qualstats`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the predicate-statistics advisor input contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not bundle predicate-stat advisor
inputs.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: IA2` in `companion/src/extension_catalog.rs`

### JS1: pg_jsonschema Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_jsonschema`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the JSON Schema substrate contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not require JSON Schema validation
support.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: JS1` in `companion/src/extension_catalog.rs`

### L11: pg_parquet Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_parquet`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the Parquet helper extension contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not package Parquet helpers as part of
its image.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: L11` in `companion/src/extension_catalog.rs`

### M6: DDL Replication Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgl_ddl_deploy`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the DDL replication contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not bundle pgl_ddl_deploy policy.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: M6` in `companion/src/extension_catalog.rs`

### M10: Track Settings Drift Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_track_settings`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the settings drift tracking extension contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not require pg_track_settings.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: M10` in `companion/src/extension_catalog.rs`

### M12: UUIDv7 Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_uuidv7`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the monotonic UUID helper contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not bundle monotonic UUID helpers.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: M12` in `companion/src/extension_catalog.rs`

### MR7: pgactive Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgactive`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the cross-region active-active reference extension contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not package pgactive conflict-policy
gates.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: MR7` in `companion/src/extension_catalog.rs`

### O7: Wait-Event Sampling Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_wait_sampling`, `pgsentinel`

**Summary**: Provides an installable SQL extension catalog runtime entry for
wait-event sampling extension contracts.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not require pg_wait_sampling or
pgsentinel.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: O7` in `companion/src/extension_catalog.rs`

### O8: OS Metrics Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgnodemx`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the SQL-visible OS metrics extension contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not require pgnodemx.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: O8` in `companion/src/extension_catalog.rs`

### O9: Kernel Stats Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_stat_kcache`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the kernel statistics extension contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not require pg_stat_kcache.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: O9` in `companion/src/extension_catalog.rs`

### O11: pg_stat_monitor Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_stat_monitor`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the pg_stat_monitor alternative statement histogram contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not package pg_stat_monitor.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: O11` in `companion/src/extension_catalog.rs`

### O12: pg_show_plans Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_show_plans`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the plan-inspection extension contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not require plan-inspection
packaging.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: O12` in `companion/src/extension_catalog.rs`

### PM1: pg_hint_plan Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_hint_plan`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the hint-plan backend contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not package hint-plan policy as an
overlay contract.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: PM1` in `companion/src/extension_catalog.rs`

### PM2: sr_plan Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `sr_plan`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the saved-plan backend contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not bundle saved-plan backends.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: PM2` in `companion/src/extension_catalog.rs`

### R6: Queue Extension Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgmq`, `pgque`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the bloat-free queue substrate contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not package pgque/pgmq as queue
policy.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: R6` in `companion/src/extension_catalog.rs`

### R11: pg_warm Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_warm`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the replica cold-start cache warming contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not require pg_warm in operand images.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: R11` in `companion/src/extension_catalog.rs`

### Search1: pg_search Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_search`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the BM25 search substrate contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not require BM25 search support.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: Search1` in `companion/src/extension_catalog.rs`

### Search4: RUM Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `rum`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the RUM search index substrate contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not require RUM search indexes.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: Search4` in `companion/src/extension_catalog.rs`

### Search5: pg_trgm Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_trgm`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the trigram search substrate contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not require trigram search support.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: Search5` in `companion/src/extension_catalog.rs`

### Search6: citext Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `citext`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the case-insensitive text search substrate contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not require citext search semantics.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: Search6` in `companion/src/extension_catalog.rs`

### Sec3: Audit Extension Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgaudit`, `pgauditlogtofile`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the SQL and file audit extension contracts.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not require this audit bundle.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: Sec3` in `companion/src/extension_catalog.rs`

### Sec4: pgsodium Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgsodium`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the libsodium crypto extension contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not bundle libsodium crypto policy.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: Sec4` in `companion/src/extension_catalog.rs`

### Sec10: pg_safeupdate Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_safeupdate`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the safe-update guard extension contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not package pg_safeupdate policy.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: Sec10` in `companion/src/extension_catalog.rs`

### Sec11: Anonymization Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `anon`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the CDC anonymization extension contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not bundle anonymization policy.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: Sec11` in `companion/src/extension_catalog.rs`

### Sec14: pgcrypto Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgcrypto`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the core crypto primitive extension contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not document pgcrypto as overlay
policy.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: Sec14` in `companion/src/extension_catalog.rs`

### Sec15: CMK Encryption Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgsodium`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the pgsodium-backed CMK encryption-at-rest extension contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not prescribe pgsodium-backed CMK
controls.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: Sec15` in `companion/src/extension_catalog.rs`

### WF1: pg_walinspect Catalog Runtime

**Overlay**: `companion/src/extension_catalog.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_walinspect`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the WAL inspection forensic workflow extension contract.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. Actual binary extension
installation, full operand image build, initdb extension creation, and operator
package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not expose this WAL inspection
workflow as an overlay contract.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: WF1` in `companion/src/extension_catalog.rs`

## V2 Contract Surface Headings

The former V2 addendum rows below now have standalone alpha feature headings.
Each heading names the executable contract evidence and keeps the runtime
boundary explicit. These are catalog-complete contract surfaces, not production
claims for the full feature behavior.

### A9: Secret Binding Via External Secrets

**Overlay**: `companion/src/ops_contracts.rs` and Helm values
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Captures the vector-provider secret-reference contract that keeps
API keys outside literal values and points operators at External Secrets.

**Current boundary**: The executable operations runner validates the expected
secret-control shape, while live External Secrets reconciliation and provider
credential rotation remain alpha.

**Citus comparison**: Vanilla Citus does not define vector-provider secret
binding.

**References**:

- In-source: `FEATURE: A9` in `companion/src/ops_contracts.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`

### A10: Streaming Chat Completion UDF

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the required policy inputs for a tenant-budgeted
streaming chat completion SQL surface.

**Current boundary**: The advanced-planner runner proves the contract metadata
is present and valid; no provider call path, streaming SRF, or billing
enforcement is production-ready yet.

**Citus comparison**: Vanilla Citus does not define streaming LLM SQL
surfaces.

**References**:

- In-source: `FEATURE: A10` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### A11: Semantic Catalog Text-To-SQL

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Records the catalog and tenant-scope inputs needed before a
semantic text-to-SQL planner can be wired to real metadata.

**Current boundary**: The contract runner verifies shape and coverage only;
semantic retrieval, SQL generation, authorization, and query execution remain
alpha.

**Citus comparison**: Vanilla Citus does not include a tenant-scoped semantic
catalog.

**References**:

- In-source: `FEATURE: A11` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### D9: Canary Upgrade Runbook

**Overlay**: `companion/src/ops_contracts.rs` and
`docs/ai-blaise/RUNBOOKS/upgrade.md`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Tracks the canary-upgrade rehearsal artifact as a required
operations contract.

**Current boundary**: The operations runner verifies the runbook contract, but
an automated canary cluster upgrade with rollback evidence is still alpha.

**Citus comparison**: Vanilla Citus does not include this canary upgrade
runbook.

**References**:

- In-source: `FEATURE: D9` in `companion/src/ops_contracts.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`

### D10: Release Hardening Runbook

**Overlay**: `companion/src/ops_contracts.rs` and
`docs/ai-blaise/RUNBOOKS/production.md`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Records the release-readiness review path, security controls, and
operational handoff checklist as a contract surface.

**Current boundary**: The companion runner validates the runbook reference;
live release certification, owner signoff, and rollback drills remain alpha.

**Citus comparison**: Vanilla Citus does not include these ai-blaise hardening
gates.

**References**:

- In-source: `FEATURE: D10` in `companion/src/ops_contracts.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`

### D11: MCP Developer Workflow

**Overlay**: `tools/citus-mcp`, `sidecar/mcp`, and `companion/src/ops_contracts.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the validation-only MCP workflow contract for exposing
Citus-oriented developer operation requests to agent tooling.

Executable alpha evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/mcp-stdio-smoke.sh` and
`ci/ai-blaise/mcp-sidecar-stdio-smoke.sh`, which drive the real
`tools/citus-mcp` and `sidecar/mcp` `serve-stdio` processes with JSON-RPC
initialize, tool-list, safe tenant query validation, cross-schema denial,
destructive-denial, and
missing-tenant-scope requests. `ci/ai-blaise/mcp-sidecar-http-smoke.sh` and
`ci/ai-blaise/kind-production-smoke.sh` verify the sidecar `serve` HTTP
JSON-RPC path, including deployed Kubernetes `POST /mcp` traffic. The
operations runner still records the broader workflow contract. MCP4 covers
read-only database execution for `tools/citus-mcp`; authentication, mutating
database execution, Kubernetes tool execution, and production sidecar
enablement remain alpha. Authenticated multi-user MCP deployment, policy
isolation beyond request validation, and live database/Kubernetes mutations
also remain alpha.

**Citus comparison**: Vanilla Citus does not expose MCP workflows for agents.

**References**:

- In-source: `FEATURE: D11` in `tools/citus-mcp/src/lib.rs`
- In-source: `FEATURE: D11` in `tools/citus-mcp/src/main.rs`
- In-source: `FEATURE: D11` in `sidecar/mcp/src/lib.rs`
- In-source: `FEATURE: D11` in `sidecar/mcp/src/main.rs`
- In-source: `FEATURE: D11` in `companion/src/ops_contracts.rs`
- Executable: `cargo run -p ai_blaise_citus_mcp -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_mcp -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`
- CI: `ci/ai-blaise/mcp-stdio-smoke.sh`
- CI: `ci/ai-blaise/mcp-sidecar-stdio-smoke.sh`
- CI: `ci/ai-blaise/mcp-sidecar-http-smoke.sh`
- CI: `ci/ai-blaise/kind-production-smoke.sh`

### Edge1: Bounded-Staleness Edge Replicas

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Captures the decision-record gate for edge read replicas with a
bounded-staleness contract.

**Current boundary**: The advanced-planner runner validates the research guard;
edge replica provisioning, freshness measurement, and failover behavior remain
alpha.

**Citus comparison**: Vanilla Citus does not model edge POP read replicas.

**References**:

- In-source: `FEATURE: Edge1` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### Edge2: libsql Read-Tier Research Guard

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Keeps the libsql-shaped read-tier concept behind an explicit
research decision record.

**Current boundary**: The contract runner proves the guard exists; no libsql
read-tier integration, replication adapter, or workload isolation is
production-ready.

**Citus comparison**: Vanilla Citus does not include a libsql-shaped research
gate.

**References**:

- In-source: `FEATURE: Edge2` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### F3: Iceberg Federation To Warehouses

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines catalog and warehouse inputs for an Iceberg federation
bridge.

**Current boundary**: The advanced-planner contract covers input validation;
Iceberg catalog connectivity, snapshot planning, and warehouse reads remain
alpha.

**Citus comparison**: Vanilla Citus does not define Iceberg warehouse
federation.

**References**:

- In-source: `FEATURE: F3` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### F4: postgres_fdw Credential Rotation

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `postgres_fdw`

**Summary**: Records the server and secret-reference inputs needed for a safe
FDW credential rotation path.

**Current boundary**: Contract validation is executable; actual credential
rollover, connection draining, and foreign server reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not prescribe FDW secret rotation.

**References**:

- In-source: `FEATURE: F4` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### L7: Citus Columnar Analytical Path

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Captures the table and columnar-policy inputs for routing
analytical work toward columnar storage.

**Current boundary**: The contract runner validates planner intent only; live
columnar conversion, cost model selection, and workload routing remain alpha.

**Citus comparison**: Vanilla Citus has columnar storage but not this overlay
tiering contract.

**References**:

- In-source: `FEATURE: L7` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### L10: Cross-Tier Query Planner

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the hot, warm, and cold tier inputs expected by the HTAP
planner contract.

**Current boundary**: Deterministic contract execution proves the declared
tiers; physical tier selection and query rewrites remain alpha.

**Citus comparison**: Vanilla Citus does not combine hot, warm, and cold tiers
through this overlay planner.

**References**:

- In-source: `FEATURE: L10` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### M4: Schema Drift Detection

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Models the observed-schema and desired-schema inputs for drift
reconciliation.

**Current boundary**: The runner verifies the contract surface; live schema
diffing, remediation planning, and operator apply behavior remain alpha.

**Citus comparison**: Vanilla Citus does not reconcile declarative schema
drift.

**References**:

- In-source: `FEATURE: M4` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### MR3: Regional Row Placement

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines region-prefix and distribution-key inputs for regional row
placement policy.

**Current boundary**: Contract validation is executable; row placement
enforcement, repartitioning, and regional admission control remain alpha.

**Citus comparison**: Vanilla Citus does not encode region in key policy.

**References**:

- In-source: `FEATURE: MR3` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### MR6: Closed-Timestamp Time Travel

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Records timestamp and maximum-staleness inputs for a
closed-timestamp read contract.

**Current boundary**: The planner contract is covered by deterministic tests;
MVCC timestamp routing, replica freshness, and stale-read execution remain
alpha.

**Citus comparison**: Vanilla Citus does not expose bounded-staleness time
travel.

**References**:

- In-source: `FEATURE: MR6` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### MR9: Region Survival Runbook

**Overlay**: `companion/src/ops_contracts.rs` and
`docs/ai-blaise/RUNBOOKS/disaster-recovery.md`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Records the regional failover drill as a required operational
artifact.

**Current boundary**: The operations runner validates the runbook reference;
live multi-region failover, PITR restore, and backup artifact restore remain
alpha.

**Citus comparison**: Vanilla Citus does not ship this regional DR runbook.

**References**:

- In-source: `FEATURE: MR9` in `companion/src/ops_contracts.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`
- Benchmark: `benchmarks/chaos/scenarios/kill-coordinator.sh`,
  `benchmarks/chaos/scenarios/network-partition.sh` (V2 gate 11 chaos
  acceptance; alpha until full runs land)

### R3: Columnstore-On-Worker Policy

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Defines table and age-threshold inputs for worker-local
columnstore policy.

**Current boundary**: Contract execution proves the policy shape; worker
storage transitions and read-path verification remain alpha.

**Citus comparison**: Vanilla Citus does not define this worker tiering policy.

**References**:

- In-source: `FEATURE: R3` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### R8: Non-Hypertable Cold Columnar Path

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Captures the table and tier inputs for cold columnar policy on
non-hypertable relations.

**Current boundary**: The contract runner validates intent; live cold-tier
movement and query-path proof remain alpha.

**Citus comparison**: Vanilla Citus does not define this cold-tier policy.

**References**:

- In-source: `FEATURE: R8` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### R12: Per-Shard Temperature Ranking

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines shard ID and temperature-score inputs for ranking data
movement candidates.

**Current boundary**: The contract is deterministic; collection of real heat
signals and automatic tier movement remain alpha.

**Citus comparison**: Vanilla Citus does not maintain shard temperature scores
for this overlay policy.

**References**:

- In-source: `FEATURE: R12` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### RT5: Phoenix-Channel-Compatible Realtime Client

**Overlay**: `companion/src/ops_contracts.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Records the supabase-js/Phoenix-channel compatibility target for
the realtime client surface.

**Current boundary**: Operations evidence validates the compatibility contract;
a live WebSocket protocol harness and client SDK matrix remain alpha.

**Citus comparison**: Vanilla Citus does not provide realtime client
compatibility gates.

**References**:

- In-source: `FEATURE: RT5` in `companion/src/ops_contracts.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`

### S1: Auto Shard Split

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines shard-group and split-threshold inputs for automated
split intent.

**Current boundary**: The contract runner validates the policy declaration;
actual shard splitting, data movement, and rollback remain alpha.

**Citus comparison**: Vanilla Citus does not expose declarative split intent.

**References**:

- In-source: `FEATURE: S1` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### S3: Clone-Node Fast Scale-Out

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Records source-worker and target-worker inputs for a clone-node
scale-out operation.

**Current boundary**: Contract execution proves required inputs; live clone,
catch-up, validation, and cutover workflows remain alpha.

**Citus comparison**: Vanilla Citus exposes clone-node primitives but not this
operator contract.

**References**:

- In-source: `FEATURE: S3` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### S7: Cross-Region Replication Via pgactive

**Overlay**: `companion/src/ops_contracts.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgactive`

**Summary**: Captures the conflict-policy gate required before pgactive-backed
cross-region replication can be enabled.

**Current boundary**: Operations contract evidence is executable; live
pgactive deployment, conflict resolution, and region-failover proof remain
alpha.

**Citus comparison**: Vanilla Citus does not bundle pgactive policy gates.

**References**:

- In-source: `FEATURE: S7` in `companion/src/ops_contracts.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`

### S8: Locality-Prefixed PKs

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines region and tenant ID inputs for locality-prefixed primary
key policy.

**Current boundary**: The contract runner validates the planner surface; live
key migration, foreign-key compatibility, and enforcement remain alpha.

**Citus comparison**: Vanilla Citus does not define region-prefixed key
policy.

**References**:

- In-source: `FEATURE: S8` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### S12: Tablespaces By Region

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Records region and tablespace inputs for regional placement
policy.

**Current boundary**: Contract validation is complete; tablespace creation,
operator reconciliation, and placement enforcement remain alpha.

**Citus comparison**: Vanilla Citus does not reconcile region tablespace
intent.

**References**:

- In-source: `FEATURE: S12` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### Sec7: External Secrets Integration

**Overlay**: `companion/src/ops_contracts.rs` and Helm values
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Records the External Secrets reference-only security control for
overlay credentials.

**Current boundary**: The operations runner verifies the intended control; live
External Secrets controller reconciliation and rotation evidence remain alpha.

**Citus comparison**: Vanilla Citus does not prescribe External Secrets refs.

**References**:

- In-source: `FEATURE: Sec7` in `companion/src/ops_contracts.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`

### Sec8: TLS Everywhere

**Overlay**: `companion/src/ops_contracts.rs` and Helm values
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Tracks TLS expectations for clients, Postgres backends, and
sidecar-to-sidecar traffic.

**Current boundary**: The security contract is executable; certificate
issuance, mTLS enforcement, and live rotation tests remain alpha.

**Citus comparison**: Vanilla Citus does not enforce this full overlay TLS
contract.

**References**:

- In-source: `FEATURE: Sec8` in `companion/src/ops_contracts.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`

### Sec9: SBOM And Cosign Attestation

**Overlay**: `companion/src/ops_contracts.rs` and release gates
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Captures the release-attestation requirement for SBOM and cosign
metadata.

**Current boundary**: The operations runner validates the control record;
signed artifact publication, verification policy, and admission enforcement
remain alpha.

**Citus comparison**: Vanilla Citus does not require ai-blaise release
attestations.

**References**:

- In-source: `FEATURE: Sec9` in `companion/src/ops_contracts.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`

### Sto2: file_attachment Domain Type

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the file attachment storage domain and backing table
contract.

**Current boundary**: The advanced-planner runner validates the domain record;
object storage wiring, retention policy, and authorization remain alpha.

**Citus comparison**: Vanilla Citus does not include a storage domain type.

**References**:

- In-source: `FEATURE: Sto2` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### T4: Hash-Table Planner Hot Path

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Records the minimum-partition lookup contract for a planner
hash-table path.

**Current boundary**: Contract execution proves the declaration; benchmarked
planner hot-path replacement and regression budgets remain alpha.

**Citus comparison**: Vanilla Citus does not expose this overlay performance
contract.

**References**:

- In-source: `FEATURE: T4` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### T6: PG18 io_uring Default

**Overlay**: `companion/src/ops_contracts.rs`, `images/citus-pg-overlay/Dockerfile`, `ci/ai-blaise/sql-extension-smoke.sh`, and Helm values
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Tracks the Postgres I/O method policy for the PG18 io_method
contract, paired with the PG version matrix in the overlay image and smoke
harness.

**Current boundary**: The operations contract validates the expected toggle.
The overlay Dockerfile now builds for PG17 and PG18 via `--build-arg PG_MAJOR`,
and `ci/ai-blaise/sql-extension-smoke.sh` runs the companion SQL contract
against both PG17 and PG18 base images on every PR, asserting `io_method`
accepts its contract value without breaking Citus or any bundled extension.
PG18 stays alpha until the full bundled-extension binary set has verified PG18
builds (see `docs/ai-blaise/BUNDLED_EXTENSIONS.md` PG version matrix) and a
real-kernel `io_method=io_uring` run is recorded under
`docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`.

**Citus comparison**: Vanilla Citus does not set ai-blaise PG18 I/O policy or
emit a multi-PG-major operand image from a single overlay contract.

**References**:

- In-source: `FEATURE: T6` in `companion/src/ops_contracts.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`
- Executable: `make -f Makefile.ai-blaise sql-extension-smoke` (runs PG17 and PG18 matrix)
- Executable: `make -f Makefile.ai-blaise build-image-matrix` (builds PG17 and PG18 overlay images)

### T7: Pipelined Client Protocol In Pool

**Overlay**: `pool/src/runtime.rs`, `pool/src/proxy.rs`, `pool/src/main.rs`,
and `companion/src/ops_contracts.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Keeps the broader transaction-batching and shard-aware pipeline
contract visible alongside the measured `T15` simple-query proxy evidence.

**Current boundary**: The pool canonical runner and operations runner validate
the contract shape; only the byte-transparent simple-query data-plane baseline
is production-ready under `T15`.

**Citus comparison**: Vanilla Citus does not ship the ai-blaise pool pipeline.

**References**:

- In-source: `FEATURE: T7` in `pool/src/runtime.rs`
- In-source: `FEATURE: T7` in `pool/src/proxy.rs`
- In-source: `FEATURE: T7` in `pool/src/main.rs`
- In-source: `FEATURE: T7` in `companion/src/ops_contracts.rs`
- Executable: `cargo run -p ai_blaise_citus_pool -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`
- CI: `ci/ai-blaise/pool-proxy-smoke.sh`

### T10: Bulk Protocol Fetch Path

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the maximum batch-row contract for bulk protocol fetches.

**Current boundary**: The advanced-planner runner validates the configured
budget; protocol implementation, backpressure, and cross-worker fetch tests
remain alpha.

**Citus comparison**: Vanilla Citus has no ai-blaise bulk-fetch contract.

**References**:

- In-source: `FEATURE: T10` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### T11: DistSQL Physical Pushdown

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Records the worker-task budget for a DistSQL physical pushdown
contract.

**Current boundary**: The planner contract is executable; physical plan
rewrites and worker execution remain alpha.

**Citus comparison**: Vanilla Citus does not expose this DistSQL contract.

**References**:

- In-source: `FEATURE: T11` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### T13: Distributed Cursors

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the open-shard budget for distributed cursor state.

**Current boundary**: Contract validation is deterministic; cursor lifecycle,
worker cleanup, and error recovery remain alpha.

**Citus comparison**: Vanilla Citus does not coordinate multi-shard cursor
state this way.

**References**:

- In-source: `FEATURE: T13` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### T14: Distributed Savepoints

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the open-shard budget for distributed savepoint state.

**Current boundary**: The contract runner validates state shape; savepoint
propagation, rollback, and worker cleanup remain alpha.

**Citus comparison**: Vanilla Citus does not coordinate savepoints through
this contract.

**References**:

- In-source: `FEATURE: T14` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### TS10: Hierarchical CAGGs Distributed

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: Captures source and target continuous aggregate inputs for
hierarchical CAGG fanout.

**Current boundary**: The planner contract validates required inputs; real
hierarchical refresh planning and worker fanout remain alpha.

**Citus comparison**: Vanilla Citus does not fan out hierarchical CAGGs across
workers.

**References**:

- In-source: `FEATURE: TS10` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### TS11: Bloom Filters On segmentby

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: Defines table and `segmentby` inputs for a bloom-filter fanout
contract.

**Current boundary**: Contract execution validates the surface; bloom filter
construction, refresh integration, and Timescale worker fanout remain alpha.

**Citus comparison**: Vanilla Citus does not define Timescale segmentby bloom
fanout.

**References**:

- In-source: `FEATURE: TS11` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`


## Bundled-Extension Microbenchmarks (MB1-MB26)

Each of the 26 always-on bundled extensions ships a microbench under
`benchmarks/microbenches/<ext>/`. The microbench surface is the
production evidence for Gate 10 (Performance) regression detection
across PostgreSQL major bumps and extension version bumps. The
aggregate runner is `benchmarks/microbenches/run-all.sh` and the
baseline gate is `benchmarks/microbenches/compare-to-baseline.sh`.
The seed baselines in each `baseline.json` are sourced from upstream
publications; the first nightly run on the 3-worker kind cluster
refines them and lands the measured numbers as a follow-up PR.

### MB1: timescaledb Microbench

**Overlay**: `benchmarks/microbenches/timescaledb/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: 100k-row insert across 7 days into a hypertable; compression runs after the workload.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/timescaledb/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb1-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/timescaledb/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `50,000 rows/s` for `hypertable_insert_rows_per_s`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB1` in `benchmarks/microbenches/timescaledb/setup.sql`
- Executable: `bash benchmarks/microbenches/timescaledb/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB2: citus Microbench

**Overlay**: `benchmarks/microbenches/citus/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `citus`

**Summary**: create_distributed_table + 100k INSERT routed across 3 worker shards via the coordinator.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/citus/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb2-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/citus/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `30,000 rows/s` for `distributed_insert_rows_per_s`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB2` in `benchmarks/microbenches/citus/setup.sql`
- Executable: `bash benchmarks/microbenches/citus/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB3: pgvector Microbench

**Overlay**: `benchmarks/microbenches/pgvector/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgvector`

**Summary**: 1k 768-dim vector INSERT plus 1k IVFFlat ANN lookups against the just-built index.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/pgvector/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb3-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/pgvector/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `2,000 qps` for `ivfflat_insert_then_lookup_qps`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB3` in `benchmarks/microbenches/pgvector/setup.sql`
- Executable: `bash benchmarks/microbenches/pgvector/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB4: pg_cron Microbench

**Overlay**: `benchmarks/microbenches/pg_cron/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_cron`

**Summary**: Schedule 100 jobs at 1-minute frequency through cron.schedule, measuring per-call overhead.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/pg_cron/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb4-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/pg_cron/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `200 schedules/s` for `job_schedule_overhead_ms`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB4` in `benchmarks/microbenches/pg_cron/setup.sql`
- Executable: `bash benchmarks/microbenches/pg_cron/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB5: pg_partman Microbench

**Overlay**: `benchmarks/microbenches/pg_partman/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_partman`

**Summary**: Create 100 child partitions for a range-partitioned parent via partman.create_parent + run_maintenance.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/pg_partman/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb5-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/pg_partman/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `50 partitions/s` for `child_partition_create_ms`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB5` in `benchmarks/microbenches/pg_partman/setup.sql`
- Executable: `bash benchmarks/microbenches/pg_partman/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB6: pgaudit Microbench

**Overlay**: `benchmarks/microbenches/pgaudit/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgaudit`

**Summary**: 10k INSERT under pgaudit.log=write compared to the un-audited baseline; gate at <= 15%.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/pgaudit/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb6-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/pgaudit/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `<= 15% overhead` for `audited_insert_overhead_pct`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB6` in `benchmarks/microbenches/pgaudit/setup.sql`
- Executable: `bash benchmarks/microbenches/pgaudit/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB7: pgsodium Microbench

**Overlay**: `benchmarks/microbenches/pgsodium/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgsodium`

**Summary**: Encrypt 1k rows with crypto_secretbox using a per-row derived key and nonce.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/pgsodium/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb7-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/pgsodium/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `5,000 rows/s` for `libsodium_encrypt_rows_per_s`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB7` in `benchmarks/microbenches/pgsodium/setup.sql`
- Executable: `bash benchmarks/microbenches/pgsodium/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB8: postgresql-hll Microbench

**Overlay**: `benchmarks/microbenches/postgresql-hll/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `postgresql-hll`

**Summary**: hll_add_agg over 100k distinct values into a single hll register.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/postgresql-hll/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb8-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/postgresql-hll/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `200,000 inserts/s` for `hll_add_agg_ms`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB8` in `benchmarks/microbenches/postgresql-hll/setup.sql`
- Executable: `bash benchmarks/microbenches/postgresql-hll/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB9: postgresql-topn Microbench

**Overlay**: `benchmarks/microbenches/postgresql-topn/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `postgresql-topn`

**Summary**: topn_add_agg over 100k rows producing the top-100 ranked entries.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/postgresql-topn/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb9-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/postgresql-topn/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `150,000 inserts/s` for `topn_add_agg_ms`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB9` in `benchmarks/microbenches/postgresql-topn/setup.sql`
- Executable: `bash benchmarks/microbenches/postgresql-topn/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB10: tdigest Microbench

**Overlay**: `benchmarks/microbenches/tdigest/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `tdigest`

**Summary**: tdigest_percentile aggregation over 100k numeric samples returning the 99th percentile.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/tdigest/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb10-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/tdigest/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `100,000 samples/s` for `tdigest_percentile_ms`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB10` in `benchmarks/microbenches/tdigest/setup.sql`
- Executable: `bash benchmarks/microbenches/tdigest/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB11: pgnodemx Microbench

**Overlay**: `benchmarks/microbenches/pgnodemx/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgnodemx`

**Summary**: 1k calls to pgnodemx.cpu() measuring per-invocation cgroup-read overhead.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/pgnodemx/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb11-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/pgnodemx/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `5,000 calls/s` for `pgnodemx_cpu_invocation_us`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB11` in `benchmarks/microbenches/pgnodemx/setup.sql`
- Executable: `bash benchmarks/microbenches/pgnodemx/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB12: postgis Microbench

**Overlay**: `benchmarks/microbenches/postgis/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `postgis`

**Summary**: ST_DWithin lookups against a 100k POINT table with a GIST spatial index.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/postgis/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb12-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/postgis/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `4,000 qps` for `st_dwithin_qps`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB12` in `benchmarks/microbenches/postgis/setup.sql`
- Executable: `bash benchmarks/microbenches/postgis/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB13: pg_search Microbench

**Overlay**: `benchmarks/microbenches/pg_search/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_search`

**Summary**: 100k doc INSERT, BM25 index build, and 1k BM25 lookups.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/pg_search/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb13-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/pg_search/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `3,000 qps` for `bm25_insert_index_lookup_qps`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB13` in `benchmarks/microbenches/pg_search/setup.sql`
- Executable: `bash benchmarks/microbenches/pg_search/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB14: pg_graphql Microbench

**Overlay**: `benchmarks/microbenches/pg_graphql/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_graphql`

**Summary**: GraphQL query joining a 10k-row orders table with a 1k-row customers table through graphql.resolve.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/pg_graphql/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb14-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/pg_graphql/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `1,500 qps` for `graphql_join_qps`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB14` in `benchmarks/microbenches/pg_graphql/setup.sql`
- Executable: `bash benchmarks/microbenches/pg_graphql/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB15: pg_jsonschema Microbench

**Overlay**: `benchmarks/microbenches/pg_jsonschema/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_jsonschema`

**Summary**: Validate 10k JSONB rows against a fixed JSON schema with jsonb_matches_schema.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/pg_jsonschema/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb15-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/pg_jsonschema/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `50,000 valid/s` for `jsonb_validate_per_s`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB15` in `benchmarks/microbenches/pg_jsonschema/setup.sql`
- Executable: `bash benchmarks/microbenches/pg_jsonschema/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB16: age Microbench

**Overlay**: `benchmarks/microbenches/age/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `age`

**Summary**: Cypher query over a 1k-node graph computing 1..2-hop paths and counts.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/age/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb16-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/age/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `800 qps` for `cypher_path_qps`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB16` in `benchmarks/microbenches/age/setup.sql`
- Executable: `bash benchmarks/microbenches/age/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB17: plrust Microbench

**Overlay**: `benchmarks/microbenches/plrust/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `plrust`

**Summary**: Call a trivial plrust function 10k times; reports per-call overhead vs the plpgsql baseline.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/plrust/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb17-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/plrust/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `200,000 calls/s` for `plrust_function_call_us`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB17` in `benchmarks/microbenches/plrust/setup.sql`
- Executable: `bash benchmarks/microbenches/plrust/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB18: plv8 Microbench

**Overlay**: `benchmarks/microbenches/plv8/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `plv8`

**Summary**: Call a trivial plv8 function 10k times; reports per-call overhead vs the plpgsql baseline.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/plv8/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb18-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/plv8/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `100,000 calls/s` for `plv8_function_call_us`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB18` in `benchmarks/microbenches/plv8/setup.sql`
- Executable: `bash benchmarks/microbenches/plv8/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB19: pg_uuidv7 Microbench

**Overlay**: `benchmarks/microbenches/pg_uuidv7/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_uuidv7`

**Summary**: Generate 100k UUIDv7 values through uuid_generate_v7().

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/pg_uuidv7/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb19-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/pg_uuidv7/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `1,000,000 gen/s` for `uuidv7_generations_per_s`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB19` in `benchmarks/microbenches/pg_uuidv7/setup.sql`
- Executable: `bash benchmarks/microbenches/pg_uuidv7/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB20: pg_repack Microbench

**Overlay**: `benchmarks/microbenches/pg_repack/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_repack`

**Summary**: Repack a 100k-row table with synthetic bloat; reports the end-to-end repack duration.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/pg_repack/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb20-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/pg_repack/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `~10 s end-to-end` for `repack_table_seconds`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB20` in `benchmarks/microbenches/pg_repack/setup.sql`
- Executable: `bash benchmarks/microbenches/pg_repack/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB21: pg_failover_slots Microbench

**Overlay**: `benchmarks/microbenches/pg_failover_slots/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_failover_slots`

**Summary**: WAL write overhead under pg_failover_slots tracking; proxy for failover-slot bookkeeping cost.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/pg_failover_slots/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb21-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/pg_failover_slots/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `<= 5% overhead` for `wal_write_overhead_pct`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB21` in `benchmarks/microbenches/pg_failover_slots/setup.sql`
- Executable: `bash benchmarks/microbenches/pg_failover_slots/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB22: pg_warm Microbench

**Overlay**: `benchmarks/microbenches/pg_warm/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_warm`

**Summary**: pg_prewarm a 100k-row table (smoke proxy for the 10 GB full-mode workload).

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/pg_warm/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb22-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/pg_warm/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `~1 GB/s` for `warm_throughput_mb_per_s`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB22` in `benchmarks/microbenches/pg_warm/setup.sql`
- Executable: `bash benchmarks/microbenches/pg_warm/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB23: pgcrypto Microbench

**Overlay**: `benchmarks/microbenches/pgcrypto/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgcrypto`

**Summary**: pgp_sym_encrypt 10k rows with a static passphrase.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/pgcrypto/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb23-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/pgcrypto/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `15,000 rows/s` for `pgp_sym_encrypt_rows_per_s`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB23` in `benchmarks/microbenches/pgcrypto/setup.sql`
- Executable: `bash benchmarks/microbenches/pgcrypto/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB24: pg_trgm Microbench

**Overlay**: `benchmarks/microbenches/pg_trgm/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_trgm`

**Summary**: Trigram similarity lookups against a GIN-trigram index on 100k rows.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/pg_trgm/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb24-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/pg_trgm/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `5,000 qps` for `trigram_similarity_qps`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB24` in `benchmarks/microbenches/pg_trgm/setup.sql`
- Executable: `bash benchmarks/microbenches/pg_trgm/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB25: citext Microbench

**Overlay**: `benchmarks/microbenches/citext/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `citext`

**Summary**: Case-insensitive equality lookup on a 100k-row citext column with a B-tree index.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/citext/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb25-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/citext/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `20,000 qps` for `citext_lookup_qps`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB25` in `benchmarks/microbenches/citext/setup.sql`
- Executable: `bash benchmarks/microbenches/citext/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

### MB26: rum Microbench

**Overlay**: `benchmarks/microbenches/rum/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `rum`

**Summary**: RUM full-text index build plus FTS lookups on 100k documents.

Production evidence: GitHub Actions and local VM runs invoke
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`, which executes
`benchmarks/microbenches/rum/bench.sh` in quick mode against a real PostgreSQL container on the
experiment VM and writes
`benchmarks/results/microbench-mb26-${BENCH_RESULT_TAG}.json`. The
nightly `ci-microbench` workflow runs the full-row-count variant via
`benchmarks/microbenches/run-all.sh` and asserts the measured `qps`
stays inside the `regression_threshold_pct` window of
`benchmarks/microbenches/rum/baseline.json` through
`benchmarks/microbenches/compare-to-baseline.sh`. The initial baseline
seed is `4,000 qps` for `rum_fts_index_build_lookup_qps`; refined baselines land
after the first measured nightly run on the 3-worker kind cluster.

**Motivation**: Gate 10 (Performance) needs per-bundled-extension
regression detection across PostgreSQL major bumps and extension
version bumps. The microbench surface keeps each extension's hot path
on a measured budget instead of relying on the four aggregate
harnesses to surface a regression.

**Citus comparison**: Vanilla Citus does not ship per-bundled-extension
microbenchmarks; the upstream test surface is correctness-only.

**References**:

- Design: `docs/ai-blaise/BENCHMARKS.md`
- In-source: `FEATURE: MB26` in `benchmarks/microbenches/rum/setup.sql`
- Executable: `bash benchmarks/microbenches/rum/bench.sh`
- CI: `ci/ai-blaise/bundled-ext-microbenches-smoke.sh`

## V2 Completion Register Addendum

No rows remain. The former V2 addendum rows were promoted to alpha feature
headings with deterministic executable evidence so the feature register has no
source-only catalog surface left.

| ID | Feature | Overlay | Status | Vanilla Citus comparison | Reference | Evidence |
|---|---|---|---|---|---|---|
