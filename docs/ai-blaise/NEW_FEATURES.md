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

## Change Data And Branching

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

### B6: Encrypted Backups

**Overlay**: `operator/src/crds/backup.rs`
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

### TO5: Tenant Region Affinity

**Overlay**: `operator/src/crds/tenant.rs`
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
