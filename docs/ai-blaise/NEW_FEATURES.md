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
- Executable: `cargo run -p ai_blaise_citus_pool -- run-canonical`

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

### T8: Toolkit Two-Step Aggregate Pushdown

**Overlay**: `companion/src/toolkit_distributed.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: `timescaledb_toolkit`

**Summary**: Defines worker partial and coordinator finalize SQL plans for
Toolkit two-step aggregates.

**Motivation**: Toolkit aggregates should execute shard-local partials before
coordinator finalization so time-series rollups do not collapse back to a
single-node plan.

**Citus comparison**: Vanilla Citus can distribute many aggregates, but it
does not ship a Toolkit-specific two-step aggregate bridge.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: T8` in `companion/src/toolkit_distributed.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

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
transaction-batching, shard-aware routing, and `FEATURE: T7` source-only
pipeline contract remain alpha.

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
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds companion DB-doctor rules for Timescale/Citus cohabitation,
non-colocated joins, missing distribution columns, hypertable bridge state,
and chunk interval drift.

**Motivation**: Cohabiting extensions need a SQL-visible preflight and lint
surface so accidental violations are caught before migrations mutate schema.

**Citus comparison**: Vanilla Citus does not ship pglinter-style,
Timescale-aware cohabitation doctor rules.

**References**:

- Design: `docs/ai-blaise/COHABITATION.md`
- In-source: `FEATURE: TS9` in `companion/src/db_doctor.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

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
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb_toolkit`

**Summary**: Adds SQL-plan contracts for shard-local gapfill with coordinator
interpolate/locf finalization.

**Motivation**: Time-series dashboards need gapfill across shards without
moving raw samples to the coordinator.

**Citus comparison**: Vanilla Citus does not provide a dedicated distributed
gapfill bridge.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TS13` in `companion/src/toolkit_distributed.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

### TS14: Distributed Metric Toolkit Aggregates

**Overlay**: `companion/src/toolkit_distributed.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb_toolkit`

**Summary**: Adds distributed contracts for counter, gauge, and heartbeat
Toolkit aggregates.

**Motivation**: Metric rollups should use Toolkit's partial/final model while
preserving Citus shard locality.

**Citus comparison**: Vanilla Citus does not ship first-class Toolkit metric
aggregate orchestration.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TS14` in `companion/src/toolkit_distributed.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

### TS15: Distributed Approximate Toolkit Aggregates

**Overlay**: `companion/src/toolkit_distributed.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: `timescaledb_toolkit`

**Summary**: Adds distributed contracts for percentile and frequency Toolkit
aggregate rollups.

**Motivation**: Approximate analytics should keep sketches shard-local until
the final coordinator merge.

**Citus comparison**: Vanilla Citus has aggregate pushdown, but not this
Toolkit-specific approximate aggregate catalog.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TS15` in `companion/src/toolkit_distributed.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

### TS16: Distributed Toolkit Downsamplers

**Overlay**: `companion/src/toolkit_distributed.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb_toolkit`

**Summary**: Adds distributed contracts for ASAP smoothing and LTTB
downsampling.

**Motivation**: Downsampling needs to occur close to shard data before
coordinator rendering.

**Citus comparison**: Vanilla Citus does not provide Toolkit-aware
downsampling orchestration.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TS16` in `companion/src/toolkit_distributed.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

### TS17: Distributed Toolkit State Aggregates

**Overlay**: `companion/src/toolkit_distributed.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb_toolkit`

**Summary**: Adds distributed contracts for candlestick, state, and range
Toolkit aggregates.

**Motivation**: Finance, state-machine, and range analytics need the same
worker-partial/coordinator-final pattern as other Toolkit aggregates.

**Citus comparison**: Vanilla Citus does not bundle this Toolkit aggregate
surface.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TS17` in `companion/src/toolkit_distributed.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

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
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

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
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

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
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Defines companion-side placement generation and local-placement
query contracts used by plan-cache invalidation and router fast paths.

