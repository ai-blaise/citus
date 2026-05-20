# New Features Register

This is the canonical register of features that `ai-blaise/citus` adds beyond
vanilla Citus. Every feature-bearing PR updates this file.

`e2e/src/operator_catalog.rs` is the current pure-Rust acceptance model for
the V2 operator catalog. It validates canonical specs for `FEATURE: A8`,
`FEATURE: B2`, `FEATURE: B6`, `FEATURE: C4`, `FEATURE: C5`, `FEATURE: C6`,
`FEATURE: C7`, `FEATURE: C8`, `FEATURE: C9`, `FEATURE: EF3`, `FEATURE: F1`,
`FEATURE: M3`, `FEATURE: MR1`, `FEATURE: MR2`, `FEATURE: MR4`, `FEATURE: MR8`,
`FEATURE: O5`, `FEATURE: R2`, `FEATURE: R7`, `FEATURE: S10`, `FEATURE: S11`,
`FEATURE: Search2`, `FEATURE: Search7`, `FEATURE: TO1`, `FEATURE: TO2`,
`FEATURE: TO5`, and `FEATURE: WH1`.
`e2e/src/runtime_contracts.rs` validates canonical runtime contracts for
`FEATURE: Auth1`, `FEATURE: Auth3`, `FEATURE: B1`, `FEATURE: B3`,
`FEATURE: B4`, `FEATURE: C1`, `FEATURE: L8`, `FEATURE: MR5`, `FEATURE: R7`,
`FEATURE: R10`, `FEATURE: RT1`, `FEATURE: RT2`, `FEATURE: RT3`,
`FEATURE: RT4`, `FEATURE: Search8`, `FEATURE: Sec12`, `FEATURE: Sto1`,
`FEATURE: Sto3`, `FEATURE: Sto4`, `FEATURE: T1`, `FEATURE: T3`, `FEATURE: T9`,
`FEATURE: T12`, `FEATURE: T15`, and `FEATURE: WH3`.
`images/citus-pg-overlay/extension-manifest.tsv` validates the bundled,
optional, and hard-blocked extension image contract for `FEATURE: Bundle1`,
`FEATURE: Search1`, `FEATURE: G1`, `FEATURE: JS1`, `FEATURE: PM1`,
`FEATURE: IA1`, `FEATURE: WF1`, and `FEATURE: F2`.
`sidecar/analytical/src/lib.rs` validates pg_lake/DataFusion/DuckDB,
lakehouse-read, Iceberg snapshot commit, federation, DuckDB extension, and
MotherDuck contracts for `FEATURE: L1`, `FEATURE: L2`, `FEATURE: L3`,
`FEATURE: L4`, `FEATURE: L5`, `FEATURE: L6`, `FEATURE: L8`,
`FEATURE: L12`, and `FEATURE: L13`.
`sidecar/cdc/src/lib.rs` validates logical replication stream, DDL capture,
anonymization, reliable delivery, NATS, and Pub/Sub contracts for
`FEATURE: C1`, `FEATURE: C2`, `FEATURE: C3`, `FEATURE: C14`, `FEATURE: C15`,
`FEATURE: L8`, and `FEATURE: WH3`.
`sidecar/edge_functions/src/lib.rs` validates Deno/Bun runtime launch, UDS
database callback, and triggered invocation contracts for `FEATURE: EF1`,
`FEATURE: EF2`, `FEATURE: EF4`, and `FEATURE: EF5`.
`sidecar/hlc/src/lib.rs` validates hybrid-logical-clock, closed timestamp, and
follower-read contracts for `FEATURE: S9`.
`sidecar/realtime/src/lib.rs` validates CDC-driven broadcast, tenant isolation,
filter, and presence contracts for `FEATURE: RT1`, `FEATURE: RT2`,
`FEATURE: RT3`, and `FEATURE: RT4`.
`sidecar/schema_job/src/lib.rs` validates online-DDL worker leases, backfill,
safety, and gh-ost shadow-table contracts for `FEATURE: C10` and
`FEATURE: M2`.
`sidecar/storage/src/lib.rs` validates object metadata, presigned URL, bucket
ACL, and antivirus contracts for `FEATURE: Sto1`, `FEATURE: Sto3`,
`FEATURE: Sto4`, and `FEATURE: Sto5`.
`sidecar/txn_status/src/lib.rs` validates parallel-commit transaction status,
intent evidence, and 2PC fallback decisions for `FEATURE: T5`.

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

