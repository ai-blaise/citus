# New Features Register

This is the canonical register of features that `ai-blaise/citus` adds beyond
vanilla Citus. Every feature-bearing PR updates this file.

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
- In-source: `FEATURE: TS1` in `companion/src/citus_timescale.rs`

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
- In-source: `FEATURE: TS5` in `companion/src/citus_timescale.rs`

### TS6: Trusted Hook Coextensions

**Overlay**: `patches/0001-allow-trusted-hook-coextensions.patch`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Allows Citus to load after preexisting PostgreSQL hooks when the
operator explicitly configures trusted cohabiting extensions.

**Motivation**: Citus's upstream guard rejects any preexisting planner,
utility, executor, or explain hook. ai-blaise/citus needs a controlled path for
validated cohabitation, starting with TimescaleDB.

**SQL surface / API**:

```conf
citus.cohabit_extensions = 'timescaledb'
```

**Citus comparison**: Vanilla Citus errors if these hooks are already set at
load time.

**References**:

- Design: `docs/ai-blaise/COHABITATION.md`
- In-source marker after patch application:
  `FEATURE: TS6` in `src/backend/distributed/shared_library_init.c`

## Topology

### S4: Coordinator-Less Topology Mode

**Overlay**: `operator/`, `pool/`
**Status**: planned
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
- Feature marker: `FEATURE: S4`

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