**Motivation**: Pool and companion routing need versioned helper APIs before
placement-generation invalidation can move beyond the pool model.

**Citus comparison**: Vanilla Citus tracks shard placements but does not
expose these helper contracts as companion APIs.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: S6` in `companion/src/router_assist.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

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
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

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
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

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
- Executable: `cargo run -p ai_blaise_citus_sidecar_schema_job -- run-canonical`

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
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines companion SQL-plan contracts for expand/contract
migrations with bounded lock timeout and backfill batch settings.

**Motivation**: Type changes, adds, drops, and renames need a reviewed
migration unit before schema-job workers and operator CRDs execute them.

**Citus comparison**: Vanilla Citus supports distributed DDL, but it does not
ship a pgroll-style expand/contract migration layer.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: M1` in `companion/src/migration.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

### M2: gh-ost-Style Online DDL

**Overlay**: `companion/src/schema_jobs.rs`, `sidecar/schema_job`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the schema-job operation/state model used by trigger and
backfill based online DDL.

**Motivation**: Online DDL needs explicit state transitions and lease
validation before a sidecar or companion UDF can execute it.

**Citus comparison**: Vanilla Citus has distributed DDL but does not provide
gh-ost-style online DDL state machinery.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: M2` in `companion/src/schema_jobs.rs`
- In-source: `FEATURE: M2` in `sidecar/schema_job/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

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
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the companion preflight contract for verifying required
shared-preload coextensions before cohabiting migrations run.

**Motivation**: Operator and migration flows must refuse bad preload state
before they install Timescale or other hook-using extension surfaces.

**Citus comparison**: Vanilla Citus enforces its load-time hook guard, but it
does not provide this controlled cohabitation preflight.

**References**:

- Design: `docs/ai-blaise/COHABITATION.md`
- In-source: `FEATURE: M7` in `companion/src/db_doctor.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

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
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines schema visualization data for distribution, hypertable,
search-index, webhook, and operator shard-placement overlays.

**Motivation**: Distributed schema design needs visual output that shows shard
and extension-specific state rather than only ordinary table relationships.

**Citus comparison**: Vanilla Citus does not ship a visual schema designer or
operator shard-map overlay model.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: M9` in `tools/citus-schema-designer/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_schema_designer -- run-canonical`

### M11: Online Column-Type Migration

**Overlay**: `companion/src/migration.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the online type-change operation used by companion
migration plans for type promotion without blocking table rewrites.

**Motivation**: Large distributed tables need type migrations that can expand,
backfill, and contract without a long exclusive lock.

**Citus comparison**: Vanilla Citus can run distributed DDL, but it does not
ship an online column-type migration contract.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: M11` in `companion/src/migration.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

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
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

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
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

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
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

## Search

### Search2: Distributed BM25 Index

**Overlay**: `operator/src/crds/search_index.rs`, `companion/src/search_bridge.rs`
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
- In-source: `FEATURE: Search2` in `companion/src/search_bridge.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`

### Search3: Hybrid BM25 + Vector Ranking

**Overlay**: `companion/src/search_bridge.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_search`, `pgvector`

**Summary**: Defines the companion SQL-plan contract that combines BM25 and
vector scores into one hybrid rank over distributed tables.

**Motivation**: Hybrid search needs one coordinator-visible ranking contract
while BM25 and vector indexes remain worker-local.

**Citus comparison**: Vanilla Citus does not ship a hybrid search ranker.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Search3` in `companion/src/search_bridge.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

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
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines provider/model/limit planning for reranking top hybrid
search results.

**Motivation**: Reranking should be explicit and auditable before LLM-provider
calls are wired into the search path.

**Citus comparison**: Vanilla Citus does not provide a search reranker UDF.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Search9` in `companion/src/search_bridge.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

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
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: `timescaledb_toolkit`

**Summary**: Defines the companion contract that keeps aggregate partials on
workers and only sends mergeable states to the coordinator.

**Motivation**: HTAP rollups need to reduce network and coordinator CPU by
finalizing after worker partials.

**Citus comparison**: Vanilla Citus supports aggregate pushdown generally, but
not this explicit Toolkit/HTAP aggregate bridge.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L9` in `companion/src/toolkit_distributed.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

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
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_graphql`