## Throughput

### T1: Settings-Bucket Connection Pool

**Overlay**: `pool/src/runtime.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the pool settings-bucket contract and stable GUC
fingerprint for sharing worker connections across sessions with identical
tracked GUC state.

**Motivation**: Citus deployments need far more client sessions than worker
backends without losing session correctness.

**Citus comparison**: Vanilla Citus does not ship an external settings-bucket
pooler.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: T1` in `pool/src/runtime.rs`

### T2: Plan Cache Placement-Generation Invalidation

**Overlay**: `pool/src/shard_map.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Tracks shard placement generations and cached query fingerprints
so cached plans can be invalidated only when the placements they depend on
change.

**Motivation**: Rebalance should not wipe the entire plan cache when only a
small subset of shard placements moved.

**Citus comparison**: Vanilla Citus has plan invalidation behavior around shard
movement but does not ship the ai-blaise pool's generation-aware cache model.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: T2` in `pool/src/shard_map.rs`

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

### T15: Transaction Pipelining In Pool

**Overlay**: `pool/src/runtime.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines pool protocol pipelining limits for in-flight work and
transaction pipelining.

**Motivation**: Pool throughput work needs an explicit backpressure contract
before pipelining reaches the data path.

**Citus comparison**: Vanilla Citus does not provide an external pool
pipelining contract.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: T15` in `pool/src/runtime.rs`

## TimescaleDB Integration

### TS1: Distributed Hypertable Bridge

**Overlay**: `companion/citus_timescale`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: Provides the SQL surface that distributes a PostgreSQL
declarative-partitioned parent table through Citus while using TimescaleDB
hypertables for worker-local partitions.

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

### TS2: Distributed Compression Policy

**Overlay**: `companion/citus_timescale`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: Adds SQL-plan rendering and a `pg18`-gated pgrx surface for
worker-fanned distributed compression policy creation.

**Motivation**: Distributed hypertables need compression policies that are
declared once and applied consistently across worker-local hypertables.

**Citus comparison**: Vanilla Citus does not fan out TimescaleDB compression
policy setup.

**References**:

- Design: `docs/ai-blaise/COHABITATION.md`
- In-source: `FEATURE: TS2` in `companion/src/citus_timescale.rs`

### TS3: Distributed Continuous Aggregate Partials

**Overlay**: `companion/citus_timescale`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: Adds SQL-plan rendering and a `pg18`-gated pgrx surface for
distributed continuous aggregate definitions and refresh-policy arguments.

**Motivation**: Continuous aggregates must be coordinated through the same
bridge as distributed hypertables so worker partials and coordinator finals are
created predictably.

**Citus comparison**: Vanilla Citus does not orchestrate TimescaleDB continuous
aggregates across shards.

**References**:

- Design: `docs/ai-blaise/COHABITATION.md`
- In-source: `FEATURE: TS3` in `companion/src/citus_timescale.rs`

### TS4: Distributed Retention Policy

**Overlay**: `companion/citus_timescale`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: Adds SQL-plan rendering and a `pg18`-gated pgrx surface for
cluster-wide retention policy setup.

**Motivation**: Retention should drop old chunks across all worker-local
hypertables without requiring operator-authored per-worker SQL.

**Citus comparison**: Vanilla Citus does not provide TimescaleDB retention
policy fanout.

**References**:

- Design: `docs/ai-blaise/COHABITATION.md`
- In-source: `FEATURE: TS4` in `companion/src/citus_timescale.rs`

### TS5: Time-Range Shard Pruner

**Overlay**: `companion/citus_timescale`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: `timescaledb`

**Summary**: Adds planner support that combines Citus shard metadata with
TimescaleDB time dimensions to prune shards for time-bound predicates.

**Motivation**: Distributed hypertables need shard pruning by tenant and time to
avoid scanning irrelevant worker-local hypertable chunks.

**SQL surface / API**:

```sql
SET companion.enable_time_range_shard_pruner = on;
```

**Citus comparison**: Vanilla Citus prunes by distribution metadata, but it does
not consult TimescaleDB dimension slices.

**References**:

- Design: `docs/ai-blaise/COHABITATION.md`
- Acceptance: `e2e/src/timescale_on_citus.rs`
- In-source: `FEATURE: TS5` in `companion/src/citus_timescale.rs`
  and `e2e/src/timescale_on_citus.rs`

### TS6: Trusted Hook Coextensions

**Overlay**:

- `patches/0001-allow-trusted-hook-coextensions.patch`
- `patches/0002-preserve-trusted-hook-chain-state.patch`

**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Allows Citus to load after preexisting PostgreSQL hooks when the
operator explicitly configures trusted cohabiting extensions, then preserves
the captured planner, executor, and non-distributed EXPLAIN hook chain.

**Motivation**: Citus's upstream guard rejects any preexisting planner,
utility, executor, or explain hook. ai-blaise/citus needs a controlled path for
validated cohabitation, starting with TimescaleDB.

**SQL surface / API**:

```conf
citus.cohabit_extensions = 'timescaledb'
```

**Citus comparison**: Vanilla Citus errors if these hooks are already set at
load time. With TS6 enabled, ai-blaise/citus remains the outer Citus hook while
delegating to trusted preexisting hooks where the Citus path can safely do so.

**References**:

- Design: `docs/ai-blaise/COHABITATION.md`
- In-source marker after patch application:
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

**Summary**: Defines the Kubernetes `Hypertable` spec and typed reconcile plan
that drive distributed hypertable creation, compression, retention, continuous
aggregate, and time-range shard-pruner reconciliation through ordered companion
SQL plans.

**Motivation**: The TimescaleDB bridge needs a declarative operator surface so
cluster state can be reconciled repeatedly instead of hand-applied.

**Citus comparison**: Vanilla Citus does not ship a Kubernetes CRD for
Timescale-aware distributed hypertables.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TS7` in `operator/src/crds/hypertable.rs`
- In-source: `FEATURE: TS7` in `operator/src/reconcile/hypertable.rs`
- Acceptance: `FEATURE: TS7` in `e2e/src/timescale_on_citus.rs`

### TS12: Distributed Reorder Policy

**Overlay**: `companion/citus_timescale`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: Adds SQL-plan rendering and a `pg18`-gated pgrx surface for
worker-fanned TimescaleDB reorder policy setup.

**Motivation**: Reorder policies need to target worker-local hypertables while
remaining declarative at the coordinator/operator layer.

**Citus comparison**: Vanilla Citus does not orchestrate TimescaleDB reorder
policies across shards.

**References**:

- Design: `docs/ai-blaise/COHABITATION.md`
- In-source: `FEATURE: TS12` in `companion/src/citus_timescale.rs`

## AI / Vector

### A1: pgai-Compatible Vectorizer DSL

**Overlay**: `companion/src/vector.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgvector`, `timescaledb`

**Summary**: Adds a Rust companion contract that renders a pgai-compatible
`ai.create_vectorizer(...)` SQL plan with loading, chunking, embedding,
destination, scheduling, queue, and usage-log setup.

**Motivation**: pgai's vectorizer DSL is the right user-facing shape, but its
archived Python worker is not a good runtime floor for this fork.

**Citus comparison**: Vanilla Citus has no AI vectorizer DSL or worker queue.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: A1` in `companion/src/vector.rs`

### A2: Vectorizer Worker

**Overlay**: `sidecar/vectorizer`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the vectorizer sidecar's embedding job model and health
surface for future pgai-compatible queue execution.

**Motivation**: pgai's Python worker is archived and coordinator-oriented. The
fork needs a Rust worker model that can run per Citus worker.

**Citus comparison**: Vanilla Citus does not ship an embedding worker.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: A2` in `sidecar/vectorizer/src/lib.rs`

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

### A5: Vectorizer Usage Accounting

**Overlay**: `sidecar/vectorizer`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: Adds usage records with tenant, provider, model, token, and
micro-cost accounting for future `ai.usage_log` writes.

**Motivation**: Cost dashboards and token budgets require a durable accounting
shape before provider calls run in production.

**Citus comparison**: Vanilla Citus does not account for embedding provider
usage.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: A5` in `sidecar/vectorizer/src/lib.rs`

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

## Topology

### S2: Topology-Aware Placement

