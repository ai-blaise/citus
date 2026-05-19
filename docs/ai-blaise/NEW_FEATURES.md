# New Features Register

This is the canonical register of features that `ai-blaise/citus` adds beyond
vanilla Citus. Every feature-bearing PR updates this file.

## Throughput

### T2: Plan Cache Placement-Generation Invalidation

**Overlay**: `pool/src/shard_map.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Tracks shard placement generations so cached plans can be
invalidated only when the placements they depend on change.

**Motivation**: Rebalance should not wipe the entire plan cache when only a
small subset of shard placements moved.

**Citus comparison**: Vanilla Citus has plan invalidation behavior around shard
movement but does not ship the ai-blaise pool's generation-aware cache model.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: T2` in `pool/src/shard_map.rs`

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
that drive distributed hypertable creation, compression, retention, and
continuous aggregate policy reconciliation.

**Motivation**: The TimescaleDB bridge needs a declarative operator surface so
cluster state can be reconciled repeatedly instead of hand-applied.

**Citus comparison**: Vanilla Citus does not ship a Kubernetes CRD for
Timescale-aware distributed hypertables.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TS7` in `operator/src/crds/hypertable.rs`
- In-source: `FEATURE: TS7` in `operator/src/reconcile/hypertable.rs`
- Acceptance: `FEATURE: TS7` in `e2e/src/timescale_on_citus.rs`

## AI / Vector

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

## Topology

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

## Observability

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