**Summary**: Binds GraphQL types to distributed tables, distribution columns,
and companion routing functions.

**Motivation**: GraphQL queries over distributed tables need explicit routing
metadata instead of relying on generic single-node table assumptions.

**Citus comparison**: Vanilla Citus does not provide GraphQL routing helpers
for distributed tables.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: API4` in `sidecar/graphql/src/lib.rs`
- In-source: `FEATURE: API4` in `companion/src/graph_bridge.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

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
generation, JWT verification, pool authentication, and auto-API integration
remain alpha until independently proven.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Sec1` in `companion/src/auth.rs`
- SQL runtime: `FEATURE: Sec1` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

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
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

### Sec5: Immutable Ledger

**Overlay**: `companion/src/ledger.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgledger`

**Summary**: Defines append-only transfer planning and hash-chain validation
for the companion ledger surface.

**Motivation**: Audit-heavy tenant operations need a tamper-evident record
before automated migrations, tenant moves, and privileged actions execute.

**Citus comparison**: Vanilla Citus does not ship an immutable ledger surface.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Sec5` in `companion/src/ledger.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

### Sec6: HMAC Tamper-Evidence On Ledger

**Overlay**: `companion/src/ledger.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgledger`, `pgcrypto`

**Summary**: Defines the `companion_ledger_seal` plan contract that seals a
ledger transfer with an external secret reference and HMAC algorithm.

**Motivation**: Ledger rows need a separable integrity seal so compromised
database writes are detectable against an out-of-band secret.

**Citus comparison**: Vanilla Citus does not provide HMAC-sealed ledger
entries.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Sec6` in `companion/src/ledger.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

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

**Overlay**: `pool/src/proxy.rs`, `deploy/k8s/helm/citus-overlay`
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
and verifies rejected-connection metrics from live pool pods. The Helm deploy
contract also renders `pool-networkpolicy.yaml` for the same allowlist.

**References**:

- In-source: `FEATURE: Sec13` in `pool/src/proxy.rs`
- Helm: `FEATURE: Sec13` in
  `deploy/k8s/helm/citus-overlay/templates/pool-networkpolicy.yaml`
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
`uid` claims are rejected. Auth1 JWT issuance, Sec2 JWT verification, and
Auth3 token caching remain alpha until their own runtime evidence exists.

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
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_hint_plan`, `sr_plan`

**Summary**: Defines companion SQL-plan contracts for freezing a repeatable
plan, binding it to a hint set, and auto-promoting it after enough consistent
executions.

**Motivation**: Planner changes in a distributed database need an explicit
escape hatch for latency-sensitive tenant queries before a regression reaches
users.

**Citus comparison**: Vanilla Citus does not ship a plan-freeze companion
module or auto-promotion policy.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: PM3` in `companion/src/plan_freeze.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

### PM4: Plan Regression Detection

**Overlay**: `companion/src/plan_freeze.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: `pg_hint_plan`, `sr_plan`

**Summary**: Adds latency and cost regression policy evaluation for frozen and
candidate plans.

**Motivation**: Auto-promoted plans need a measurable guardrail that flags
candidate regressions before they replace a known-good plan.

**Citus comparison**: Vanilla Citus exposes plans and costs, but it does not
ship this persistent regression detector.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: PM4` in `companion/src/plan_freeze.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

## Index Advisor

### IA3: Companion Advisor

**Overlay**: `companion/src/index_advisor.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `hypopg`, `pg_qualstats`

**Summary**: Defines ranked index-advisor output that emits
`CREATE INDEX CONCURRENTLY` scripts from what-if cost deltas and predicate
counts.

**Motivation**: Operators need reviewable index suggestions that rank real
workload benefit before applying changes to distributed tables.