**Overlay**: `operator/src/crds/shard_group.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Defines the `ShardGroup` placement policy surface used to keep
replicated shard placements spread across topology domains such as Kubernetes
zones.

**Motivation**: Placement decisions need an operator-owned policy before the
fork can prove zone-aware replication and survival-goal behavior.

**Citus comparison**: Vanilla Citus tracks placements but does not ship a
Kubernetes-native CRD for topology spread constraints.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: S2` in `operator/src/crds/shard_group.rs`
- Acceptance: `e2e/src/timescale_on_citus.rs`

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

### S5: Raft Per Shard Group

**Overlay**: `sidecar/raft`, `operator/`
**Status**: planned
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
- Feature marker: `FEATURE: S5`

### S6: Per-Shard Placement Generation

**Overlay**: `companion/src/router_assist.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Defines companion-side placement generation and local-placement
query contracts used by plan-cache invalidation and router fast paths.

**Motivation**: Pool and companion routing need stable helper APIs before
placement-generation invalidation can move beyond the pool model.

**Citus comparison**: Vanilla Citus tracks shard placements but does not
expose these helper contracts as companion APIs.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: S6` in `companion/src/router_assist.rs`

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

### S13: Range-Based Dynamic Sharding

**Overlay**: `companion/src/router_assist.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Adds hash and range routing plan shapes so companion and pool code
can reason about non-hash shard assignment through one API.

**Motivation**: Dynamic sharding needs a router contract before planner and
operator work can safely mix hash and range distribution.

**Citus comparison**: Vanilla Citus primarily exposes hash distribution
contracts and does not ship this range-routing helper surface.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: S13` in `companion/src/router_assist.rs`

### S14: Tenant Migration Online

**Overlay**: `companion/src/tenants.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Defines tenant move plans between workers with optional region
affinity preservation.

**Motivation**: Tenant moves must be represented as validated plans before the
operator and companion coordinate online migration.

**Citus comparison**: Vanilla Citus can rebalance shards but does not expose a
tenant-level online migration plan.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: S14` in `companion/src/tenants.rs`

## Resource Efficiency

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

### R4: Idle-In-Transaction Reaper

**Overlay**: `companion/src/observability.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines a guardrail plan for logging, canceling, or terminating
sessions that sit idle in transaction beyond a configured limit.

**Motivation**: Distributed transactions can hold locks and snapshots across
workers; stale idle transactions need a predictable mitigation contract.

**Citus comparison**: Vanilla Citus does not ship an idle-transaction reaper
helper.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: R4` in `companion/src/observability.rs`

### R7: REPACK CONCURRENTLY Adoption

**Overlay**: `operator/src/crds/scheduled_repack.rs`, `sidecar/shared/src/contracts.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: `pg_repack`

**Summary**: Defines the scheduled repack policy surface for online shard-table
maintenance, with strategy selection for `pg_repack` and future PostgreSQL 19
`REPACK CONCURRENTLY`.

**Motivation**: Repack cadence and target tables need to be auditable and
reconciled rather than run as one-off maintenance commands.

**Citus comparison**: Vanilla Citus can use external maintenance tooling but
does not provide a scheduled repack CRD.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: R7` in `operator/src/crds/scheduled_repack.rs`
- In-source: `FEATURE: R7` in `sidecar/shared/src/contracts.rs`

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

## Change Data And Branching

### C4: Active-Active Conflict Policy

**Overlay**: `operator/src/crds/conflict_policy.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgactive`

**Summary**: Defines table-scoped conflict policy for future active-active
reference-table replication.

**Motivation**: Cross-region writes need explicit resolution rules before
replication can be enabled safely.

**Citus comparison**: Vanilla Citus does not ship active-active conflict
policy objects.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C4` in `operator/src/crds/conflict_policy.rs`

### C5: Replication Conflict Taxonomy

**Overlay**: `operator/src/crds/conflict_policy.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `spock`

**Summary**: Carries the seven conflict classes used by the future
replication-conflict companion and active-active reconcilers.

**Motivation**: Conflict resolution cannot be observable or testable if all
conflicts collapse into one undifferentiated failure state.

**Citus comparison**: Vanilla Citus does not expose a Spock-style conflict
classification contract.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C5` in `operator/src/crds/conflict_policy.rs`

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

### C8: Branch Promote

**Overlay**: `operator/src/crds/branch.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Establishes typed branch identity and source-cluster state for a
future atomic branch promotion reconciler.