**Citus comparison**: Vanilla Citus does not ship a HypoPG/pg_qualstats-backed
index advisor.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: IA3` in `companion/src/index_advisor.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

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
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines companion webhook registration and trigger-install SQL
plans for `INSERT`, `UPDATE`, and `DELETE` events.

**Motivation**: Declarative webhook CRDs need a companion SQL surface that
turns table/event/url configuration into queue-backed triggers.

**Citus comparison**: Vanilla Citus does not install outbound HTTP trigger
helpers.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: WH2` in `companion/src/webhooks.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

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

**Summary**: Defines the Model Context Protocol tool request contract for
cluster inspection and guarded operations, plus a runnable canonical sidecar
session emitter.

**Motivation**: AI agents need a narrow, typed operation surface rather than
direct database or Kubernetes access.

**Citus comparison**: Vanilla Citus does not ship MCP tooling.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: MCP1` in `tools/citus-mcp/src/lib.rs`
- In-source: `FEATURE: MCP1` in `sidecar/mcp/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_mcp -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_mcp -- run-canonical`

### MCP2: Safe-Mode Tools

**Overlay**: `tools/citus-mcp`, `sidecar/mcp`
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
- In-source: `FEATURE: MCP2` in `sidecar/mcp/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_mcp -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_mcp -- run-canonical`

### MCP3: Tenant-Scoped Tools

**Overlay**: `tools/citus-mcp`, `sidecar/mcp`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds tenant scope and allowed-schema validation to MCP tool
requests.

**Motivation**: Agent-visible tools must enforce tenant boundaries before
multi-tenant operator usage.

**Citus comparison**: Vanilla Citus has no tenant-scoped AI-agent tool layer.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: MCP3` in `tools/citus-mcp/src/lib.rs`
- In-source: `FEATURE: MCP3` in `sidecar/mcp/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_mcp -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_mcp -- run-canonical`

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
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the initial contract for the rainfrog-based terminal UI
shell with Citus-specific panels and guarded operator actions.

**Motivation**: Operators need an interactive terminal workflow that can inspect
cluster topology, shards, hypertables, search indexes, vectorizer backlog,
tenants, and branches while keeping mutating workflows behind explicit safety
gates.

**Citus comparison**: Vanilla Citus does not include an interactive terminal
administration shell.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: D3` in `tools/citus-tui/src/lib.rs`
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
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the initial route and action contract for the WhoDB-based
web administration UI.

**Motivation**: Administrators need a browser UI for topology, shard,
Timescale, vectorizer, branch, tenant, backup, and realtime debugging
workflows, with mutating actions requiring exact confirmations.

**Citus comparison**: Vanilla Citus does not ship a web administration UI.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: D5` in `tools/citus-admin/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_admin -- run-canonical`

### D6: citus-schema-designer Visual

**Overlay**: `tools/citus-schema-designer`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the initial contract for the DrawDB-based visual schema
designer's Citus overlays.

**Motivation**: Schema designers need a versioned model for distribution,
hypertable, search, webhook, and shard-placement layers before the UI reads
operator CRD or companion state.

**Citus comparison**: Vanilla Citus does not include a visual schema designer.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: D6` in `tools/citus-schema-designer/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_schema_designer -- run-canonical`

### D12: citus-watch Dashboard

**Overlay**: `tools/citus-watch`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the dashboard data-source and panel contract for the
`citus-watch` operator view.

**Motivation**: Operators need a single terminal dashboard that can read
companion metadata, Prometheus metrics, and pool signals without hand-built
queries.

**Citus comparison**: Vanilla Citus does not ship a unified TUI dashboard.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: D12` in `tools/citus-watch/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_watch -- run-canonical`

### D7: Helm One-Line Install

**Overlay**: `deploy/k8s/helm/citus-overlay`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides a production-safe direct Helm install surface for the
ai-blaise overlay. The chart defaults in `values.yaml` require immutable
operator/pool image digests and keep alpha sidecars, tools, and alpha
runtime/security intent disabled. Non-production image-matrix coverage moved to
the explicit `values-exhaustive.yaml` profile, while `values-dev.yaml` remains
the small developer profile.

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
operator/pool replicas, rejects alpha workload deployments, and runs live SQL
plus operator admin traffic through the installed release.

**References**:

- In-source: `FEATURE: D7` in `companion/src/ops_contracts.rs`
- Helm chart: `deploy/k8s/helm/citus-overlay`
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
traffic through the installed release, and proves the wrapper install path is
part of `make -f Makefile.ai-blaise gate-close`. `ci/ai-blaise/deploy-check.sh`
statically rejects regressions that remove the production default,
digest-inputs, mutable-tag escape hatch, or non-production install refusal.

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
`deploy/k8s/argo/app.yaml` targets the `main` release branch and
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
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `age`

**Summary**: Defines Apache AGE graph distribution plans over colocated Citus
vertex and edge tables.

**Motivation**: Graph queries need shard-local subgraphs before Cypher traffic
can safely run over distributed datasets.

**Citus comparison**: Vanilla Citus does not provide an Apache AGE
distributed-graph bridge.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: G2` in `companion/src/graph_bridge.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

### G3: Graph Colocation Policy

**Overlay**: `companion/src/graph_bridge.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `age`

**Summary**: Captures the required vertex/edge colocation policy for
distributed graph tables.

**Motivation**: Traversals are only efficient when vertices and edges share
placement by tenant or graph key.

**Citus comparison**: Vanilla Citus has colocation groups, but no graph-aware
policy layer.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: G3` in `companion/src/graph_bridge.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

## JSON Schema

### JS2: Distributed JSON Schema Validation

**Overlay**: `companion/src/jsonschema_bridge.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_jsonschema`

**Summary**: Defines schema registration and shard-trigger fanout for JSON
Schema validation on distributed tables.

**Motivation**: JSON validation must run on every shard, not only where a
coordinator migration happened to install a trigger.

**Citus comparison**: Vanilla Citus does not manage distributed
pg_jsonschema trigger fanout.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: JS2` in `companion/src/jsonschema_bridge.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

### M13: JSON Schema Validation On Insert

**Overlay**: `companion/src/jsonschema_bridge.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_jsonschema`

**Summary**: Defines insert/update trigger timing for JSON Schema validation
on distributed tables.

**Motivation**: Migration and schema contracts need fail-fast JSON validation
before malformed tenant data is accepted.

**Citus comparison**: Vanilla Citus does not ship JSON Schema validation
helpers.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: M13` in `companion/src/jsonschema_bridge.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

## Geo

### Geo2: Geo-Aware Citus Distribution

**Overlay**: `companion/src/geo_distributed.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `postgis`

**Summary**: Defines geohash-derived distribution planning for PostGIS-backed
tables.

**Motivation**: Location-heavy workloads need spatially meaningful shard keys
so nearby data can be routed and rebalanced coherently.

**Citus comparison**: Vanilla Citus can distribute geometry tables but does
not create geo-aware distribution keys.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Geo2` in `companion/src/geo_distributed.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

### Geo3: Geo Shard Pruning Planner Input

**Overlay**: `companion/src/geo_distributed.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `postgis`

**Summary**: Defines bbox/grid planner input used to prune shards for spatial
queries.

**Motivation**: Spatial queries should avoid scanning shards whose geohash
grid cells cannot intersect the requested bounding box.

**Citus comparison**: Vanilla Citus does not expose geo-shard pruning
metadata.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Geo3` in `companion/src/geo_distributed.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`

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

**Overlay**: `deploy/k8s/helm/citus-overlay/templates/observability-dashboards.yaml`
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
  `deploy/k8s/helm/citus-overlay/templates/observability-dashboards.yaml`
- CI: `ci/ai-blaise/kind-production-smoke.sh`
- CI: `ci/ai-blaise/deploy-check.sh`