**Motivation**: Promote/cut-over workflows need the same branch object that
created and suspended the branch, so status and ownership stay consistent.

**Citus comparison**: Vanilla Citus does not provide branch promotion.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C8` in `operator/src/crds/branch.rs`

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

### C10: Online DDL State Machine

**Overlay**: `companion/src/schema_jobs.rs`, `sidecar/schema_job`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds the schema-job state machine for `DELETE_ONLY`,
`WRITE_ONLY`, `BACKFILL`, and `PUBLIC` transitions.

**Motivation**: Online schema changes need a validated state model before the
operator and schema-job sidecar can coordinate DDL safely.

**Citus comparison**: Vanilla Citus does not ship an F1-style schema-change
state machine.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C10` in `companion/src/schema_jobs.rs`
- In-source: `FEATURE: C10` in `sidecar/schema_job/src/lib.rs`

### C1: CDC Sidecar

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/cdc`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines logical-replication slot, publication, sink, and retry
contracts for the CDC sidecar.

**Motivation**: Realtime, webhooks, analytical mirrors, and external sinks all
need one validated CDC stream contract.

**Citus comparison**: Vanilla Citus does not ship an out-of-process CDC
sidecar.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C1` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: C1` in `sidecar/cdc/src/lib.rs`

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

## Migrations

### M2: gh-ost-Style Online DDL

**Overlay**: `companion/src/schema_jobs.rs`, `sidecar/schema_job`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the schema-job operation/state model used by future
trigger and backfill based online DDL.

**Motivation**: Online DDL needs explicit state transitions and lease
validation before a sidecar or companion UDF can execute it.

**Citus comparison**: Vanilla Citus has distributed DDL but does not provide
gh-ost-style online DDL state machinery.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: M2` in `companion/src/schema_jobs.rs`
- In-source: `FEATURE: M2` in `sidecar/schema_job/src/lib.rs`

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

### MR8: Leader Pinning Per Region

**Overlay**: `operator/src/crds/region.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Carries leader-pinning intent on regions so future HA reconcilers
can constrain primaries to chosen failure domains.

**Motivation**: Multi-region clusters need explicit write-leader placement to
control latency and failover behavior.

**Citus comparison**: Vanilla Citus leaves primary placement to external HA
tooling.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: MR8` in `operator/src/crds/region.rs`

## Backup / PITR

### B1: Backup Sidecar

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/backup`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines schedule and archive URI contracts for the backup sidecar.

**Motivation**: Backup execution needs a sidecar contract that matches the
operator CRD before WAL archive implementation begins.

**Citus comparison**: Vanilla Citus delegates backup sidecars to deployment
tooling.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: B1` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: B1` in `sidecar/backup/src/lib.rs`

### B2: Backup CRD

**Overlay**: `operator/src/crds/backup.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines backup schedule, retention, object-store target, and
provider for the future backup sidecar reconciler.

**Motivation**: PITR and backup-as-data-source workflows need an auditable
declarative schedule.

**Citus comparison**: Vanilla Citus does not ship a cluster backup CRD.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: B2` in `operator/src/crds/backup.rs`

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

### TO3: Tenant Migration Online

**Overlay**: `companion/src/tenants.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Defines tenant move plans that carry source worker, target
worker, and optional region affinity.

**Motivation**: Tenant movement needs a typed plan that can be validated before
rebalance, pool draining, and schema routing are coordinated.

**Citus comparison**: Vanilla Citus rebalances shards, but does not expose a
tenant-level online move contract.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TO3` in `companion/src/tenants.rs`

### TO4: Tenant Archive

**Overlay**: `companion/src/tenants.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines tenant archive plans with destination URI and retention
policy.

**Motivation**: Tenant offboarding needs an auditable archive operation before
data removal can be automated.

**Citus comparison**: Vanilla Citus does not include tenant archive
automation.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TO4` in `companion/src/tenants.rs`

### TO5: Tenant Region Affinity

**Overlay**: `operator/src/crds/tenant.rs`, `companion/src/tenants.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Records the preferred region for a tenant so placement and
migration reconcilers can keep tenant data close to its users.

**Motivation**: Region affinity needs to be part of tenant intent, not hidden
inside one-off placement annotations.

**Citus comparison**: Vanilla Citus does not model tenant-region affinity.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TO5` in `operator/src/crds/tenant.rs`
- In-source: `FEATURE: TO5` in `companion/src/tenants.rs`

## Search

### Search2: Distributed BM25 Index

**Overlay**: `operator/src/crds/search_index.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_search`

**Summary**: Defines distributed search-index intent with text/vector column
roles and BM25 or hybrid scoring.

**Motivation**: Search indexes must be declared once and fanned out across
workers without losing table ownership or scorer semantics.

**Citus comparison**: Vanilla Citus does not ship a distributed BM25 search
index CRD.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Search2` in `operator/src/crds/search_index.rs`

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

### Search8: Search-Aware Cold Tier

**Overlay**: `sidecar/shared/src/contracts.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds search-index enablement to the analytical mirror contract so
cold-tier data can preserve search semantics.

**Motivation**: Cold-tier movement should not discard full-text or hybrid
search availability.

**Citus comparison**: Vanilla Citus does not manage search-aware cold-tier
mirrors.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Search8` in `sidecar/shared/src/contracts.rs`

## HTAP

### L1: pg_lake Analytical Substrate

**Overlay**: `sidecar/analytical`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_lake`

**Summary**: Defines the analytical sidecar plan that binds a logical mirror
to a lakehouse read path.

**Motivation**: HTAP routing needs a concrete sidecar contract before pg_lake
or equivalent execution is wired into queries.

**Citus comparison**: Vanilla Citus does not ship a pg_lake-backed analytical
sidecar.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L1` in `sidecar/analytical/src/lib.rs`

### L2: Rust Analytical Server

**Overlay**: `sidecar/analytical`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines Rust-native analytical engine selection for DataFusion,
DuckDB, or pg_lake-backed execution.

**Motivation**: The analytical path should avoid a Python server in the hot
query path.

**Citus comparison**: Vanilla Citus does not include an out-of-process Rust
analytical server.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L2` in `sidecar/analytical/src/lib.rs`

### L3: Iceberg, Parquet, and Delta Reads

**Overlay**: `sidecar/analytical`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_lake`, `pg_parquet`

**Summary**: Defines the lakehouse read plan for Iceberg, Parquet, and Delta
objects.

**Motivation**: Warm and cold analytical storage needs one validated format and
object-URI contract before execution engines fan out reads.

**Citus comparison**: Vanilla Citus does not read Iceberg, Parquet, or Delta
tables through a sidecar.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L3` in `sidecar/analytical/src/lib.rs`

### L4: DataFusion Pushdown

**Overlay**: `sidecar/analytical`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines projected-column, predicate, and limit pushdown contracts
for DataFusion execution.

**Motivation**: Analytical execution has to preserve pool and planner
predicate intent instead of scanning full object-store tables.

**Citus comparison**: Vanilla Citus does not push plans into DataFusion.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L4` in `sidecar/analytical/src/lib.rs`

### L5: Iceberg Snapshot Commit At Prepare

**Overlay**: `sidecar/analytical`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines transaction, snapshot, prepare-LSN, and manifest URI
contracts for aligning Iceberg snapshot commits with distributed prepare.

**Motivation**: Warm-tier visibility must line up with Citus distributed
transaction boundaries.

**Citus comparison**: Vanilla Citus has no Iceberg snapshot commit protocol.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L5` in `sidecar/analytical/src/lib.rs`

### L6: Lakehouse Federation Catalogs

**Overlay**: `sidecar/analytical`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines external Iceberg catalog publication targets for
Snowflake, Trino, Spark, and Databricks.

**Motivation**: External analytical readers need a stable federation contract
without learning Citus shard placement directly.

**Citus comparison**: Vanilla Citus does not publish lakehouse catalogs for
external engines.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L6` in `sidecar/analytical/src/lib.rs`

### L8: Mooncake-Style Logical-Replication Mirror

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/cdc`, `sidecar/analytical`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the analytical mirror contract binding a CDC slot to
mirror name and object-storage URI.

**Motivation**: HTAP without dual-write requires a validated mirror stream
before analytical sidecars materialize warm columnar copies.

**Citus comparison**: Vanilla Citus does not ship a logical-replication
analytical mirror.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L8` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: L8` in `sidecar/cdc/src/lib.rs`
- In-source: `FEATURE: L8` in `sidecar/analytical/src/lib.rs`

### L12: DuckDB Extension Catalog

**Overlay**: `sidecar/analytical`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_duckdb`