### O10: Alert Rules For Top Pains

**Overlay**: `deploy/k8s/helm/citus-overlay/templates/observability-prometheusrules.yaml`
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
  `deploy/k8s/helm/citus-overlay/templates/observability-prometheusrules.yaml`
- CI: `ci/ai-blaise/kind-production-smoke.sh`
- CI: `ci/ai-blaise/deploy-check.sh`

### O13: citus-watch TUI

**Overlay**: `tools/citus-watch`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the initial Rust contract for the `citus-watch` unified
operator view across cluster topology, shards, hypertables, EXPLAIN,
rebalance, vectorizer backlog, search indexes, tenants, and branches.

**Motivation**: Runtime operations need a compact, terminal-native view that
tracks the same companion and metrics surfaces used by dashboards and alerts.

**Citus comparison**: Vanilla Citus does not ship a dedicated runtime
operations TUI.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: O13` in `tools/citus-watch/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_watch -- run-canonical`

## V2 Completion Register Addendum

The entries below close the V2 catalog surfaces that are implemented as
contracts in the companion, deployment, pool, MCP, image, and runbook overlays.
Each row records the vanilla Citus comparison, grepable source marker, and
deterministic executable evidence command. These rows remain alpha contract
evidence unless promoted to standalone feature headings with production
evidence.

| ID | Feature | Overlay | Status | Vanilla Citus comparison | Reference | Evidence |
|---|---|---|---|---|---|---|
| A7 | pgvector cohabitation | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not pin a bundled vector-extension contract. | `FEATURE: A7` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| A9 | Secret binding via External Secrets | `companion/src/ops_contracts.rs` and Helm values | alpha | Vanilla Citus does not define vector-provider secret binding. | `FEATURE: A9` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical` |
| A10 | Streaming chat completion UDF | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not define streaming LLM SQL surfaces. | `FEATURE: A10` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| A11 | Semantic catalog text-to-SQL | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not include a tenant-scoped semantic catalog. | `FEATURE: A11` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| A12 | vchord alternate vector index | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not track optional vector-index alternatives. | `FEATURE: A12` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| C11 | DDL replication via pgl_ddl_deploy | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not bundle cross-region DDL replication policy. | `FEATURE: C11` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| C12 | Replication-slot failover | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not require logical slot failover packaging. | `FEATURE: C12` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| C13 | Subscription failover | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not package subscription failover contracts. | `FEATURE: C13` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| D9 | Canary upgrade runbook | `companion/src/ops_contracts.rs` and `docs/ai-blaise/RUNBOOKS/upgrade.md` | alpha | Vanilla Citus does not include this canary upgrade runbook. | `FEATURE: D9` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical` |
| D10 | Release hardening runbook | `companion/src/ops_contracts.rs` and `docs/ai-blaise/RUNBOOKS/production.md` | alpha | Vanilla Citus does not include these hardening gates. | `FEATURE: D10` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical` |
| D11 | MCP developer workflow | `tools/citus-mcp/src/lib.rs`, `tools/citus-mcp/src/main.rs`, and `companion/src/ops_contracts.rs` | alpha | Vanilla Citus does not expose MCP workflows for agents. | `FEATURE: D11` | `cargo run -p ai_blaise_citus_mcp -- run-canonical` |
| EF6 | In-database JavaScript and Rust UDF substrate | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not bundle plv8/plrust as a platform contract. | `FEATURE: EF6` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| Edge1 | Bounded-staleness edge replicas | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not model edge POP read replicas. | `FEATURE: Edge1` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| Edge2 | libsql read-tier research guard | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not include a libsql-shaped research gate. | `FEATURE: Edge2` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| F2 | Foreign data wrapper bundle | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not bundle the overlay FDW catalog policy. | `FEATURE: F2` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| F3 | Iceberg federation to warehouses | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not define Iceberg warehouse federation. | `FEATURE: F3` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| F4 | postgres_fdw credential rotation | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not prescribe FDW secret rotation. | `FEATURE: F4` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| F5 | Outbound HTTP extensions | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not package pgsql-http or pg_net policy. | `FEATURE: F5` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| G1 | Apache AGE bundled | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not require Apache AGE in every operand image. | `FEATURE: G1` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| Geo1 | PostGIS bundled | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not require PostGIS in every operand image. | `FEATURE: Geo1` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| IA1 | HypoPG bundled | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not bundle hypothetical-index advisor inputs. | `FEATURE: IA1` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| IA2 | pg_qualstats bundled | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not bundle predicate-stat advisor inputs. | `FEATURE: IA2` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| JS1 | pg_jsonschema bundled | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not require JSON Schema validation support. | `FEATURE: JS1` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| L7 | Citus columnar analytical path | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus has columnar storage but not this tiering contract. | `FEATURE: L7` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| L10 | Cross-tier query planner | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not combine hot, warm, and cold tiers. | `FEATURE: L10` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| L11 | pg_parquet bundled | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not package Parquet helpers as part of its image. | `FEATURE: L11` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| M4 | Schema drift detection | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not reconcile declarative schema drift. | `FEATURE: M4` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| M6 | DDL replication | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not bundle pgl_ddl_deploy policy. | `FEATURE: M6` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| M10 | Track settings drift | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not require pg_track_settings. | `FEATURE: M10` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| M12 | UUIDv7 primary keys | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not bundle monotonic UUID helpers. | `FEATURE: M12` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| MR3 | Locality-prefixed primary keys | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not encode region in key policy. | `FEATURE: MR3` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| MR6 | Closed-timestamp time travel | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not expose bounded-staleness time travel. | `FEATURE: MR6` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| MR7 | Cross-region active-active references | `companion/src/extension_catalog.rs` and `companion/src/ops_contracts.rs` | alpha | Vanilla Citus does not package pgactive conflict-policy gates. | `FEATURE: MR7` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| MR9 | Region survival runbook | `companion/src/ops_contracts.rs` and `docs/ai-blaise/RUNBOOKS/disaster-recovery.md` | alpha | Vanilla Citus does not ship this regional DR runbook. | `FEATURE: MR9` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical` |
| O7 | Wait-event sampling | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not require pg_wait_sampling/pgsentinel. | `FEATURE: O7` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| O8 | OS metrics via SQL | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not require pgnodemx. | `FEATURE: O8` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| O9 | Kernel stats via SQL | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not require pg_stat_kcache. | `FEATURE: O9` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| O11 | pg_stat_monitor alternative | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not package pg_stat_monitor. | `FEATURE: O11` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| O12 | pg_show_plans plan-inspection contract | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not require plan-inspection packaging. | `FEATURE: O12` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| PM1 | pg_hint_plan bundled | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not package hint-plan policy as an overlay contract. | `FEATURE: PM1` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| PM2 | sr_plan bundled | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not bundle saved-plan backends. | `FEATURE: PM2` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| R3 | Columnstore-on-worker policy | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not define this worker tiering policy. | `FEATURE: R3` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| R6 | Bloat-free queue substrate | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not package pgque/pgmq as queue policy. | `FEATURE: R6` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| R8 | Non-hypertable cold columnar path | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not define this cold-tier policy. | `FEATURE: R8` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| R11 | pg_warm bundled | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not require pg_warm in operand images. | `FEATURE: R11` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| R12 | Per-shard temperature ranking | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not maintain shard temperature scores. | `FEATURE: R12` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| RT5 | Phoenix-channel-compatible realtime client | `companion/src/ops_contracts.rs` | alpha | Vanilla Citus does not provide realtime client compatibility gates. | `FEATURE: RT5` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical` |
| S1 | Auto shard split | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not expose declarative split intent. | `FEATURE: S1` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| S3 | Clone-node fast scale-out | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus exposes clone-node primitives but not this operator contract. | `FEATURE: S3` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| S7 | Cross-region replication via pgactive | `companion/src/ops_contracts.rs` | alpha | Vanilla Citus does not bundle pgactive policy gates. | `FEATURE: S7` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical` |
| S8 | Locality-prefixed PKs | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not define region-prefixed key policy. | `FEATURE: S8` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| S12 | Tablespaces by region | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not reconcile region tablespace intent. | `FEATURE: S12` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| Search1 | pg_search bundled | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not require BM25 search support. | `FEATURE: Search1` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| Search4 | RUM index bundled | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not require RUM search indexes. | `FEATURE: Search4` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| Search5 | pg_trgm bundled | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not require trigram search support. | `FEATURE: Search5` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| Search6 | citext bundled | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not require citext search semantics. | `FEATURE: Search6` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| Sec3 | pgaudit and file audit | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not require this audit bundle. | `FEATURE: Sec3` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| Sec4 | pgsodium crypto | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not bundle libsodium crypto policy. | `FEATURE: Sec4` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| Sec7 | External Secrets integration | `companion/src/ops_contracts.rs` and Helm values | alpha | Vanilla Citus does not prescribe External Secrets refs. | `FEATURE: Sec7` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical` |
| Sec8 | TLS everywhere | `companion/src/ops_contracts.rs` and Helm values | alpha | Vanilla Citus does not enforce this full overlay TLS contract. | `FEATURE: Sec8` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical` |
| Sec9 | SBOM and cosign attestation | `companion/src/ops_contracts.rs` and release gates | alpha | Vanilla Citus does not require ai-blaise release attestations. | `FEATURE: Sec9` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical` |
| Sec10 | pg_safeupdate guard | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not package pg_safeupdate policy. | `FEATURE: Sec10` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| Sec11 | CDC anonymization extension | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not bundle anonymization policy. | `FEATURE: Sec11` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| Sec14 | pgcrypto bundled | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not document pgcrypto as overlay policy. | `FEATURE: Sec14` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| Sec15 | Encryption-at-rest with CMK | `companion/src/extension_catalog.rs` and Helm values | alpha | Vanilla Citus does not prescribe pgsodium-backed CMK controls. | `FEATURE: Sec15` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
| Sto2 | file_attachment domain type | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not include a storage domain type. | `FEATURE: Sto2` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| T4 | Hash-table planner hot path | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not expose this overlay performance contract. | `FEATURE: T4` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| T6 | PG18 io_uring default | `companion/src/ops_contracts.rs` and Helm values | alpha | Vanilla Citus does not set ai-blaise PG18 I/O policy. | `FEATURE: T6` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical` |
| T7 | Pipelined client protocol in pool | `pool/src/runtime.rs`, `pool/src/proxy.rs`, `pool/src/main.rs`, and `companion/src/ops_contracts.rs` | alpha | Vanilla Citus does not ship the ai-blaise pool pipeline. | `FEATURE: T7` | `cargo run -p ai_blaise_citus_pool -- run-canonical` |
| T10 | Bulk protocol fetch path | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus has no ai-blaise bulk-fetch contract. | `FEATURE: T10` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| T11 | DistSQL physical pushdown | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not expose this DistSQL contract. | `FEATURE: T11` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| T13 | Distributed cursors | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not coordinate multi-shard cursor state this way. | `FEATURE: T13` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| T14 | Distributed savepoints | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not coordinate savepoints through this contract. | `FEATURE: T14` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| TS10 | Hierarchical CAGGs distributed | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not fan out hierarchical CAGGs across workers. | `FEATURE: TS10` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| TS11 | Bloom filters on segmentby | `companion/src/advanced_planner.rs` | alpha | Vanilla Citus does not define Timescale segmentby bloom fanout. | `FEATURE: TS11` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical` |
| WF1 | pg_walinspect bundled | `companion/src/extension_catalog.rs` and `images/citus-pg-overlay` | alpha | Vanilla Citus does not package WAL inspection as an overlay policy. | `FEATURE: WF1` | `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical` |