**Summary**: Defines the allow-list of DuckDB extensions that analytical
sidecars may enable.

**Motivation**: DuckDB extension use needs to be explicit before sidecars load
code from extension repositories.

**Citus comparison**: Vanilla Citus does not manage DuckDB extension catalogs.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L12` in `sidecar/analytical/src/lib.rs`

### L13: MotherDuck Connector

**Overlay**: `sidecar/analytical`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_duckdb`

**Summary**: Defines MotherDuck database and token-secret binding for optional
cloud analytical routing.

**Motivation**: MotherDuck connectivity should be an explicit opt-in secret
binding rather than an ambient runtime setting.

**Citus comparison**: Vanilla Citus does not include a MotherDuck connector.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L13` in `sidecar/analytical/src/lib.rs`

## Realtime

### RT1: Realtime Sidecar

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/realtime`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines realtime WebSocket topic contracts fed by CDC events.

**Motivation**: Realtime broadcasts need typed topic and tenant binding before
the WebSocket sidecar is implemented.

**Citus comparison**: Vanilla Citus does not ship realtime WebSocket
broadcasts.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: RT1` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: RT1` in `sidecar/realtime/src/lib.rs`

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

## Edge Functions

### EF1: Deno Runtime Sidecar

**Overlay**: `sidecar/edge_functions`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `deno`

**Summary**: Defines Deno runtime launch plans for HTTP, scheduled, and
CDC-triggered edge functions.

**Motivation**: Edge functions need a typed runtime contract before the
sidecar starts executing user code.

**Citus comparison**: Vanilla Citus does not ship a Deno edge-function
runtime.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: EF1` in `sidecar/edge_functions/src/lib.rs`

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

## Security / Auth

### Auth1: JWT-Issuing Service

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/auth`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines issuer, signing key reference, token TTL, and tenant claim
contract for the auth sidecar.

**Motivation**: SQL helpers and the pool need the same token contract before
the auth sidecar starts issuing JWTs.

**Citus comparison**: Vanilla Citus does not ship a JWT issuer.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Auth1` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: Auth1` in `sidecar/auth/src/lib.rs`

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

### Sec1: RLS Helpers

**Overlay**: `companion/src/auth.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Defines the tenant RLS policy plan used by future `auth.*`
companion UDFs.

**Motivation**: Tenant-safe auto-API and pool integration need one validated
mapping from session claims to tenant columns.

**Citus comparison**: Vanilla Citus supports PostgreSQL RLS but does not ship
tenant-aware helper UDFs.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Sec1` in `companion/src/auth.rs`

### Sec2: JWT Verification UDF

**Overlay**: `companion/src/auth.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines issuer, audience, and JWKS secret binding for SQL-visible
JWT verification.

**Motivation**: Auth sidecars and SQL helpers need the same verified claim
contract to avoid split-brain authorization behavior.

**Citus comparison**: Vanilla Citus does not provide JWT verification helpers.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Sec2` in `companion/src/auth.rs`

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

### Auth2: Tenant-Aware Claims

**Overlay**: `companion/src/auth.rs`, `sidecar/auth`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the session-claim shape carrying `uid`, `role`,
`tenant_id`, and optional JWT ID.

**Motivation**: Pool, sidecar, and SQL helper code must agree on tenant claim
names before RLS enforcement is wired through.

**Citus comparison**: Vanilla Citus does not model application tenant claims.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Auth2` in `companion/src/auth.rs`
- In-source: `FEATURE: Auth2` in `sidecar/auth/src/lib.rs`

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

## Storage

### Sto1: Storage Sidecar

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/storage`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines bucket and metadata-table contracts for the storage
sidecar.

**Motivation**: S3-compatible file storage needs a stable table and bucket
mapping before upload/download paths are implemented.

**Citus comparison**: Vanilla Citus does not ship an object storage sidecar.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Sto1` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: Sto1` in `sidecar/storage/src/lib.rs`

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

### Sto4: Bucket-Level ACLs

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/storage`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Carries tenant-column ACL binding for object metadata rows.

**Motivation**: Storage ACLs must line up with tenant RLS rather than live only
in object-store policy.

**Citus comparison**: Vanilla Citus does not manage storage ACLs.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Sto4` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: Sto4` in `sidecar/storage/src/lib.rs`

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

## MCP

### MCP1: citus-mcp Server

**Overlay**: `tools/citus-mcp`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the Model Context Protocol tool request contract for
cluster inspection and guarded operations.

**Motivation**: AI agents need a narrow, typed operation surface rather than
direct database or Kubernetes access.

**Citus comparison**: Vanilla Citus does not ship MCP tooling.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: MCP1` in `tools/citus-mcp/src/lib.rs`

### MCP2: Safe-Mode Tools

**Overlay**: `tools/citus-mcp`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds safe-mode validation that denies destructive MCP tools by
default.

**Motivation**: Agent operations should be inspect-first and dry-run-biased
unless explicitly allowed.

**Citus comparison**: Vanilla Citus does not provide safe-mode agent tooling.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: MCP2` in `tools/citus-mcp/src/lib.rs`

### MCP3: Tenant-Scoped Tools

**Overlay**: `tools/citus-mcp`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds tenant scope and allowed-schema validation to MCP tool
requests.

**Motivation**: Agent-visible tools must enforce tenant boundaries before
multi-tenant production usage.

**Citus comparison**: Vanilla Citus has no tenant-scoped AI-agent tool layer.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: MCP3` in `tools/citus-mcp/src/lib.rs`

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

### D2: citusctl apply

**Overlay**: `tools/citusctl`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Requires an explicit plan ID before apply-mode CLI execution.

**Motivation**: Mutating operations should only run from a reviewed plan so
operator and CI behavior stay auditable.

**Citus comparison**: Vanilla Citus does not ship this plan-gated apply
workflow.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: D2` in `tools/citusctl/src/lib.rs`

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

## Observability

### O1: Query Percentile Views

**Overlay**: `companion/src/observability.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: `pg_stat_statements`

**Summary**: Defines the plan for companion percentile views over
`pg_stat_statements` latency data.

**Motivation**: Production operators need p95/p99/p99.9 query latency without
building one-off SQL at each installation.

**Citus comparison**: Vanilla Citus exposes distributed execution stats but
does not ship this percentile view contract.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: O1` in `companion/src/observability.rs`

### O2: Distributed Stats View

**Overlay**: `companion/src/observability.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Defines the cross-worker distributed stats view contract used by
the future companion observability SQL.

**Motivation**: Operators need one view of coordinator and worker behavior to
debug distributed plans.

**Citus comparison**: Vanilla Citus exposes many stats views, but not this
single companion-owned rollup contract.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: O2` in `companion/src/observability.rs`

### O3: Distributed Replication Lag View

**Overlay**: `companion/src/observability.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Defines the replication-lag view plan with region coverage and
lag budget.

**Motivation**: Multi-region and follower-read features need one companion
surface for lag budgets before HA gates can assert readiness.

**Citus comparison**: Vanilla Citus does not provide an ai-blaise regional lag
view contract.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: O3` in `companion/src/observability.rs`

### O4: Sidecar Health And Metrics Contract

**Overlay**: `sidecar/shared`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines shared sidecar health, readiness, and drain state used by
the future sidecar HTTP/gRPC health and metrics endpoints.

**Motivation**: All ai-blaise sidecars need the same readiness semantics before
they can safely participate in Kubernetes rollout, drain, and chaos gates.

**Citus comparison**: Vanilla Citus does not ship out-of-process Rust sidecars
or a sidecar health contract.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: O4` in `sidecar/shared/src/lib.rs`

### O5: OpenTelemetry Traces And Sidecar Deployment Contract

**Overlay**: `operator/src/crds/sidecar.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the operator-facing sidecar deployment contract for
replicas, resources, and type-specific configuration across the V2 sidecar
surface.

**Motivation**: Traces and rollout behavior are only useful if every sidecar is
declared and reconciled through a consistent resource contract.

**Citus comparison**: Vanilla Citus does not ship out-of-process sidecar
deployment objects.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: O5` in `operator/src/crds/sidecar.rs`
