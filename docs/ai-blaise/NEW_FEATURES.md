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
`cargo run -p ai_blaise_citus_operator -- run-reconcile-plans-batch-c` and
`ci/ai-blaise/operator-reconcilers-batch-c-smoke.sh` guard the Batch C operator
reconcile plans for `FEATURE: R7`, `FEATURE: C9`, `FEATURE: M3`, `FEATURE:
M14`, `FEATURE: C4`, `FEATURE: C5`, and `FEATURE: O5`.
`e2e/src/runtime_contracts.rs` validates canonical runtime contracts for
`FEATURE: Auth1`, `FEATURE: Auth3`, `FEATURE: B1`, `FEATURE: B3`,
`FEATURE: B4`, `FEATURE: C1`, `FEATURE: L8`, `FEATURE: MR5`, `FEATURE: R7`,
`FEATURE: R10`, `FEATURE: RT1`, `FEATURE: RT2`, `FEATURE: RT3`,
`FEATURE: RT4`, `FEATURE: Search8`, `FEATURE: Sec12`, `FEATURE: Sto1`,
`FEATURE: Sto3`, `FEATURE: Sto4`, `FEATURE: T1`, `FEATURE: T3`, `FEATURE: T9`,
`FEATURE: T12`, `FEATURE: T15`, and `FEATURE: WH3`.
`pool/src/main.rs` executes a real PostgreSQL TCP proxy in `serve` mode, with
upstream-aware admin readiness on a separate port; `ci/ai-blaise/pool-proxy-smoke.sh`
verifies live SQL, CIDR allow/deny behavior, active-connection overload,
tenant-quota fail-closed denial, Auth3 startup-token introspection against the
real auth sidecar, revoked-token fail-closed denial, upstream-unreachable
fail-closed routing, and pipelined PostgreSQL simple-query frames through that
data port. The binary still emits the deterministic pool runtime and shard-map
summary for `FEATURE: Auth3`, `FEATURE: MR5`, `FEATURE: R10`, `FEATURE:
Sec12`, `FEATURE: T1`, `FEATURE: T2`, `FEATURE: T3`, `FEATURE: T7`,
`FEATURE: T9`, `FEATURE: T12`, and `FEATURE: T15`.
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
`companion/src/plan_runtime.rs` executes a deterministic PM3/PM4 companion
runtime for durable plan promotion and regression evaluation, including
idempotency replay, bounded retry, audit events, and unknown-plan failure
handling through the same companion binary.
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
The analytical smoke also starts the sidecar probe server on a loopback TCP
port and verifies health, readiness, metrics, and drain behavior.
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
`sidecar/hlc/src/runtime.rs` runs the deterministic peer clock-exchange and
closed-timestamp runtime for `FEATURE: S9` and `FEATURE: MR6`.
`sidecar/hlc/src/main.rs` emits the canonical closed-timestamp follower-read
and runtime runners for `FEATURE: S9`.
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
`sidecar/raft/src/runtime.rs` runs the deterministic election, AppendEntries,
quorum commit, durable log, and snapshot-boundary runtime for `FEATURE: S5`.
`sidecar/raft/src/main.rs` emits the canonical shard-group failover, runtime,
and durable-log runners for `FEATURE: S5`.
`sidecar/realtime/src/lib.rs` validates CDC-driven broadcast, tenant isolation,
filter, and presence contracts for `FEATURE: RT1`, `FEATURE: RT2`,
`FEATURE: RT3`, and `FEATURE: RT4`.
`sidecar/realtime/src/lib.rs` also models deterministic realtime runtime
fan-out for those features, covering active connections, filtered subscribers,
frame sizing, delivered message counts, and presence snapshot accounting.
`sidecar/repack/src/lib.rs` validates online repack command planning and
per-shard targets for `FEATURE: R7`.
`sidecar/repack/src/main.rs` emits canonical and live online repack runners
for `FEATURE: R7`.
`sidecar/schema_job/src/lib.rs` validates online-DDL worker leases, backfill,
safety, gh-ost shadow-table contracts, and fail-closed manifest/apply SQL
boundaries for `FEATURE: C10` and `FEATURE: M2`.
`sidecar/schema_job/src/main.rs` emits canonical online-DDL worker,
controller-tick, and manifest-validation runners for `FEATURE: C10` and
`FEATURE: M2`.
`sidecar/storage/src/lib.rs` validates object metadata, presigned URL, bucket
ACL, and antivirus contracts for `FEATURE: Sto1`, `FEATURE: Sto3`,
`FEATURE: Sto4`, and `FEATURE: Sto5`.
`sidecar/storage/src/lib.rs` also runs a deterministic storage flow for those
features: presigned URL issuance, tenant bucket ACL checks, object size
enforcement, metadata persistence, and antivirus quarantine decisions.
`sidecar/txn_status/src/lib.rs` validates parallel-commit transaction status,
intent evidence, and 2PC fallback decisions for `FEATURE: T5`.
`sidecar/txn_status/src/runtime.rs` runs the Raft-backed staging/finalize state
machine and parallel-commit microbenchmark for `FEATURE: T5`.
`sidecar/txn_status/src/main.rs` emits the canonical parallel-commit status,
runtime, microbenchmark, and loopback HTTP stage/finalize/ack runners for
`FEATURE: T5`.
`companion/src/txn_coord.rs` renders the companion SQL/UDF coordination plan
for `FEATURE: T5`.
The tool overlays expose deterministic canonical runners for their library
contracts: `tools/citus-mcp/src/main.rs`, `tools/citus-admin/src/main.rs`,
`tools/citus-schema-designer/src/main.rs`, `tools/citus-tui/src/main.rs`, and
`tools/citus-watch/src/main.rs`.
`ci/ai-blaise/citusctl-smoke.sh` exercises the real `citusctl` binary for the
`FEATURE: D2` plan-id guard.

## Operand Image

### Bundle1: Bundled Extension Image Contract

**Overlay**: `images/citus-pg-overlay`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: see `images/citus-pg-overlay/extension-manifest.tsv`

**Summary**: Defines the operand-image manifest, preload order, required
extension initialization SQL, explicit PG17 source-build targets for the
feasible PGDG-missing Bundle1 extensions, and a complete initdb path that
exercises every required extension against the live overlay image.
`FEATURE: Bundle1 is production-ready` for the
`full-bundle-required-minus-plrust` boundary: the new
`bundle1-pgdg-runtime` Dockerfile stage installs every PGDG and Timescale
binary-package extension listed as required in
`extension-manifest.tsv`; `bundle1-final-light` and
`bundle1-final-full` layer in the source-built citus, pgsodium, topn,
pg_jsonschema, pg_graphql (light) and pg_search, plv8 (heavy) extensions; the
canonical `shared-preload-libraries.conf` only references actually-installed
shared libraries; and `/docker-entrypoint-initdb.d/00-ai-blaise-extensions.sql`
runs `CREATE EXTENSION` for every required Bundle1 extension at container
start. The source-build smoke
(`BUNDLE1_BUILD_IMAGE=1 REQUIRE_DOCKER=1 bash ci/ai-blaise/sql-extension-smoke.sh`
and the heavy variant `BUNDLE1_BUILD_HEAVY=1`) verifies pg_extension catalog
records every required Bundle1 extension after initdb and records the result in
`bundle1-source-build-evidence.tsv`.

**Current boundary**: This production-ready claim covers the required Bundle1
extension set minus plrust, which remains the lone alpha-deferred entry in
`extension-manifest.tsv`. The plrust PG17 upstream gap is unchanged
(upstream pg13-pg16 pgrx 0.11.0 only); plrust has been moved from
`required` to `optional` in the manifest and is tracked separately under
`FEATURE: EF6`. The image labels record this scope explicitly:
`ai-blaise.citus.bundle1.evidence-scope=full-bundle-required-minus-plrust`
and `ai-blaise.citus.bundle1.full-initdb-path=true`. The bundle is not
evidence for plrust Rust UDFs, PG18 source-build of the heavy extensions,
operand image release certification by command-center, or production
multi-region Kubernetes deployment correctness.

Production evidence: `BUNDLE1_BUILD_IMAGE=1 BUNDLE1_EVIDENCE_FILE=images/citus-pg-overlay/bundle1-source-build-evidence.tsv REQUIRE_DOCKER=1 bash ci/ai-blaise/sql-extension-smoke.sh` builds
`bundle1-final-light` (PG17, full PGDG + Timescale + source-built bundle),
starts a container with the canonical `shared_preload_libraries` set, waits
for the docker-entrypoint `PostgreSQL init process complete` log line so the
verification phase is not racing the initdb-script-runner / final-server
restart, and then verifies pg_extension catalog records every required Bundle1
extension (25 entries) and pg_warm/seed_extension_catalog functional smoke
output. The evidence row is appended to
`images/citus-pg-overlay/bundle1-source-build-evidence.tsv` with the
`bundle1-final-light` image digest. The heavy variant
`BUNDLE1_BUILD_HEAVY=1` extends the same path through pg_search and plv8.

**Motivation**: The fork needs one machine-checkable contract for always-on,
optional, and hard-blocked extensions before image builds and Helm values can
be safely automated.

**Citus comparison**: Vanilla Citus does not ship an ai-blaise operand image
with TimescaleDB, search, graph, vector, storage, observability, security, and
federation extension policy.

**References**:

- Design: `docs/ai-blaise/BUNDLED_EXTENSIONS.md`
- CI: `ci/ai-blaise/image-check.sh`
- Structured Bundle1 contract check: `ci/ai-blaise/bundle1-contract-check.py`
- Source-build lockfile: `images/citus-pg-overlay/bundle1-source-build.lock.tsv`
- Source-build smoke (light):
  `BUNDLE1_BUILD_IMAGE=1 REQUIRE_DOCKER=1 bash ci/ai-blaise/sql-extension-smoke.sh`
- Source-build smoke (heavy with pg_search + plv8):
  `BUNDLE1_BUILD_IMAGE=1 BUNDLE1_BUILD_HEAVY=1 REQUIRE_DOCKER=1 bash ci/ai-blaise/sql-extension-smoke.sh`
- pg_cron cohabitation smoke: `REQUIRE_DOCKER=1 bash ci/ai-blaise/pg-cron-cohabitation-smoke.sh`
- Evidence file: `images/citus-pg-overlay/bundle1-source-build-evidence.tsv`
- In-source: `FEATURE: Bundle1` in
  `images/citus-pg-overlay/extension-manifest.tsv`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-extension-catalog-canonical`

## Throughput

### T1: Settings-Bucket Connection Pool

**Overlay**: `pool/src/runtime.rs`, `pool/src/settings_bucket.rs`, `pool/src/proxy.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Implements the pool settings-bucket contract with opaque,
versioned GUC fingerprints, live startup-option parsing, and per-fingerprint
backend accounting for tracked GUC state. The production-ready surface is the
real proxy's tracked-GUC isolation and borrow/release accounting; reusable
transaction-pooling of backend sessions remains alpha until it has separate
wire-protocol reset/reuse evidence.


Production evidence: `ci/ai-blaise/pool-proxy-smoke.sh` runs the real pool
against a `postgres:17` container with
`AI_BLAISE_POOL_SETTINGS_BUCKET_GUCS=citus.enable_repartition_joins`, opens raw
PostgreSQL clients through the pool data port with startup `options` setting
that tracked GUC to `on` and `off`, verifies each client observes its own
`current_setting('citus.enable_repartition_joins', true)`, verifies simultaneous
clients use distinct `pg_backend_pid()` values, and asserts Prometheus metrics
for unique fingerprints, backend borrows, zero assigned connections after
release, and zero release errors. This proves tracked-GUC startup parsing plus
borrow/release accounting through the live proxy; it is not a claim of broad
transaction pooling correctness or backend reuse.

**Motivation**: Citus deployments need far more client sessions than worker
backends without losing session correctness.

**Citus comparison**: Vanilla Citus does not ship an external settings-bucket
pooler.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: T1` in `pool/src/runtime.rs`
- In-source: `FEATURE: T1` in `pool/src/settings_bucket.rs`
- In-source: `FEATURE: T1` in `pool/src/proxy.rs`
- Executable: `cargo run -p ai_blaise_citus_pool -- run-canonical`
- CI: `ci/ai-blaise/pool-proxy-smoke.sh`
- Benchmark: `benchmarks/sysbench/run-suite.sh` (TPS / p95 per workload)
- Benchmark: `benchmarks/tpcc/run.sh` (tpmC, p99 latency, error rate)

### T2: Plan Cache Placement-Generation Invalidation

**Overlay**: `pool/src/shard_map.rs`, `pool/src/placement_subscriber.rs`,
`pool/src/prepared.rs`, `companion/src/router_assist.rs`,
`patches/0003-guc-report-citus-userset.patch`,
`patches/0005-placement-generation-counter.patch`
**Status**: production-ready
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

Production evidence: `cargo test -p ai_blaise_citus_companion --lib
router_assist` runs the placement-generation subscriber contract end to end
(initial/unchanged/advanced/reset transitions, catalog SELECT shape,
sample validation). `cargo test -p ai_blaise_citus_pool --lib shard_map`
runs the pool-side plan-cache generation contract.
`ci/ai-blaise/placement-generation-udf-contract-smoke.sh` verifies that the C
symbol, fresh-install SQL, 15.0 upgrade SQL, versioned UDF snapshots,
companion query string, and upstream patch artifact all expose
`pg_catalog.citus_placement_generation()`.
`ci/ai-blaise/pg-cron-cohabitation-smoke.sh` now boots a live patched Citus PG17
runtime, creates two real distributed tables, records
`placement_generation_initial`,
`placement_generation_after_first_distribution`,
`placement_generation_after_second_distribution`, and
`placement_generation_placements`, and asserts that the installed
`pg_catalog.citus_placement_generation()` counter advances monotonically while
Citus creates placement metadata. The same smoke opens a raw PostgreSQL protocol
connection and verifies a `citus_shard_count_parameter_status` packet after
`SET citus.shard_count TO 7`, proving the `GUC_REPORT`/ParameterStatus contract
for a live `citus.*` USERSET GUC. This closes the T2 Citus patch runtime
contract for the bounded placement-generation and GUC-reporting surface; it does
not claim production latency, rebalance throughput, or the unpublished pool
data-plane serving traffic under real tenant load.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: T2` in `pool/src/shard_map.rs`
- In-source: `FEATURE: T2` in `pool/src/placement_subscriber.rs`
- In-source: `FEATURE: T2` in `pool/src/prepared.rs`
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
- CI: `ci/ai-blaise/placement-generation-udf-contract-smoke.sh`
- CI: `ci/ai-blaise/pg-cron-cohabitation-smoke.sh`
- Patches: `patches/0003-guc-report-citus-userset.patch`,
  `patches/0005-placement-generation-counter.patch`

### T3: Fast-Path Single-Shard Router

**Overlay**: `pool/src/runtime.rs`, `companion/src/router_assist.rs`,
`patches/0006-fast-path-router-no-coord-rt.patch`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Defines the pool routing contract, shard-map route selection, and
upstream-targetable Citus locality probe used to send eligible single-shard
requests directly to the worker path with a coordinator fallback.

**Motivation**: Coordinator-less topology needs a pool-level fast path before
query execution patches are wired in.

**Citus comparison**: Vanilla Citus plans single-shard queries but does not
ship this pool routing layer or an external pool locality probe.

Production evidence: `ci/ai-blaise/router-patch-smoke.sh` now verifies the
integrated Citus source, patch artifact, SQL catalog registration, and a live
PG17 Docker runtime built from this fork. With `REQUIRE_DOCKER=1`, the smoke
creates the `citus` extension, verifies
`citus.enable_fast_path_router_skip_coordinator`, asserts NULL/zero/negative and
unknown shard probes fail closed, creates a real single-shard distributed table,
proves `pg_catalog.citus_fast_path_router_can_skip_coordinator(shard_id)`
returns true for the single local active placement, proves the GUC-off path
returns false, and records 30 successful locality probes with
`coordinator_round_trips_per_single_shard_query=0` in
`benchmarks/citus-patches/results/0006-fast-path-router-skip.json`. This closes
the bounded T3 production surface for the SQL-visible locality probe and pool
contract. It does not claim broad multi-region coordinator-less serving,
replica-choice routing, all query-shape coverage, or fleet latency under tenant
load.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: T3` in `pool/src/runtime.rs`
- In-source: `FEATURE: T3` in `pool/src/shard_map.rs`
- In-source: `FEATURE: T3` in `pool/src/virtual_pid.rs`
- In-source: `FEATURE: T3` in `pool/src/proxy.rs`
- In-source: `FEATURE: T3` in `companion/src/router_assist.rs`
- Patch: `patches/0006-fast-path-router-no-coord-rt.patch`
- Executable: `cargo run -p ai_blaise_citus_pool -- run-canonical`
- Executable: `cargo test -p ai_blaise_citus_companion --lib router_assist`
- CI: `ci/ai-blaise/router-patch-smoke.sh`

### T5: Parallel Commit Transaction Status

**Overlay**: `sidecar/txn_status`
**Status**: production-ready
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
- In-source: `FEATURE: T5` in `sidecar/txn_status/src/runtime.rs`
- In-source: `FEATURE: T5` in `companion/src/txn_coord.rs`
- SQL runtime: `FEATURE: T5` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_sidecar_txn_status -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_txn_status -- run-runtime-canonical`
- CI: `ci/ai-blaise/parallel-commits-smoke.sh`
- CI: `ci/ai-blaise/txn-status-networked-raft-smoke.sh`
- CI: `ci/ai-blaise/schema-txn-runtime-smoke.sh`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
Production evidence: the sidecar has deterministic Raft-backed
staging/finalize runtime evidence, a loopback HTTP runtime smoke that drives
stage -> wait -> ack -> commit with serde-validated JSON and malformed-input
rejection, and `txn-status-networked-raft-smoke.sh` starts three separate
`ai_blaise_citus_sidecar_raft serve` OS processes plus a real
`ai_blaise_citus_sidecar_txn_status serve` process configured with
`AI_BLAISE_TXN_RAFT_LEADER_ADDR`. The smoke elects `worker-a`, stages
`txn-live-raft-1`, proves the Raft log commits
`stage:txn-live-raft-1:worker-a` before the transaction record is returned,
waits without appending when replication evidence is incomplete, records shard
acks, proves the Raft log commits `commit:txn-live-raft-1` before the sidecar
reports committed, verifies every voter converges on commit index 2, and
proves follower-backed replication failures fail closed without materialising a
transaction record. The microbenchmark proves the modeled fast-path step count,
and the SQL extension installs `companion.txn_stage`/
`companion.txn_finalize` against real PostgreSQL. This production-ready
boundary is the networked transaction-status sidecar API and SQL contract;
integration with the Citus distributed executor, PostgreSQL-core commit
timestamp patches, and Kubernetes operator wiring remain alpha.
- Executable: `patches/postgres/0001-logical-commit-clock.patch` carries the
  PostgreSQL-core logical commit clock the parallel-commit path depends on for
  monotonic shard-finalize ordering. The patch is the upstream-quality diff that
  makes the integrated gate concrete. Tracked under FEATURE: PGC1.
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

**Overlay**: `pool/src/runtime.rs`, `pool/src/mirror.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds deterministic per-tenant and per-query-class canary
mirroring with target and sample-percentage policy.

**Current production-ready boundary**: `TenantMirrorPolicy` now has a
fail-closed `tenant:query-class:sample-percent` parser, duplicate-rule
detection, query-class validation, tenant-id validation, deterministic hash
bucket reporting, and non-secret route evidence in the pool canonical report.
This proves the bounded routing/security contract only; live production traffic
fan-out, canary backend execution, response comparison, and rollout control
remain alpha until proven by a data-plane canary smoke.

Production evidence: `cargo test -p ai_blaise_citus_pool --all-targets`,
`cargo run -p ai_blaise_citus_pool -- run-canonical`, and
`ci/ai-blaise/pool-routing-security-smoke.sh` assert the fail-closed parser and
route report columns (`mirror_rule_count`, `mirror_decision_bucket`, and
`mirrored_canary_routes`).

**Motivation**: Planner, pool, and sidecar changes need low-risk A/B traffic
before they become default paths.

**Citus comparison**: Vanilla Citus does not mirror query traffic.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: T9` in `pool/src/runtime.rs`
- In-source: `FEATURE: T9` in `pool/src/mirror.rs`
- Executable: `cargo run -p ai_blaise_citus_pool -- run-canonical`
- CI: `ci/ai-blaise/pool-routing-security-smoke.sh`

### T12: Pool HTAP Routing

**Overlay**: `pool/src/runtime.rs`, `pool/src/htap.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines HTAP routing policy and a conservative query-feature
classifier from the pool to the analytical sidecar with staleness budget and
predicate hints.

**Current production-ready boundary**: `QueryFeatures::from_contract_flags`
parses the compact pool feature report fail-closed, rejecting unknown keys,
duplicates, malformed booleans, and malformed limits. `route_report` exposes
bounded target, staleness, hint-count, and reason evidence for the conservative
classifier. This does not claim live SQL parsing in the pool hot path,
analytical sidecar execution, freshness enforcement, or query-result routing.

Production evidence: `cargo test -p ai_blaise_citus_pool --all-targets`,
`cargo run -p ai_blaise_citus_pool -- run-canonical`, and
`ci/ai-blaise/pool-routing-security-smoke.sh` assert analytical-route evidence
and fail-closed parser rejection through `htap_analytical_routes` and
`htap_fail_closed_rejections`.

**Motivation**: Hot/warm/cold query routing needs a single contract before the
pool starts classifying real SQL.

**Citus comparison**: Vanilla Citus does not route HTAP queries to sidecars.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: T12` in `pool/src/runtime.rs`
- In-source: `FEATURE: T12` in `pool/src/htap.rs`
- Executable: `cargo run -p ai_blaise_citus_pool -- run-canonical`
- CI: `ci/ai-blaise/pool-routing-security-smoke.sh`

### T15: Transaction Pipelining In Pool

**Overlay**: `pool/src/runtime.rs`, `pool/src/pipeline.rs`,
`pool/src/proxy.rs`, `pool/src/admin.rs`
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
same Docker-backed smoke also proves active-connection overload rejection,
tenant quota fail-closed denial, and upstream-unreachable fail-closed routing
on the real data port. The Makefile `pool-proxy-smoke` target sets
`REQUIRE_DOCKER=1`, and `gate-close` depends on that target, so missing Docker
cannot silently skip this evidence.

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
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: Provides the SQL surface that distributes a PostgreSQL
declarative-partitioned parent table through Citus while using TimescaleDB
hypertables for worker-local partitions. The `apply_distribute_hypertable`
SQL function executes the TimescaleDB and Citus calls when both extensions are
loaded, then records bridge state for operator/readiness inspection.

Production evidence: `ci/ai-blaise/timescale-bridge-smoke.sh` proves
`apply_distribute_hypertable(...)` fails closed when the Citus distribution
entrypoint is absent, then calls real TimescaleDB `create_hypertable(...)` in
`timescale/timescaledb-ha:pg17-ts2.27` with only the Citus entrypoint stubbed.
`ci/ai-blaise/timescale-cohabitation-smoke.sh` then builds this fork into the
same pinned HA image, creates real `citus`, `timescaledb`, and
`ai_blaise_citus`, runs `apply_distribute_hypertable(...)`, verifies a real
Timescale hypertable plus Citus `pg_dist_partition` metadata, and records
`policy_execution_scope=entrypoints-and-catalog-state-only` evidence. This is
the bounded bridge-entrypoint/catalog-state claim; it does not claim full
TimescaleDB functionality, multi-worker fanout, rebalance behavior, planner
pushdown, or operator reconciliation.

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
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: Adds SQL-plan rendering, SQL apply execution, bridge-state
recording, and a `pg18`-gated pgrx surface for worker-fanned distributed
compression policy creation.

Production evidence: `ci/ai-blaise/timescale-bridge-smoke.sh` and
`ci/ai-blaise/timescale-cohabitation-smoke.sh` execute
`apply_compression_policy_distributed(...)` against
`timescale/timescaledb-ha:pg17-ts2.27`; the non-stubbed cohabitation run uses
real `citus`, `timescaledb`, and `ai_blaise_citus`, verifies bridge-state rows,
and records `policy_execution_scope=entrypoints-and-catalog-state-only`. This
is production-ready for the bounded SQL apply/catalog-state surface only; it
does not claim full TimescaleDB functionality, compression background job
completion, compressed-chunk performance, multi-worker policy fanout, or
operator reconciliation.

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
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: Adds SQL-plan rendering, SQL apply execution, bridge-state
recording, and a `pg18`-gated pgrx surface for distributed continuous
aggregate definitions and refresh-policy arguments.

Production evidence: `ci/ai-blaise/timescale-bridge-smoke.sh` and
`ci/ai-blaise/timescale-cohabitation-smoke.sh` execute
`apply_continuous_aggregate_distributed(...)` against
`timescale/timescaledb-ha:pg17-ts2.27`; the non-stubbed cohabitation run uses
real `citus`, `timescaledb`, and `ai_blaise_citus`, verifies the continuous
aggregate relation is created, verifies bridge-state rows, and records
`policy_execution_scope=entrypoints-and-catalog-state-only`. This is
production-ready for the bounded SQL apply/catalog-state surface only; it does
not claim full TimescaleDB functionality, continuous aggregate refresh or
materialized-result correctness, worker partial/final planning, or operator
reconciliation.

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
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: Adds SQL-plan rendering, SQL apply execution, bridge-state
recording, and a `pg18`-gated pgrx surface for cluster-wide retention policy
setup.

Production evidence: `ci/ai-blaise/timescale-bridge-smoke.sh` and
`ci/ai-blaise/timescale-cohabitation-smoke.sh` execute
`apply_retention_policy_distributed(...)` against
`timescale/timescaledb-ha:pg17-ts2.27`; the non-stubbed cohabitation run uses
real `citus`, `timescaledb`, and `ai_blaise_citus`, verifies bridge-state rows,
and records `policy_execution_scope=entrypoints-and-catalog-state-only`. This
is production-ready for the bounded SQL apply/catalog-state surface only; it
does not claim full TimescaleDB functionality, retention background job
completion, chunk-drop scheduling, multi-worker policy fanout, or operator
reconciliation.

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
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: `timescaledb`

**Summary**: Adds planner support that combines Citus shard metadata with
TimescaleDB time dimensions to prune shards for time-bound predicates. The SQL
extension now records enabled pruner state through an executable
`apply_time_range_shard_pruner` surface.

Production evidence: `ci/ai-blaise/timescale-bridge-smoke.sh` and
`ci/ai-blaise/timescale-cohabitation-smoke.sh` execute
`apply_time_range_shard_pruner(...)` against
`timescale/timescaledb-ha:pg17-ts2.27`; the non-stubbed cohabitation run uses
real `citus`, `timescaledb`, and `ai_blaise_citus`, verifies bridge-state rows,
and records `policy_execution_scope=entrypoints-and-catalog-state-only`. This
is production-ready for the bounded SQL enablement/catalog-state surface only;
it does not claim full TimescaleDB functionality, live planner pushdown,
shard-pruning latency, multi-worker fanout, or operator reconciliation.

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
real `timescale/timescaledb-ha:pg17-ts2.27` image with this Citus fork installed,
starts PostgreSQL with `shared_preload_libraries=timescaledb,citus` and
`citus.cohabit_extensions=timescaledb`, then creates `citus`, `timescaledb`,
and `ai_blaise_citus` in the same server. The VM run in the production audit
records the Git SHA, image identity, command path, PostgreSQL version,
TimescaleDB extension version, Citus extension version, and explicit
`real_citus_distribution=true` / `stubbed_citus_distribution=false` evidence.
This proves the trusted cohabitation startup/loading guard for the measured
image only; it does not prove full TimescaleDB planner pushdown, distributed
hypertable execution correctness, background policy execution, compression job
completion, or continuous aggregate refresh. The smoke is part of
`make -f Makefile.ai-blaise gate-close`.

Forward-compatibility gate: `ci/ai-blaise/ts-version-matrix-smoke.sh`
iterates the TS minor lines pinned under `tests/cohab-matrix/`, reads each
exact `image-tag.txt`, runs the single-version cohabitation smoke for
published images, and compares the running container against the per-version
expected hook-claim table. TS 2.27 is load-bearing through
`timescale/timescaledb-ha:pg17-ts2.27`. TS 2.28 is not production evidence yet:
the VM registry probe on 2026-05-24 found no `timescale/timescaledb-ha:pg17-ts2.28`,
`timescale/timescaledb-ha:pg17-ts2.28.0`, or
`timescale/timescaledb-ha:pg17-ts2.28.1` image, so the 2.28 row records `skip-with-note` until the tag
is published and all `unknown` hook rows are measured.

**References**:

- Design: `docs/ai-blaise/COHABITATION.md`
- Executable: `ci/ai-blaise/timescale-cohabitation-smoke.sh`
- Executable: `ci/ai-blaise/ts-version-matrix-smoke.sh`
- Matrix: `tests/cohab-matrix/2.27/`, `tests/cohab-matrix/2.28/`
- In-source marker after patch application:
  `FEATURE: TS6` in
  `src/backend/distributed/shared_library_init.c`,
  `src/backend/distributed/planner/distributed_planner.c`,
  `src/backend/distributed/executor/multi_executor.c`,
  `src/backend/distributed/planner/multi_explain.c`

### TS7: Hypertable CRD Reconciler

**Overlay**: `operator/src/controllers/hypertable.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: Provides the Kubernetes `Hypertable` CRD controller for the
bounded Timescale/Citus bridge surface: the controller renders the guarded
apply plan, executes ordered SQL against a live Postgres target, patches the
status subresource, and skips already-applied bridge-state steps on requeue.

**Motivation**: The TimescaleDB bridge needs a declarative operator surface so
cluster state can be reconciled repeatedly instead of hand-applied.

**Citus comparison**: Vanilla Citus does not ship a Kubernetes CRD for
Timescale-aware distributed hypertables.

Production evidence: VM proof run
`ci/ai-blaise/operator-hypertable-live-smoke.sh` builds the real operator image
and the current Timescale/Citus cohabitation image, creates a kind cluster,
loads the generated `Hypertable` CRD with status subresource, runs the operator
in-cluster with `AI_BLAISE_OPERATOR_EXECUTION_MODE=apply`, initializes a live
PostgreSQL pod with `timescaledb,citus` and `citus.cohabit_extensions=timescaledb`,
then applies a `Hypertable` CR for `operator_metrics` with separate
`timeColumn=metric_time` and `distributionColumn=tenant_id`. The smoke requires
`.status.phase=Applied`, a stable SQL hash, real `_timescaledb_catalog.hypertable`
and `pg_dist_partition` rows, a continuous aggregate, five bridge-state feature
ids, recorded `observedGeneration`, and an immediate annotate-triggered
re-reconcile with `skippedStepCount >= 5` and no duplicate bridge-state rows.
This production-ready claim covers the Kubernetes CRD/status reconciliation and
live SQL apply path for the same bounded bridge surface as TS1/TS2/TS3/TS4/TS5;
it does not claim multi-worker fanout correctness, Timescale background job
completion, CAGG refresh accuracy, rebalance behavior, or full TimescaleDB
functionality.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TS7` in `operator/src/crds/hypertable.rs`
- In-source: `FEATURE: TS7` in `operator/src/controllers/hypertable.rs`
- In-source: `FEATURE: TS7` in `operator/src/reconcile/hypertable.rs`
- Acceptance: `FEATURE: TS7` in `e2e/src/timescale_on_citus.rs`
  and canonical SQL emitter `e2e/src/bin/timescale_apply_plan.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Live smoke: `ci/ai-blaise/operator-hypertable-live-smoke.sh`

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
`apply_distribute_hypertable(...)` suppresses that warning. The same smoke
drives file-backed LSP-style `Content-Length` JSON-RPC stdio frames through
`citus-lsp serve-stdio --metadata <metadata.tsv>` for opened-file diagnostics
and fail-closed malformed input behavior. Broader editor integration,
workspace indexing, automatic file rewrites, live metadata refresh, and full
PostgreSQL grammar coverage remain alpha.

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
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: Adds SQL-plan rendering, SQL apply execution, bridge-state
recording, and a `pg18`-gated pgrx surface for worker-fanned TimescaleDB
reorder policy setup.

Production evidence: `ci/ai-blaise/timescale-bridge-smoke.sh` and
`ci/ai-blaise/timescale-cohabitation-smoke.sh` execute
`apply_reorder_policy_distributed(...)` against
`timescale/timescaledb-ha:pg17-ts2.27`; the non-stubbed cohabitation run uses
real `citus`, `timescaledb`, and `ai_blaise_citus`, verifies bridge-state rows,
and records `policy_execution_scope=entrypoints-and-catalog-state-only`. This
is production-ready for the bounded SQL apply/catalog-state surface only; it
does not claim full TimescaleDB functionality, reorder background job
completion, chunk reorder performance, multi-worker policy fanout, or operator
reconciliation.

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
a real `timescale/timescaledb-ha:pg17-ts2.27` container, verifies that
`apply_distribute_hypertable(...)` fails closed before a Citus distribution
entrypoint is visible, stubs only that Citus distribution entrypoint, and
records `policy_execution_scope=entrypoints-and-catalog-state-only` evidence
for real TimescaleDB entrypoint calls and bridge-state rows.
`ci/ai-blaise/timescale-cohabitation-smoke.sh` closes the previous stub gap by
building this Citus fork into the pinned TimescaleDB HA PG17/TS2.27 image, loading
`timescaledb,citus` with `citus.cohabit_extensions=timescaledb`, creating real
`citus`, `timescaledb`, and `ai_blaise_citus` extensions, enforcing the
expected PG/Timescale minor when configured by the version matrix, requiring
real `create_distributed_table` rows in `pg_dist_partition`, and executing the
TS1/TS2/TS3/TS4/TS5/TS12 apply functions against that live cohabiting server
without defining any Citus stub. Those six feature entries are production-ready
for the same bounded SQL apply/catalog-state surface. TS7 separately proves the
Kubernetes controller execution and status reconciliation path for that surface.

The TS18 production-ready boundary is intentionally narrow: it proves SQL
apply functions invoke the expected TimescaleDB/Citus entrypoints, create the
measured coordinator catalog objects, fail closed when required dependency
functions are absent, and record deterministic bridge-state rows. It does not
claim full TimescaleDB functionality, continuous aggregate refresh/materialized
execution, compression/retention/reorder background job completion,
distributed hypertables across workers, planner pushdown, rebalance behavior,
or production cohabitation beyond the measured startup/load/apply guard.

**References**:

- Design: `docs/ai-blaise/COHABITATION.md`
- SQL extension: `FEATURE: TS18` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- CI: `ci/ai-blaise/timescale-bridge-smoke.sh`
- CI: `ci/ai-blaise/timescale-cohabitation-smoke.sh`

### TS19: pg_cron Clock Cohabitation

**Overlay**:

- `patches/0007-citus-clock-cohabit-pg-cron.patch`
- `src/backend/distributed/clock/causal_clock.c`
- `src/backend/distributed/sql/udfs/citus_cohabit_clock_tick_reserved/latest.sql`
- `src/include/distributed/causal_clock.h`

**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_cron`

**Summary**: Reserves and exposes a Citus hybrid-logical-clock cohabit flag
when the operator explicitly lists `pg_cron` in `citus.cohabit_extensions`.
The reservation request is made during `_PG_init` and applied when the logical
clock shared memory is initialized, so pg_cron background-worker paths can
verify that Citus clock state was initialized before scheduled jobs call Citus
clock UDFs. The live smoke proves the production boundary: PG17 boots with
`shared_preload_libraries=pg_cron,citus`, parses
`citus.cohabit_extensions=pg_cron`, creates real `citus`, `pg_cron`, and
`ai_blaise_citus`, verifies `pg_catalog.citus_cohabit_clock_tick_reserved()` is
true, runs a scheduled pg_cron worker job that calls Citus clock UDFs, records
nonzero worker evidence rows, and fails closed when the cohabit allowlist is
missing.

**Motivation**: pg_cron jobs may run inside the same postmaster as Citus and
can call Citus clock functions from scheduled maintenance. The clock side of
cohabitation needs an explicit, auditable reservation path instead of relying
on load-order folklore.

**Citus comparison**: Vanilla Citus initializes the clock shared-memory area
but does not record an operator-approved cohabit reservation for pg_cron.

Production evidence: `REQUIRE_DOCKER=1 bash ci/ai-blaise/pg-cron-cohabitation-smoke.sh`
builds this Citus fork into a PG17 image with `postgresql-17-cron`, starts a
real postmaster with `pg_cron,citus`, asserts the SQL-visible TS19 reservation
flag, waits for a scheduled pg_cron worker to insert clock-reserved evidence
rows using `citus_get_node_clock()`, and proves the missing-allowlist case
keeps the reservation false and rejects `assert_cohabit_extension_ready('pg_cron')`.
This production claim is limited to the pg_cron clock-reservation path; it does
not make `pg_cron` a trusted hook-chain coextension, and does not claim
all Bundle1 cohabitation behavior.

**References**:

- Patch: `FEATURE: TS19` in
  `patches/0007-citus-clock-cohabit-pg-cron.patch`
- In-source: `FEATURE: TS19` in
  `src/backend/distributed/clock/causal_clock.c`
- In-source: `FEATURE: TS19` in `src/include/distributed/causal_clock.h`
- CI: `ci/ai-blaise/patches-check.sh`
- CI: `ci/ai-blaise/pg-cron-cohabitation-smoke.sh`
- Evidence file: `artifacts/pg-cron-cohabitation-evidence.tsv`

### TS20: Cohabit Extensions Detection API

**Overlay**:

- `patches/0008-cohabit-extensions-detection-api.patch`
- `src/include/distributed/shared_library_init.h`
- `src/backend/distributed/shared_library_init.c`
- `src/backend/distributed/sql/udfs/citus_cohabit_extension_role/latest.sql`
- `src/backend/distributed/sql/udfs/citus_cohabit_extension_configured/latest.sql`
- `companion/src/extension_catalog.rs`
- `companion/src/citus_timescale.rs`
- `ci/ai-blaise/cohabit-detection-smoke.sh`

**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`, `pg_cron`, `pg_partman`

**Summary**: Adds a role-aware cohabit-extension detection API. The Citus-side
classification distinguishes `timescaledb` as the trusted hook-chain
coextension, `pg_cron` as the clock-side background-worker coextension, and
`pg_partman` as a partition-management cohabitant that is detected without
receiving hook-chain trust. The companion runtime and the `ai_blaise_citus`
SQL fallback mirror that contract with fail-closed detectors and runtime
assertions.

**Motivation**: Cohabitation needs a stable API boundary that can recognize
supported neighbors without turning every listed extension into a trusted hook
owner.

**Citus comparison**: Vanilla Citus has a binary first-hook check and no
role-aware cohabitation classifier.

Production evidence: `REQUIRE_DOCKER=1 bash ci/ai-blaise/pg-cron-cohabitation-smoke.sh`
builds this Citus fork into a PG17 image, starts a real postmaster with
`shared_preload_libraries=pg_cron,citus` and `citus.cohabit_extensions=pg_cron`,
and calls `pg_catalog.citus_cohabit_extension_role(text)` plus
`pg_catalog.citus_cohabit_extension_configured(text)` from the Citus extension
itself. The evidence records `pg_cron` as `clock-worker` and configured,
`timescaledb` as `trusted-hook`, `pg_partman` as `partition-manager`, an unknown
name as `unsupported`, and a negative boot where `pg_cron` remains classified but
not configured without the allowlist. The deterministic companion smoke still
covers the mirrored `ai_blaise_citus` SQL detector. This production claim covers
the role/configuration classifier boundary only; it does not prove live
TimescaleDB or pg_partman extension execution, grant hook-chain trust to
`pg_cron`, or make Bundle1 cohabitation production-ready as a whole.

**References**:

- Patch: `FEATURE: TS20` in
  `patches/0008-cohabit-extensions-detection-api.patch`
- In-source: `FEATURE: TS20` in
  `src/backend/distributed/shared_library_init.c`
- In-source: `FEATURE: TS20` in `companion/src/extension_catalog.rs`
- In-source: `FEATURE: TS20` in `companion/src/citus_timescale.rs`
- SQL UDF: `pg_catalog.citus_cohabit_extension_role(text)`
- SQL UDF: `pg_catalog.citus_cohabit_extension_configured(text)`
- SQL runtime: `companion_internal.cohabit_extension_detection_report(...)`
- SQL runtime: `companion_internal.assert_cohabit_extension_ready(...)`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-cohabit-detection-canonical`
- CI: `ci/ai-blaise/cohabit-detection-smoke.sh`
- CI: `ci/ai-blaise/pg-cron-cohabitation-smoke.sh`
- Evidence file: `artifacts/pg-cron-cohabitation-evidence.tsv`

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
records `companion_vectorizer_definitions`, creates a queue table, enqueues a
document, records `companion_vectorizer_usage_log`, and verifies missing source
columns and invalid chunk overlap fail closed. The Rust sidecar now executes the
worker queue and provider runtime for `A2`-`A6`; vector index creation and
operator reconciliation remain alpha.

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
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Runs the vectorizer sidecar as a long-lived async service with
health, readiness, drain, metrics, manual `/vectorize`, and queue-processing
HTTP surfaces. The production path connects to PostgreSQL, bootstraps the
`ai.vectorizer_queue`, `ai.tenant_budget`, and `ai.usage_log` tables, and
drives the same deterministic worker model that the canonical report exercises.

Production evidence: VM tests run `cargo test -p
ai_blaise_citus_sidecar_vectorizer`, `cargo run -q -p
ai_blaise_citus_sidecar_vectorizer -- run-canonical`, and
`REQUIRE_DOCKER=1 ci/ai-blaise/sidecar-vectorizer-smoke.sh`. The smoke starts
PostgreSQL 17 in Docker, launches the real sidecar binary in `serve` mode on a
fresh ephemeral loopback port, waits on `/readyz`, enqueues 100 rows, verifies
every row reaches `succeeded`, checks usage rows, budget decrementing,
Prometheus metrics, manual `/vectorize` success, fail-closed invalid
`/vectorize` requests, and `/queue/status`.

**Current boundary**: The production-ready claim covers the local Rust
sidecar runtime, mock-provider queue processing, HTTP health/readiness/drain,
metrics, validation, and PostgreSQL-backed queue/budget/usage tables exercised
by the smoke. It does not claim production-scale queue throughput, Kubernetes
execution, tenant billing-system integration, GPU inference, or real external
embedding-provider calls.

**Motivation**: pgai's Python worker is archived and coordinator-oriented. The
fork needs a Rust worker model that can run per Citus worker.

**Citus comparison**: Vanilla Citus does not ship an embedding worker.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: A2` in `sidecar/vectorizer/src/lib.rs`
- Runtime: `FEATURE: A2` in `sidecar/vectorizer/src/runtime/worker.rs` and
  `sidecar/vectorizer/src/runtime/server.rs`
- Executable: `FEATURE: A2` in `sidecar/vectorizer/src/main.rs`
- CI: `ci/ai-blaise/sidecar-vectorizer-smoke.sh`

### A3: Vector Provider Routing

**Overlay**: `sidecar/vectorizer`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides a config-validated embedding-provider registry and
routing policy for deterministic mock providers plus OpenAI-compatible, Azure
OpenAI-compatible, Voyage, Cohere, Ollama, and vLLM-compatible client shapes.
The runtime supports `mock`, `live`, and `mixed` provider modes, but live
network provider modes fail closed unless
`AI_BLAISE_VECTORIZER_ALLOW_LIVE_PROVIDERS=1` is explicitly set, and `mixed`
mode also requires at least one configured live provider. The verified
runtime classifies provider errors as retryable or permanent and retries
transient transport/rate-limit/server failures with bounded exponential backoff
before failing rows.

Production evidence: deterministic unit tests cover provider registry ordering,
mock embeddings, explicit live-provider opt-in parsing, retryable/permanent
error classification, and a flaky provider that succeeds only after retry. The
PostgreSQL-backed smoke runs the real binary through provider routing in `mock`
mode without external network credentials.

**Current boundary**: The production-ready claim covers provider-mode policy,
mock-provider routing, and retry/error handling in the local runtime. It does
not claim successful OpenAI, Azure OpenAI, Voyage, Cohere, Ollama, vLLM, or any
other external provider operation.

**Motivation**: The vectorizer must validate provider routes before spending
tenant budget or dispatching requests.

**Citus comparison**: Vanilla Citus does not route embedding provider calls.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: A3` in `sidecar/vectorizer/src/runtime/provider.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_vectorizer -- run-canonical`
- CI: `ci/ai-blaise/sidecar-vectorizer-smoke.sh`

### A4: Per-Tenant Token Budgets

**Overlay**: `sidecar/vectorizer`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Enforces per-tenant token budgets through atomic PostgreSQL
compare-and-decrement updates on `ai.tenant_budget`. The worker reserves tokens
before provider calls, fails over-budget or unprovisioned tenants without
calling providers, refunds reservations on provider failure, and reconciles
reserved tokens against provider-reported billed usage.

Production evidence: unit and end-to-end tests cover successful reservation,
refund, overrun rejection, missing-budget failure, queue rows from tenants
without budgets, and manual `/vectorize` refund when usage logging fails. The
Docker smoke seeds a real tenant budget and verifies the budget is decremented
after 100 PostgreSQL-backed vectorization jobs while invalid manual requests
fail before budget reservation.

**Current boundary**: The production-ready claim covers local token-budget
reservation, refund, overrun rejection, and provider-billed reconciliation. It
does not claim integration with tenant billing, invoices, credit ledgers, or
external provider billing APIs.

**Motivation**: Vectorization must be multi-tenant-safe before provider calls
are wired in.

**Citus comparison**: Vanilla Citus has no AI-provider budget accounting.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: A4` in `sidecar/vectorizer/src/runtime/budget.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_vectorizer -- run-canonical`
- CI: `ci/ai-blaise/sidecar-vectorizer-smoke.sh`

### A5: Vectorizer Usage Accounting

**Overlay**: `sidecar/vectorizer`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`

**Summary**: Records durable usage rows with tenant, provider, model, token,
cost, and timestamp fields in `ai.usage_log`. The schema is plain-PostgreSQL
compatible and TimescaleDB-hypertable-ready on `recorded_at`, so production can
turn the table into a hypertable without changing the sidecar contract. Queue
processing and manual `/vectorize` both write usage entries through the same
cost table.

Production evidence: unit tests cover usage entry validation and aggregation;
end-to-end tests assert one usage row per succeeded queue row; the Docker smoke
checks at least 100 `ai.usage_log` rows after real sidecar processing; manual
`/vectorize` tests verify budget is refunded if usage logging fails.

**Current boundary**: The production-ready claim covers the local PostgreSQL
usage-log table contract and sidecar writes from the verified mock-provider
runtime. It does not claim TimescaleDB hypertable creation, chargeback pipeline
integration, or external provider invoice reconciliation.

**Motivation**: Cost dashboards and token budgets require a durable accounting
shape before provider calls are enabled for tenant workloads.

**Citus comparison**: Vanilla Citus does not account for embedding provider
usage.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: A5` in `sidecar/vectorizer/src/runtime/usage_log.rs`
- Runtime: `FEATURE: A5` in `sidecar/vectorizer/src/runtime/worker.rs` and
  `sidecar/vectorizer/src/runtime/server.rs`
- Executable: `FEATURE: A5` in `sidecar/vectorizer/src/main.rs`
- CI: `ci/ai-blaise/sidecar-vectorizer-smoke.sh`

### A6: Shard-Local Distributed Vectorize

**Overlay**: `sidecar/vectorizer`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Processes shard-local queue rows with PostgreSQL `FOR UPDATE SKIP
LOCKED` semantics, stale in-flight-row reclamation through a visibility
timeout, per-worker lock ownership, success/failure state transitions, and
embedding storage on succeeded rows. The runtime validates dynamic queue,
budget, and usage table names as `schema.table` identifiers before building SQL.

Production evidence: in-memory tests cover queue lock/complete/failure
semantics, zero-duration runtime knob rejection, and batch length validation;
the PostgreSQL smoke proves the real `ai.vectorizer_queue` table is locked,
processed, and marked succeeded by the actual sidecar binary.

**Current boundary**: The production-ready claim covers a single-node
PostgreSQL-backed shard-local queue worker boundary with `FOR UPDATE SKIP
LOCKED`, visibility timeout, and worker ownership semantics. It does not claim
production-scale distributed queue throughput, cross-worker fairness under
load, or broad semantic-search correctness.

**Motivation**: Distributed vectorization must preserve shard locality and
avoid pushing every embedding job through the coordinator.

**Citus comparison**: Vanilla Citus does not include shard-local embedding
workers.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: A6` in `sidecar/vectorizer/src/runtime/queue.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_vectorizer -- run-canonical`
- CI: `ci/ai-blaise/sidecar-vectorizer-smoke.sh`

### A8: Vector Dimension Via CRD

**Overlay**: `operator/src/crds/vectorizer.rs`, `sidecar/vectorizer`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgvector`

**Summary**: Defines the `Vectorizer` operator spec for source columns,
embedding provider/model selection, destination vector dimensions, chunking,
scheduling, and secret binding. The CRD validator now checks supported
provider/model dimension pairs, renders the sidecar runtime contract as
`AI_BLAISE_VECTORIZER_CONTRACT_*` values, and the sidecar consumes that
contract to fail closed when queue rows or manual `/vectorize` requests use a
different provider/model or when a provider returns an embedding with the wrong
dimension.

Production evidence: VM tests run the operator vectorizer CRD tests, the
sidecar runtime contract tests, `cargo run -q -p ai_blaise_citus_operator --
run-canonical`, and `REQUIRE_DOCKER=1 ci/ai-blaise/sidecar-vectorizer-smoke.sh`.
The smoke starts PostgreSQL 17 and the real vectorizer binary with the A8
contract set to `mock/embed-v1/8`, proves successful rows store eight-dimensional
embeddings, proves manual and queued model mismatches fail before budget or
usage writes, and proves startup rejects an inconsistent mock dimension
contract.

**Current boundary**: The production-ready claim covers supported CRD
provider/model dimension validation, operator-rendered sidecar env contracts,
and local sidecar enforcement in the PostgreSQL-backed mock-provider runtime.
It does not claim live external provider credentials, GPU inference, production
pgvector index creation, or Kubernetes admission/webhook enforcement for every
possible provider model.

**Motivation**: Vectorizer workers need a declarative contract before they can
fan embedding jobs across Citus workers safely.

**Citus comparison**: Vanilla Citus does not ship an AI vectorizer CRD.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: A8` in `operator/src/crds/vectorizer.rs`
- Runtime: `FEATURE: A8` in `sidecar/vectorizer/src/runtime/contract.rs` and
  `sidecar/vectorizer/src/runtime/worker.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- CI: `ci/ai-blaise/sidecar-vectorizer-smoke.sh`

## Topology

### S2: Topology-Aware Placement

**Overlay**: `operator/src/crds/shard_group.rs`, `operator/src/reconcile/shard_group.rs`, `operator/src/reconcile/citus_cluster.rs`, `operator/src/controllers/boundary.rs`
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
sidecar so the operator-owned reconcile contract is executable end-to-end. The
controller boundary model renders typed Conditions and retry classification for
`CitusCluster`, `Hypertable`, `Migration`, and `Tenant` so dry-run planning is
explicit and alpha mutation paths cannot be mistaken for implemented apply
behavior.

Production evidence: Local and VM proof runs `cargo test -p
ai_blaise_citus_operator` (unit tests including reconcile-plan and controller
boundary coverage) and `cargo run -p ai_blaise_citus_operator --
run-reconcile-plans`, which emits the canonical reconcile-plan TSV row
`ai-blaise-citus\t4\t4\ttrue\tfalse\t5\t1\t3\ttrue`. `cargo run -p
ai_blaise_citus_operator -- run-controller-boundary` emits the canonical
dry-run boundary TSV and `ci/ai-blaise/operator-boundary-smoke.sh` proves
`AI_BLAISE_OPERATOR_EXECUTION_MODE=apply` fails closed while Kubernetes apply,
direct SQL execution, and `.status` mutation are still `AlphaNotImplemented`.
The matching SQL apply plan and CloudNativePG cluster manifest are produced
from the canonical `CitusClusterSpec` and `ShardGroupSpec` without external
Kubernetes dependencies. Live in-cluster reconciliation (a Kubernetes
controller loop that watches the CRDs, applies the manifests, and updates
`.status`) remains gated behind the alpha `operator.controllerRbac.enabled`
profile because the operator runtime currently exposes only
health/readiness/metrics, plan-builder helpers, and non-mutating boundary
reports.

**Motivation**: Placement decisions need an operator-owned policy before the
fork can prove zone-aware replication and survival-goal behavior.

**Citus comparison**: Vanilla Citus tracks placements but does not ship a
Kubernetes-native CRD for topology spread constraints.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: S2` in `operator/src/crds/shard_group.rs`,
  `operator/src/reconcile/shard_group.rs`,
  `operator/src/reconcile/citus_cluster.rs`,
  `operator/src/controllers/boundary.rs`
- Acceptance: `e2e/src/timescale_on_citus.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcile-plans`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-controller-boundary`
- CI: `bash ci/ai-blaise/operator-boundary-smoke.sh`
- CI: `cargo test -p ai_blaise_citus_operator`

### S4: Coordinator-Less Topology Mode

**Overlay**: `operator/`, `pool/`, `e2e/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Allows any node to serve as the entry point for single-shard
queries while multi-shard plans route to a chosen plan leader.

Production evidence: `ci/ai-blaise/coordinatorless-mx-live-smoke.sh` runs a
real three-node Citus topology from the VM cohabitation image, registers two
workers from a bootstrap coordinator, creates a distributed
`public.s4_orders` table, calls `start_metadata_sync_to_node` for both
workers, and proves a metadata-synced worker can serve
`SELECT sum(total) ... WHERE tenant_id = 1` with `worker_entry_sum=550`. The
worker-side `EXPLAIN (COSTS OFF)` must contain `Custom Scan (Citus Adaptive)`
and `Task Count: 1` and must not route back through the bootstrap
coordinator. The same smoke then starts the real ai-blaise pool proxy with
`AI_BLAISE_POOL_UPSTREAM_ADDR` pointed at that worker and proves the pool entry
point returns `pool_worker_entry_sum=550`. This closes the bounded S4
production surface for Citus MX metadata sync, worker entry point serving,
pool proxy entry, and the operator CRD admission/reconcile contract that keeps
dedicated coordinators at zero.

The S4 production-ready boundary does not claim coordinator bootstrap removal,
dynamic shard-aware pool routing, multi-shard plan-leader execution,
Kubernetes reconciliation, WAN or cross-region behavior, or every Citus query
shape.

**Motivation**: The classic coordinator is a throughput and availability
bottleneck.

**Citus comparison**: Upstream Citus supports MX metadata on workers but does
not ship ai-blaise's pool/operator topology mode.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: S4` in `operator/src/crds/citus_cluster.rs`
- Acceptance: `e2e/src/timescale_on_citus.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- CI: `ci/ai-blaise/coordinatorless-mx-live-smoke.sh`
- CI: `ci/ai-blaise/topology-consensus-smoke.sh`

### S5: Raft Per Shard Group

**Overlay**: `sidecar/raft`, `operator/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides the bounded Raft sidecar runtime for per-shard-group
coordination: deterministic elections, AppendEntries replication, quorum
commit, durable log/snapshot replay, live HTTP transport between sidecar
processes, and fail-closed placement/failover validation.

Production evidence: `ci/ai-blaise/sidecar-raft-smoke.sh` runs the canonical
three-node round trip, verifies durable log replay and snapshot watermarking,
then starts three separate `ai_blaise_citus_sidecar_raft serve` OS processes on
loopback ports. The live transport proof elects `worker-a` through
`/raft/campaign`, commits `networked-placement-intent` through
`/raft/propose`, verifies `/raft/status` on every voter reports the same
leader, term, commit index, last-log index, and committed payload, and verifies
follower proposals plus malformed `/raft/message` bodies fail closed. The
production-ready boundary is this sidecar consensus/transport component only;
operator-driven membership changes, CNPG failover execution, Citus placement
synchronization, WAN latency/partition behavior, and Kubernetes reconciliation
remain alpha.

**Motivation**: The fork needs sub-five-second failover targets without baking
consensus logic into Postgres backends.

**Citus comparison**: Vanilla Citus relies on external PostgreSQL HA tooling.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: S5` in `sidecar/raft/src/lib.rs`
- In-source: `FEATURE: S5` in `sidecar/raft/src/runtime.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_raft -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_raft -- run-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_raft -- run-durable-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_raft -- serve`
- CI: `ci/ai-blaise/sidecar-raft-smoke.sh`
- CI: `ci/ai-blaise/topology-consensus-smoke.sh`

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
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides the bounded HLC sidecar closed-timestamp gate for
follower reads: local clock ticks, peer clock-exchange observation,
closed-timestamp derivation, `/closed_ts` publication, and fail-closed
`/follower_read` serve/reject decisions.

Production evidence: `ci/ai-blaise/sidecar-hlc-smoke.sh` starts the real
`ai_blaise_citus_sidecar_hlc serve` process with a three-replica shard group,
waits for `/readyz`, verifies the initial `/closed_ts`, advances the local
clock through `/clock/tick`, merges a peer timestamp through `/clock/observe`,
verifies the peer appears in `/closed_ts`, proves `/follower_read` serves an
`AS OF` exactly at the closed timestamp, proves an `AS OF` newer than the
closed timestamp is rejected with HTTP 409, and verifies unknown peers fail
closed. The production-ready boundary is the sidecar closed-timestamp and
follower-read gate only; MVCC snapshot execution, replica query routing,
planner integration, stale-read SQL syntax, cross-region clock discipline, and
Kubernetes reconciliation remain alpha.

**Motivation**: Bounded-staleness reads need a shared clock and closed
timestamp contract before replicas can serve `AS OF` queries.

**Citus comparison**: Vanilla Citus does not provide closed-timestamp follower
reads.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: S9` in `sidecar/hlc/src/lib.rs`
- In-source: `FEATURE: S9` in `sidecar/hlc/src/runtime.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_hlc -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_hlc -- run-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_hlc -- serve`
- CI: `ci/ai-blaise/sidecar-hlc-smoke.sh`
- CI: `ci/ai-blaise/topology-consensus-smoke.sh`

### S10: Schema-Based Tenancy

**Overlay**: `operator/src/crds/tenant.rs`, `operator/src/reconcile/tenant.rs`, `operator/src/controllers/tenant.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Defines the `Tenant` operator spec for one-tenant-per-schema
layouts and reconciles it into a deterministic tenant apply plan.

Production evidence: VM proof for Reconcilers Batch A runs `cargo test -p
ai_blaise_citus_operator` and `ci/ai-blaise/operator-reconcilers-batch-a-smoke.sh`.
The reconciler validates `Tenant` CRs through the kube-rs controller, emits an
idempotent `CREATE SCHEMA IF NOT EXISTS` step, binds installable
`companion_internal.set_tenant_quota(...)` and
`set_tenant_region_affinity(...)` SQL where applicable, publishes a bounded
pool ConfigMap name/payload, and produces an archive-safe delete plan that does
not drop tenant schemas. Executing those SQL steps against a live cluster,
writing Kubernetes status, and enforcing storage bytes in the pool data plane
remain alpha.

**Motivation**: SaaS tenancy needs a declarative lifecycle boundary before
tenant quotas, region affinity, migration, and archive jobs can reconcile.

**Citus comparison**: Vanilla Citus supports schema-based sharding but does not
ship a Kubernetes tenant CRD.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: S10` in `operator/src/crds/tenant.rs`
- In-source: `FEATURE: TO1` / `FEATURE: TO2` / `FEATURE: TO5` in
  `operator/src/reconcile/tenant.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcilers-batch-a`
- CI: `ci/ai-blaise/operator-reconcilers-batch-a-smoke.sh`

### S11: Survival Goals

**Overlay**: `operator/src/crds/survival_goal.rs`, `operator/src/reconcile/survival_goal.rs`, `operator/src/controllers/survival_goal.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines zone-failure and region-failure survival targets and
reconciles them into topology-spread, pgactive, and CNPG replication policy
intent.

Production evidence: VM proof for Reconcilers Batch A runs `cargo test -p
ai_blaise_citus_operator` and `ci/ai-blaise/operator-reconcilers-batch-a-smoke.sh`.
The reconciler rejects duplicate regions, missing region inventory, missing
ShardGroup topology policies, loose region skew, and insufficient replication
factor in unit coverage, and the kube-rs `SurvivalGoal` controller validates
standalone CR shape before building the policy plan. Applying topology spread
to live StatefulSets/Deployments, mutating CNPG replication, and configuring
pgactive remain alpha.

**Motivation**: Replication factor alone is ambiguous; users need an explicit
failure domain goal for topology-aware reconciliation.

**Citus comparison**: Vanilla Citus does not expose a survival-goal API.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: S11` in `operator/src/crds/survival_goal.rs`
- In-source: `FEATURE: S11` / `FEATURE: MR2` in
  `operator/src/reconcile/survival_goal.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcilers-batch-a`
- CI: `ci/ai-blaise/operator-reconcilers-batch-a-smoke.sh`

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
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_lake`, `pg_parquet`

**Summary**: Defines table-granular image and delta layer files for cold shard
storage on object stores, plus runnable canonical move-plan and runtime
emitters.

Production evidence: VM proof runs `cargo test -q -p ai_blaise_citus_sidecar_coldtier --all-targets`,
`cargo run -q -p ai_blaise_citus_sidecar_coldtier -- run-canonical`,
`cargo run -q -p ai_blaise_citus_sidecar_coldtier -- run-runtime-canonical`, and
`cargo run -q -p ai_blaise_citus_sidecar_coldtier -- run-local-file-materialization-canonical`,
with `ci/ai-blaise/sidecar-coldtier-runtime-smoke.sh` wired into CI and `gate-close`. The
production-ready boundary is deterministic local `file://` cold-tier artifact materialization:
layer URIs must stay below the shard object URI, paths fail closed on traversal/relative/space-bearing
forms, the runtime writes canonical image and delta Parquet artifacts under `/tmp/ai-blaise-coldtier`,
verifies `coldtier_local_file_materialization=passed`, `materialized_artifact_count=4`,
`materialized_bytes=1408`, `materialized_layer_files=2`, `object_store_io_attempted=false`, and
`citus_cold_read_serving=false`, and rejects non-file materialization. S3/GCS/Azure object-store
writes, pageserver deployment, Citus cold-read serving, distributed query planner integration,
operator/Kubernetes scheduling, and production object-store lifecycle remain alpha.

**Motivation**: Cold shard data needs a predictable object layout before
operators can evict low-temperature shards from the hot tier.

**Citus comparison**: Vanilla Citus does not provide an S3-backed cold shard
tier.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: R1` in `sidecar/coldtier/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_coldtier -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_coldtier -- run-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_coldtier -- run-local-file-materialization-canonical`
- CI: `ci/ai-blaise/sidecar-coldtier-runtime-smoke.sh`

### R2: Scale-To-Zero Compute

**Overlay**: `operator/src/crds/branch.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds branch-level suspend intent so short-lived compute branches
can scale to zero while retaining their storage declaration.

Production evidence: VM proof runs `ci/ai-blaise/operator-branch-lifecycle-smoke.sh`
and `REQUIRE_DOCKER=1 ci/ai-blaise/operator-branch-scale-to-zero-live-smoke.sh`. The
live smoke creates a real kind Kubernetes cluster, applies a `branch-review` Deployment with one
available replica, validates the operator `run-branch-lifecycle-canonical` suspend plan
(`ready` to `suspended`, six steps, including `ScaleTargetComputeToZero`), executes
`kubectl scale deployment/branch-review --replicas=0`, and verifies
`branch_scale_to_zero_live=passed`, `kubernetes_deployment_scaled_to_zero=true`,
`spec_replicas_after_scale=0`, `observed_replicas_after_scale=0`,
`active_sessions_fail_closed=true`, and `pending_migrations_fail_closed=true`. The
production-ready boundary is the bounded Kubernetes compute scale-down primitive plus
operator fail-closed suspend admission. CSI `VolumeSnapshot` creation, PVC cloning, full branch
suspend/resume reconciliation, Service/DNS retargeting, traffic cut-over, and branch promotion
remain alpha under C6/C7/C8.

**Motivation**: Development, analytics, and point-in-time investigation
branches should not burn compute while idle.

**Citus comparison**: Vanilla Citus does not provide branch lifecycle or
scale-to-zero semantics.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: R2` in `operator/src/crds/branch.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-branch-lifecycle-canonical`
- CI: `ci/ai-blaise/operator-branch-lifecycle-smoke.sh`
- CI: `ci/ai-blaise/operator-branch-scale-to-zero-live-smoke.sh`

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
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines temperature-score thresholds and generated shard move
plans between hot, warm, and cold tiers, then accounts for canonical runtime
move execution.

Production evidence: VM proof runs `cargo test -q -p ai_blaise_citus_sidecar_coldtier --all-targets`,
`cargo run -q -p ai_blaise_citus_sidecar_coldtier -- run-runtime-canonical`,
`cargo run -q -p ai_blaise_citus_sidecar_coldtier -- run-local-file-materialization-canonical`, and
`ci/ai-blaise/sidecar-coldtier-runtime-smoke.sh`. The production-ready boundary is deterministic
policy planning plus local `file://` runtime materialization: invalid threshold order fails closed,
canonical hot-to-cold movement emits one move, and runtime counters record moved shards,
`materialized_artifact_count=4`, `materialized_layer_files=2`, `materialized_bytes=1408`,
`planner_routes_refreshed=1`,
`cold_tier_reads=1`, `object_store_io_attempted=false`, and `citus_cold_read_serving=false` for
the bounded local sidecar cycle. Operator scheduling, live shard relocation, distributed query
planner execution, S3/GCS/Azure object-store IO, pageserver deployment, and Citus cold-read serving remain alpha.

**Motivation**: Tiering policy needs deterministic move plans before an
operator or sidecar starts relocating shard data.

**Citus comparison**: Vanilla Citus does not automate hot/warm/cold shard
movement.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: R5` in `sidecar/coldtier/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_coldtier -- run-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_coldtier -- run-local-file-materialization-canonical`
- CI: `ci/ai-blaise/sidecar-coldtier-runtime-smoke.sh`

### R7: REPACK CONCURRENTLY Adoption

**Overlay**: `operator/src/crds/scheduled_repack.rs`, `sidecar/shared/src/contracts.rs`, `sidecar/repack`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: `pg_repack`

**Summary**: Defines the scheduled repack policy surface for online shard-table
maintenance. The implemented production path validates deterministic repack
plans, fails closed unless the selected capability is available, and executes
the real `pg_repack` binary through the repack sidecar when a database URL is
provided.

**Motivation**: Repack cadence and target tables need to be auditable and
reconciled rather than run as one-off maintenance commands.

**Citus comparison**: Vanilla Citus can use external maintenance tooling but
does not provide a scheduled repack CRD.

**Current boundary**: `run-canonical` remains a deterministic dry-run contract
with `evidence_boundary=dry-run-plan-only`, while `run-live-pg-repack` invokes
the real `pg_repack` binary and reports
`dry_run=false`, `executed=true`, and
`evidence_boundary=live-pg-repack-execution`. `REQUIRE_DOCKER=1`
`ci/ai-blaise/sidecar-repack-smoke.sh` verifies that path against PostgreSQL 17
with the packaged `postgresql-17-repack` extension. This is production-ready for
a sidecar-owned live `pg_repack` invocation against a single local PostgreSQL
target and existing operator plan rendering. It is not production evidence for
PostgreSQL 19 `REPACK CONCURRENTLY`, Kubernetes-scheduled repack execution, or
Citus shard fanout across workers.

Production evidence: `cargo test -p ai_blaise_citus_sidecar_repack --all-targets`
covers fail-closed request validation and DSN redaction;
`bash ci/ai-blaise/sidecar-repack-smoke.sh` preserves the deterministic
`evidence_boundary=dry-run-plan-only` contract; and
`REQUIRE_DOCKER=1 bash ci/ai-blaise/sidecar-repack-smoke.sh` builds the sidecar
into a PostgreSQL 17 container with `postgresql-17-repack`, installs
`pg_repack`, executes the real `pg_repack` command through
`run-live-pg-repack`, and asserts `dry_run=false`, `executed=true`,
`evidence_boundary=live-pg-repack-execution`, row count, and extension presence
after execution.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: R7` in `operator/src/crds/scheduled_repack.rs`
- In-source: `FEATURE: R7` in `operator/src/reconcile/scheduled_repack.rs`
- In-source: `FEATURE: R7` in `operator/src/controllers/scheduled_repack.rs`
- In-source: `FEATURE: R7` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: R7` in `sidecar/repack/src/lib.rs`
- In-source: `FEATURE: R7` in `sidecar/repack/src/main.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_repack -- run-canonical`
- Executable: `AI_BLAISE_REPACK_DATABASE_URL=postgres cargo run -p ai_blaise_citus_sidecar_repack -- run-live-pg-repack`
- CI: `ci/ai-blaise/sidecar-repack-smoke.sh`
- VM evidence: `REQUIRE_DOCKER=1 ci/ai-blaise/sidecar-repack-smoke.sh`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcile-plans-batch-c`
- CI: `ci/ai-blaise/operator-reconcilers-batch-c-smoke.sh`

### R9: Cross-Tier Query Planner Input

**Overlay**: `sidecar/coldtier`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Exposes cold-tier object URIs and shard table identity for
planner paths that span hot, warm, and cold storage, with runtime planner-route
refresh accounting.

Production evidence: VM proof runs `cargo test -q -p ai_blaise_citus_sidecar_coldtier --all-targets`,
`cargo run -q -p ai_blaise_citus_sidecar_coldtier -- run-runtime-canonical`,
`cargo run -q -p ai_blaise_citus_sidecar_coldtier -- run-local-file-materialization-canonical`, and
`ci/ai-blaise/sidecar-coldtier-runtime-smoke.sh`. The production-ready boundary is deterministic route
metadata publication plus local `file://` materialized artifacts: the runtime emits shard id, table,
storage tier, object URI, format, artifact URIs, `planner_routes_refreshed=1`, `cold_tier_reads=1`,
`local_file_materialized=true`, `materialized_artifact_count=4`, `materialized_bytes=1408`,
`object_store_io_attempted=false`, and `citus_cold_read_serving=false`.
The `cold_tier_reads` value is a sidecar accounting counter, not Citus executor evidence. Companion
planner integration, distributed query planner integration, S3/GCS/Azure object-store IO, pageserver
deployment, Citus cold-read serving, production Citus route changes, and Kubernetes traffic remain alpha.

**Motivation**: Cross-tier planning needs machine-readable cold-shard location
and format metadata before the companion planner can combine tiers.

**Citus comparison**: Vanilla Citus does not plan queries across object-store
cold shard layers.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: R9` in `sidecar/coldtier/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_coldtier -- run-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_coldtier -- run-local-file-materialization-canonical`
- CI: `ci/ai-blaise/sidecar-coldtier-runtime-smoke.sh`

### R10: TLS Session Ticket Reuse In Pool

**Overlay**: `pool/src/runtime.rs`, `pool/src/tls.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the pool TLS session-ticket reuse and rotation contract
with a current/previous key ring boundary.

**Current production-ready boundary**: `TicketKey::from_hex` validates 32-byte
hex material fail-closed, `TicketKeyRing` tracks current/previous acceptance and
rotation due state, and `TicketRotationReport` exposes only a redacted
fingerprint length plus boolean evidence. This does not claim rustls listener
integration, mounted Kubernetes Secret loading, external TLS infrastructure,
client session resumption traffic, or certificate rotation.

Production evidence: `cargo test -p ai_blaise_citus_pool --all-targets`,
`cargo run -p ai_blaise_citus_pool -- run-canonical`, and
`ci/ai-blaise/pool-routing-security-smoke.sh` assert rotation due,
previous-key validity, previous-key presence, and non-secret fingerprint-length
columns.

**Motivation**: Connection churn should not pay full TLS handshakes when
rotation and reuse can be controlled explicitly.

**Citus comparison**: Vanilla Citus does not include an external TLS pooler
contract.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: R10` in `pool/src/runtime.rs`
- In-source: `FEATURE: R10` in `pool/src/tls.rs`
- Executable: `cargo run -p ai_blaise_citus_pool -- run-canonical`
- CI: `ci/ai-blaise/pool-routing-security-smoke.sh`

## Change Data And Branching

### C4: Active-Active Conflict Policy

**Overlay**: `operator/src/crds/conflict_policy.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgactive`

**Summary**: Defines table-scoped conflict policy for active-active
reference-table replication contracts.

**Motivation**: Cross-region writes need explicit resolution rules before
replication can be enabled safely.

**Citus comparison**: Vanilla Citus does not ship active-active conflict
policy objects.

Production evidence: `REQUIRE_DOCKER=1 ci/ai-blaise/operator-reconcilers-batch-c-smoke.sh`
generates `run-conflict-policy-runtime-canonical`, boots `CONFLICT_POLICY_IMAGE`,
installs `ai_blaise_citus`, applies the generated SQL to live PostgreSQL/Citus,
and verifies `conflict_policy_live_row` output for `accounts-lww` with
`update_origin_differs`/`apply_remote_if_newer` plus `accounts-merge` with
`update_exists`/`merge_function` and `public.merge_remote_into_local`. The same
runtime smoke verifies `replication_conflict_status` and the companion
`conflict_classes` `7` taxonomy/audit contract. This does not claim live
pgactive conflict traffic, live Spock apply traffic, multi-node active-active
replication, or remote conflict replay.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C4` in `operator/src/crds/conflict_policy.rs`
- In-source: `FEATURE: C4` in `companion/src/replication_conflict.rs`
- In-source: `FEATURE: C4` in `operator/src/reconcile/conflict_policy.rs`
- In-source: `FEATURE: C4` in `operator/src/controllers/conflict_policy.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcile-plans-batch-c`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-conflict-policy-runtime-canonical`
- CI: `ci/ai-blaise/companion-runtime-depth-a-smoke.sh`
- CI: `ci/ai-blaise/operator-reconcilers-batch-c-smoke.sh`

### C5: Replication Conflict Taxonomy

**Overlay**: `operator/src/crds/conflict_policy.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `spock`

**Summary**: Carries the seven conflict classes used by replication-conflict
companion contracts and active-active reconcilers.

**Motivation**: Conflict resolution cannot be observable or testable if all
conflicts collapse into one undifferentiated failure state.

**Citus comparison**: Vanilla Citus does not expose a Spock-style conflict
classification contract.

Production evidence: `ci/ai-blaise/companion-runtime-depth-a-smoke.sh` and
`REQUIRE_DOCKER=1 ci/ai-blaise/operator-reconcilers-batch-c-smoke.sh` verify
that the companion resolves seven conflict classes, emits
`companion.replication_conflict_audit` SQL for every canonical case, reports two
reject outcomes and four fail-closed guards, and that the operator maps the
runtime policy classes to `insert_exists`, `update_exists`,
`update_origin_differs`, `update_missing`, `delete_origin_differs`,
`delete_missing`, and `delete_exists`. The live metadata apply proof records
`conflict_classes` `7` beside `conflict_policy_live_row` evidence for
`accounts-lww` and `accounts-merge`. This does not claim live pgactive conflict
traffic, live Spock apply traffic, PGC1/PGC2 runtime activation, or a production
replication apply worker.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C5` in `operator/src/crds/conflict_policy.rs`
- In-source: `FEATURE: C5` in `companion/src/replication_conflict.rs`
- In-source: `FEATURE: C5` in `operator/src/reconcile/conflict_policy.rs`
- In-source: `FEATURE: C5` in `operator/src/controllers/conflict_policy.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcile-plans-batch-c`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-conflict-policy-runtime-canonical`
- CI: `ci/ai-blaise/companion-runtime-depth-a-smoke.sh`
- CI: `ci/ai-blaise/operator-reconcilers-batch-c-smoke.sh`
- Executable: `patches/postgres/0001-logical-commit-clock.patch` and
  `patches/postgres/0002-per-subtrans-commit-ts.patch` provide the PG-core
  pieces the seven-class conflict resolver needs: monotonic commit timestamps
  to break last-update-wins ties deterministically, and per-subtransaction
  origin attribution so a forced delta apply keeps the remote node id instead
  of the apply worker's. Tracked under FEATURE: PGC1 and FEATURE: PGC2.

### PGC1: PostgreSQL Logical Commit Clock

**Overlay**: `patches/postgres/0001-logical-commit-clock.patch`
**Status**: production-ready
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

Production evidence: `REQUIRE_DOCKER=1 bash ci/ai-blaise/postgres-core-patches-live-smoke.sh`
builds the `images/citus-pg-overlay/Dockerfile.pgcore-patches` runtime from
PostgreSQL `REL_17_10`, applies `patches/postgres/series`, compiles Citus
against the patched `pg_config`, installs the smoke-only
`ai_blaise_pgc_probe` extension, starts the patched server with
`shared_preload_libraries=citus` and `track_commit_timestamp=on`, creates
both `citus` and `ai_blaise_pgc_probe`, and verifies the PGC1 logical-clock
hook changes a subsequent local commit timestamp after the probe advances the
shared `XLogCtl` clock.

**Current production-ready boundary**: PGC1 is production-ready for the PG17
patched-source build, Citus build-against-patched-`pg_config`, initdb/start,
and local logical-clock hook execution path. It does not claim live pgactive or
Spock apply traffic, multi-node active-active conflict replay, PG18, or the
full Bundle1 operand image.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- Upstream: `docs/ai-blaise/UPSTREAM_SYNC.md` (pgsql-hackers + pgEdge/spock
  links)
- In-source: `FEATURE: PGC1` in
  `patches/postgres/0001-logical-commit-clock.patch`
- In-source: `FEATURE: PGC1 PGC2` in `images/citus-pg-overlay/Dockerfile`
- In-source: `FEATURE: PGC1 PGC2` in
  `images/citus-pg-overlay/Dockerfile.pgcore-patches`
- In-source: `FEATURE: PGC1 PGC2` in `ci/ai-blaise/pgc_probe`
- Executable: `make -f Makefile.ai-blaise patches-check` validates the diff
  format and FEATURE markers.
- Executable: `REQUIRE_DOCKER=1 bash ci/ai-blaise/postgres-core-patches-live-smoke.sh`

### PGC2: PostgreSQL Per-Subtransaction Commit Timestamps

**Overlay**: `patches/postgres/0002-per-subtrans-commit-ts.patch`
**Status**: production-ready
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

Production evidence: `REQUIRE_DOCKER=1 bash ci/ai-blaise/postgres-core-patches-live-smoke.sh`
uses the same patched PostgreSQL 17 + Citus runtime as PGC1, then calls the
patch-only `SubTransactionIdSetCommitTsData` path through
`ai_blaise_pgc_probe`. The smoke commits a row whose xid has an override
timestamp, verifies `pg_xact_commit_timestamp(xid)` equals that override, and
stops the server before `pg_waldump` verifies the `SUBTRANS_TS` record name.

**Current production-ready boundary**: PGC2 is production-ready for the PG17
patched-source build, Citus build-against-patched-`pg_config`, initdb/start,
local commit timestamp override, and WAL identity proof. It does not claim live
pgactive or Spock delta-apply traffic, multi-node active-active conflict replay,
PG18, or the full Bundle1 operand image.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- Upstream: `docs/ai-blaise/UPSTREAM_SYNC.md` (pgsql-hackers + pgEdge/spock
  links)
- In-source: `FEATURE: PGC2` in
  `patches/postgres/0002-per-subtrans-commit-ts.patch`
- In-source: `FEATURE: PGC1 PGC2` in `images/citus-pg-overlay/Dockerfile`
- In-source: `FEATURE: PGC1 PGC2` in
  `images/citus-pg-overlay/Dockerfile.pgcore-patches`
- In-source: `FEATURE: PGC1 PGC2` in `ci/ai-blaise/pgc_probe`
- Executable: `make -f Makefile.ai-blaise patches-check` validates the diff
  format and FEATURE markers.
- Executable: `REQUIRE_DOCKER=1 bash ci/ai-blaise/postgres-core-patches-live-smoke.sh`

### C6: CSI Snapshot Branching

**Overlay**: `operator/src/crds/branch.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the branch source-cluster, target-cluster, storage,
branch-type, and CSI snapshot-class contract needed for snapshot-backed cluster
branches, and proves the snapshot-and-clone narrative end to end against a
live kind cluster with the csi-driver-host-path snapshot stack.

**Motivation**: Branching needs an operator-owned API before CSI snapshot and
copy-on-write implementations can be reconciled safely.

**Current boundary**: The C6 production-ready claim is bounded to the kind +
csi-driver-host-path live evidence path in
`ci/ai-blaise/operator-branch-lifecycle-live-smoke.sh`. It does not claim
cloud provider CSI behavior, multi-zone snapshot replication, regional
snapshot transport, Citus distributed data-plane during branch operations, or
full PVC lifecycle reconciliation driven by the ai-blaise operator binary.
The deterministic operator contract checks (run-branch-lifecycle-canonical)
continue to gate the apply/suspend/promote plan shape.

Production evidence: `REQUIRE_DOCKER=1 bash ci/ai-blaise/operator-branch-lifecycle-live-smoke.sh`
creates a kind cluster, installs external-snapshotter v8.2.0 +
csi-driver-host-path v1.14.0, creates a primary StatefulSet with a PVC
seeded by an initContainer-written tenant-marker, takes a real
`VolumeSnapshot` of the primary PVC, waits for `readyToUse=true`,
creates a branch PVC with `spec.dataSource` pointing at the snapshot,
attaches it to a branch StatefulSet, and verifies the branch pod observes
the snapshotted tenant-marker. The evidence row is appended to
`artifacts/branch-lifecycle-live-evidence.tsv` with the primary marker,
the branch-side marker, the kind node image, and the namespace.

**Citus comparison**: Vanilla Citus does not ship snapshot branch automation.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C6` in `operator/src/crds/branch.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-branch-lifecycle-canonical`
- CI: `ci/ai-blaise/operator-branch-lifecycle-smoke.sh`

### C7: Branch Suspend

**Overlay**: `operator/src/crds/branch.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Carries suspend intent on the branch spec, validates the
operator-side suspend transition from `ready` to `suspended`, and proves
the live StatefulSet scale-to-zero / resume cycle against a kind cluster.

**Motivation**: Branch lifecycle must be declarative to avoid orphaned compute
or ad hoc suspend state.

**Current boundary**: The C7 production-ready claim covers the kind StatefulSet
scale-to-zero and back-to-one cycle proven by
`ci/ai-blaise/operator-branch-lifecycle-live-smoke.sh`, complementing the
existing R2 scale-to-zero compute primitive evidence. It does not claim
production connection-draining semantics, in-flight transaction quiesce, or
operator reconciler-driven suspend orchestration; those remain separately
tracked.

Production evidence: the live smoke scales `statefulset/branch-review` to
0 replicas after C6 materialization, waits for `spec.replicas=0` and
`status.replicas=0`, then scales back to 1 and confirms the resumed pod
is healthy before C8 Service cutover.

**Citus comparison**: Vanilla Citus has no branch suspend/resume surface.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C7` in `operator/src/crds/branch.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-branch-lifecycle-canonical`
- CI: `ci/ai-blaise/operator-branch-lifecycle-smoke.sh`

### C8: Branch Promote

**Overlay**: `operator/src/crds/branch.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Establishes typed branch identity, source/target readiness checks
for deterministic branch promotion planning, and proves live Service-endpoint
cutover from primary to branch against a kind cluster.

**Motivation**: Promote/cut-over workflows need the same branch object that
created and suspended the branch, so status and ownership stay consistent.

**Current boundary**: The C8 production-ready claim covers the kind Service
selector cutover proven by
`ci/ai-blaise/operator-branch-lifecycle-live-smoke.sh`. It does not claim
production DNS retargeting, BGP/anycast network reconvergence, multi-region
cutover, or write-side drain semantics; those remain separately tracked.

Production evidence: the live smoke creates a `client-service` selecting
`branch-source` pods, then patches the selector to `branch-review` and
waits for endpoints to converge onto `branch-review-0`. The evidence file
records `cutover_endpoint_pod=branch-review-0` to confirm the Service-level
cutover succeeded against real Kubernetes.

**Citus comparison**: Vanilla Citus does not provide branch promotion.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C8` in `operator/src/crds/branch.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-branch-lifecycle-canonical`
- CI: `ci/ai-blaise/operator-branch-lifecycle-smoke.sh`

### C9: Migration Framework

**Overlay**: `operator/src/crds/migration.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the migration CRD surface for pgroll-style and gh-ost
style online DDL workflows.

Production evidence: Local and VM proof run `cargo test -p ai_blaise_citus_operator migration`
and `ci/ai-blaise/operator-reconcilers-batch-c-smoke.sh`. The promoted boundary is
fail-closed CRD/controller validation and deterministic reconcile/apply-plan rendering: invalid
YAML, unknown migration types, unknown conflict actions, unsafe backfill SQL, missing
`companion_internal.verify_two_version_invariant()` prechecks, missing rollback references,
missing worker lists, and unverified PUBLIC cutovers are rejected before any apply plan is emitted.
Live Kubernetes sidecar invocation and data-plane DDL execution remain alpha-blocked.

**Motivation**: Expand/contract schema changes need an operator-visible unit
that can coordinate validation, retries, and conflict handling.

**Citus comparison**: Vanilla Citus does not ship an online-migration CRD.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C9` in `operator/src/crds/migration.rs`
- In-source: `FEATURE: C9` in `operator/src/reconcile/migration.rs`
- In-source: `FEATURE: C9` in `operator/src/controllers/migration.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcile-plans-batch-c`
- CI: `ci/ai-blaise/operator-reconcilers-batch-c-smoke.sh`

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
zero leases fail closed. `ci/ai-blaise/schema-txn-runtime-smoke.sh` also runs
the real schema-job binary through canonical worker output, controller advance
/ wait / rollback output, manifest validation, unsafe SQL rejection, malformed
JSON rejection, and loopback probe behavior. Actual DDL execution workers, dual-write triggers,
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
- Executable: `cargo run -p ai_blaise_citus_sidecar_schema_job -- run-controller-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_schema_job -- validate-manifest <path>`
- CI: `ci/ai-blaise/schema-txn-runtime-smoke.sh`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### C1: CDC Sidecar

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/cdc`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Implements a live CDC sidecar runtime with wal2json ingest, a pgoutput logical-frame decoder boundary, checkpoint/ack state, health/readiness/metrics, sink wire encoders, DLQ tracking, and a CDC-to-realtime bridge.

**Motivation**: Realtime, webhooks, analytical mirrors, and external sinks all
need one validated CDC stream contract.

**Citus comparison**: Vanilla Citus does not ship an out-of-process CDC
sidecar.

Production evidence: VM worker D on experiment-playground, 2026-05-23: `cargo test -p ai_blaise_citus_sidecar_cdc`, `bash ci/ai-blaise/sidecar-cdc-smoke.sh`, and `cargo run -q -p ai_blaise_citus_sidecar_cdc -- run-live-canonical` prove wal2json ingest, pgoutput logical-frame decoder boundary, checkpoint/ack advancement, health/ready/metrics, seven sink wire frames, PII anonymization before dispatch, file/in-memory DLQ, and the realtime bridge frame format.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C1` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: C1` in `sidecar/cdc/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_cdc -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_cdc -- run-runtime-canonical`
- In-source: `FEATURE: C1` in `sidecar/cdc/src/source.rs`
- In-source: `FEATURE: C1` in `sidecar/cdc/src/live.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_cdc -- run-live-canonical`
- CI: `ci/ai-blaise/sidecar-cdc-smoke.sh`

### C2: Schema-Aware CDC Sinks

**Overlay**: `sidecar/cdc`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Carries schema/table metadata through every CDC event and sink frame, plus DDL stream-table and included-schema contracts for consumers that need schema-change timelines.

**Motivation**: Downstream mirrors and queues need a pgstream-style schema
timeline so consumers do not decode WAL against stale table metadata.

**Citus comparison**: Vanilla Citus does not ship schema-aware CDC sink
coordination.

Production evidence: `cargo test -p ai_blaise_citus_sidecar_cdc --lib` covers the schema-capture parser and live-runtime report path, and `REQUIRE_DOCKER=1 bash ci/ai-blaise/sidecar-cdc-smoke.sh` boots the real CDC sidecar plus a live `postgres:17-bookworm` DDL capture harness. The smoke creates a `cdc.ddl_events` stream table, installs `CREATE EVENT TRIGGER ai_blaise_capture_ddl`, executes `CREATE TABLE public.cdc_schema_smoke`, converts the captured row into a wal2json frame, posts it through the same `/ingest` replication boundary, and asserts `ddl_events_total`, `ddl_stream_table`, `command_tag`, `object_schema`, `object_identity`, and per-event `ddl_event` JSON. This makes C2 production-ready for schema-qualified row-event delivery, sink wire encoding, and DDL stream-table parsing through the sidecar runtime; it does not claim managed broker delivery, multi-node Kubernetes traffic, or long-running logical replication slot tailing beyond the existing CDC consumer contract.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C2` in `sidecar/cdc/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_cdc -- run-runtime-canonical`
- In-source: `FEATURE: C2` in `sidecar/cdc/src/source.rs`
- CI: `ci/ai-blaise/sidecar-cdc-smoke.sh`

### C3: CDC PII Anonymization

**Overlay**: `sidecar/cdc`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `anon`

**Summary**: Applies table/column anonymization rules inside the CDC runtime before any sink frame is encoded or bridged to realtime.

**Motivation**: CDC frequently leaves the Postgres trust boundary; tagged PII
columns need a first-class redaction contract before external sink delivery.

**Citus comparison**: Vanilla Citus does not apply anonymization policy to
logical replication streams.

Production evidence: VM worker D on experiment-playground, 2026-05-23: `cargo test -p ai_blaise_citus_sidecar_cdc` covers hash/null/redact policies and `bash ci/ai-blaise/sidecar-cdc-smoke.sh` proves the live `/ingest` path hashes `email` before all sink frames are emitted.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C3` in `sidecar/cdc/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_cdc -- run-runtime-canonical`
- In-source: `FEATURE: C3` in `sidecar/cdc/src/anon.rs`
- CI: `ci/ai-blaise/sidecar-cdc-smoke.sh`

### C14: CDC NATS Sink

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/cdc`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds validated NATS subject routing plus a deterministic NATS core `PUB` frame encoder and plain TCP dispatch boundary for CDC event delivery.

**Motivation**: Low-latency event consumers need a NATS route with the same
retry and dead-letter policy as webhook and realtime sinks.

**Citus comparison**: Vanilla Citus does not publish CDC events to NATS.

Production evidence: VM Worker CDC-Sinks on experiment-playground, 2026-05-24: `cargo test -p ai_blaise_citus_sidecar_shared -p ai_blaise_citus_sidecar_cdc` covers fail-closed NATS subject/auth URL validation, deterministic `PUB` frame encoding, and live-dispatch DLQ retry accounting; `bash ci/ai-blaise/sidecar-cdc-smoke.sh` proves the serve-runtime and canonical stdout paths expose the NATS target and encoded frame.

Current boundary: The production-evidenced surface is the protocol frame, strict local validation, and DLQ-on-dispatch-failure accounting. Live broker authentication, TLS, JetStream, and managed NATS operations remain alpha.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C14` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: C14` in `sidecar/cdc/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_cdc -- run-runtime-canonical`
- In-source: `FEATURE: C14` in `sidecar/cdc/src/sinks.rs`
- CI: `ci/ai-blaise/sidecar-cdc-smoke.sh`

### C15: CDC GCP Pub/Sub Sink

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/cdc`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds validated GCP Pub/Sub project/topic routing plus deterministic `messages.publish` request-body encoding for CDC event delivery.

**Motivation**: Managed cloud consumers need a Pub/Sub route without forking
the CDC sidecar delivery model.

**Citus comparison**: Vanilla Citus does not publish CDC events to GCP
Pub/Sub.

Production evidence: VM Worker CDC-Sinks on experiment-playground, 2026-05-24: `cargo test -p ai_blaise_citus_sidecar_shared -p ai_blaise_citus_sidecar_cdc` covers fail-closed Pub/Sub project/topic validation and deterministic `messages.publish` request-body encoding; `bash ci/ai-blaise/sidecar-cdc-smoke.sh` proves the serve-runtime and canonical stdout paths expose the Pub/Sub target and encoded frame.

Current boundary: The production-evidenced surface is the deterministic request body and strict local validation. Live GCP authentication, IAM, retries against Pub/Sub, and managed-topic operations remain alpha.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: C15` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: C15` in `sidecar/cdc/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_cdc -- run-runtime-canonical`

- In-source: `FEATURE: C15` in `sidecar/cdc/src/sinks.rs`
- CI: `ci/ai-blaise/sidecar-cdc-smoke.sh`

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
migration. The companion runtime depth A smoke also exercises the Rust
`canonical_migration_runtime_report()` path alongside the durable queue and
replication-conflict reports. Actual distributed DDL execution, schema-job
workers, online backfill, lock orchestration, rollback execution, and operator
CRD reconciliation remain alpha.

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
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- CI: `ci/ai-blaise/companion-runtime-depth-a-smoke.sh`
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
plan. `ci/ai-blaise/schema-txn-runtime-smoke.sh` also verifies the real
schema-job manifest validator accepts bounded backfill manifests and rejects
unsafe SQL fragments before any apply boundary. Actual online DDL execution,
trigger dual-writes, backfill workers, cutover validation, rollback, and
distributed-table orchestration remain alpha.

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
- Executable: `cargo run -p ai_blaise_citus_sidecar_schema_job -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_schema_job -- validate-manifest <path>`
- CI: `ci/ai-blaise/schema-txn-runtime-smoke.sh`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`

### M3: Migration CRD

**Overlay**: `operator/src/crds/migration.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds typed migration declarations with inline YAML DSL and
conflict handling mode.

Production evidence: Local and VM proof run `cargo test -p ai_blaise_citus_operator migration`
and `ci/ai-blaise/operator-reconcilers-batch-c-smoke.sh`. The production-ready boundary is the
CRD validation and dry-run reconcile contract only: the YAML DSL is parsed structurally,
operation names and required fields are checked, unsafe multi-statement/commented SQL fragments
are refused, the 2VI precheck and rollback SQL references are mandatory, and the rendered apply
plan starts with an idempotent 2VI preflight step. Live migration execution, dual-write triggers,
and distributed backfill workers remain alpha.

**Motivation**: Migration runs need to be reviewed and reconciled as desired
state instead of launched imperatively.

**Citus comparison**: Vanilla Citus provides distributed DDL primitives but no
operator-owned migration object.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: M3` in `operator/src/crds/migration.rs`
- In-source: `FEATURE: M3` in `operator/src/reconcile/migration.rs`
- In-source: `FEATURE: M3` in `operator/src/controllers/migration.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcile-plans-batch-c`
- CI: `ci/ai-blaise/operator-reconcilers-batch-c-smoke.sh`

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
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the CLI plan/apply execution contract, including
rendered diffs, preflight checks, apply execution, and audit-record steps.

**Current production-ready boundary**: M8 is production-ready for two real
binary paths. The D1 dev lifecycle path remains bounded to
`citusctl plan/apply dev ... --state-dir ... --format json|tsv`: dry-run plan
rendering, fail-closed stable plan-id validation, deterministic JSON/TSV
output, idempotent up/down state transitions, local audit append, and
state-file-only cleanup guardrails. The Kubernetes manifest path is now live:
`citusctl plan/apply apply <manifest> --namespace <namespace> --state-dir
<dir> --format json|tsv` validates the YAML manifest, performs a real
`kubectl apply --dry-run=server`, emits a deterministic `k8s-apply-*` plan id,
requires apply to match that rendered plan id, runs real `kubectl apply`,
verifies resources with `kubectl get -f`, and writes a local
`k8s-manifest-apply.audit.tsv` log with `live-kubernetes-manifest-apply`
evidence. It does not claim Docker/kind lifecycle orchestration, migrations,
backups, PITR, WAL replay, multi-step Citus data-plane rollout semantics, or
production cluster lifecycle management beyond applying the supplied manifest.

Production evidence: `ci/ai-blaise/citusctl-smoke.sh`,
`ci/ai-blaise/citusctl-dev-lifecycle-smoke.sh`, and
`ci/ai-blaise/citusctl-k8s-apply-live-smoke.sh` exercise the real CLI binary.
The live smoke creates a kind cluster, runs `citusctl plan apply` through
server-side dry-run, rejects a mismatched apply plan id, applies a ConfigMap to
Kubernetes, verifies it with `kubectl get`, proves idempotent reapply, rejects
a malformed manifest, and checks `k8s-manifest-apply.audit.tsv`.

**Motivation**: Operator actions need a Terraform-style preview before
mutating clusters, tenants, branches, migrations, backups, or extension state.

**Citus comparison**: Vanilla Citus does not ship an operator CLI with
two-step plan/apply semantics.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: M8` in `tools/citusctl/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citusctl -- run-canonical`
- Executable: `cargo run -p ai_blaise_citusctl -- run-dev-lifecycle-canonical`
- CI: `ci/ai-blaise/citusctl-k8s-apply-live-smoke.sh`
- CI: `ci/ai-blaise/citusctl-dev-lifecycle-smoke.sh`

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
closed. The shared tools runtime also fails closed on duplicate snapshot
identities and vectorizer/realtime tenant references to unknown tenants. Direct
DrawDB front-end embedding, browser collaboration, and live operator/companion
watch streams remain alpha.

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
orchestration remain alpha. The companion runtime depth A smoke includes
this Rust migration report in its deterministic evidence row.

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
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- CI: `ci/ai-blaise/companion-runtime-depth-a-smoke.sh`
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
forward-progress phase honors the two-version invariant.
`ci/ai-blaise/schema-txn-runtime-smoke.sh` also verifies the real schema-job
binary emits deterministic controller advance, wait, and rollback decisions
with 2VI SQL evidence. Distributed backfill workers, kube-rs
MigrationReconciler client, dual-write triggers, and live planner-hook
enforcement of the WRITE_ONLY/DELETE_ONLY read/write invariants remain alpha.

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
- In-source: `FEATURE: M14` in `operator/src/controllers/migration.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcile-plans-batch-c`
- CI: `ci/ai-blaise/operator-reconcilers-batch-c-smoke.sh`
- SQL runtime: `FEATURE: M14` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_sidecar_schema_job -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_schema_job -- run-controller-canonical`
- CI: `ci/ai-blaise/schema-txn-runtime-smoke.sh`
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

**Overlay**: `operator/src/crds/region.rs`, `operator/src/reconcile/region.rs`, `operator/src/controllers/region.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines named regions with Kubernetes zone and tablespace mapping,
then reconciles them into PostgreSQL tablespace and Kubernetes affinity intent.

Production evidence: VM proof for Reconcilers Batch A runs `cargo test -p
ai_blaise_citus_operator` and `ci/ai-blaise/operator-reconcilers-batch-a-smoke.sh`.
The `RegionReconcilePlan` validates CR input, emits a tablespace inspection
query, renders a separately executable `CREATE TABLESPACE` statement, and
publishes deterministic node-affinity and optional leader-pinning labels. Live
PostgreSQL tablespace execution, CNPG primary relocation, and Kubernetes status
updates remain alpha.

**Motivation**: Multi-region placement and tenant affinity need stable region
objects rather than repeated stringly typed zone settings.

**Citus comparison**: Vanilla Citus has tablespaces and placements but no
region CRD.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: MR1` in `operator/src/crds/region.rs`
- In-source: `FEATURE: MR1` in `operator/src/reconcile/region.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcilers-batch-a`
- CI: `ci/ai-blaise/operator-reconcilers-batch-a-smoke.sh`

### MR2: SurvivalGoal CRD

**Overlay**: `operator/src/crds/survival_goal.rs`, `operator/src/reconcile/survival_goal.rs`, `operator/src/controllers/survival_goal.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Declares whether the cluster should survive zone or region
failure and how many replicas must remain available.

Production evidence: VM proof for Reconcilers Batch A runs `cargo test -p
ai_blaise_citus_operator` and `ci/ai-blaise/operator-reconcilers-batch-a-smoke.sh`.
The reconciler maps region failure to `topology.kubernetes.io/region`, maps
zone failure to `topology.kubernetes.io/zone`, and validates ShardGroup
replication/topology inventory in deterministic unit tests. Live topology
mutation and cross-region replication setup remain alpha.

**Motivation**: The operator must be able to reject impossible survival goals
before it places shards.

**Citus comparison**: Vanilla Citus does not encode failure-domain objectives.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: MR2` in `operator/src/crds/survival_goal.rs`
- In-source: `FEATURE: MR2` in `operator/src/reconcile/survival_goal.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcilers-batch-a`
- CI: `ci/ai-blaise/operator-reconcilers-batch-a-smoke.sh`

### MR4: Tablespaces By Region

**Overlay**: `operator/src/crds/region.rs`, `operator/src/reconcile/region.rs`, `operator/src/controllers/region.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Adds a declarative region-to-tablespace mapping for region-affine
storage placement.

Production evidence: VM proof for Reconcilers Batch A runs `cargo test -p
ai_blaise_citus_operator` and `ci/ai-blaise/operator-reconcilers-batch-a-smoke.sh`.
The reconciler sanitizes tablespace paths, quotes identifiers and literals, and
keeps the inspection step separate from `CREATE TABLESPACE` because PostgreSQL
does not allow `CREATE TABLESPACE` inside a transactional `DO` block. Live
execution against PostgreSQL and storage-class placement remain alpha.

**Motivation**: Tablespaces are the PostgreSQL primitive, but the operator
needs a higher-level region policy to keep placements understandable.

**Citus comparison**: Vanilla Citus can use PostgreSQL tablespaces but does not
manage them as region objects.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: MR4` in `operator/src/crds/region.rs`
- In-source: `FEATURE: MR4` in `operator/src/reconcile/region.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcilers-batch-a`
- CI: `ci/ai-blaise/operator-reconcilers-batch-a-smoke.sh`

### MR5: Pool GeoIP Routing

**Overlay**: `pool/src/runtime.rs`, `pool/src/geoip.rs`, `pool/src/proxy.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines and executes bounded pool region routing from CIDR/GeoIP
region resolution to nearest preferred replicas.

Production evidence: `ci/ai-blaise/pool-geoip-live-smoke.sh` starts two real
`postgres:17-bookworm` regional replicas with different `geo_route_marker`
rows, starts the real `ai_blaise_citus_pool serve` data plane with
`AI_BLAISE_POOL_GEO_DEFAULT_REGION=us-east-1`,
`AI_BLAISE_POOL_GEO_RULES=127.0.0.0/8=us-east-1`, and
`AI_BLAISE_POOL_GEO_REPLICAS` pointing at the two regional PostgreSQL ports,
and proves a PostgreSQL client query through the pool reaches the selected
`us-east-1` replica with `geoip_pool_route_selected_region=us-east-1`. The
same smoke restarts the pool with a rule resolving to an unknown region and
proves default-region fallback reaches `us-east-1` with
`geoip_pool_fallback_region=us-east-1`, then proves an invalid CIDR fails
closed during pool startup. The pool exposes
`ai_blaise_citus_pool_geo_routes_total` and
`ai_blaise_citus_pool_geo_fallback_routes_total` metrics for this route path.

The MR5 production-ready boundary is the static env-configured CIDR/replica
pool data-plane path only. It does not claim managed MaxMind DB loading,
Region-CR synchronization, hot-swap reloads, cross-region/WAN traffic behavior,
edge-replica traffic, or Kubernetes traffic.

**Motivation**: Multi-region reads need a pool-side routing contract before
GeoIP and edge-replica behavior can be enforced.

**Citus comparison**: Vanilla Citus does not provide GeoIP-aware pool routing.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: MR5` in `pool/src/runtime.rs`
- In-source: `FEATURE: MR5` in `pool/src/geoip.rs`
- In-source: `FEATURE: MR5` in `pool/src/proxy.rs`
- Executable: `cargo test -p ai_blaise_citus_pool --all-targets`
- Executable: `cargo run -p ai_blaise_citus_pool -- run-canonical`
- CI: `ci/ai-blaise/pool-routing-security-smoke.sh`
- CI: `ci/ai-blaise/pool-geoip-live-smoke.sh`

### MR8: Leader Pinning Per Region

**Overlay**: `operator/src/crds/region.rs`, `operator/src/reconcile/region.rs`, `operator/src/controllers/region.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Carries leader-pinning intent on regions so HA reconcilers can
constrain primaries to chosen failure domains.

Production evidence: VM proof for Reconcilers Batch A runs `cargo test -p
ai_blaise_citus_operator` and `ci/ai-blaise/operator-reconcilers-batch-a-smoke.sh`.
The region reconciler emits the deterministic `ai-blaise.com/leader-region`
label only when `leader_pinned` is true and the kube-rs controller validates
the CR before logging the resolved plan. CNPG primary relocation and failover
orchestration remain alpha.

**Motivation**: Multi-region clusters need explicit write-leader placement to
control latency and failover behavior.

**Citus comparison**: Vanilla Citus leaves primary placement to external HA
tooling.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: MR8` in `operator/src/crds/region.rs`
- In-source: `FEATURE: MR8` in `operator/src/reconcile/region.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcilers-batch-a`
- CI: `ci/ai-blaise/operator-reconcilers-batch-a-smoke.sh`

## Backup / PITR

### B1: Backup Sidecar

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/backup`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Runs a WAL-G-backed backup HTTP runtime with command execution,
base-backup listing/status, scheduled backup cycles, retention pruning,
health/readiness, and Prometheus metrics.

Production evidence: `ci/ai-blaise/sidecar-backup-smoke.sh` builds the real
`ai_blaise_citus_sidecar_backup` binary on the VM/CI runner, starts `serve`
on a fresh ephemeral loopback port with deterministic local `wal-g`, `pg_ctl`,
and `psql` fakes, then exercises `/readyz`, `/metrics`, `/backups/run`,
`/backups/status`, `/backups`, `/backups/delete-old`, `/wal/status`,
`/pitr/restore`, `/pitr/status/<job>`, and `/branches/queryable`. Unit tests
cover the WAL-G command builder, provider-specific WAL-G prefix environment,
non-zero exit propagation, strict object-store URI and UTC target-time
validation, retention and WAL-G failure accounting, schedule calculation, HTTP
body parsing, scheduler due-run behavior, and branch read-only config
materialization. This proves the local sidecar runtime and
process-orchestration contract without requiring or claiming a live cloud
bucket.

**Current boundary**: This production-ready claim is intentionally narrow: it
covers the sidecar process, command materialization/execution against local
fakes, HTTP API, scheduler, retention accounting, failure metrics, and status
paths. It does not prove a cloud object-store account, external secret wiring,
Backup CR reconciliation, successful WAL-G publish/fetch against remote
storage, or a full Kubernetes restore against a real WAL-G archive.

**Motivation**: Backup execution needs a sidecar runtime that matches the
operator CRD and fails through auditable command/status surfaces.

**Citus comparison**: Vanilla Citus delegates backup sidecars to deployment
tooling.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: B1` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: B1` in `sidecar/backup/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_backup -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_backup -- run-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_backup -- serve`
- CI: `ci/ai-blaise/sidecar-backup-smoke.sh`

### B2: Backup CRD

**Overlay**: `operator/src/crds/backup.rs`, `operator/src/reconcile/backup.rs`, `operator/src/controllers/backup.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines backup schedule, retention, object-store target, and
provider consumed by the backup sidecar reconciler and runtime contracts.

Production evidence: VM proof for Reconcilers Batch A runs `cargo test -p
ai_blaise_citus_operator` and `ci/ai-blaise/operator-reconcilers-batch-a-smoke.sh`.
The backup reconciler validates the CR, derives provider-specific archive URIs,
emits per-resource sidecar Deployment and ConfigMap plan names, registers
backup status endpoints, and rejects unknown providers. Actual WAL-G execution,
object-store writes, archive deletion, and restore orchestration remain alpha.

**Motivation**: PITR and backup-as-data-source workflows need an auditable
declarative schedule.

**Citus comparison**: Vanilla Citus does not ship a cluster backup CRD.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: B2` in `operator/src/crds/backup.rs`
- In-source: `FEATURE: B2` in `operator/src/reconcile/backup.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcilers-batch-a`
- CI: `ci/ai-blaise/operator-reconcilers-batch-a-smoke.sh`

### B3: PITR Restore

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/backup`, `tools/citusctl`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Validates PITR target windows, invokes `wal-g backup-fetch`,
records restore jobs, and exposes restore status through the backup sidecar
HTTP runtime.

Production evidence: `cargo test -p ai_blaise_citus_sidecar_backup` covers
PITR plan validation, calendar-valid canonical UTC target timestamps,
wrong-archive rejection before WAL-G invocation, out-of-window target
rejection, WAL-G failure accounting, success/failure job recording, and HTTP
`/pitr/restore` plus `/pitr/status/<job>` behavior. The backup smoke then
starts the real binary and proves both an out-of-window restore failure and an
in-window restore success through the sidecar HTTP API using a deterministic
safe WAL-G fake.

**Current boundary**: The production-ready claim covers sidecar PITR
orchestration, validation, local WAL-G command execution, accounting, and
status reporting. It does not claim that a real production archive, cloud
credential, in-place restore, or operator-driven target-cluster rollout has
been live-exercised.

**Motivation**: PITR restore needs explicit target validation before sidecar
code executes recovery.

**Citus comparison**: Vanilla Citus does not ship PITR restore orchestration.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: B3` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: B3` in `sidecar/backup/src/lib.rs`
- In-source: `FEATURE: B3` in `tools/citusctl/src/lib.rs`
- In-source: `FEATURE: B3` in `e2e/src/dr_restore_depth.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_backup -- run-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_e2e --bin dr_restore_depth_report`
- CI: `REQUIRE_DOCKER=1 ci/ai-blaise/dr-restore-depth-check.sh`

### B4: Backup-As-Data-Source

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/backup`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Restores a backup into a named branch directory, writes
PostgreSQL recovery configuration, starts the branch with `pg_ctl`, probes
`default_transaction_read_only`, and exposes branch creation/listing through
`/branches/queryable`.

Production evidence: unit tests cover recovery file generation,
`postgresql.auto.conf` read-only settings, `pg_ctl` command construction,
`psql` read-only probe construction, strict branch-name and port validation,
duplicate branch rejection, and HTTP create/list behavior. The backup smoke
exercises the real binary end to end with local `wal-g`, `pg_ctl`, and `psql`
fakes and verifies that invalid branch ports fail closed, a branch is created,
listed, and rejected on duplicate creation.

**Current boundary**: The production-ready claim covers the sidecar branch
mount boundary, local restore command orchestration, and read-only enforcement
probe. It does not prove long-running query load, operator lifecycle
management, or a restore from a real remote archive.

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
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds CLI validation for UTC time-travel targets before follower
read and backup-backed query paths execute.

**Current production-ready boundary**: B5 is production-ready for the real
`citusctl plan/apply time-travel <target_time> --now <utc_now>
--max-staleness-seconds <seconds> --state-dir <dir> --format json|tsv` intent
path. The CLI performs strict RFC3339 UTC calendar validation, rejects
ahead-of-now targets, rejects targets older than the explicit staleness window, emits a
deterministic `time-travel-*` plan id, requires apply to match that rendered
plan id, and appends `time-travel-intent.audit.tsv` with
`time-travel-intent-validation-only` evidence. This does not execute follower
reads, backup-backed query replay, closed-timestamp MVCC reads, or Citus
executor integration; those remain covered by S9/MR6 and backup runtime
boundaries.

Production evidence: `ci/ai-blaise/citusctl-time-travel-intent-smoke.sh`
drives the real CLI binary through deterministic JSON planning, TSV apply,
audit append, mismatched plan-id rejection, invalid UTC timestamp rejection,
out-of-window rejection, and ahead-of-now rejection. `cargo test -p
ai_blaise_citusctl --all-targets` covers leap-day, invalid calendar, age, and
ahead-of-now validation at the Rust boundary.

**Motivation**: Time-travel operations need explicit timestamp validation at
the operator entrypoint before sidecars and companion GUCs consume the request.

**Citus comparison**: Vanilla Citus does not ship time-travel orchestration.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: B5` in `tools/citusctl/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citusctl -- run-canonical`
- CI: `ci/ai-blaise/citusctl-time-travel-intent-smoke.sh`

### B6: Encrypted Backups

**Overlay**: `operator/src/crds/backup.rs`, `operator/src/reconcile/backup.rs`, `sidecar/backup`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Validates the backup encryption contract from the operator
backup reconciler through the sidecar WAL-G runtime, including KMS references
and `WALG_GPG_KEY_ID` materialization before encrypted backup work is accepted.

Production evidence: `cargo test -p ai_blaise_citus_sidecar_backup` verifies
that encrypted plans render `WALG_GPG_KEY_ID`, reject missing encryption
environment, and keep encrypted-artifact accounting in the runtime report. The
backup smoke runs the real binary with the canonical encrypted plan and proves
base backup command execution against local fakes, status, PITR, retention,
and queryable branch paths preserve the encrypted runtime state. VM proof for
Reconcilers Batch A runs
`cargo test -p ai_blaise_citus_operator` and
`ci/ai-blaise/operator-reconcilers-batch-a-smoke.sh`; the backup reconciler
validates non-empty KMS references and emits a distinct `KmsBinding` apply step
into the sidecar configuration plan, while unencrypted plans skip that step
deterministically.

**Current boundary**: The production-ready claim covers operator-side KMS
binding validation, sidecar environment validation, and encrypted backup
orchestration against the local WAL-G command boundary. It does not prove
hardware KMS, External Secrets, cloud IAM, key rotation, or encrypted
object-store archives with real WAL-G credentials.

**Motivation**: Backup encryption must be configured with the schedule, not
attached later by an external script.

**Citus comparison**: Vanilla Citus delegates backup encryption entirely to
deployment-specific tooling.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: B6` in `operator/src/crds/backup.rs`
- In-source: `FEATURE: B6` in `operator/src/reconcile/backup.rs`
- In-source: `FEATURE: B6` in `sidecar/backup/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_backup -- run-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcilers-batch-a`
- CI: `ci/ai-blaise/operator-reconcilers-batch-a-smoke.sh`
- Executable: `cargo run -p ai_blaise_citus_e2e --bin dr_restore_depth_report`
- CI: `REQUIRE_DOCKER=1 ci/ai-blaise/dr-restore-depth-check.sh`

## Tenant Operations

### TO1: Tenant CRD

**Overlay**: `operator/src/crds/tenant.rs`, `operator/src/reconcile/tenant.rs`, `operator/src/controllers/tenant.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Introduces the tenant lifecycle object used by tenant migration,
archive, quotas, and region-affinity workflows.

Production evidence: VM proof for Reconcilers Batch A runs `cargo test -p
ai_blaise_citus_operator` and `ci/ai-blaise/operator-reconcilers-batch-a-smoke.sh`.
The tenant reconciler validates the CR through kube-rs, emits a quoted
`CREATE SCHEMA IF NOT EXISTS` step, builds bounded DNS-safe pool ConfigMap
names, and generates an archive-first delete plan that never drops tenant
schemas. Live schema execution and status writes remain alpha.

**Motivation**: Tenant operations require a first-class unit of ownership
rather than interpreting arbitrary schema names.

**Citus comparison**: Vanilla Citus does not ship tenant lifecycle objects.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TO1` in `operator/src/crds/tenant.rs`
- In-source: `FEATURE: TO1` in `operator/src/reconcile/tenant.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcilers-batch-a`
- CI: `ci/ai-blaise/operator-reconcilers-batch-a-smoke.sh`

### TO2: Tenant Quotas

**Overlay**: `operator/src/crds/tenant.rs`, `operator/src/reconcile/tenant.rs`, `operator/src/controllers/tenant.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds connection, QPS, and storage quotas to tenant declarations.

Production evidence: VM proof for Reconcilers Batch A runs `cargo test -p
ai_blaise_citus_operator` and `ci/ai-blaise/operator-reconcilers-batch-a-smoke.sh`.
The reconciler rejects zero quotas, maps connection/QPS limits to the
`companion_internal.set_tenant_quota(...)` SQL contract, and carries
storage bytes in the pool ConfigMap contract for downstream admission control.
Live pool enforcement of storage bytes remains alpha.

**Motivation**: Pool and sidecar enforcement need a typed quota source before
runtime admission control is wired in.

**Citus comparison**: Vanilla Citus has no per-tenant quota CRD.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: TO2` in `operator/src/crds/tenant.rs`
- In-source: `FEATURE: TO2` in `operator/src/reconcile/tenant.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcilers-batch-a`
- CI: `ci/ai-blaise/operator-reconcilers-batch-a-smoke.sh`

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
execution, worker index rollout, distributed DDL application, and shard-aware
query fanout remain alpha. The `SearchIndex` operator plan-builder and
kube-rs watch/controller path are covered separately by `Search7`.

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

**Overlay**: `operator/src/crds/search_index.rs`,
`operator/src/reconcile/search_index.rs`,
`operator/src/controllers/search_index.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_search`

**Summary**: Adds the Kubernetes-facing `SearchIndex` object for declarative
text and hybrid search indexes, plus an operator reconcile plan that installs
`pg_search`, creates the operator-owned provenance tables, delegates distributed
worker fanout to the companion search bridge, and records hybrid vector linkage
for status/provenance.

Production evidence: VM proof run `cargo check -p ai_blaise_citus_operator
--tests`, `cargo test -p ai_blaise_citus_operator`, and `cargo run -q -p
ai_blaise_citus_operator -- run-reconcilers-batch-b`. The canonical batch-B
row reports `search_apply_steps=5` and `search_hybrid=true`. The kube-rs
`SearchIndex` controller mirrors the CR into the authoritative Rust spec and
builds the same plan during `serve`. Actual pg_search BM25 execution,
distributed DDL application, and shard-aware query fanout remain alpha.

**Motivation**: Search indexes need lifecycle and validation before companion
SQL and sidecar cold-tier integration can be reconciled.

**Citus comparison**: Vanilla Citus does not provide search-index lifecycle
objects.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Search7` in `operator/src/crds/search_index.rs`
- In-source: `FEATURE: Search7` in `operator/src/reconcile/search_index.rs`
- Controller: `operator/src/controllers/search_index.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcilers-batch-b`

### Search8: Search-Aware Cold Tier

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/coldtier`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds search-index enablement to the analytical mirror contract so
cold-tier data can preserve search semantics, and materializes cold-tier
Tantivy/LanceDB search artifacts in the runtime report.

Production evidence: VM proof runs `cargo test -q -p ai_blaise_citus_sidecar_coldtier --all-targets`,
`cargo run -q -p ai_blaise_citus_sidecar_coldtier -- run-runtime-canonical`,
`cargo run -q -p ai_blaise_citus_sidecar_coldtier -- run-local-file-materialization-canonical`, and
`ci/ai-blaise/sidecar-coldtier-runtime-smoke.sh`. The production-ready boundary is deterministic
search artifact materialization for local `file://` cold-tier artifacts: index columns must be SQL
identifiers, index URIs must be typed Tantivy/LanceDB/index paths, the runtime writes the two search
artifact files under `/tmp/ai-blaise-coldtier/indexes`, and the smoke verifies
`search_indexes_materialized=2`, `materialized_artifact_count=4`, `materialized_bytes=1408`,
`object_store_io_attempted=false`, and `citus_cold_read_serving=false`. Real Tantivy/LanceDB index
construction, real Tantivy/LanceDB query execution, Citus query fanout, S3/GCS/Azure object-store IO,
pageserver deployment, and Citus cold-read serving remain alpha.

**Motivation**: Cold-tier movement should not discard full-text or hybrid
search availability.

**Citus comparison**: Vanilla Citus does not manage search-aware cold-tier
mirrors.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Search8` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: Search8` in `sidecar/coldtier/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_coldtier -- run-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_coldtier -- run-local-file-materialization-canonical`
- CI: `ci/ai-blaise/sidecar-coldtier-runtime-smoke.sh`

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
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_lake`

**Summary**: Defines the analytical sidecar plan that binds a logical mirror to a lakehouse read path, plus runnable canonical execution-plan and runtime emitters. The pg_lake engine label appears in the binding contract surface; the composite live IO evidence path is proven via the L3 local Parquet read smoke, the L5 local Iceberg snapshot commit smoke, and the F3 Apache Iceberg REST catalog smoke.

**Motivation**: HTAP routing needs a concrete sidecar contract before pg_lake
or equivalent execution is wired into queries.

**Citus comparison**: Vanilla Citus does not ship a pg_lake-backed analytical
sidecar.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L1` in `sidecar/analytical/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_analytical -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_analytical -- run-runtime-canonical`
- CI: `ci/ai-blaise/sidecar-analytical-smoke.sh`

**Current boundary**: The L1 production-ready claim covers the lakehouse-read binding contract (sidecar/analytical engine field accepts pg_lake) and the composite live IO evidence chain (L3 Parquet + L5 Iceberg snapshot + F3 Iceberg REST catalog). It does NOT extend to the upstream pg_lake Postgres extension runtime (alpha-deferred until pg_lake ships as an installable PG extension), Delta runtime reads, MotherDuck cloud sessions, Citus planner integration with pg_lake, or full Kubernetes lakehouse-traffic orchestration.

Production evidence: `bash ci/ai-blaise/l1-pg-lake-analytical-live-smoke.sh` runs the analytical sidecar runtime canonical (engine=datafusion, format=iceberg, lakehouse_reads=1, snapshot_commits=1, federated_catalog_publications=4, duckdb_extension_loads=2, query_engine_executions=1), asserts the sidecar/analytical/src/lib.rs source references the pg_lake engine label, and cross-references the composite live IO evidence chain (L3 Parquet read + L5 Iceberg snapshot commit + F3 Iceberg REST catalog round-trip). Evidence row in `artifacts/l1-pg-lake-analytical-evidence.tsv`.

### L2: Rust Analytical Server

**Overlay**: `sidecar/analytical`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines Rust-native analytical engine selection for DataFusion,
DuckDB, or pg_lake-backed execution, plus bounded local DataFusion runtime
accounting.

Production evidence: `ci/ai-blaise/sidecar-analytical-smoke.sh` runs
`cargo run -p ai_blaise_citus_sidecar_analytical -- run-runtime-canonical` and
verifies a real in-process DataFusion query over an Arrow `RecordBatch`, with
`query_engine_executed=true`, `datafusion_output_rows=2`,
`external_io_attempted=false`, and
`evidence_boundary=local-datafusion-recordbatch-only`. The same smoke starts
the analytical sidecar probe server and verifies `/healthz`, `/readyz`,
`/metrics`, and drain behavior.

**Current production-ready boundary**: L2 is production-ready only for the
local Rust sidecar runtime and in-process DataFusion execution surface. It does
not claim pg_lake, external object-store IO, Iceberg runtime reads, Delta
runtime reads, DuckDB runtime, MotherDuck runtime, logical-replication mirror
materialization, Citus planner integration, Kubernetes traffic, or production
query routing through the pool.

**Motivation**: The analytical path should avoid a Python server in the hot
query path.

**Citus comparison**: Vanilla Citus does not include an out-of-process Rust
analytical server.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L2` in `sidecar/analytical/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_analytical -- run-runtime-canonical`
- CI: `ci/ai-blaise/sidecar-analytical-smoke.sh`

### L3: Iceberg, Parquet, and Delta Reads

**Overlay**: `sidecar/analytical`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_lake`, `pg_parquet`

**Summary**: Defines the lakehouse read plan for Iceberg, Parquet, and Delta
objects, then executes the canonical lakehouse read report.

Production evidence: `ci/ai-blaise/sidecar-analytical-parquet-read-smoke.sh`
runs `cargo run -p ai_blaise_citus_sidecar_analytical -- run-local-parquet-read-canonical`,
writes a real local Parquet file with `ArrowWriter`, registers that file through
DataFusion `ParquetReadOptions`, and queries it with projected columns,
`total > 0`, and `LIMIT 2`. The smoke requires
`parquet_lakehouse_read_live=passed`, `l3_local_parquet_file_created=true`,
`l3_datafusion_parquet_read_executed=true`, `l3_source_rows=4`,
`l3_source_total=5500`, `l3_datafusion_output_rows=2`,
`l3_datafusion_output_total=3000`, and
`evidence_boundary=local-datafusion-parquet-file-only`.

**Current production-ready boundary**: L3 is production-ready only for local
Parquet file materialization plus local DataFusion Parquet reads. It does not
claim Iceberg runtime reads, Delta runtime reads, object-store IO, pg_lake,
MotherDuck, Citus planner integration, warehouse federation, or Kubernetes
traffic.

**Motivation**: Warm and cold analytical storage needs one validated format and
object-URI contract before execution engines fan out reads.

**Citus comparison**: Vanilla Citus does not read Iceberg, Parquet, or Delta
tables through a sidecar.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L3` in `sidecar/analytical/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_analytical -- run-local-parquet-read-canonical`
- CI: `ci/ai-blaise/sidecar-analytical-parquet-read-smoke.sh`
- CI: `ci/ai-blaise/sidecar-analytical-smoke.sh`

### L4: DataFusion Pushdown

**Overlay**: `sidecar/analytical`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines projected-column, predicate, and limit pushdown contracts
for DataFusion execution and verifies their bounded local runtime shape.

Production evidence: `ci/ai-blaise/sidecar-analytical-smoke.sh` verifies the
runtime report from a real DataFusion query over a local Arrow `RecordBatch`:
`projection_pushdown_executed=true`, `filter_pushdown_executed=true`,
`limit_pushdown_executed=true`, `datafusion_output_rows=2`,
`datafusion_output_total=3000`, and
`evidence_boundary=local-datafusion-recordbatch-only`.

**Current production-ready boundary**: L4 is production-ready only for local
DataFusion projection/filter/limit execution inside the analytical sidecar
runtime. It does not claim object-store scan pushdown, Parquet/Iceberg/Delta
file pruning, Citus planner integration, distributed pushdown across workers,
pool data-plane query routing, Kubernetes traffic, or benchmarked analytical
performance.

**Motivation**: Analytical execution has to preserve pool and planner
predicate intent instead of scanning full object-store tables.

**Citus comparison**: Vanilla Citus does not push plans into DataFusion.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L4` in `sidecar/analytical/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_analytical -- run-runtime-canonical`
- CI: `ci/ai-blaise/sidecar-analytical-smoke.sh`

### L5: Iceberg Snapshot Commit At Prepare

**Overlay**: `sidecar/analytical`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines transaction, snapshot, prepare-LSN, and manifest URI
contracts for aligning Iceberg snapshot commits with distributed prepare, plus
canonical runtime commit reporting.

Production evidence: `ci/ai-blaise/sidecar-analytical-iceberg-snapshot-smoke.sh`
runs `cargo run -p ai_blaise_citus_sidecar_analytical -- run-local-iceberg-snapshot-commit-canonical`,
writes a local manifest JSON, local Iceberg-style metadata JSON, and a
`current-snapshot.txt` pointer using temp-file plus atomic rename and fsync,
then reads the artifacts back. The smoke requires
`iceberg_snapshot_commit_live=passed`, `l5_local_metadata_written=true`,
`l5_local_manifest_written=true`, `l5_current_pointer_committed=true`,
`l5_prepare_lsn_recorded=true`, `l5_snapshot_metadata_round_tripped=true`,
`atomic_rename_used=true`, `fsync_executed=true`, and
`evidence_boundary=local-iceberg-snapshot-metadata-commit-only`.

**Current production-ready boundary**: L5 is production-ready only for a local
prepare-LSN metadata commit primitive that durably writes manifest, metadata,
and current-snapshot pointer artifacts. It does not claim live Iceberg catalog
commits, object-store IO, a Citus prepare hook, multi-writer conflict detection,
warehouse federation, or Kubernetes traffic.

**Motivation**: Warm-tier visibility must line up with Citus distributed
transaction boundaries.

**Citus comparison**: Vanilla Citus has no Iceberg snapshot commit protocol.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L5` in `sidecar/analytical/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_analytical -- run-local-iceberg-snapshot-commit-canonical`
- CI: `ci/ai-blaise/sidecar-analytical-iceberg-snapshot-smoke.sh`
- CI: `ci/ai-blaise/sidecar-analytical-smoke.sh`

### L6: Lakehouse Federation Catalogs

**Overlay**: `sidecar/analytical`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Publishes a bounded, versioned local federation-catalog artifact for
Databricks, Snowflake, Trino, and Spark Iceberg catalog targets, then proves that
artifact can be served and consumed over local HTTP without touching external
warehouses or object storage.

Production evidence: VM proof run
`bash ci/ai-blaise/sidecar-analytical-federation-catalog-live-smoke.sh` invokes
`run-federation-catalog-publication-canonical`, writes the v1 JSON catalog
artifact for the four canonical targets, parses it with Python `json.load`,
serves the artifact with a loopback `python3 -m http.server`, fetches it through
`curl`, and byte-compares the fetched payload with the generated artifact. The
smoke requires `federation_catalog_publication_live=passed`,
`l6_catalog_version=v1`, `l6_catalog_count=4`,
`l6_federation_targets=databricks,snowflake,trino,spark`,
`l6_local_catalog_artifact_created=true`,
`l6_local_http_catalog_served=true`, and
`evidence_boundary=local-federation-catalog-artifact-http-only`.

The production-ready claim is intentionally bounded to local federation catalog
artifact publication and loopback HTTP serving. The smoke and report require
`external_warehouse_connections_attempted=false`,
`object_store_io_attempted=false`, and `catalog_auth_exercised=false`. It is not
evidence for live Snowflake, live Trino, live Spark, or live Databricks
warehouse connections, catalog authentication, object-store catalog reads, F3
warehouse federation, or Kubernetes traffic.

**Motivation**: External analytical readers need a versioned federation contract
without learning Citus shard placement directly.

**Citus comparison**: Vanilla Citus does not publish lakehouse catalogs for
external engines.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L6` in `sidecar/analytical/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_analytical -- run-federation-catalog-publication-canonical`
- CI: `ci/ai-blaise/sidecar-analytical-federation-catalog-live-smoke.sh`

### L8: Mooncake-Style Logical-Replication Mirror

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/cdc`, `sidecar/analytical`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides a bounded live logical-replication mirror materialization
path: PostgreSQL `test_decoding` changes for `public.l8_orders` are consumed
from stdin, validated, written to a local TSV mirror artifact, and queried via
DataFusion as the analytical read surface.

Production evidence: `ci/ai-blaise/sidecar-analytical-mirror-live-smoke.sh`
starts a real PostgreSQL 17 container with `wal_level=logical`, creates a
`test_decoding` logical slot, inserts three rows into `public.l8_orders`,
consumes `pg_logical_slot_get_changes`, runs
`run-logical-mirror-materialization-from-stdin`, writes the local mirror
artifact, registers that `.tsv` artifact through `CsvReadOptions`, queries it
through DataFusion, and verifies `logical_mirror_live=passed`,
`l8_test_decoding_slot_consumed=true`, `l8_materialized_rows=3`,
`l8_materialized_total=6000`, and `l8_datafusion_mirror_query_executed=true`.
The release gate wires the same smoke through `sidecar-analytical-mirror-live-smoke`.

Boundary: this production claim is the local live logical-decoding to local TSV
artifact plus DataFusion `.tsv` read path only. It intentionally records
`object_store_io_attempted=false`, `long_running_slot_tailing=false`,
`checkpoint_persistence_exercised=false`, and `kubernetes_traffic_exercised=false`;
it must not be cited as evidence for object-store mirror writes, a long-running
logical-replication mirror daemon, exactly-once checkpoint persistence, Citus
distributed mirror routing, or Kubernetes traffic.

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
- Executable: `cargo run -p ai_blaise_citus_sidecar_analytical -- run-logical-mirror-materialization-from-stdin`
- CI: `ci/ai-blaise/sidecar-analytical-smoke.sh`
- CI: `ci/ai-blaise/sidecar-analytical-mirror-live-smoke.sh`

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
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_duckdb`

**Summary**: Provides a bounded DuckDB extension allow-list and live runtime
proof that the catalog's `httpfs` and `iceberg` extensions can be installed,
loaded, and observed through DuckDB's `duckdb_extensions()` catalog.

Production evidence: `ci/ai-blaise/sidecar-analytical-duckdb-extension-live-smoke.sh`
runs `run-duckdb-extension-catalog-canonical`, verifies the sidecar catalog emits
`INSTALL httpfs`, `LOAD httpfs`, `INSTALL iceberg`, and `LOAD iceberg`, then runs
a pinned DuckDB container
`duckdb/duckdb@sha256:ddc7ffc382dfd3f8213ac3d29435a7ce0ea4446fb3fc966a57a28d39b46174b1`.
Inside that real DuckDB runtime, the smoke executes the install/load statements
and queries `duckdb_extensions()` for `httpfs,true,true` and `iceberg,true,true`.
It requires `duckdb_extension_catalog_live=passed`,
`l12_extensions_installed=2`, `l12_extensions_loaded=2`, and
`l12_duckdb_extensions_catalog_queried=true`.

Boundary: this production claim is the pinned DuckDB container extension-catalog
load path only. It intentionally records `pg_duckdb_runtime_exercised=false`,
`motherduck_session_exercised=false`, `object_store_io_attempted=false`, and
`extension_repository_mirror_verified=false`; it must not be cited as evidence
for pg_duckdb inside PostgreSQL, MotherDuck cloud sessions, object-store reads,
warehouse federation, or an internally mirrored DuckDB extension repository.

**Motivation**: DuckDB extension use needs to be explicit before sidecars load
code from extension repositories.

**Citus comparison**: Vanilla Citus does not manage DuckDB extension catalogs.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: L12` in `sidecar/analytical/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_analytical -- run-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_analytical -- run-duckdb-extension-catalog-canonical`
- CI: `ci/ai-blaise/sidecar-analytical-smoke.sh`
- CI: `ci/ai-blaise/sidecar-analytical-duckdb-extension-live-smoke.sh`

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
- CI: `ci/ai-blaise/sidecar-analytical-smoke.sh`

## Auto API

### API1: PostgREST Sidecar

**Overlay**: `sidecar/postgrest`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `postgrest`

**Summary**: Defines schemas and REST routes exposed by the PostgREST
sidecar, plus a supervised upstream PostgREST launch and proxy path.

Production evidence: VM proof run `bash ci/ai-blaise/postgrest-live-data-plane-smoke.sh` builds the real `ai_blaise_citus_sidecar_postgrest` binary, copies the official PostgREST 12.2.12 binary from `postgrest/postgrest:v12.2.12`, launches it through `run-live-postgrest`, writes `postgrest.conf` with environment-backed secret references, starts `serve` with `AI_BLAISE_POSTGREST_UPSTREAM`, and verifies table-backed GET/POST traffic through `/api/public/orders` against a live PostgreSQL/Citus data plane. The smoke also verifies unauthenticated requests fail closed, method rejection still happens at the sidecar, and database URI/JWT secret material do not appear in the dependency report or generated config. `bash ci/ai-blaise/graphql-postgrest-runtime-smoke.sh` remains the fast runtime proof for probes, OpenAPI JSON, config rendering, descriptor fallback, malformed input, and dependency validation.

**Motivation**: Auto-REST needs a validated route surface before the sidecar
starts serving table-backed endpoints.

**Citus comparison**: Vanilla Citus does not ship a PostgREST sidecar.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: API1` in `sidecar/postgrest/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_postgrest -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_postgrest -- run-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_postgrest -- run-live-postgrest`
- CI: `ci/ai-blaise/sidecar-api-runtime-smoke.sh`
- CI: `ci/ai-blaise/graphql-postgrest-runtime-smoke.sh`
- CI: `ci/ai-blaise/postgrest-live-data-plane-smoke.sh`

### API2: Distributed PostgREST Views

**Overlay**: `sidecar/postgrest`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `postgrest`

**Summary**: Binds REST routes to helper views with distribution column and
shard-count metadata.

Production evidence: VM proof run `bash ci/ai-blaise/postgrest-live-data-plane-smoke.sh` starts a Citus-capable PostgreSQL image, creates the real `citus` extension, distributes `public.orders` with `create_distributed_table('public.orders', 'tenant_id')`, asserts the table is registered in `pg_dist_partition`, creates a security-invoker `api.orders` view, and verifies sidecar `/api/orders` requests are proxied to upstream PostgREST with the `api` schema profile while `/api/public/orders` exercises the base table. The production-ready claim covers the sidecar-to-PostgREST distributed-view/profile binding over a live Citus catalog; it does not claim operator-driven multi-worker placement, rebalance orchestration, or GraphQL execution.

**Motivation**: Auto-REST over distributed tables needs a versioned view contract
so requests route through Citus-aware helper views.

**Citus comparison**: Vanilla Citus does not generate PostgREST helper views.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: API2` in `sidecar/postgrest/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_postgrest -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_postgrest -- run-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_postgrest -- run-live-postgrest`
- CI: `ci/ai-blaise/sidecar-api-runtime-smoke.sh`
- CI: `ci/ai-blaise/graphql-postgrest-runtime-smoke.sh`
- CI: `ci/ai-blaise/postgrest-live-data-plane-smoke.sh`

### API3: GraphQL Sidecar

**Overlay**: `sidecar/graphql`, `companion/src/graph_bridge.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_graphql`

**Summary**: Defines the GraphQL endpoint path, schema bindings, and exposed
tables for the GraphQL sidecar, plus a runnable canonical binding emitter.

Production evidence: VM proof runs
`bash ci/ai-blaise/graphql-postgrest-runtime-smoke.sh` and
`bash ci/ai-blaise/graphql-pggraphql-live-smoke.sh`. The runtime smoke builds
and serves `ai_blaise_citus_sidecar_graphql`, then verifies live GraphiQL HTML,
POST `/graphql/v1` query handling with tenant JWT claims, missing-claim and
introspection-denied error taxonomy, malformed body handling, `/graphql/ws`
subscription registration, WebSocket-upgrade boundary errors, persistent
`/drain` readiness, metrics, and fail-closed runtime dependency validation for
database URL and JWT secret inputs. The live data-plane smoke starts a
PostgreSQL image containing `pg_graphql`, creates an RLS-protected
`public.account` table with tenant A and tenant B rows, runs the real GraphQL
sidecar in `AI_BLAISE_GRAPHQL_LIVE_EXECUTION=1` mode against that database,
posts tenant-scoped queries through `/graphql/v1`, proves the sidecar executes
`graphql.resolve(...)` and returns only the caller tenant's row, verifies the
opposite tenant row is hidden by PostgreSQL RLS, and checks database URL/JWT
secret material is absent from GraphQL responses. This production-ready
boundary covers live query execution through `pg_graphql`, tenant-claim
installation via `request.jwt.claims`, RLS-preserving HTTP query handling, and
the existing subscription registration boundary; it does not claim durable
GraphQL subscription fan-out, GraphQL query planning across multiple Citus
workers (multi-worker GraphQL planning), or Kubernetes traffic.

**Motivation**: GraphQL routing needs a typed endpoint and schema-binding
contract before exposing pg_graphql to tenants.

**Citus comparison**: Vanilla Citus does not ship a GraphQL sidecar.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: API3` in `sidecar/graphql/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_graphql -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_graphql -- run-runtime-canonical`
- CI: `ci/ai-blaise/sidecar-api-runtime-smoke.sh`
- CI: `ci/ai-blaise/graphql-postgrest-runtime-smoke.sh`
- CI: `ci/ai-blaise/graphql-pggraphql-live-smoke.sh`

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
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Requires RLS, JWT secret references, and tenant claims for
auto-API routes.

Production evidence: VM proof run `bash ci/ai-blaise/postgrest-live-data-plane-smoke.sh` signs HS256 JWTs with `role=web_user` and `tenant_id` claims, sends them through the PostgREST sidecar proxy to upstream PostgREST, and verifies PostgreSQL RLS enforces tenant isolation end to end: unauthenticated reads fail closed, tenant A and tenant B SELECTs only return their own rows, a tenant A INSERT for tenant A succeeds, and a tenant A cross-tenant INSERT for tenant B is rejected and leaves no row behind. The same smoke verifies database URI and JWT secret values stay out of `check-runtime-dependencies` output and `postgrest.conf`. The production-ready claim covers the PostgREST auto-REST data-plane path; API3 has separate live `pg_graphql` execution evidence in `ci/ai-blaise/graphql-pggraphql-live-smoke.sh`.

**Motivation**: Auto-generated APIs must preserve tenant isolation rather than
exposing raw distributed tables.

**Citus comparison**: Vanilla Citus does not enforce RLS-aware auto-API
policy.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: API5` in `sidecar/postgrest/src/lib.rs`
- In-source: `FEATURE: API5` in `sidecar/graphql/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_postgrest -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_postgrest -- run-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_postgrest -- run-live-postgrest`
- Executable: `cargo run -p ai_blaise_citus_sidecar_graphql -- run-runtime-canonical`
- CI: `ci/ai-blaise/sidecar-api-runtime-smoke.sh`
- CI: `ci/ai-blaise/graphql-postgrest-runtime-smoke.sh`
- CI: `ci/ai-blaise/postgrest-live-data-plane-smoke.sh`

### API6: Auto OpenAPI Document

**Overlay**: `sidecar/postgrest`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `postgrest`

**Summary**: Defines the OpenAPI path, title, and version served by the
PostgREST sidecar.

Production evidence: VM proof run `bash ci/ai-blaise/graphql-postgrest-runtime-smoke.sh` builds the real `ai_blaise_citus_sidecar_postgrest` binary, starts `serve` on a loopback TCP listener, and fetches `/openapi.json` over HTTP. The smoke parses the response as JSON and verifies OpenAPI 3.0 metadata, title/version, canonical `/orders` GET/POST operations, `public.orders` tags, `x-ai-blaise` schemas, `rls_required=true`, `tenant_claim=tenant_id`, and absence of database URI or JWT secret material. The same smoke verifies health, readiness, metrics, drain behavior, fail-closed startup dependency validation, and generated `postgrest.conf` secret references.

**Current boundary**: The API6 production-ready claim covers the deterministic OpenAPI document served by the Rust PostgREST sidecar front door. API1/API2/API5 have separate production evidence for the PostgREST REST data-plane path in `ci/ai-blaise/postgrest-live-data-plane-smoke.sh`; API3 has separate live `pg_graphql` query execution evidence in `ci/ai-blaise/graphql-pggraphql-live-smoke.sh`; API6 itself does not claim GraphQL OpenAPI generation.

**Motivation**: Client generation and API inspection need a predictable
OpenAPI endpoint.

**Citus comparison**: Vanilla Citus does not serve OpenAPI documents.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: API6` in `sidecar/postgrest/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_postgrest -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_postgrest -- run-runtime-canonical`
- CI: `ci/ai-blaise/sidecar-api-runtime-smoke.sh`
- CI: `ci/ai-blaise/graphql-postgrest-runtime-smoke.sh`

## Realtime

### RT1: Realtime Sidecar

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/realtime`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Implements the realtime WebSocket sidecar: Phoenix-compatible channel joins, CDC ingest, fan-out hub, and probe/metrics endpoints.

**Motivation**: Realtime broadcasts need typed topic and tenant binding before
the WebSocket sidecar is implemented.

**Citus comparison**: Vanilla Citus does not ship realtime WebSocket
broadcasts.

Production evidence: VM worker D on experiment-playground, 2026-05-23: `cargo test -p ai_blaise_citus_sidecar_realtime`, `bash ci/ai-blaise/sidecar-realtime-smoke.sh`, and the raw-socket integration test prove Phoenix-compatible WebSocket upgrade, `phx_join`, tenant/topic filters, presence diffs, CDC ingest over Unix-domain socket, `postgres_changes` fan-out, and health/ready/metrics on the WS listener.

Current boundary: The production-ready claim is limited to the single-node raw WebSocket/Phoenix runtime and CDC-ingest fan-out exercised by `ci/ai-blaise/sidecar-realtime-smoke.sh`. The canonical runtime reports `runtime_boundary=single-node-raw-ws-cdc-ingest`, `websocket_network_exercised=true`, `browser_client_exercised=false`, `cdc_tailing_integrated=false`, `multi_node_pubsub=false`, and `kubernetes_traffic_exercised=false`; browser client behavior, WebSocket extension negotiation, live CDC tailing, multi-node pubsub, and Kubernetes traffic remain outside this proof. The presence timestamp guard enforces a UTC-looking `online_at` shape ending in `Z`, not a full calendar semantic parse.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: RT1` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: RT1` in `sidecar/realtime/src/lib.rs`
- In-source: `FEATURE: RT1` in `pool/src/realtime_hook.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_realtime -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_realtime -- run-runtime-canonical`
- In-source: `FEATURE: RT1` in `sidecar/realtime/src/live.rs`
- In-source: `FEATURE: RT1` in `sidecar/realtime/src/hub.rs`
- CI: `ci/ai-blaise/sidecar-realtime-smoke.sh`

### RT2: Per-Tenant Topic Isolation

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/realtime`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Enforces tenant IDs on subscriptions and CDC events so realtime topics do not leak row changes across tenants.

**Motivation**: Realtime streams must not leak row changes across tenants.

**Citus comparison**: Vanilla Citus does not model realtime topic tenancy.

Production evidence: VM worker D on experiment-playground, 2026-05-23: `cargo test -p ai_blaise_citus_sidecar_realtime`, `bash ci/ai-blaise/sidecar-realtime-smoke.sh`, and the raw-socket integration test prove Phoenix-compatible WebSocket upgrade, `phx_join`, tenant/topic filters, presence diffs, CDC ingest over Unix-domain socket, `postgres_changes` fan-out, and health/ready/metrics on the WS listener.

Current boundary: The production-ready claim is limited to the single-node raw WebSocket/Phoenix runtime and CDC-ingest fan-out exercised by `ci/ai-blaise/sidecar-realtime-smoke.sh`. The canonical runtime reports `runtime_boundary=single-node-raw-ws-cdc-ingest`, `websocket_network_exercised=true`, `browser_client_exercised=false`, `cdc_tailing_integrated=false`, `multi_node_pubsub=false`, and `kubernetes_traffic_exercised=false`; browser client behavior, WebSocket extension negotiation, live CDC tailing, multi-node pubsub, and Kubernetes traffic remain outside this proof. The presence timestamp guard enforces a UTC-looking `online_at` shape ending in `Z`, not a full calendar semantic parse.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: RT2` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: RT2` in `sidecar/realtime/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_realtime -- run-runtime-canonical`
- In-source: `FEATURE: RT2` in `sidecar/realtime/src/live.rs`
- In-source: `FEATURE: RT2` in `sidecar/realtime/src/hub.rs`
- CI: `ci/ai-blaise/sidecar-realtime-smoke.sh`

### RT3: Realtime Filter Expressions

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/realtime`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Applies server-side equality and operation filters before WebSocket fan-out so non-matching CDC events are not delivered.

**Motivation**: Subscribers need filtered streams without pushing every CDC
event over the socket.

**Citus comparison**: Vanilla Citus does not ship realtime filters.

Production evidence: VM worker D on experiment-playground, 2026-05-23: `cargo test -p ai_blaise_citus_sidecar_realtime`, `bash ci/ai-blaise/sidecar-realtime-smoke.sh`, and the raw-socket integration test prove Phoenix-compatible WebSocket upgrade, `phx_join`, tenant/topic filters, presence diffs, CDC ingest over Unix-domain socket, `postgres_changes` fan-out, and health/ready/metrics on the WS listener.

Current boundary: The production-ready claim is limited to the single-node raw WebSocket/Phoenix runtime and CDC-ingest fan-out exercised by `ci/ai-blaise/sidecar-realtime-smoke.sh`. The canonical runtime reports `runtime_boundary=single-node-raw-ws-cdc-ingest`, `websocket_network_exercised=true`, `browser_client_exercised=false`, `cdc_tailing_integrated=false`, `multi_node_pubsub=false`, and `kubernetes_traffic_exercised=false`; browser client behavior, WebSocket extension negotiation, live CDC tailing, multi-node pubsub, and Kubernetes traffic remain outside this proof. The presence timestamp guard enforces a UTC-looking `online_at` shape ending in `Z`, not a full calendar semantic parse.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: RT3` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: RT3` in `sidecar/realtime/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_realtime -- run-runtime-canonical`
- In-source: `FEATURE: RT3` in `sidecar/realtime/src/live.rs`
- In-source: `FEATURE: RT3` in `sidecar/realtime/src/hub.rs`
- CI: `ci/ai-blaise/sidecar-realtime-smoke.sh`

### RT4: Presence

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/realtime`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Implements channel presence join/leave state and Phoenix-compatible `presence_diff` broadcasts.

**Motivation**: Presence needs to be declared with the channel so the realtime
sidecar can account for subscribers consistently.

**Citus comparison**: Vanilla Citus has no presence-channel surface.

Production evidence: VM worker D on experiment-playground, 2026-05-23: `cargo test -p ai_blaise_citus_sidecar_realtime`, `bash ci/ai-blaise/sidecar-realtime-smoke.sh`, and the raw-socket integration test prove Phoenix-compatible WebSocket upgrade, `phx_join`, tenant/topic filters, presence diffs, CDC ingest over Unix-domain socket, `postgres_changes` fan-out, and health/ready/metrics on the WS listener.

Current boundary: The production-ready claim is limited to the single-node raw WebSocket/Phoenix runtime and CDC-ingest fan-out exercised by `ci/ai-blaise/sidecar-realtime-smoke.sh`. The canonical runtime reports `runtime_boundary=single-node-raw-ws-cdc-ingest`, `websocket_network_exercised=true`, `browser_client_exercised=false`, `cdc_tailing_integrated=false`, `multi_node_pubsub=false`, and `kubernetes_traffic_exercised=false`; browser client behavior, WebSocket extension negotiation, live CDC tailing, multi-node pubsub, and Kubernetes traffic remain outside this proof. The presence timestamp guard enforces a UTC-looking `online_at` shape ending in `Z`, not a full calendar semantic parse.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: RT4` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: RT4` in `sidecar/realtime/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_realtime -- run-runtime-canonical`

- In-source: `FEATURE: RT4` in `sidecar/realtime/src/live.rs`
- In-source: `FEATURE: RT4` in `sidecar/realtime/src/hub.rs`
- CI: `ci/ai-blaise/sidecar-realtime-smoke.sh`

## Edge Functions
### EF1: Deno Runtime Sidecar

**Overlay**: `sidecar/edge_functions`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `deno`

**Summary**: Defines Deno runtime launch plans for HTTP, scheduled, and
CDC-triggered edge functions, plus a bounded live inline-Deno execution path.

Production evidence: VM proof runs
`bash ci/ai-blaise/sidecar-edge-functions-runtime-smoke.sh` and
`bash ci/ai-blaise/edge-functions-deno-live-smoke.sh` build the real Rust
`ai_blaise_citus_sidecar_edge_functions` binary, verify Deno launch-command
planning with `--no-prompt`, boot the live HTTP sidecar, prove malformed JSON,
path, payload-size, timeout, and unknown-function requests fail closed, then
execute inline user code through a real Deno process with
`AI_BLAISE_EDGE_RUNTIME_EXECUTION=1` and `AI_BLAISE_DENO_BIN`. The live smoke
verifies disabled live mode fails closed, `execution_mode=live` returns
`status=executed`, `user_code_executed=true`, and a JSON
`runtime_response_json`, default environment access is denied by Deno
permissions, and an over-timeout function returns HTTP 504. This production
claim is limited to explicit opt-in inline Deno execution owned by the sidecar:
bundle URI/Git source fetch, package installation, custom permission grants,
secret injection into user code, user-code initiated database callbacks,
queue/broker delivery, durable retries, and Kubernetes deployment remain outside
EF1. Bun execution is covered by the separate EF2 production boundary. Scheduled
and CDC trigger dispatch have their own EF5 production boundary.

**Motivation**: Edge functions need a typed runtime contract before the
sidecar starts executing user code.

**Citus comparison**: Vanilla Citus does not ship a Deno edge-function
runtime.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: EF1` in `sidecar/edge_functions/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-bun-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-registry-canonical`
- CI: `ci/ai-blaise/sidecar-edge-functions-runtime-smoke.sh`
- CI: `ci/ai-blaise/edge-functions-deno-live-smoke.sh`
- CI: `ci/ai-blaise/sidecar-api-runtime-smoke.sh`

### EF2: Bun Runtime Alternative

**Overlay**: `sidecar/edge_functions`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `bun`

**Summary**: Adds Bun runtime launch planning plus a bounded live inline-Bun
execution path for edge-function bundles.

Production evidence: VM proof run
`bash ci/ai-blaise/edge-functions-bun-live-smoke.sh` builds the real
`ai_blaise_citus_sidecar_edge_functions` binary, installs or discovers a pinned
Bun binary, boots the live HTTP sidecar, verifies disabled live mode fails closed,
then executes inline Bun user code with `AI_BLAISE_EDGE_RUNTIME_EXECUTION=1` and
`AI_BLAISE_BUN_BIN`. The smoke verifies `status=executed`,
`execution_mode=live`, `user_code_executed=true`, and a JSON
`runtime_response_json`; it also verifies the Bun child environment is cleared,
timeout requests return HTTP 504, the runtime stdout cap rejects oversized user
output, and scheduled and CDC trigger dispatch both execute inline Bun functions.
`bash ci/ai-blaise/sidecar-edge-functions-runtime-smoke.sh` keeps the plan-only
Bun launch-command contract covered through `run-bun-runtime-canonical`.

This production claim is limited to explicit opt-in inline Bun execution owned
by the sidecar. Package installation, bundle URI/Git source fetch, npm registry
access, secret injection into user code, user-code initiated database callbacks,
queue/broker delivery, durable retries, and Kubernetes deployment remain outside
this proof.

**Motivation**: Some workloads prefer Bun startup and package compatibility;
the sidecar needs runtime selection without changing the CRD shape.

**Citus comparison**: Vanilla Citus does not ship a Bun edge-function runtime.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: EF2` in `sidecar/edge_functions/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-bun-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-registry-canonical`
- CI: `ci/ai-blaise/sidecar-edge-functions-runtime-smoke.sh`
- CI: `ci/ai-blaise/edge-functions-bun-live-smoke.sh`
- CI: `ci/ai-blaise/sidecar-api-runtime-smoke.sh`

### EF3: Function CRD

**Overlay**: `operator/src/crds/function.rs`,
`operator/src/reconcile/function.rs`, `operator/src/controllers/function.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines edge-function runtime, source, triggers, and secret
bindings for Deno and Bun deployments, plus an operator reconcile plan that
posts a stable sidecar registration payload, renders HTTP Service/Ingress
manifests, records function provenance, and registers scheduled or event
trigger metadata.

Production evidence: VM proof run `cargo check -p ai_blaise_citus_operator
--tests`, `cargo test -p ai_blaise_citus_operator`, and `cargo run -q -p
ai_blaise_citus_operator -- run-reconcilers-batch-b`. The canonical batch-B
row reports `function_apply_steps=6`, `function_sidecar_steps=1`, and
`function_kubernetes_steps=2`; unit tests cover HTTP, scheduled, and event
trigger plans plus teardown. Edge-function source execution, runtime sandboxing,
and gateway traffic remain alpha and are not claimed by this CRD/controller
promotion.

**Motivation**: Function deployment needs to be declarative so auth, pool, and
sidecar runtimes can share the same desired state.

**Citus comparison**: Vanilla Citus does not ship an edge-function CRD.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: EF3` in `operator/src/crds/function.rs`
- In-source: `FEATURE: EF3` in `operator/src/reconcile/function.rs`
- Controller: `operator/src/controllers/function.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcilers-batch-b`

### EF4: Database Callback Over UDS

**Overlay**: `sidecar/edge_functions`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Executes bounded, sidecar-managed Unix-domain-socket callbacks so
edge functions can call back into PostgreSQL with a configured database role and
statement timeout.

Production evidence: VM proof run
`bash ci/ai-blaise/edge-functions-db-callback-uds-smoke.sh` builds the real
`ai_blaise_citus_sidecar_edge_functions` binary, starts a real `postgres:17`
container with a mounted `.s.PGSQL.5432` Unix socket, registers an HTTP edge
function with `db_callback_socket`, `db_callback_database`,
`db_callback_role`, and `db_callback_statement_timeout_ms`, verifies callback
requests fail closed unless `AI_BLAISE_EDGE_DB_CALLBACK_EXECUTION=1` is set,
rejects unsafe multi-statement callback SQL before it mutates PostgreSQL,
executes one insert through the UDS callback path, reports
`db_callback_statement_executed=true` and `db_callback_rows=1`, and verifies the
inserted row in PostgreSQL. The bounded production surface is the sidecar-owned
PostgreSQL UDS callback executor and HTTP registration/invocation contract.
Bun DB-callback integration, user-code initiated callback RPC, queue delivery,
and Kubernetes deployment remain out of scope for EF4. The separate EF1
production boundary covers explicit opt-in inline Deno execution, the EF2
production boundary covers explicit opt-in inline Bun execution, both without
database callbacks. These remain separate EF1/EF2 production boundaries, and EF5
covers sidecar-owned trigger dispatch.

**Motivation**: Function runtimes need a local, explicit Postgres callback
contract rather than ad hoc TCP credentials in user code.

**Citus comparison**: Vanilla Citus does not expose an edge-function DB
callback path.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: EF4` in `sidecar/edge_functions/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-bun-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-registry-canonical`
- CI: `ci/ai-blaise/sidecar-edge-functions-runtime-smoke.sh`
- CI: `ci/ai-blaise/edge-functions-deno-live-smoke.sh`
- CI: `ci/ai-blaise/edge-functions-db-callback-uds-smoke.sh`
- CI: `ci/ai-blaise/sidecar-api-runtime-smoke.sh`

### EF5: Triggered Edge Functions

**Overlay**: `sidecar/edge_functions`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Executes scheduled and CDC-event trigger dispatch through the live
edge-functions sidecar.

Production evidence: VM proof run
`bash ci/ai-blaise/edge-functions-deno-live-smoke.sh` builds the real
`ai_blaise_citus_sidecar_edge_functions` binary, boots the live HTTP sidecar
with `AI_BLAISE_EDGE_RUNTIME_EXECUTION=1`, registers a scheduled-only function
and a CDC-event-only function, posts `POST /triggers/scheduled` with
`epoch_seconds=0`, posts `POST /triggers/cdc` for `public.edge_orders insert`,
and verifies both dispatch responses report `matched=1`, `dispatched=1`,
`execution_mode=live`, `user_code_executed=true`, and a function-specific
`runtime_response_json`. `bash ci/ai-blaise/edge-functions-bun-live-smoke.sh`
exercises the same trigger ingress and dispatch path with inline Bun functions.
`bash ci/ai-blaise/sidecar-edge-functions-runtime-smoke.sh` keeps the plan-only
registry and fail-closed trigger-request boundary covered. This production claim
is limited to sidecar-owned trigger ingress and dispatch into already-registered
inline Deno and Bun functions. Queue/broker integration, long-running CDC slot
tailing, distributed trigger fan-out, durable retry/DLQ, package installation,
non-inline source fetching, and Kubernetes deployment remain outside this proof.

**Motivation**: Cron and event-driven functions need the same validation path
as HTTP functions before queue integration is wired in.

**Citus comparison**: Vanilla Citus does not invoke external edge functions
from schedules or CDC events.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: EF5` in `sidecar/edge_functions/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-bun-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-registry-canonical`
- CI: `ci/ai-blaise/edge-functions-deno-live-smoke.sh`
- CI: `ci/ai-blaise/sidecar-edge-functions-runtime-smoke.sh`
- CI: `ci/ai-blaise/sidecar-api-runtime-smoke.sh`

## Security / Auth

### Auth1: JWT-Issuing Service

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/auth`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides the auth sidecar's HS256 JWT issuer and verifier runtime,
including explicit signing-secret configuration, access-token issuance,
refresh-token exchange, token introspection, JTI revocation, and live
health/readiness/metrics serving.

**Motivation**: SQL helpers, the pool, and sidecar APIs need one runtime token
contract with the same claim names and HS256 wire format as the Sec2 SQL
verifier.

**Citus comparison**: Vanilla Citus does not ship a JWT issuer.

Production evidence: `ci/ai-blaise/auth-sidecar-smoke.sh` starts the real
`ai_blaise_citus_sidecar_auth serve` binary with an explicit
`AI_BLAISE_AUTH_HS256_SECRET`, verifies `/healthz`, `/readyz`, and `/metrics`,
registers a user, logs in, verifies the JWT, introspects it, refreshes the
session, logs out, and proves the revoked token is rejected. With
`REQUIRE_DOCKER=1`, the same smoke applies `sidecar/auth/migrations/0001_auth_schema.sql`
against a real `postgres:17` container and verifies the durable auth tables.
This production-ready boundary covers the local HS256 issuer/verifier,
refresh-token session map, JTI revocation, auth-service introspection cache,
and TOTP-backed login path only. RS256/JWKS discovery, external IdP token
exchange, key rotation, persistent runtime loading from the auth schema,
WebAuthn ceremonies, and pool data-plane authentication remain alpha until
they have their own live evidence.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Auth1` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: Auth1` in `sidecar/auth/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_auth -- run-canonical`
- CI: `ci/ai-blaise/auth-sidecar-smoke.sh`
- Schema: `sidecar/auth/migrations/0001_auth_schema.sql`

### Auth3: Token Introspection Cache

**Overlay**: `pool/src/runtime.rs`, `pool/src/auth_cache.rs`,
`pool/src/auth_introspection.rs`, `pool/src/proxy.rs`, `sidecar/auth`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides the bounded production pool authentication path: the pool
extracts a startup JWT, validates it through the auth sidecar's introspection
endpoint before opening an upstream PostgreSQL socket, caches verified claims
when configured, strips pool-only auth parameters before forwarding startup
bytes, and fails closed for missing, inactive, revoked, expired, or
cross-tenant tokens.

**Motivation**: Auth verification must be fast enough for pooled connection
paths without repeatedly hitting the auth sidecar, while still denying revoked
or malformed credentials before backend capacity is spent.

**Citus comparison**: Vanilla Citus does not include token introspection or a
pool-side startup-token admission gate.

Production evidence: `REQUIRE_DOCKER=1 ci/ai-blaise/pool-proxy-smoke.sh`
starts a real `postgres:17` container, starts the real auth sidecar, creates a
user, issues an HS256 access token through `/auth/login`, starts the real pool
proxy with `AI_BLAISE_POOL_AUTH_INTROSPECTION_URL`, sends a raw PostgreSQL
startup packet containing pool-only `ai_blaise.jwt` and `ai_blaise.tenant_id`
parameters, verifies a live SQL query succeeds through the pool after token
introspection, verifies the pool strips those auth parameters before forwarding
startup bytes to PostgreSQL, verifies a missing token fails closed before
upstream routing, logs out the token through `/auth/logout`, verifies the
revoked token fails closed before upstream routing, and asserts the pool
metrics `ai_blaise_citus_pool_auth_verified_connections_total`,
`ai_blaise_citus_pool_auth_rejections_total`, and
`ai_blaise_citus_pool_fail_closed_routes_total`. `cargo test -p
ai_blaise_citus_pool --lib` covers cache TTL/expiry/revocation behavior and
startup-parameter extraction. `ci/ai-blaise/auth-sidecar-smoke.sh` separately
proves the auth service's introspection and revocation semantics. This status
covers HS256 auth-sidecar introspection and the pool startup-token gate only;
OAuth2/OIDC providers (`Auth4`), WebAuthn MFA, non-HS256 issuer modes, and
application-level RLS policy generation remain separately scoped.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Auth3` in `pool/src/runtime.rs`
- In-source: `FEATURE: Auth3` in `pool/src/auth_cache.rs`
- In-source: `FEATURE: Auth3` in `pool/src/auth_introspection.rs`
- In-source: `FEATURE: Auth3` in `sidecar/auth/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_pool -- run-canonical`
- Executable: `cargo test -p ai_blaise_citus_pool --lib`
- CI: `ci/ai-blaise/pool-proxy-smoke.sh`
- CI: `ci/ai-blaise/auth-sidecar-smoke.sh`

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

**Overlay**: `pool/src/runtime.rs`, `pool/src/admission.rs`, `pool/src/proxy.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Enforces a narrow pool data-plane token-bucket admission quota for
tenant-scoped PostgreSQL startup packets. Broader tenant resource planning in
operator specs and non-pool sidecars remains alpha under the tenant-operations
and workload-specific feature entries until those paths have separate live
evidence.

**Motivation**: Tenant quotas need pool-side enforcement before noisy tenants
can consume unbounded data-plane connection admission.

**Citus comparison**: Vanilla Citus does not enforce per-tenant pool quotas.

Production evidence: `ci/ai-blaise/pool-proxy-smoke.sh` starts the real pool
against a `postgres:17` container with `AI_BLAISE_POOL_QUOTA_TENANT_ID`,
`AI_BLAISE_POOL_QUOTA_BURST`, and
`AI_BLAISE_POOL_QUOTA_REFILL_PER_SECOND`, admits the first tenant-scoped
PostgreSQL startup through the real backend, denies the next over-budget
startup with a PostgreSQL `ErrorResponse` before upstream routing, and asserts
`ai_blaise_citus_pool_tenant_quota_rejections_total` plus
`ai_blaise_citus_pool_fail_closed_routes_total` metrics.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Sec12` in `pool/src/runtime.rs`
- In-source: `FEATURE: Sec12` in `pool/src/admission.rs`
- In-source: `FEATURE: Sec12` in `pool/src/proxy.rs`
- Executable: `cargo run -p ai_blaise_citus_pool -- run-canonical`
- CI: `ci/ai-blaise/pool-proxy-smoke.sh`

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
`uid` claims are rejected. Auth1 HS256 issuance and Auth3 pool-side token introspection have separate
runtime evidence; non-HS256 issuer modes remain alpha until their own runtime
evidence exists. Sec2 JWT verification has a separate SQL-runtime evidence
boundary.

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
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Implements the OIDC provider pre-exchange runtime boundary: issuer,
authorization endpoint, client ID, secret ref, redirect URI, and scope
validation plus login/callback routes with state, nonce, and replay handling.

**Motivation**: Auth sidecars need an auditable provider contract before
Google, GitHub, Apple, Okta, Azure AD, or custom OIDC integrations are wired.

**Citus comparison**: Vanilla Citus does not ship OAuth2/OIDC auth services.

Production evidence: `cargo test -p ai_blaise_citus_sidecar_auth --all-targets`
proves provider validation, allowlisted HTTPS redirect URI enforcement,
state/nonce generation, callback validation, and callback replay rejection.
`ci/ai-blaise/auth-sidecar-smoke.sh` starts the real auth binary with a stub
provider config, verifies `/auth/oidc/login` emits an authorization URL for the
allowed redirect URI, proves unknown providers and disallowed redirect URIs fail
closed, proves nonce mismatch and replayed callback state fail closed, and
requires the validated callback to stop at `501 idp_exchange_unavailable`.
This is not external IdP certification: token exchange, ID-token/JWKS
verification, account linking, provider-specific claim mapping, and secret
resolver integration remain outside this production-ready boundary.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Auth4` in `sidecar/auth/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_auth -- run-canonical`
- CI: `ci/ai-blaise/auth-sidecar-smoke.sh`
- Schema: `sidecar/auth/migrations/0001_auth_schema.sql`

### Auth5: MFA Policy Contracts

**Overlay**: `sidecar/auth`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds MFA policy validation for TOTP and WebAuthn enablement plus
bounded retry attempts. The auth sidecar now implements TOTP enrollment and
verification while keeping WebAuthn ceremony routes fail-closed.

**Motivation**: MFA behavior needs a declarative sidecar contract before token
issuance can enforce step-up authentication.

**Citus comparison**: Vanilla Citus does not ship MFA policy management.

Production evidence: unit tests verify RFC 6238 TOTP vectors, TOTP enrollment,
TOTP login, missing-code denial, and max-attempt lockout. The auth smoke starts
the real binary with `AI_BLAISE_AUTH_MFA_MAX_ATTEMPTS=2`, proves repeated bad
TOTP verification locks the user out, proves the locked login remains denied,
and proves a separate enrolled user can still complete TOTP-backed login.
WebAuthn register/finish routes remain fail-closed with `501`; WebAuthn
challenge generation, credential persistence, signature verification, replay
protection, and persistent TOTP loading from the auth schema remain outside this
production-ready boundary.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Auth5` in `sidecar/auth/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_auth -- run-canonical`
- CI: `ci/ai-blaise/auth-sidecar-smoke.sh`
- Schema: `sidecar/auth/migrations/0001_auth_schema.sql`

## Plan Management

### PM3: Plan Freeze Companion Module

**Overlay**: `companion/src/plan_freeze.rs`, `companion/src/plan_runtime.rs`
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
fails closed. This status covers the local SQL plan-freeze registry and promotion-policy
state plus the deterministic companion runtime contract for durable
idempotency, bounded retry, promotion decisions, and audit emission. Actual
planner enforcement, hint injection, pg_hint_plan/sr_plan integration,
auto-promotion workers, distributed plan capture, external durable storage,
and plan XML validation remain alpha.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: PM3` in `companion/src/plan_freeze.rs` and
  `companion/src/plan_runtime.rs`
- SQL runtime: `FEATURE: PM3` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-plan-runtime-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- CI: `ci/ai-blaise/companion-plan-runtime-smoke.sh`

### PM4: Plan Regression Detection

**Overlay**: `companion/src/plan_freeze.rs`, `companion/src/plan_runtime.rs`
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
samples are recorded, and verifies a missing frozen plan fails closed. This status covers the local SQL regression-policy evaluator and sample log
plus the deterministic companion runtime contract for candidate acceptance,
regression rejection, idempotency replay, bounded retry, and audit emission.
Automatic production-plan replacement, query capture, pg_hint_plan/sr_plan
enforcement, workload baselining, external durable storage, and distributed
planner integration remain alpha.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: PM4` in `companion/src/plan_freeze.rs` and
  `companion/src/plan_runtime.rs`
- SQL runtime: `FEATURE: PM4` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-domain-contracts-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-plan-runtime-canonical`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- CI: `ci/ai-blaise/companion-plan-runtime-smoke.sh`

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

**Overlay**: `operator/src/crds/webhook.rs`,
`operator/src/reconcile/webhook.rs`, `operator/src/controllers/webhook.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines outbound HTTP trigger declarations with events, URL,
header secret reference, retry policy, and payload template, plus an operator
reconcile plan that installs `ai_blaise_citus`, delegates trigger/queue setup
to the companion webhook helper, and records webhook provenance, payload
template, and dead-letter target metadata.

Production evidence: VM proof run `cargo check -p ai_blaise_citus_operator
--tests`, `cargo test -p ai_blaise_citus_operator`, and `cargo run -q -p
ai_blaise_citus_operator -- run-reconcilers-batch-b`. The canonical batch-B
row reports `webhook_apply_steps=6` and `webhook_events=2`; unit tests cover
full, minimal, invalid, and teardown plans. Outbound HTTP delivery workers,
secret resolution, and retry/dead-letter execution remain alpha until the
sidecar runtime evidence lands separately.

**Motivation**: Webhook delivery needs an operator-controlled contract before
CDC and queue sidecars can guarantee retry behavior.

**Citus comparison**: Vanilla Citus does not include webhook lifecycle
management.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: WH1` in `operator/src/crds/webhook.rs`
- In-source: `FEATURE: WH1` in `operator/src/reconcile/webhook.rs`
- Controller: `operator/src/controllers/webhook.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcilers-batch-b`

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
dead-letter execution, and secret resolution remain alpha; the operator
Webhook CRD and reconcile plan are covered by `WH1`.

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
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Implements max-attempt/dead-letter policy for CDC-backed webhook and sink delivery, with in-memory and append-only file DLQ records.

**Motivation**: Webhooks need at-least-once retry and dead-letter behavior
before delivery sidecars can be trusted.

**Citus comparison**: Vanilla Citus does not include webhook retry contracts.

Production evidence: VM worker D on experiment-playground, 2026-05-23: `cargo test -p ai_blaise_citus_sidecar_cdc` covers DLQ persistence and live-dispatch failure routing, while `bash ci/ai-blaise/sidecar-cdc-smoke.sh` proves the runtime reports DLQ state and advances LSNs only through the dispatch path.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: WH3` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: WH3` in `sidecar/cdc/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_cdc -- run-runtime-canonical`

- In-source: `FEATURE: WH3` in `sidecar/cdc/src/dlq.rs`
- In-source: `FEATURE: WH3` in `sidecar/cdc/src/live.rs`
- CI: `ci/ai-blaise/sidecar-cdc-smoke.sh`

## Storage
### Sto1: Storage Sidecar

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/storage`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines bucket and metadata-table contracts for the storage
sidecar, plus a runnable canonical metadata/presign emitter.

Production evidence: VM proof run `bash ci/ai-blaise/storage-sidecar-runtime-smoke.sh` builds and serves the real `ai_blaise_citus_sidecar_storage` binary, verifies `/healthz`, `/readyz`, `/metrics`, persistent `/drain`, `/storage/policy`, `/storage/state`, clean upload storage, infected upload quarantine, and fail-closed JSON errors through live HTTP. This promotes the bounded storage sidecar metadata, policy, and in-process object-record runtime; external object-store writes remain outside this feature scope.

**Motivation**: S3-compatible file storage needs a stable table and bucket
mapping before upload/download paths are implemented.

**Citus comparison**: Vanilla Citus does not ship an object storage sidecar.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Sto1` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: Sto1` in `sidecar/storage/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_storage -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_storage -- run-runtime-canonical`
- CI: `ci/ai-blaise/storage-sidecar-runtime-smoke.sh`

### Sto3: Presigned URL Signing

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/storage`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines presigned upload URL TTL policy for the storage sidecar.

Production evidence: VM proof run `bash ci/ai-blaise/storage-sidecar-runtime-smoke.sh` exercises live HTTP `POST /storage/presign`, verifies deterministic signed URL output for tenant `tenant-a`, and verifies `ttl_seconds=901` fails closed against the 900-second policy. This promotes presign policy enforcement and URL issuance in the storage sidecar runtime.

**Motivation**: Direct uploads need a bounded signing window to keep file
access auditable.

**Citus comparison**: Vanilla Citus does not generate presigned object-store
URLs.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Sto3` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: Sto3` in `sidecar/storage/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_storage -- run-runtime-canonical`
- CI: `ci/ai-blaise/storage-sidecar-runtime-smoke.sh`

### Sto4: Bucket-Level ACLs

**Overlay**: `sidecar/shared/src/contracts.rs`, `sidecar/storage`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Carries tenant-column ACL binding for object metadata rows.

Production evidence: VM proof run `bash ci/ai-blaise/storage-sidecar-runtime-smoke.sh` verifies the live storage policy exposes the tenant-bound `tenant_read_write` ACL and that upload/presign requests execute only for the configured `tenant-files` and `tenant-a` policy. The Rust unit suite covers ACL method rejection and policy lookup failures.

**Motivation**: Storage ACLs must line up with tenant RLS rather than existing
only in object-store policy.

**Citus comparison**: Vanilla Citus does not manage storage ACLs.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Sto4` in `sidecar/shared/src/contracts.rs`
- In-source: `FEATURE: Sto4` in `sidecar/storage/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_storage -- run-runtime-canonical`
- CI: `ci/ai-blaise/storage-sidecar-runtime-smoke.sh`

### Sto5: Antivirus Scan Integration

**Overlay**: `sidecar/storage`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds antivirus scanner endpoint and quarantine-bucket validation
for object uploads.

Production evidence: VM proof run `bash ci/ai-blaise/storage-sidecar-runtime-smoke.sh` verifies antivirus fail-closed policy metadata, clean upload storage, infected `malware:eicar-test` quarantine, scanned-object accounting, and quarantine-state reporting through live HTTP. This promotes scanner-policy enforcement and quarantine routing; managed scanner deployment remains a release environment dependency.

**Motivation**: File attachments need a fail-closed malware scanning contract
before direct uploads are exposed to tenants.

**Citus comparison**: Vanilla Citus does not manage object-store antivirus
policy.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: Sto5` in `sidecar/storage/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citus_sidecar_storage -- run-runtime-canonical`
- CI: `ci/ai-blaise/storage-sidecar-runtime-smoke.sh`

## MCP

### MCP1: citus-mcp Server

**Overlay**: `tools/citus-mcp`, `sidecar/mcp`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides the `tools/citus-mcp` line-delimited JSON-RPC stdio
server and the `sidecar/mcp` `serve-stdio` policy bridge for `initialize`,
`tools/list`, and validation-only guarded `tools/call` requests, including
deployed exhaustive-profile sidecar `POST /mcp` traffic.

Production evidence: VM proof runs `ci/ai-blaise/mcp-stdio-smoke.sh`, `ci/ai-blaise/mcp-sidecar-stdio-smoke.sh`, `ci/ai-blaise/mcp-sidecar-http-smoke.sh`, and `REQUIRE_DOCKER=1 ci/ai-blaise/mcp-db-smoke.sh`. They launch the real `tools/citus-mcp` and `sidecar/mcp` stdio/HTTP processes, verify JSON-RPC initialize and tool-list behavior, execute a read-only tenant query against a real PostgreSQL container, list Citus shard metadata from `pg_dist_shard`, and verify safe-mode destructive denial plus cross-schema rejection. This promotes the MCP server, sidecar bridge, and read-only database execution boundary; mutating database/Kubernetes execution remains outside the production scope.

Sidecar-specific evidence now also proves malformed JSON-RPC returns JSON-RPC
errors without terminating the process, unknown methods fail with `-32601`, the
tool registry exposes the exact nine expected descriptors with object input
schemas, `/healthz`, `/readyz`, `/metrics`, and `/drain` are live HTTP
responses with persistent readiness state, malformed HTTP is rejected with 400,
and an unreachable `AI_BLAISE_MCP_DATABASE_URL` returns MCP `isError: true`
while subsequent requests still succeed. The sidecar claim is not a full
external MCP service claim: authentication token verification, durable MCP
sessions, streaming remote transport, sidecar-owned live database execution,
Kubernetes execution, and mutating tool execution remain alpha. The real
read-only database execution production claim remains `FEATURE: MCP4` in
`tools/citus-mcp`.

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
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds validation-only safe-mode checks that deny destructive MCP
tool requests by default.

Production evidence: VM proof runs `ci/ai-blaise/mcp-stdio-smoke.sh`, `ci/ai-blaise/mcp-sidecar-stdio-smoke.sh`, `ci/ai-blaise/mcp-sidecar-http-smoke.sh`, and `REQUIRE_DOCKER=1 ci/ai-blaise/mcp-db-smoke.sh`. They drive real JSON-RPC `tools/call` requests through stdio, sidecar HTTP, and database-backed modes, and verify destructive `tenant_archive` calls return `isError: true` with the safe-mode denial message while allowed read-only calls execute or validate successfully.

For `sidecar/mcp`, this remains a safe-mode bridge and fail-closed boundary: it
does not prove mutating tools, Kubernetes operations, per-user authorization, or
external MCP session enforcement.

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
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds tenant scope and allowed-schema validation to MCP tool
requests, including fail-closed rejection for obvious cross-schema SQL/table
references.

Production evidence: VM proof runs `ci/ai-blaise/mcp-stdio-smoke.sh`, `ci/ai-blaise/mcp-sidecar-stdio-smoke.sh`, `ci/ai-blaise/mcp-sidecar-http-smoke.sh`, and `REQUIRE_DOCKER=1 ci/ai-blaise/mcp-db-smoke.sh`. They verify tenant-scoped requests include tenant metadata, reject missing tenant scope, reject `tenant_b` SQL when only `tenant_a` is allowed, and execute allowed read-only SQL against a real PostgreSQL tenant schema. Per-user authentication and mutating tool authorization remain separate feature scope.

For `sidecar/mcp`, tenant scope is request-policy evidence only; it is not a
claim for authenticated user/tenant binding, externally durable sessions, or
sidecar-owned database execution.

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
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds the typed `dev up` and `dev down` command contract for local
cluster lifecycle operations.

**Current boundary**: The production-ready D1 scope is the real CLI local
state-file lifecycle for `dev up`/`dev down` with an explicit `--state-dir`:
dry-run planning writes no state, apply requires a stable plan ID, JSON and TSV
outputs are deterministic, repeated up/down operations are idempotent, every
apply appends a local audit row, and down removes only the tracked state file.
Starting Docker, kind, Kubernetes, Postgres/Citus, or extension services
remains alpha.

Production evidence: `ci/ai-blaise/citusctl-dev-lifecycle-smoke.sh` runs the
real `ai_blaise_citusctl` binary locally, on the VM, and in the GitHub Actions
`tools` workflow. The smoke requires missing `--state-dir` and unstable plan
IDs to fail closed, verifies exact `--format json` plan output, verifies
`--format tsv` apply output for changed and idempotent up/down transitions,
checks that only `dev-lifecycle.state` is removed on down, and requires the
local `dev-lifecycle.audit.tsv` log to retain one row per apply. This evidence
is not evidence for Docker/kind startup, Kubernetes deployment, Postgres/Citus
data-plane health, extension-service orchestration, or production cluster
lifecycle management.

**Motivation**: Contributors need a single CLI entrypoint for local end-to-end
clusters before the kind runner and image builder are wired.

**Citus comparison**: Vanilla Citus has development scripts, but not the
ai-blaise single-command local cluster contract.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: D1` in `tools/citusctl/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citusctl -- run-canonical`
- Executable: `cargo run -p ai_blaise_citusctl -- run-dev-lifecycle-canonical`
- CI: `ci/ai-blaise/citusctl-dev-lifecycle-smoke.sh`

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
`--unsafe-allow-mutation --confirm CONFIRM`. The shared tools runtime also fails
closed on duplicate snapshot identities and vectorizer/realtime tenant
references to unknown tenants before the TUI renders or previews actions. The
broader interactive ratatui event loop, direct database sessions, and live
mutation execution remain alpha.

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
missing search analyzers, the file-backed
`citus-lsp analyze --metadata <metadata.tsv> --sql <migration.sql>` CLI, and a
file-backed `citus-lsp serve-stdio --metadata <metadata.tsv>` JSON-RPC stdio
transport for diagnostics on opened SQL documents.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/citus-lsp-smoke.sh`, which feeds a metadata TSV plus a real SQL
file into `citus-lsp analyze --metadata <metadata.tsv> --sql <migration.sql>`,
verifies missing distribution column, non-colocated join,
distribution-column alter, hypertable invariant, missing tenant filter, and
missing search analyzer diagnostics, and verifies fail-closed behavior for bad
metadata or missing metadata. The same smoke drives real LSP-style
`Content-Length` stdio frames through `citus-lsp serve-stdio --metadata`,
covering initialize, textDocument/didOpen publishDiagnostics,
textDocument/diagnostic pull diagnostics, malformed JSON, unknown method, and
unopened-document failure. Production evidence is limited to this file-backed
stdio diagnostic service; editor integration, workspace indexing, automatic
file rewrites, live metadata refresh, and full PostgreSQL grammar coverage
remain alpha.

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
rebalance to emit an accepted dry-run receipt. The shared tools runtime also
fails closed on duplicate snapshot identities and vectorizer/realtime tenant
references to unknown tenants before admin routes or dry-run receipts are
rendered. Full WhoDB front-end embedding, browser sessions, live database
writes, and Kubernetes-side admin deployment remain alpha.

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
references to fail closed. The shared tools runtime also rejects duplicate
snapshot identities and vectorizer/realtime tenant references to unknown
tenants. Full DrawDB front-end integration, collaborative editing, and live
operator/companion refresh remain alpha.

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
The smoke also proves duplicate snapshot identities and vectorizer tenant
references to unknown tenants fail closed through the same `citus-watch` runner.
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

**Summary**: Provides the Citus-side contract for a production-safe direct Helm
install surface while the actual chart lives in `ai-blaise/command-center`.
This repository no longer owns `deploy/k8s/`; it owns the render and traffic
harness that points at the external chart and fails real mode when image refs,
strict production-values guardrails, or required SQL service traffic are missing.
HTTP probes are exercised only when the rendered chart exposes an HTTP surface.

**Motivation**: A direct `helm upgrade --install` command should be validated
against the real chart and real images, not against in-repo stubs or synthetic
responder containers.

**Citus comparison**: Vanilla Citus does not ship the ai-blaise overlay chart
or this external-chart validation harness.

Production evidence: after the 2026-05-22 chart fold, full Helm rendering
and live chart smoke evidence live with `ai-blaise/command-center`. This
repository keeps the Citus-side deploy contract that the chart must preserve:
`ci/ai-blaise/deploy-check.sh` validates the rendered
`deploy/contracts/k8s-production-guardrails.yaml` bundle, including 49 HPA,
PodDisruptionBudget, and NetworkPolicy resources for the operator, pool, and
sidecar surfaces. The production-readiness workflow and `gate-close` run that
contract so Citus-side image/runtime changes cannot drift from the Kubernetes
guardrails consumed by command-center.

Production evidence: `ci/ai-blaise/deploy-check.sh` runs the CI-safe dry-run
entrypoint for the external command-center chart. When `CHART_DIR` or
`COMMAND_CENTER_DIR` is supplied, it runs Helm lint/template, records the
rendered image set, and can run client-side Kubernetes validation when
`KUBECTL_CLIENT_DRY_RUN=1` is set. Dry-run mode is not runtime evidence.
`ci/ai-blaise/kind-production-smoke.sh` switches to real traffic only with
`LIVE_K8S_MODE=kind` or `LIVE_K8S_MODE=real`; in those modes it now enables
`PRODUCTION_VALUES_STRICT=1`, rejects mutable/latest image refs and alpha
sidecar render leaks, waits for Kubernetes readiness, and requires live SQL
traffic through a Kubernetes service by default. HTTP probes are opt-in because
the current command-center chart can expose a PostgreSQL service on port 5432
without an HTTP surface.

Fallback substrate evidence: `ci/ai-blaise/k8s-production-values-live-smoke.sh`
is the VM-runnable live production-values substrate smoke for this repository.
It creates an ephemeral Helm chart and `values-production.yaml`, pins the
PostgreSQL operand by immutable `@sha256` digest, keeps alpha sidecars disabled,
renders and client-validates the manifests, installs into a real kind cluster,
waits for the StatefulSet and Ready pod condition, runs a separate in-cluster
SQL client Job through the Kubernetes Service DNS path, records
Helm/kubectl/log/image evidence under `artifacts/k8s-production-values-live/`,
and writes `claim_boundary=postgres_substrate_only`. This proves the live
Kubernetes harness and production-values guardrails only; it is not proof of
unpublished Citus application container behavior, operator reconciliation, pool
routing, or the command-center production chart unless the caller supplies the
exact digest-pinned Citus chart values and images to `live-k8s-e2e.sh`.

**References**:

- In-source: `FEATURE: D7` in `companion/src/ops_contracts.rs`
- Helm chart: `ai-blaise/command-center: helm/charts/citus-cluster`
- CI: `ci/ai-blaise/deploy-check.sh`
- Kubernetes smoke: `ci/ai-blaise/kind-production-smoke.sh`
- Production-values live smoke: `ci/ai-blaise/k8s-production-values-live-smoke.sh`
- Gate: `make -f Makefile.ai-blaise gate-close`

### D8: Infrastructure Deploy Wrapper

**Overlay**: `scripts/citus-scale/deploy.sh`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Preserves the in-repo deploy-wrapper boundary after the Helm chart
folded into `ai-blaise/command-center`. `scripts/citus-scale/deploy.sh` now
fails closed with a pointer to the command-center chart, while
`ci/ai-blaise/live-k8s-e2e.sh` provides the real render/install/traffic path
when operators pass `CHART_DIR` or `COMMAND_CENTER_DIR`.

**Motivation**: Operators need the old in-repo deploy wrapper to stop before it
can install stale chart content, and they need a current harness that validates
the external chart against real Kubernetes traffic.

**Citus comparison**: Vanilla Citus does not ship the ai-blaise deploy wrapper
boundary or external-chart traffic harness.

Production evidence: `scripts/citus-scale/deploy.sh` exits nonzero with a
clear command-center handoff instead of rendering removed manifests.
`ci/ai-blaise/deploy-check.sh`, `ci/ai-blaise/kind-production-smoke.sh`, and
`ci/ai-blaise/k8s-production-values-live-smoke.sh` are the supported validation
entrypoints in this repo. Real chart mode requires external chart input,
strict production render validation, and image refs that are published or
locally loaded; if branch images are unpublished, the harness reports that
explicitly and tells the operator to pass `AI_BLAISE_STACK_IMAGE_REF`,
`HELM_SET_ARGS`, and `LOCAL_IMAGE_REFS` rather than faking traffic. The
self-contained live smoke covers only the VM/kind production-values substrate
path with a pinned PostgreSQL image and SQL service traffic. Full
Citus app-container production evidence still requires the exact command-center
release chart and immutable Citus image digests.

**References**:

- In-source: `FEATURE: D8` in `scripts/citus-scale/deploy.sh`
- CI: `ci/ai-blaise/deploy-check.sh`
- Kubernetes smoke: `ci/ai-blaise/kind-production-smoke.sh`
- Production-values live smoke: `ci/ai-blaise/k8s-production-values-live-smoke.sh`
- Gate: `make -f Makefile.ai-blaise gate-close`

### D13: Production Runtime Image Matrix

**Overlay**: `images/rust-runtime`, `scripts/citus-scale`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Builds real Rust application images for the operator, pool,
sidecars, and `citusctl`, with service images defaulting to long-running
`serve` commands and the `citusctl` tool image defaulting to `plan inspect
cluster`. The live Kubernetes harness validates whatever command-center chart
is supplied, waits for the rendered workloads, probes HTTP services when the
chart exposes an HTTP surface, sends SQL through a PostgreSQL service, and
collects diagnostics on failure. The harness does not pretend a dry-run is
runtime evidence, and the self-contained VM smoke marks its PostgreSQL-only
substrate boundary explicitly.

**Motivation**: Production Kubernetes verification must exercise the exact app
containers and PostgreSQL traffic paths under review rather than chart-only
tests. When those app images are unavailable, the VM smoke proves only the live
Kubernetes harness and SQL service path, with that boundary called out.

**Citus comparison**: Vanilla Citus does not ship the ai-blaise operator,
pool, sidecar, or tool image matrix.

Production evidence: `scripts/citus-scale/build-app-images.sh` builds and, for
release runs, pushes the Rust image matrix while writing
`artifacts/ai-blaise-image-digests.tsv` and failing if pushed images do not
report immutable digests. `ci/ai-blaise/kind-production-smoke.sh` defaults to a
CI-safe dry-run, but `LIVE_K8S_MODE=kind` or `LIVE_K8S_MODE=real` turns it into
a fail-closed chart test: render the external chart, run strict production
values validation, reject mutable/latest image refs and alpha sidecar leaks,
verify image availability or load `LOCAL_IMAGE_REFS` into kind, install into a
namespace, wait for rollouts and ready pods, run SQL traffic through an exposed
PostgreSQL service, collect logs/events/Helm state on failure, and tear down
unless debugging opts out. This is runtime evidence only when real mode
completes against the exact chart and image refs under review.

Fallback substrate evidence: `ci/ai-blaise/k8s-production-values-live-smoke.sh`
closes the repo-local VM evidence gap without inventing a fake Citus data-plane
claim. It runs a real kind deployment from a generated Helm chart and
`values-production.yaml`, pins the operand image as
`docker.io/library/postgres:16-alpine@sha256:16bc17c64a573ef34162af9298258d1aec548232985b33ed7b1eac33ba35c229`,
keeps alpha sidecars disabled, waits for Kubernetes readiness, executes an
in-cluster SQL client Job through the Service DNS path, writes evidence
artifacts, and records `claim_boundary=postgres_substrate_only`. That proves the
Kubernetes production-values substrate, immutable-image guardrail, and live SQL
network path only. It does not prove unpublished Citus app container behavior,
operator reconciliation, pool routing, Citus data-plane semantics, or
command-center release chart production readiness unless the caller supplies
those digest-pinned images and values to the strict real chart harness.

**References**:

- Build script: `FEATURE: D13` in
  `scripts/citus-scale/build-app-images.sh`
- Runtime Dockerfile: `FEATURE: D13` in
  `images/rust-runtime/Dockerfile`
- Live SQL smoke: `ci/ai-blaise/pool-proxy-smoke.sh`
- Kubernetes smoke: `ci/ai-blaise/kind-production-smoke.sh`
- Production-values live smoke: `ci/ai-blaise/k8s-production-values-live-smoke.sh`
- CI: `.github/workflows/ci-deploy.yml`
- Gate: `make -f Makefile.ai-blaise gate-close`
- CI: `ci/ai-blaise/image-check.sh`

### WF2: WAL Replay Debugger Command

**Overlay**: `tools/citusctl`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pg_walinspect`

**Summary**: Provides a fixture-backed `citusctl plan wal-replay ... --json`
debugger command that validates WAL source URI, UTC target time, fixture bounds,
and emits a deterministic JSON replay plan.

**Motivation**: WAL forensics need to enter through plan/apply machinery so
replay and restore commands can share preflight and audit behavior.

**Citus comparison**: Vanilla Citus does not ship a WAL replay debugger CLI.

Production evidence: `ci/ai-blaise/citusctl-smoke.sh` runs the real
`ai_blaise_citusctl` binary against a temporary local WAL fixture. It verifies
exact deterministic JSON for an in-range target and verifies unsupported source
URI schemes plus out-of-range target times fail closed.

**Current boundary**: The production-ready claim covers only the local
fixture-backed debugger plan path exercised by `ci/ai-blaise/citusctl-smoke.sh`:
it reads a key/value WAL fixture, rejects unsupported source schemes, rejects
out-of-range target times, and emits exact deterministic JSON. Real WAL segment
inspection, PostgreSQL `pg_walinspect` execution, restore/replay mutation, and
production cluster operations remain alpha.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: WF2` in `tools/citusctl/src/lib.rs`
- Executable: `cargo run -p ai_blaise_citusctl -- run-canonical`
- CI: `ci/ai-blaise/citusctl-smoke.sh`

## Federation

### F1: Federation CRD

**Overlay**: `operator/src/crds/federation.rs`,
`operator/src/reconcile/federation.rs`, `operator/src/controllers/federation.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `oracle_fdw`, `mysql_fdw`, `mongo_fdw`

**Summary**: Defines external federation links for warehouse, document, and
legacy database targets using secret-backed connection references, plus an
operator reconcile plan that chooses the FDW or Iceberg bridge backend, installs
FDW extensions where required, creates foreign server/user-mapping SQL, and
records deterministic federation provenance.

Production evidence: VM proof run `cargo check -p ai_blaise_citus_operator
--tests`, `cargo test -p ai_blaise_citus_operator`, and `cargo run -q -p
ai_blaise_citus_operator -- run-reconcilers-batch-b`. The canonical batch-B
row reports `federation_apply_steps=4` and `federation_iceberg=true`; unit
tests cover Snowflake/Iceberg routing, MySQL FDW routing, teardown, invalid
spec propagation, and Kubernetes CR mirror parsing. Actual external warehouse
connectivity and Iceberg snapshot reads remain alpha under `F3` and analytical
runtime evidence.

**Motivation**: FDW and lakehouse federation need a typed source of desired
state before credentials and foreign schema creation are reconciled.

**Citus comparison**: Vanilla Citus can participate in FDW queries but does
not ship a federation CRD.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: F1` in `operator/src/crds/federation.rs`
- In-source: `FEATURE: F1` in `operator/src/reconcile/federation.rs`
- Controller: `operator/src/controllers/federation.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcilers-batch-b`

## Graph

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
still keep alpha feature sidecars disabled by default. The additional
`ci/ai-blaise/observability-contracts-check.sh` gate starts the operator,
shared runtime, and every sidecar `serve` binary on VM/CI loopback and asserts
their real JSON `/healthz` and `/readyz` payloads plus Prometheus `/metrics`
exposition. This status applies
only to the shared probe/metrics runtime.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: O4` in `sidecar/shared/src/lib.rs`
- In-source: `FEATURE: O4` in `sidecar/shared/src/runtime.rs`
- CI: `ci/ai-blaise/observability-contracts-check.sh`
- Executable: `FEATURE: O4` in `sidecar/shared/src/main.rs`

### SC7: Sidecar EndpointSlice Retarget Contract

**Overlay**: `operator/src/reconcile/sidecar_endpoint.rs`, `sidecar/shared/src/ha.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the narrow sidecar HA retarget contract between the
sidecar health selector and the operator: sidecars produce a deterministic
`RetargetDecision`, the operator validates Kubernetes EndpointSlice candidates,
renders a single-selected-endpoint EndpointSlice plus a Service merge patch that
removes the Service selector, and renders an empty EndpointSlice when no
candidate is eligible. The fail-closed path prevents stale sidecar endpoints
from remaining selected after health selection returns no endpoint.

**Motivation**: HA sidecars need a small, reviewable integration surface before
any broader Kubernetes controller watches EndpointSlices or coordinates regional
failover. This contract makes the retarget artifact deterministic and testable
without claiming automated cross-region failover.

**Citus comparison**: Vanilla Citus does not ship ai-blaise sidecars, sidecar
health selection, or Kubernetes EndpointSlice retargeting for sidecar traffic.

Production evidence: VM verification runs `cargo test -p
ai_blaise_citus_sidecar_shared`, `cargo test -p ai_blaise_citus_operator`,
`cargo run -q -p ai_blaise_citus_sidecar_shared -- ha-canonical`,
`cargo run -q -p ai_blaise_citus_operator --
run-endpointslice-retarget-canonical`, and
`ci/ai-blaise/endpointslice-retarget-smoke.sh`. These cover config parsing,
health-aware selection, drain/failure exclusion, EndpointSlice candidate
validation, deterministic manifest rendering, deterministic Service merge patch
rendering, and the empty EndpointSlice fail-closed case. Live in-cluster
EndpointSlice watches, Service patch application, leader election, and
cross-region failover remain alpha and are not claimed by this feature.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: SC7` in `sidecar/shared/src/ha.rs`
- In-source: `FEATURE: SC7` in `operator/src/reconcile/sidecar_endpoint.rs`
- Executable: `cargo run -q -p ai_blaise_citus_sidecar_shared -- ha-canonical`
- Executable: `cargo run -q -p ai_blaise_citus_operator -- run-endpointslice-retarget-canonical`
- CI: `ci/ai-blaise/endpointslice-retarget-smoke.sh`

### O5: Sidecar Deployment Contract

**Overlay**: `operator/src/crds/sidecar.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the operator-facing sidecar deployment contract for
replicas, resources, digest-pinned images, and type-specific configuration
across the V2 sidecar surface. The production-ready boundary is the
Kubernetes controller apply path for `Sidecar` CRs: digest-pinned image
validation, Deployment and Service creation, owner references, status patching,
scoped in-cluster RBAC, and live probe traffic through the generated Service.
It does not claim OpenTelemetry trace propagation, collector export,
configuration loading, PostgreSQL helper modules, autoscaling/rollout policy,
or full production semantics for every sidecar application. Those remain
separate feature surfaces until runtime code and measured evidence exist.

**Motivation**: Rollout behavior is only useful if every sidecar is declared
and reconciled through a consistent resource contract.

**Citus comparison**: Vanilla Citus does not ship out-of-process sidecar
deployment objects.

Production evidence: `ci/ai-blaise/sidecar-controller-live-smoke.sh` boots a
real kind cluster, builds the actual `ai_blaise_citus_operator` and
`ai_blaise_citus_sidecar_realtime` containers from `images/rust-runtime/Dockerfile`,
pushes both to a local OCI registry, consumes their immutable `@sha256`
digests, applies the live `Sidecar` CRD emitted by `print-sidecar-crd`, runs the
operator in `AI_BLAISE_OPERATOR_EXECUTION_MODE=apply` with
`AI_BLAISE_OPERATOR_CONTROLLERS=sidecar`, applies a realtime `Sidecar` CR, and
verifies the generated Deployment, Service, owner references, status fields,
`/healthz`, `/readyz`, `/metrics`, and scoped `sidecars/status` RBAC. The same
smoke applies a mutable `:latest` image and verifies the operator rejects it
before creating a Deployment. `gate-close` depends on this target so the live
Kubernetes apply proof is part of the release boundary.

**References**:

- Design: `docs/ai-blaise/ARCHITECTURE.md`
- In-source: `FEATURE: O5` in `operator/src/crds/sidecar.rs`
- In-source: `FEATURE: O5` in `operator/src/reconcile/sidecar.rs`
- In-source: `FEATURE: O5` in `operator/src/controllers/sidecar.rs`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-reconcile-plans-batch-c`
- Executable: `cargo run -p ai_blaise_citus_operator -- print-sidecar-crd`
- CI: `ci/ai-blaise/operator-reconcilers-batch-c-smoke.sh`
- CI: `ci/ai-blaise/sidecar-controller-live-smoke.sh`

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
vectorizer backlog signals, and the companion shard-placement query plan. The
smoke also proves duplicate snapshot identities and vectorizer tenant references
to unknown tenants fail closed through the same `citus-watch` runner.
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

**Overlay**: `sidecar/shared/src/otel.rs`, `sidecar/shared/src/runtime.rs`,
`pool/src/trace_tap.rs`, `companion/src/trace_context.rs`, and
`images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Threads a W3C `traceparent` end-to-end from pool to PostgreSQL,
companion SQL, and sidecar HTTP ingress. The shared `otel` module exposes a
`TraceContext` extract / inject trait with three carriers — `HeaderMap` (HTTP),
`MetadataMap` (gRPC), and `SetLocalBuilder` (PostgreSQL `SET LOCAL`). The pool
proxy taps the PostgreSQL startup envelope for an embedded traceparent in three
places (custom `traceparent` startup parameter, `options=-c trace.parent=`, and
a backwards-compatible raw `application_name` startup field) and records counters
for tapped versus absent connections without modifying the byte stream.

Production evidence: VM proof run
`REQUIRE_DOCKER=1 bash ci/ai-blaise/otel-trace-propagation-smoke.sh` starts a
real `postgres:17` container with `ai_blaise_citus` installed, runs the pool
proxy against it, sends a traceparent via libpq `PGOPTIONS`, and verifies
PostgreSQL `current_setting('trace.parent')`, `companion.current_traceparent`,
`companion.current_tracestate`, and
`companion.project_traceparent_from_application_name(...)` all preserve the
trace context and fail closed for an invalid traceparent. The same smoke asserts
the pool `trace_tap=present` log line, `traceparent_tapped_total`, and
`traceparent_absent_total`, then starts the real shared sidecar HTTP server and
verifies `/tracez` returns the incoming `traceparent`/`tracestate` and reports
`valid=false` when headers are absent. Focused Rust tests cover HTTP, gRPC, and
PostgreSQL carriers, startup-parameter priority, corrupt traceparent rejection,
and the `/tracez` parser. With `REQUIRE_KIND=1`, the smoke additionally boots a
3-node kind cluster with Jaeger, sends a synthetic OTLP span keyed to the trace
ID accepted by the pool tap, and queries Jaeger's `/api/traces/<trace_id>`
endpoint for `pool.trace_tap`. This production-ready boundary is trace-context
extraction, propagation, SQL projection, sidecar ingress visibility, and Jaeger
correlation harness evidence; it is not automatic OTLP span export from every
component, not a production dashboard/SLO certification, and not a claim that
every business endpoint emits child spans.

**Motivation**: Distributed-database observability needs a single trace ID that
survives the libpq wire so per-sidecar spans, companion spans, and operator
spans can be correlated in Jaeger or Tempo without sampling drift.

**Citus comparison**: Vanilla Citus does not propagate W3C trace-context through
libpq.

**References**:

- Design: `docs/ai-blaise/OBSERVABILITY.md`
- In-source: `FEATURE: O14` in `sidecar/shared/src/otel.rs`
- In-source: `FEATURE: O14` in `sidecar/shared/src/runtime.rs`
- In-source: `FEATURE: O14` in `pool/src/proxy.rs`
- In-source: `FEATURE: O14` in `pool/src/trace_tap.rs`
- In-source: `FEATURE: O14` in `companion/src/trace_context.rs`
- SQL runtime: `FEATURE: O14` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/otel-trace-propagation-smoke.sh`

### O15: Per-Sidecar Structured-Log Schema

**Overlay**: `sidecar/shared/src/log_schema.rs`, `companion/src/log_view.rs`
**Status**: production-ready
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


Production evidence: `ci/ai-blaise/structured-log-ingestion-smoke.sh`
builds the real `companion_contracts` and `ai_blaise_citus_sidecar_shared`
binaries, emits the canonical sidecar JSON log records, renders the companion
`run-log-view-sql-canonical` SQL bundle, starts a real `postgres:17` container,
creates `companion.sidecar_log_raw`, applies all 17 generated typed views,
ingests all 17 sidecar records as jsonb, queries every `sidecar_*_log` view,
verifies traceparent, tenant, and request-id projection for every sidecar, and
checks typed vectorizer columns as PostgreSQL `timestamp with time zone`,
`bigint`, and `double precision` types. The existing `ci/ai-blaise/observability-contracts-check.sh` gate
continues to validate the loopback `serve` surfaces, schema catalog, generated
record fixtures, and unknown-field/type validation in the shared runtime.

Current boundary: the production-ready claim covers the canonical per-sidecar
JSON schema, generated companion SQL views, and PostgreSQL-backed typed-view
ingestion/query path. It does not claim Vector, fluent-bit, Loki, or Kubernetes
log-shipping deployment, Grafana dashboard correlation, or the broader O14
trace propagation path.


**References**:

- Design: `docs/ai-blaise/OBSERVABILITY.md`
- In-source: `FEATURE: O15` in `sidecar/shared/src/lib.rs`
- In-source: `FEATURE: O15` in `sidecar/shared/src/log_schema.rs`
- In-source: `FEATURE: O15` in `companion/src/log_view.rs`
- Acceptance: `cargo test -p ai_blaise_citus_companion --lib log_view`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-log-view-sql-canonical`
- Executable: `FEATURE: O15` in `sidecar/shared/src/main.rs` (`log-schema-canonical`, `log-schema-records-canonical`)
- CI: `ci/ai-blaise/observability-contracts-check.sh`
- CI: `ci/ai-blaise/structured-log-ingestion-smoke.sh`

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

**Overlay**: `companion/src/extension_catalog.rs`, `companion/src/queue.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgmq`, `pgque`

**Summary**: Provides an installable SQL extension catalog runtime entry for
the bloat-free queue substrate contract plus deterministic companion queue
runtime primitives.

Production evidence: Local, VM, and GitHub Actions proof run
`ci/ai-blaise/sql-extension-smoke.sh`, which installs `ai_blaise_citus` into a
real PostgreSQL server and verifies the installable SQL extension catalog
runtime through `companion_internal.seed_extension_catalog`,
`companion_extension_catalog`, `companion_extension_feature_coverage`,
`companion_extension_required`, `companion_required_preload_libraries`, and
that hard-blocked extensions fail closed. The companion runtime depth A smoke
also exercises `DurableQueueRuntime` idempotent enqueue, lease, retry, ack,
and dead-letter behavior with deterministic SQL evidence. Actual binary
extension installation, full operand image build, initdb extension creation,
and operator package reconciliation remain alpha.

**Citus comparison**: Vanilla Citus does not package pgque/pgmq as queue
policy.

**References**:

- SQL runtime: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- CI: `ci/ai-blaise/sql-extension-smoke.sh`
- In-source: `FEATURE: R6` in `companion/src/extension_catalog.rs`
- In-source: `FEATURE: R6` in `companion/src/queue.rs`
- CI: `ci/ai-blaise/companion-runtime-depth-a-smoke.sh`

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
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Captures the vector-provider secret-reference contract that keeps
API keys outside literal values and points operators at External Secrets.

Production evidence: `ci/ai-blaise/security-external-secrets-tls-live-smoke.sh`
boots kind, installs External Secrets Operator chart `0.10.7`, creates a
fake-provider `ExternalSecret` named `ai-blaise-vector-provider-openai`, waits
for it to reconcile into a real Kubernetes Secret, verifies the rendered
ExternalSecret manifest does not contain the vector provider API key literal,
hashes the reconciled `apiKey` value in
`artifacts/security-external-secrets-tls-live-evidence.tsv`, and verifies the
runtime ServiceAccount is denied Secret API reads. This production-ready claim
covers reference-only vector-provider secret binding through live External
Secrets reconciliation with the deterministic fake provider; it does not claim
cloud provider authentication, provider credential rotation, production
rotation SLOs, or runtime loading of that key by a live vectorizer data plane.

**Citus comparison**: Vanilla Citus does not define vector-provider secret
binding.

**References**:

- In-source: `FEATURE: A9` in `companion/src/ops_contracts.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-multiregion-contracts-canonical`
- CI: `ci/ai-blaise/operator-multiregion-contracts-smoke.sh`
- CI: `ci/ai-blaise/security-supply-chain-smoke.sh`
- Live smoke: `ci/ai-blaise/security-external-secrets-tls-live-smoke.sh`

### A10: Streaming Chat Completion UDF

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: alpha
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Adds an installable SQL contract for registering a
redacted AI provider binding and requesting a tenant-scoped chat completion
intent from SQL.

**Current boundary**: The SQL extension installs
`companion_internal.register_ai_provider_binding`,
`companion_ai_provider_bindings`, and `companion_ai_chat_stream`. The smoke
`ci/ai-blaise/ai-sql-contract-smoke.sh` proves deterministic input validation,
secret-reference redaction, tenant binding checks, and the
`sql-intent-fail-closed-only` payload. A10 remains alpha and not
production-ready: it does not call a live model provider, does not produce real
streaming provider chunks, and raises a fail-closed error if provider execution
is requested.

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

**Summary**: Adds an installable SQL contract for registering a
semantic catalog object and emitting a deterministic text-to-SQL request intent
for a tenant-scoped catalog object.

**Current boundary**: The SQL extension installs
`companion_internal.register_semantic_catalog_object`,
`companion_semantic_catalog_objects`, and
`companion_semantic_text_to_sql_intent`. The smoke
`ci/ai-blaise/ai-sql-contract-smoke.sh` proves strict catalog-object,
identifier, question-shape, and optional provider-binding validation, then
emits a `sql-intent-fail-closed-only` JSON report with a deterministic template.
A11 remains alpha and not production-ready: it does not call a live text-to-SQL
model, does not execute generated SQL, and raises a fail-closed error if query
execution is requested.

**Citus comparison**: Vanilla Citus does not include a tenant-scoped semantic
catalog.

**References**:

- In-source: `FEATURE: A11` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### D9: Canary Upgrade Runbook

**Overlay**: `companion/src/ops_contracts.rs`,
`docs/ai-blaise/RUNBOOKS/upgrade.md`, and
`images/citus-pg-overlay/extensions/ai_blaise_citus-upgrade-manifest.tsv`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Tracks the canary-upgrade rehearsal artifact as a required
operations contract, validates the local companion extension transition
manifest, and now executes a live reversible PostgreSQL canary drill for the
`ai_blaise_citus` SQL extension.

Production evidence: VM proof run
`REQUIRE_DOCKER=1 bash ci/ai-blaise/canary-upgrade-rollback-smoke.sh` starts a
real `postgres:17` container with the shipped control, install, upgrade, and
downgrade SQL files mounted into the PostgreSQL extension directory. The smoke
creates `ai_blaise_citus` at `0.1.0`, runs `ALTER EXTENSION ai_blaise_citus
UPDATE TO '0.1.1'`, records a canary event through
`companion_internal.record_extension_upgrade_event`, verifies
`companion_extension_upgrade_events`, rolls back with `ALTER EXTENSION
ai_blaise_citus UPDATE TO '0.1.0'`, and proves the 0.1.1 event table and
recorder are removed after rollback. `ci/ai-blaise/upgrade-rollback-guardrails.sh`
keeps the manifest, reverse SQL, Dockerfile packaging, Make target, release
docs, and runbook wiring fail-closed. This production-ready boundary is the
local companion-extension canary upgrade/rollback path and runbook gate; it is
not full upstream Citus upgrade-matrix evidence, does not certify an operand
image release, and does not perform human production promotion.

**Citus comparison**: Vanilla Citus does not include this canary upgrade
runbook or ai-blaise companion-extension rollback contract.

**References**:

- In-source: `FEATURE: D9` in `companion/src/ops_contracts.rs`
- SQL transition: `FEATURE: D9` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0--0.1.1.sql`
- SQL rollback: `FEATURE: D9` in
  `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.1--0.1.0.sql`
- CI: `ci/ai-blaise/upgrade-rollback-guardrails.sh`
- CI: `ci/ai-blaise/canary-upgrade-rollback-smoke.sh`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-multiregion-contracts-canonical`
- CI: `ci/ai-blaise/operator-multiregion-contracts-smoke.sh`

### D10: Release Hardening Runbook

**Overlay**: `companion/src/ops_contracts.rs` and
`docs/ai-blaise/RUNBOOKS/production.md`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Records the release-readiness review path, security controls, and
operational handoff checklist as a contract surface.

Production evidence: VM proof run
`bash ci/ai-blaise/release-hardening-runbook-smoke.sh` executes the real
companion `run-release-hardening-canonical` contract, verifies all 19 required
release gates and 10 required release-record fields, runs
`runbook-command-check.sh`, `docs-evidence-boundary-check.sh`, and
`production-gap-audit.sh`, runs `production-readiness-check.sh
production-release` and requires it to fail closed while alpha features remain
in release scope, verifies D10 is no longer listed as the blocker, and renders a
release record containing source revision, digest-manifest requirement,
audit/check status, alpha scope, rollback checkpoint requirement, and owner
signoff requirement. The production-ready surface is the fail-closed release
hardening runbook and release-record contract. It does not claim that a release
candidate has been certified, that owner signoff has occurred, or that D9
canary upgrade/rollback drills have run for a particular release.

**Citus comparison**: Vanilla Citus does not include these ai-blaise hardening
gates.

**References**:

- In-source: `FEATURE: D10` in `companion/src/ops_contracts.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-release-hardening-canonical`
- CI: `ci/ai-blaise/release-hardening-runbook-smoke.sh`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-multiregion-contracts-canonical`
- CI: `ci/ai-blaise/operator-multiregion-contracts-smoke.sh`

### D11: MCP Developer Workflow

**Overlay**: `tools/citus-mcp`, `sidecar/mcp`, and `companion/src/ops_contracts.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines the validation-only MCP workflow contract for exposing
Citus-oriented developer operation requests to agent tooling.

Production evidence: VM proof runs `ci/ai-blaise/mcp-stdio-smoke.sh`, `ci/ai-blaise/mcp-sidecar-stdio-smoke.sh`, `ci/ai-blaise/mcp-sidecar-http-smoke.sh`, and `REQUIRE_DOCKER=1 ci/ai-blaise/mcp-db-smoke.sh`. They exercise the scoped MCP workflow through real MCP stdio validation, sidecar stdio, sidecar HTTP, and PostgreSQL-backed read-only execution paths, including initialize, malformed JSON, invalid params, tool discovery, safe tenant query, shard listing, cross-schema denial, destructive-denial, and missing-scope rejection. Production evidence is limited to validation plus read-only database execution; authenticated multi-user deployment and mutating Kubernetes/database operations remain outside this production-ready workflow.

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
- Executable: `cargo run -p ai_blaise_citus_operator -- run-multiregion-contracts-canonical`
- CI: `ci/ai-blaise/operator-multiregion-contracts-smoke.sh`
- CI: `ci/ai-blaise/mcp-stdio-smoke.sh`
- CI: `ci/ai-blaise/mcp-sidecar-stdio-smoke.sh`
- CI: `ci/ai-blaise/mcp-sidecar-http-smoke.sh`
- CI: `ci/ai-blaise/kind-production-smoke.sh`

### Edge1: Bounded-Staleness Edge Replicas

**Overlay**: `companion/src/advanced_planner.rs`, `sidecar/hlc`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Captures the decision-record gate for edge read replicas with a
bounded-staleness contract.

Production evidence: `ci/ai-blaise/sidecar-hlc-smoke.sh` starts the real
`ai_blaise_citus_sidecar_hlc serve` process with configured
`AI_BLAISE_HLC_EDGE_REPLICAS`, waits for `/readyz`, verifies `/closed_ts`,
advances the local clock through `/clock/tick`, observes peer clock evidence
through `/clock/observe`, and drives `/edge_read` against a configured edge
region. The live smoke proves `/edge_read` serves an `AS OF` exactly at the
closed timestamp, rejects a newer-than-closed edge read with HTTP 409, rejects a
read outside the configured `max_staleness_ms` budget with HTTP 409, rejects a
replica/edge-region mismatch with HTTP 409, and rejects an unknown edge region
with HTTP 409. It emits `edge_bounded_staleness_gate=passed`,
`edge_read_as_of_closed_served=true`,
`edge_read_newer_than_closed_rejected=true`,
`edge_read_too_stale_rejected=true`,
`edge_read_replica_mismatch_rejected=true`, and
`edge_unknown_region_rejected=true`.

The production-ready boundary is the sidecar edge read-eligibility gate only:
closed timestamp publication, configured edge-region to replica mapping, maximum
staleness enforcement, and fail-closed HTTP decisions. Edge replica
provisioning, POP/WAN network deployment, SQL/MVCC snapshot execution, planner
integration, data-plane query routing, failover automation, and Kubernetes
traffic remain alpha.

**Citus comparison**: Vanilla Citus does not model edge POP read replicas.

**References**:

- In-source: `FEATURE: Edge1` in `companion/src/advanced_planner.rs`
- In-source: `FEATURE: Edge1` in `sidecar/hlc/src/lib.rs`
- In-source: `FEATURE: Edge1` in `sidecar/hlc/src/runtime.rs`
- In-source: `FEATURE: Edge1` in `sidecar/hlc/src/main.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_hlc -- serve`
- CI: `ci/ai-blaise/sidecar-hlc-smoke.sh`

### Edge2: libsql Read-Tier Research Guard

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Keeps the libsql-shaped read-tier concept behind an explicit
fail-closed research guard so production surfaces cannot imply libsql behavior
without a separate implementation and measured runtime evidence.

**Current production-ready boundary**: Edge2 is production-ready only for the
negative guard. The companion contract records
`docs/ai-blaise/ADR/0009-libsql-read-tier-research-guard.md`, blocks the
`libsql production read tier` integration, enumerates five promotion evidence
requirements, enumerates four forbidden runtime claims, and emits a canonical
report with `live_execution_claims=0`, `replication_adapter_claimed=false`,
`workload_isolation_claimed=false`, and
`production_query_routing_claimed=false`. No libsql read-tier integration,
replication adapter, workload isolation, production query routing, operator
reconciliation, or Kubernetes traffic is production-ready.

Production evidence:

- In-source: `FEATURE: Edge2` in `companion/src/advanced_planner.rs`
- ADR: `docs/ai-blaise/ADR/0009-libsql-read-tier-research-guard.md`
- Executable: `cargo test -p ai_blaise_citus_companion edge2_libsql_research_guard_is_fail_closed`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-libsql-read-tier-guard-canonical`
- CI: `ci/ai-blaise/edge2-libsql-research-guard-smoke.sh`
- Evidence markers: `edge2_libsql_research_guard_smoke`,
  `guard_status=fail-closed`, `live_execution_claims=0`,
  `replication_adapter_claimed=false`, `workload_isolation_claimed=false`, and
  `production_query_routing_claimed=false`

**Citus comparison**: Vanilla Citus does not include a libsql-shaped research
gate.

**References**:

- In-source: `FEATURE: Edge2` in `companion/src/advanced_planner.rs`
- ADR: `docs/ai-blaise/ADR/0009-libsql-read-tier-research-guard.md`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-libsql-read-tier-guard-canonical`
- CI: `ci/ai-blaise/edge2-libsql-research-guard-smoke.sh`

### F3: Iceberg Federation To Warehouses

**Overlay**: `companion/src/advanced_planner.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines catalog and warehouse inputs for an Iceberg federation bridge and proves real Apache Iceberg REST-catalog round-trip via the tabulario/iceberg-rest image.

**Current boundary**: The F3 production-ready claim covers Iceberg REST catalog connectivity + snapshot planning + catalog-metadata read end to end. It does NOT claim live Snowflake, Databricks, Trino, or Spark warehouse query execution; those remain alpha and require external warehouse credentials.

Production evidence: `REQUIRE_DOCKER=1 bash ci/ai-blaise/f3-iceberg-federation-live-smoke.sh` boots tabulario/iceberg-rest:latest on a loopback port, creates a warehouse namespace iceberg_federation, creates an Iceberg v2 table tenant_orders with a 4-field schema, reads the table metadata back through the REST API including metadata-location, lists tables in the namespace, and emits the canonical evidence row to `artifacts/f3-iceberg-federation-evidence.tsv` with namespace+table+format counters.

**Citus comparison**: Vanilla Citus does not define Iceberg warehouse
federation.

**References**:

- In-source: `FEATURE: F3` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`

### F4: postgres_fdw Credential Rotation

**Overlay**: `companion/src/advanced_planner.rs`, `companion/src/fdw_rotation.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `postgres_fdw`

**Summary**: Renders a secret-safe `postgres_fdw` user-mapping credential
rotation plan and proves it against a live local PostgreSQL-to-PostgreSQL FDW
link.

**Current boundary**: Production-ready for rotating one existing
`postgres_fdw` user mapping by rendering a parameterized `ALTER USER MAPPING`
statement, disconnecting cached FDW connections with
`postgres_fdw_disconnect_all()`, and validating the new credential through a
foreign table query. Managed secret backends, Kubernetes `ExternalSecret`
reconciliation, application connection draining outside `postgres_fdw`,
cross-region FDW topology changes, and multi-tenant secret distribution are not
claimed by this feature.

Production evidence: `REQUIRE_DOCKER=1 ci/ai-blaise/fdw-credential-rotation-live-smoke.sh`
starts two `postgres:17-bookworm` containers, creates a real `postgres_fdw`
foreign server, proves the original mapping can read the remote table, changes
the remote password, proves the stale mapping is rejected
(`old_password_rejected=true`), executes the companion-rendered SQL plan, proves
the rotated mapping succeeds (`new_password_succeeded=true`), and verifies the
rendered plan uses a psql secret variable with `plan_secret_literals=false`.

**Citus comparison**: Vanilla Citus does not prescribe FDW secret rotation.

**References**:

- In-source: `FEATURE: F4` in `companion/src/advanced_planner.rs`
- In-source: `FEATURE: F4` in `companion/src/fdw_rotation.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-fdw-credential-rotation-canonical`
- SQL: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-fdw-credential-rotation-sql-canonical`
- CI: `ci/ai-blaise/fdw-credential-rotation-live-smoke.sh`
- Runbook: `docs/ai-blaise/RUNBOOKS/fdw-credential-rotation.md`

### L7: Citus Columnar Analytical Path

**Overlay**: `companion/src/advanced_planner.rs`, `companion/src/columnar_tiering.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: `citus_columnar`

**Summary**: Provides a bounded live Citus columnar analytical path: the
companion renders a read-only catalog verification plan, and the VM smoke proves
a distributed `USING columnar` table executes through Citus with a `ColumnarScan`.

Production evidence: `REQUIRE_DOCKER=1 bash ci/ai-blaise/columnar-tiering-live-smoke.sh`
starts a real Citus coordinator and worker from the cohabitation image, installs
`citus` and `citus_columnar` on both nodes, creates `public.columnar_orders`
with `USING columnar`, calls `create_distributed_table('public.columnar_orders',
'tenant_id', shard_count => 4)`, inserts 12 rows, and executes the
companion-rendered SQL from `run-columnar-tiering-sql-canonical`. The smoke
requires `l7_columnar_access_method=true`,
`l7_distributed_columnar_table=true`, `l7_columnar_query_rows=12`,
`l7_columnar_query_total=3024`, `l7_citus_custom_scan_executed=true`, and
`l7_columnar_scan_executed=true` from a real `EXPLAIN` containing Citus adaptive
execution and `ColumnarScan`.

**Current production-ready boundary**: L7 is production-ready only for this
bounded live Citus columnar read path: creating a distributed columnar table,
verifying Citus catalog placement, preserving row results, and executing an
aggregate through `ColumnarScan`. Cost-model tier selection,
automatic hot/warm/cold movement, workload-routing rewrites, cross-tier planner
rewrites, background schedulers, object-store cold reads, and Kubernetes traffic
remain outside this evidence boundary.

**Citus comparison**: Vanilla Citus has columnar storage; this overlay adds a
machine-checked production boundary for the ai-blaise tiering contract.

**References**:

- In-source: `FEATURE: L7` in `companion/src/advanced_planner.rs`
- In-source: `FEATURE: L7` in `companion/src/columnar_tiering.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-columnar-tiering-canonical`
- SQL: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-columnar-tiering-sql-canonical`
- CI: `ci/ai-blaise/columnar-tiering-live-smoke.sh`

### L10: Cross-Tier Query Planner

**Overlay**: `companion/src/advanced_planner.rs`, `companion/src/cross_tier_query.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `citus_columnar`

**Summary**: Executes a bounded hot/warm/cold query plan over live Citus tables:
a hot row table plus warm and cold columnar tables, all distributed by the same
tenant key.

Production evidence: `REQUIRE_DOCKER=1 bash ci/ai-blaise/cross-tier-query-live-smoke.sh`
starts a real Citus coordinator and worker from the local cohabitation image,
installs `citus` and `citus_columnar`, creates distributed `public.l10_hot_orders`,
`public.l10_warm_orders`, and `public.l10_cold_orders`, inserts deterministic hot,
warm, and cold rows, and executes the companion-rendered SQL from
`run-cross-tier-query-sql-canonical`. The smoke requires
`cross_tier_query_live=passed`, `l10_hot_tier_rows=4`, `l10_warm_tier_rows=4`,
`l10_cold_tier_rows=4`, `l10_cross_tier_rows=12`, `l10_cross_tier_total=6678`,
`l10_citus_custom_scan_executed=true`, and `l10_columnar_scan_executed=true`.
The real `EXPLAIN` must contain Citus adaptive execution and two `ColumnarScan`
entries for the warm and cold tiers.

**Current production-ready boundary**: L10 is production-ready only for this
companion-rendered, read-only cross-tier SQL shape over live distributed Citus
row plus columnar tables. It proves tier access-method validation, Citus shard
placement catalog checks, `UNION ALL` tier composition, per-tier rollup
preservation, and combined result preservation. It does not claim automatic
workload routing, automatic query rewrites for arbitrary user SQL, cost-model
tier selection, object-store cold reads, background tier movement, or Kubernetes
traffic; the smoke requires `automatic_workload_routing_exercised=false`,
`automatic_query_rewrite_exercised=false`, `cost_model_selection_exercised=false`,
`object_store_cold_read_exercised=false`, and `kubernetes_traffic_exercised=false`.

**Citus comparison**: Vanilla Citus does not provide this ai-blaise companion
contract for combining declared hot, warm, and cold tiers with explicit evidence
and nonclaim markers.

**References**:

- In-source: `FEATURE: L10` in `companion/src/advanced_planner.rs`
- In-source: `FEATURE: L10` in `companion/src/cross_tier_query.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-cross-tier-query-canonical`
- SQL: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-cross-tier-query-sql-canonical`
- CI: `ci/ai-blaise/cross-tier-query-live-smoke.sh`

### M4: Schema Drift Detection

**Overlay**: `companion/src/advanced_planner.rs`, `companion/src/schema_drift.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Renders a live `information_schema.columns` drift detector for a
declared table-column manifest.

**Current boundary**: Production-ready for detecting missing columns,
unexpected columns, data-type mismatches, and nullability mismatches on existing
PostgreSQL tables using a temporary expected-schema table plus live
`information_schema.columns` introspection. Remediation planning, DDL
execution, operator apply behavior, cross-database inventory fanout, and
automatic migration generation are not claimed by this feature.

Production evidence: `REQUIRE_DOCKER=1 ci/ai-blaise/schema-drift-live-smoke.sh`
starts a `postgres:17-bookworm` container, creates a live `public.accounts`
table with one `missing_column`, one `type_mismatch`, one
`nullability_mismatch`, and one `unexpected_column`, executes the
companion-rendered SQL plan, verifies all four drift rows, fixes the schema,
and reruns the same plan to prove `clean_schema_zero_drift=true`.

**Citus comparison**: Vanilla Citus does not reconcile declarative schema
drift.

**References**:

- In-source: `FEATURE: M4` in `companion/src/advanced_planner.rs`
- In-source: `FEATURE: M4` in `companion/src/schema_drift.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-schema-drift-canonical`
- SQL: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-schema-drift-sql-canonical`
- CI: `ci/ai-blaise/schema-drift-live-smoke.sh`
- Runbook: `docs/ai-blaise/RUNBOOKS/schema-drift-detection.md`

### MR3: Regional Row Placement

**Overlay**: `companion/src/advanced_planner.rs`,
`companion/src/regional_row_placement.rs`, `operator/src/crds/region.rs`,
`operator/src/reconcile/region.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Defines region-prefix and distribution-key inputs for regional row
placement policy, plus a bounded live multi-worker Citus execution path for
explicit regional locality keys.

Production evidence: VM proof run
`REQUIRE_DOCKER=1 bash ci/ai-blaise/regional-placement-live-smoke.sh` now runs
two phases. The first phase preserves the S8/S12 locality-key and tablespace
catalog proof. The MR3 phase starts a real Citus coordinator with two workers,
creates `public.mr3_orders`, registers only the `us-east-1` worker before table
creation, inserts `us-east-1:tenant-a` and `eu-west-1:tenant-b` rows, registers
the `eu-west-1` worker, and executes companion-rendered SQL from
`run-regional-row-placement-sql-canonical`. That SQL calls
`isolate_tenant_to_new_shard` for both regional locality keys, calls
`citus_move_shard_placement` to move the EU shard to the EU worker, and verifies
`mr3_shards_isolated=true`, `mr3_citus_move_shard_placement_executed=true`,
`mr3_worker_placement_enforced=true`, `mr3_matched_region_count=2`, and
`mr3_rows_preserved=true` with row details `us-east-1:8:36` and
`eu-west-1:8:360`.

**Current production-ready boundary**: MR3 is production-ready only for bounded
explicit-key regional row placement in a live multi-worker Citus deployment:
locality-key shard isolation, shard placement movement to declared region
workers, and row-preservation/catalog verification. It does not claim
WAN/multi-region network execution, Kubernetes operator reconciliation,
automatic repartition scheduling, regional admission control in Kubernetes,
regional traffic routing, GeoIP routing, or regional failover. MR9 is production-ready for the bounded two-region drill via `ci/ai-blaise/mr9-regional-failover-live-smoke.sh` and remains alpha for full Kubernetes-orchestrated region failover, DNS cutover, GeoIP routing, and cross-region pgactive replication.

**Citus comparison**: Vanilla Citus does not encode region in key policy.

**References**:

- In-source: `FEATURE: MR3` in `companion/src/advanced_planner.rs`
- In-source: `FEATURE: MR3` in `companion/src/regional_row_placement.rs`
- In-source: `FEATURE: MR3` in `operator/src/crds/region.rs`
- In-source: `FEATURE: MR3` in `operator/src/reconcile/region.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-regional-row-placement-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-regional-row-placement-sql-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-multiregion-contracts-canonical`
- CI: `ci/ai-blaise/operator-multiregion-contracts-smoke.sh`
- CI: `ci/ai-blaise/regional-placement-live-smoke.sh`

### MR6: Closed-Timestamp Time Travel

**Overlay**: `companion/src/advanced_planner.rs`, `sidecar/hlc`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides the bounded closed-timestamp time-travel gate used by
follower reads: timestamp/staleness intent validation, live HLC peer observation,
closed-timestamp publication, and fail-closed `AS OF` serve/reject decisions.

Production evidence: `ci/ai-blaise/sidecar-hlc-smoke.sh` starts the real
`ai_blaise_citus_sidecar_hlc serve` process with a three-replica shard group,
waits for `/readyz`, verifies `/closed_ts`, advances the local clock through
`/clock/tick`, merges peer timestamp evidence through `/clock/observe`, confirms
the peer appears in the published closed timestamp, proves `/follower_read`
serves an `AS OF` exactly at the closed timestamp, proves an `AS OF` newer than
the closed timestamp is rejected with HTTP 409, and verifies unknown peers fail
closed. The smoke emits `closed_timestamp_time_travel_gate=passed`,
`follower_read_as_of_closed_served=true`,
`follower_read_newer_than_closed_rejected=true`, and
`closed_ts_peer_exchange_observed=true`.

The production-ready boundary is the closed-timestamp time-travel gate only. It
does not claim MVCC snapshot execution, replica query routing, stale-read SQL
syntax, planner integration, cross-region clock discipline, or Kubernetes
reconciliation.

**Citus comparison**: Vanilla Citus does not expose bounded-staleness time
travel.

**References**:

- In-source: `FEATURE: MR6` in `companion/src/advanced_planner.rs`
- In-source: `FEATURE: MR6` in `sidecar/hlc/src/runtime.rs`
- In-source: `FEATURE: MR6` in `sidecar/hlc/src/main.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_hlc -- run-runtime-canonical`
- Executable: `cargo run -p ai_blaise_citus_sidecar_hlc -- serve`
- CI: `ci/ai-blaise/sidecar-hlc-smoke.sh`
- CI: `ci/ai-blaise/topology-consensus-smoke.sh`

### MR9: Region Survival Runbook

**Overlay**: `companion/src/ops_contracts.rs` and
`docs/ai-blaise/RUNBOOKS/disaster-recovery.md`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Records the regional failover drill as a required operational
artifact and proves the failover narrative end to end against live
PostgreSQL containers. `FEATURE: MR9 is production-ready` for the bounded
two-region drill scope: pre-failover tenant write, `pg_basebackup`
checkpoint, declared data-loss window for post-backup writes, primary
container stop (region-loss simulation), surviving region boot directly on
the backup data directory with automatic WAL recovery from the streamed
`pg_wal/` contents, client traffic recovery against the surviving region,
and validation queries for per-tenant counts and per-region marker counts.

**Current boundary**: The MR9 production-ready claim is bounded to the
two-container failover drill in
`ci/ai-blaise/mr9-regional-failover-live-smoke.sh`. It does not claim
cross-region pgactive conflict resolution (S7), managed object-store
backup transport (B family), Kubernetes pod-level failover, geographically
distributed network propagation, GeoIP pool routing across the boundary
(MR5), closed-timestamp follower reads across regions (MR6), or regional
row movement (MR3). PITR restore depth has its own evidence path under
`ci/ai-blaise/dr-restore-depth-check.sh`.

Production evidence: `REQUIRE_DOCKER=1 bash ci/ai-blaise/mr9-regional-failover-live-smoke.sh`
boots two postgres:17-bookworm containers labeled us-east-1 (primary) and
us-west-2 (surviving), writes 100 tenant rows, takes a `pg_basebackup`,
writes 50 post-backup rows that become the declared data-loss window,
stops us-east-1 to simulate region loss, boots us-west-2 directly on the
backup data dir, verifies the surviving region serves the pre-backup 100
rows (with tenant-a=50 + tenant-b=50 + us-east-1 marker=100), records the
cutover window in seconds, and appends an evidence row to
`artifacts/mr9-regional-failover-evidence.tsv`. The smoke also accepts
`MR9_POSTGRES_IMAGE` to retarget the operand image, and
`MR9_EVIDENCE_FILE` to redirect the evidence sink.

**Citus comparison**: Vanilla Citus does not ship this regional DR runbook.

**References**:

- In-source: `FEATURE: MR9` in `companion/src/ops_contracts.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-multiregion-contracts-canonical`
- CI (operator contract): `ci/ai-blaise/operator-multiregion-contracts-smoke.sh`
- CI (regional failover live smoke):
  `REQUIRE_DOCKER=1 ci/ai-blaise/mr9-regional-failover-live-smoke.sh`
- CI (PITR restore-depth): `REQUIRE_DOCKER=1 ci/ai-blaise/dr-restore-depth-check.sh`
- Evidence file: `artifacts/mr9-regional-failover-evidence.tsv`
- Executable: `cargo run -p ai_blaise_citus_e2e --bin dr_restore_depth_report`
- Benchmark: `benchmarks/chaos/scenarios/kill-coordinator.sh`,
  `benchmarks/chaos/scenarios/network-partition.sh` (V2 gate 11 chaos
  acceptance; full measured runs are tracked separately)

### R3: Columnstore-On-Worker Policy

**Overlay**: `companion/src/advanced_planner.rs`, `companion/src/columnar_tiering.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: `citus_columnar`

**Summary**: Verifies that the columnstore policy reaches a live Citus worker:
worker-local access method and row preservation are checked directly against the
worker after distributed columnar writes.

Production evidence: `REQUIRE_DOCKER=1 bash ci/ai-blaise/columnar-tiering-live-smoke.sh`
starts a real Citus coordinator plus worker, creates a distributed
`public.columnar_orders` table with `USING columnar`, and then connects directly
to the worker container. The worker checks require
`r3_worker_columnstore_policy_live=true`, `r3_worker_access_method=columnar`, and
`r3_worker_columnar_rows_preserved=true` with details `rows=12 total=3024`.

**Current production-ready boundary**: R3 is production-ready only for bounded
worker-local verification that a Citus distributed columnar table is visible on
the worker with the `columnar` access method and preserved rows. Automatic
policy scheduling, age-threshold telemetry collection, tier movement,
rebalancing, cost-based placement, and Kubernetes/operator orchestration remain
outside this evidence boundary.

**Citus comparison**: Vanilla Citus exposes columnar storage, but it does not
define this ai-blaise worker tiering policy or its fail-closed evidence markers.

**References**:

- In-source: `FEATURE: R3` in `companion/src/advanced_planner.rs`
- In-source: `FEATURE: R3` in `companion/src/columnar_tiering.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-columnar-tiering-canonical`
- SQL: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-columnar-tiering-sql-canonical`
- CI: `ci/ai-blaise/columnar-tiering-live-smoke.sh`

### R8: Non-Hypertable Cold Columnar Path

**Overlay**: `companion/src/advanced_planner.rs`, `companion/src/columnar_tiering.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: `citus_columnar`

**Summary**: Proves the cold columnar path for ordinary non-hypertable
relations by checking a live Citus columnar table is not registered as a
Timescale hypertable.

Production evidence: `REQUIRE_DOCKER=1 bash ci/ai-blaise/columnar-tiering-live-smoke.sh`
creates the same live distributed `public.columnar_orders` table and executes
`run-columnar-tiering-sql-canonical`, whose SQL checks `pg_am`, Citus
`pg_dist_*` catalogs, and `_timescaledb_catalog.hypertable` when that catalog is
present. The smoke requires `r8_non_hypertable_cold_columnar_path=true` while
also requiring the columnar table query to preserve 12 rows and total 3024.

**Current production-ready boundary**: R8 is production-ready only for bounded
live verification that a normal, non-hypertable Citus table can use the
`columnar` access method and remain queryable through the distributed read path.
Live cold-tier movement, background archival, object-store handoff, automatic
hot-to-cold migration, hypertable conversion, and Kubernetes traffic remain
outside this evidence boundary.

**Citus comparison**: Vanilla Citus has columnar storage, but it does not define
this non-hypertable cold-tier policy boundary.

**References**:

- In-source: `FEATURE: R8` in `companion/src/advanced_planner.rs`
- In-source: `FEATURE: R8` in `companion/src/columnar_tiering.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-columnar-tiering-canonical`
- SQL: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-columnar-tiering-sql-canonical`
- CI: `ci/ai-blaise/columnar-tiering-live-smoke.sh`

### R12: Per-Shard Temperature Ranking

**Overlay**: `companion/src/shard_temperature.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: Citus catalog (`pg_dist_shard`)

**Summary**: Ranks distributed table shards by live Citus shard catalog rows and
a validated metrics table so operators can identify hot, warm, and cold movement
candidates without mutating placement.

**Current production-ready boundary**: R12 is production-ready for the bounded
read-only ranking query emitted by the companion. The query joins real
`pg_dist_shard`, `pg_class`, and `pg_namespace` rows to
`public.ai_blaise_shard_temperature_samples`, validates required identifiers and
threshold ordering fail-closed, computes deterministic temperature scores, and
labels `hot`, `warm`, and `cold` target tiers. It does not collect production
telemetry, execute automatic tier movement, invoke cold-tier moves, change Citus
placements, or integrate with the distributed planner.

Production evidence:

- In-source: `FEATURE: R12` in `companion/src/shard_temperature.rs` and
  `companion/src/advanced_planner.rs`
- Executable: `cargo test -p ai_blaise_citus_companion shard_temperature -- --nocapture`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-shard-temperature-ranking-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-shard-temperature-ranking-sql-canonical`
- Executable: `REQUIRE_DOCKER=1 bash ci/ai-blaise/shard-temperature-ranking-live-smoke.sh`
- Evidence markers: `shard_temperature_ranking_live=passed`,
  `citus_pg_dist_shard_joined=true`, `temperature_scores_ranked=true`,
  `automatic_tier_movement=false`, and `coldtier_moves_executed=false`

**Citus comparison**: Vanilla Citus exposes shard metadata in `pg_dist_shard`,
but it does not maintain this overlay temperature-score table or rank shards
into hot/warm/cold movement candidates.

**References**:

- In-source: `FEATURE: R12` in `companion/src/shard_temperature.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-shard-temperature-ranking-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-shard-temperature-ranking-sql-canonical`
- Executable: `REQUIRE_DOCKER=1 bash ci/ai-blaise/shard-temperature-ranking-live-smoke.sh`

### RT5: Phoenix-Channel-Compatible Realtime Client

**Overlay**: `sidecar/realtime`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Provides a Phoenix-channel-compatible realtime protocol surface exercised by raw WebSocket clients and CDC ingest smokes.

**Citus comparison**: Vanilla Citus does not provide realtime client
compatibility gates.

Production evidence: VM worker D on experiment-playground, 2026-05-23: `cargo test -p ai_blaise_citus_sidecar_realtime`, `bash ci/ai-blaise/sidecar-realtime-smoke.sh`, and the raw-socket integration test prove Phoenix-compatible WebSocket upgrade, `phx_join`, tenant/topic filters, presence diffs, CDC ingest over Unix-domain socket, `postgres_changes` fan-out, and health/ready/metrics on the WS listener.

Current boundary: The production-ready claim is limited to the single-node raw WebSocket/Phoenix runtime and CDC-ingest fan-out exercised by `ci/ai-blaise/sidecar-realtime-smoke.sh`. The canonical runtime reports `runtime_boundary=single-node-raw-ws-cdc-ingest`, `websocket_network_exercised=true`, `browser_client_exercised=false`, `cdc_tailing_integrated=false`, `multi_node_pubsub=false`, and `kubernetes_traffic_exercised=false`; browser client behavior, WebSocket extension negotiation, live CDC tailing, multi-node pubsub, and Kubernetes traffic remain outside this proof. The presence timestamp guard enforces a UTC-looking `online_at` shape ending in `Z`, not a full calendar semantic parse.

**References**:

- In-source: `FEATURE: RT5` in `companion/src/ops_contracts.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-multiregion-contracts-canonical`
- CI: `ci/ai-blaise/operator-multiregion-contracts-smoke.sh`
- In-source: `FEATURE: RT5` in `sidecar/realtime/src/live.rs`
- In-source: `FEATURE: RT5` in `sidecar/realtime/src/hub.rs`
- CI: `ci/ai-blaise/sidecar-realtime-smoke.sh`

### S1: Auto Shard Split

**Overlay**: `companion/src/advanced_planner.rs`, `companion/src/shard_split.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Defines shard-group and split-threshold inputs for automated
split intent, plus a bounded live Citus tenant-isolation shard split contract
for a distributed table.

**Current production-ready boundary**: S1 is production-ready for the bounded
single-node Citus tenant-isolation primitive exercised by
`ci/ai-blaise/shard-split-live-smoke.sh`.

Production evidence: `ci/ai-blaise/shard-split-live-smoke.sh` starts a real
Citus server with `wal_level=logical`, creates `public.s1_orders` as a
distributed table with `split_shard_count_before=4`, calls
`isolate_tenant_to_new_shard('public.s1_orders'::regclass, 4, 'CASCADE',
'block_writes')` through the companion-rendered SQL plan, and verifies
`split_shard_count_after=6`, `split_tenant_rows_preserved=10`,
`split_new_shard_created=true`, `split_tenant_shard_changed=true`, and
`split_isolated_range_exact=true`. The companion canonical report exposes the
same boundary through `ShardSplitPlan`, `run-shard-split-canonical`, and
`run-shard-split-sql-canonical`.

This status does not claim an automated policy scheduler, threshold telemetry,
rollback automation, multi-node movement, cross-table cascade coverage beyond
the tested Citus call, autonomous rebalancing, or Kubernetes traffic.

**Citus comparison**: Vanilla Citus exposes `isolate_tenant_to_new_shard` as a
manual tenant-isolation primitive; ai-blaise adds a validated companion contract,
live evidence gate, and explicit nonclaim markers for the surrounding automated
split workflow.

**References**:

- In-source: `FEATURE: S1` in `companion/src/advanced_planner.rs`
- In-source: `FEATURE: S1` in `companion/src/shard_split.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-shard-split-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-shard-split-sql-canonical`
- CI: `ci/ai-blaise/shard-split-live-smoke.sh`

### S3: Clone-Node Fast Scale-Out

**Overlay**: `companion/src/advanced_planner.rs`, `companion/src/clone_node.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Records source-worker and target-worker inputs for a clone-node
scale-out operation, plus a bounded live Citus physical-replica clone promotion
contract.

**Current production-ready boundary**: S3 is production-ready for the bounded
VM-proven clone-node flow in `ci/ai-blaise/clone-node-live-smoke.sh`: one real
Citus coordinator, one primary worker, and one PostgreSQL physical streaming
replica clone.

Production evidence: the live smoke starts the real Citus containers, registers
the primary worker through companion-rendered setup SQL, creates distributed
`public.s3_orders` with four shard placements, builds the clone worker with
`pg_basebackup`, verifies the clone is in recovery before promotion, executes
`citus_add_clone_node` and `citus_promote_clone_and_rebalance` through
companion-rendered promote SQL, waits for Citus catch-up and `pg_promote`, and
verifies `clone_rows_preserved=20`, `clone_sum_preserved=5060`,
`clone_role_after_promote=primary`, `clone_active_after_promote=true`,
`clone_should_have_shards_after_promote=true`, `clone_shard_placements_after=2`,
and `primary_shard_placements_after=2`. The companion canonical report exposes
the same boundary through `CloneNodePlan`, `run-clone-node-canonical`,
`run-clone-node-setup-sql-canonical`, and
`run-clone-node-promote-sql-canonical`.

This status does not claim Kubernetes clone orchestration, CSI snapshot based
cloning, automatic capacity policy, WAN/cross-region clone operation,
service/DNS retargeting, or production traffic cutover.

**Citus comparison**: Vanilla Citus exposes `citus_add_clone_node` and
`citus_promote_clone_and_rebalance`; ai-blaise adds a validated companion
contract, a live physical-replica proof, and explicit nonclaim markers around
the surrounding automated scale-out workflow.

**References**:

- In-source: `FEATURE: S3` in `companion/src/advanced_planner.rs`
- In-source: `FEATURE: S3` in `companion/src/clone_node.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-clone-node-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-clone-node-setup-sql-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-clone-node-promote-sql-canonical`
- CI: `ci/ai-blaise/clone-node-live-smoke.sh`

### S7: Cross-Region Replication Via pgactive

**Overlay**: `companion/src/ops_contracts.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `pgactive`

**Summary**: Captures the conflict-policy gate required before pgactive-backed cross-region replication can be enabled and proves the pgactive runtime stack against a source-built PostgreSQL+pgactive image with the FDW-driven group lifecycle and conflict-policy infrastructure exercised end to end.

**Current boundary**: The S7 production-ready claim covers the pgactive runtime + conflict-policy infrastructure on a single regional node: shared_preload_libraries, CREATE EXTENSION pgactive, pgactive_fdw FDW + user mapping, pgactive_create_group + pgactive_wait_for_node_ready, the pgactive_conflict_history table, and the three conflict-logging GUCs (log_conflicts_to_table, log_conflicts_to_logfile, conflict_logging_include_tuples). The pgactive image is source-built from aws/pgactive upstream via `images/citus-pg-overlay/pgactive/Dockerfile`. Multi-host active-active bootstrap via the AWS-recommended pgactive_init_copy binary remains alpha-deferred under this same S7 contract: upstream pgactive_join_group logical-copy bootstrap races with the joining node pre-ready catalog entry on the target node, manifesting as a previous init failed loop; the supported path uses pgactive_init_copy which requires cross-host orchestration outside this CI smoke.

Production evidence: `REQUIRE_DOCKER=1 bash ci/ai-blaise/s7-pgactive-active-active-live-smoke.sh` boots the pgactive-pg17 image, installs the extension, configures pgactive_fdw, calls pgactive_create_group + pgactive_wait_for_node_ready, and verifies node_status=r, is_active=true, conflict_history table, 3 conflict GUCs, and pgactive in shared_preload_libraries. Evidence row in `artifacts/s7-pgactive-runtime-evidence.tsv`.

**Citus comparison**: Vanilla Citus does not bundle pgactive policy gates.

**References**:

- In-source: `FEATURE: S7` in `companion/src/ops_contracts.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-multiregion-contracts-canonical`
- CI: `ci/ai-blaise/operator-multiregion-contracts-smoke.sh`

### S8: Locality-Prefixed PKs

**Overlay**: `companion/src/regional_placement.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: Citus catalog (`pg_dist_partition`)

**Summary**: Validates that a regional table uses a locality key followed by the
tenant key as the primary-key prefix, and that Citus has a distributed-table
catalog row for the locality-key distribution column.

**Current production-ready boundary**: S8 is production-ready for the bounded
read-only catalog guard only. The companion-rendered SQL checks `pg_index`,
`pg_attribute`, and `pg_dist_partition` against a live Citus table and fails
closed when the primary-key prefix or distribution catalog evidence is missing.
It does not rewrite keys, migrate existing data, enforce foreign-key
compatibility, move shards, rebalance workers, or prove multi-region request
routing.

Production evidence:

- In-source: `FEATURE: S8` in `companion/src/regional_placement.rs` and
  `companion/src/advanced_planner.rs`
- Executable: `cargo test -p ai_blaise_citus_companion regional_placement -- --nocapture`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-regional-placement-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-regional-placement-sql-canonical`
- Executable: `REQUIRE_DOCKER=1 bash ci/ai-blaise/regional-placement-live-smoke.sh`
- Evidence markers: `regional_placement_live=passed`,
  `locality_prefixed_pk_valid=true`, `citus_distribution_present=true`,
  `automatic_rebalance_executed=false`, and `shard_movement_executed=false`

**Citus comparison**: Vanilla Citus can distribute by a chosen column, but it
does not define this overlay locality-prefix admission guard.

**References**:

- In-source: `FEATURE: S8` in `companion/src/regional_placement.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-regional-placement-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-regional-placement-sql-canonical`
- Executable: `REQUIRE_DOCKER=1 bash ci/ai-blaise/regional-placement-live-smoke.sh`

### S12: Tablespaces By Region

**Overlay**: `companion/src/regional_placement.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: PostgreSQL tablespace catalog

**Summary**: Validates explicit region-to-table/tablespace mappings against
live PostgreSQL catalog rows so regional placement policy has a concrete storage
boundary before higher-level reconciliation is enabled.

**Current production-ready boundary**: S12 is production-ready for the bounded
read-only catalog guard only. The live smoke creates real PostgreSQL
tablespaces, creates regional tables in those tablespaces, and the
companion-rendered SQL verifies the mappings through `pg_class` and
`pg_tablespace`. It does not create tablespaces in production clusters,
reconcile Kubernetes or operator state, enforce worker-level shard placement,
move shards, rebalance workers, or prove multi-region failover.

Production evidence:

- In-source: `FEATURE: S12` in `companion/src/regional_placement.rs` and
  `companion/src/advanced_planner.rs`
- Executable: `cargo test -p ai_blaise_citus_companion regional_placement -- --nocapture`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-regional-placement-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-regional-placement-sql-canonical`
- Executable: `REQUIRE_DOCKER=1 bash ci/ai-blaise/regional-placement-live-smoke.sh`
- Evidence markers: `region_tablespace_mappings_valid=true`,
  `region_tablespace_count=2`, `worker_placement_enforced=false`, and
  `multi_region_failover_exercised=false`

**Citus comparison**: Vanilla Citus does not reconcile this overlay
region-to-tablespace intent.

**References**:

- In-source: `FEATURE: S12` in `companion/src/regional_placement.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-regional-placement-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-regional-placement-sql-canonical`
- Executable: `REQUIRE_DOCKER=1 bash ci/ai-blaise/regional-placement-live-smoke.sh`

### Sec7: External Secrets Integration

**Overlay**: `companion/src/ops_contracts.rs`, `operator/src/reconcile/security.rs`, and deployment values
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Records and verifies the External Secrets reference-only security
control for overlay credentials. The production-ready boundary is controller
reconciliation and runtime consumption of referenced Kubernetes Secret material:
inline values are rejected by the operator contract, every runtime Secret ref
requires an ExternalSecret binding, the live kind smoke installs External
Secrets Operator chart `0.10.7`, reconciles fake-provider `ExternalSecret`
objects into real Kubernetes Secrets, and proves the runtime ServiceAccount can
mount those Secrets while `kubectl auth can-i get secrets` remains `no`.

The boundary does not claim cloud provider authentication, cloud KMS policy,
SecretStore credentials, production rotation SLOs, or provider-specific status
condition behavior beyond the fake provider used for deterministic CI.

**Citus comparison**: Vanilla Citus does not prescribe External Secrets refs.

Production evidence: `ci/ai-blaise/security-external-secrets-tls-live-smoke.sh`
boots kind, installs External Secrets Operator, creates
`SecretStore/ai-blaise-cluster-secrets` with the fake provider, reconciles
`ai-blaise-citus-pool-postgres-auth`, `ai-blaise-vector-provider-openai`,
`ai-blaise-citus-pool-tls`, and `ai-blaise-citus-pool-client-tls`
ExternalSecrets into Kubernetes Secrets, checks their `Ready` conditions,
verifies the pool runtime ServiceAccount is denied Secret API reads, records
the vector-provider secret binding evidence for `FEATURE: A9`, and writes
`artifacts/security-external-secrets-tls-live-evidence.tsv`. `gate-close`
depends on this smoke.

**References**:

- In-source: `FEATURE: Sec7` in `companion/src/ops_contracts.rs`
- In-source: `FEATURE: Sec7` in `operator/src/reconcile/security.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-security-supply-chain-canonical`
- CI: `ci/ai-blaise/security-enforcement-smoke.sh`
- CI: `ci/ai-blaise/security-supply-chain-smoke.sh`
- CI: `ci/ai-blaise/security-external-secrets-tls-live-smoke.sh`

### Sec8: TLS Everywhere

**Overlay**: `companion/src/ops_contracts.rs`, `operator/src/reconcile/security.rs`, and deployment values
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Tracks and verifies TLS expectations for clients, Postgres
backends, and sidecar-to-sidecar traffic. The production-ready boundary is the
operator security contract plus live Kubernetes mTLS enforcement: TLS 1.3 is
required, client certificates are required, TLS material must be Secret-backed,
weak TLS versions fail closed, and the live kind smoke mounts reconciled TLS
Secrets into pods and proves a TLS 1.3 client-cert connection succeeds while
no-client-cert and TLS 1.2 clients fail.

The boundary does not claim cloud certificate issuance, cert-manager
integration, automatic rotation, production CA hierarchy, service-mesh policy,
or every application protocol path. Those require separate provider and traffic
evidence.

**Citus comparison**: Vanilla Citus does not enforce this full overlay TLS
contract.

Production evidence: `ci/ai-blaise/security-external-secrets-tls-live-smoke.sh`
generates a short-lived CA plus server and client certificates, stores them in
External Secrets fake-provider data, reconciles `kubernetes.io/tls` Secrets,
mounts the server Secret into an in-cluster TLS server, mounts the client Secret
into client Jobs, verifies TLS 1.3 mTLS success, verifies a client without a
certificate fails, verifies a TLS 1.2 client fails, and records the evidence TSV.
The existing `security-enforcement-smoke.sh` and
`security-supply-chain-smoke.sh` remain the fast contract checks for TLS 1.3,
client-cert requirement, Secret references, no Secret RBAC, and weak-TLS
fail-closed behavior. `gate-close` depends on the live smoke.

**References**:

- In-source: `FEATURE: Sec8` in `companion/src/ops_contracts.rs`
- In-source: `FEATURE: Sec8` in `operator/src/reconcile/security.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-security-supply-chain-canonical`
- CI: `ci/ai-blaise/security-enforcement-smoke.sh`
- CI: `ci/ai-blaise/security-supply-chain-smoke.sh`
- CI: `ci/ai-blaise/security-external-secrets-tls-live-smoke.sh`

### Sec9: SBOM And Cosign Attestation

**Overlay**: `companion/src/ops_contracts.rs` and release gates
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Captures the release-attestation requirement for SBOM and cosign
metadata.

Production evidence: VM proof run
`bash ci/ai-blaise/security-sbom-cosign-live-smoke.sh` starts a local OCI
registry, pushes the digest-pinned
`ai-blaise-citus-timescale-cohabitation:local` image, generates a real SPDX
2.3 SBOM with Syft, signs the image digest with Cosign, verifies the signature
by public key and annotation, attaches and verifies both SPDX and SLSA
provenance attestations with Cosign, and signs/verifies the SBOM blob with a
`.sigstore.json` bundle. The older
`bash ci/ai-blaise/security-supply-chain-smoke.sh` remains the fast contract
proof for fail-closed mutable-image, malformed-SBOM, and metadata-shape
validation. The production-ready claim covers release artifact
SBOM/signature/attestation generation and verification in a registry-backed
flow; Kubernetes admission-policy enforcement, public release registry
publication, and keyless transparency-log policy remain outside this feature
boundary.

**Citus comparison**: Vanilla Citus does not require ai-blaise release
attestations.

**References**:

- In-source: `FEATURE: Sec9` in `companion/src/ops_contracts.rs`
- In-source: `FEATURE: Sec9` in `operator/src/reconcile/security.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-multiregion-contracts-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-security-supply-chain-canonical`
- CI: `ci/ai-blaise/operator-multiregion-contracts-smoke.sh`
- CI: `ci/ai-blaise/security-supply-chain-smoke.sh`
- CI: `ci/ai-blaise/security-sbom-cosign-live-smoke.sh`

### Sto2: file_attachment Domain Type

**Overlay**: `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql` and `companion/src/advanced_planner.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Installs the `storage.file_attachment` SQL domain, constructor,
accessors, URI helper, and `storage.file_attachment_refs` metadata table for
tenant/owner-scoped file reference records.

**Current boundary**: Production-ready for the SQL domain and metadata runtime
only. The domain validates JSON object shape, bucket names, object keys,
content types, bounded non-negative `size_bytes`, lowercase 64-hex SHA-256,
and optional object metadata; the refs table persists tenant/owner metadata
with useful lookup indexes. Object storage upload/download, retention
automation, malware scanning, and authorization remain separately scoped.

Production evidence: `ci/ai-blaise/sql-extension-smoke.sh` installs
`storage.file_attachment`, exercises valid constructor/accessors/URI/table
persistence, and verifies invalid bucket, path traversal, malformed SHA-256,
and negative `size_bytes` fail closed against real PostgreSQL containers.

**Citus comparison**: Vanilla Citus does not include a storage domain type.

**References**:

- In-source: `FEATURE: Sto2` in `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql`
- In-source: `FEATURE: Sto2` in `companion/src/advanced_planner.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`
- Executable: `REQUIRE_DOCKER=1 SQL_EXTENSION_SMOKE_PG_MAJORS=17 bash ci/ai-blaise/sql-extension-smoke.sh`

### T4: Hash-Table Planner Hot Path

**Overlay**: `companion/src/advanced_planner.rs`,
`companion/src/router_assist.rs`,
`patches/0004-hashtable-on-planner-hotpath.patch`,
`benchmarks/router-planner/`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: none

**Summary**: Adds an upstream-targetable Citus patch that replaces the
router-planner placement-intersection nested loop with a hashed endpoint lookup
for non-tiny placement lists while preserving legacy result semantics.

**Citus comparison**: Vanilla Citus still uses the linear placement-list
intersection on this planner path.

Production evidence: `./configure PG_CONFIG=$(command -v pg_config)
--without-pg-version-check && make -j2 V=0` builds the integrated Citus source
with `FEATURE: T4` in `src/backend/distributed/planner/multi_router_planner.c`.
`ci/ai-blaise/router-patch-smoke.sh` verifies patch applicability against
upstream `release-14.0`, source integration of `IntersectPlacementListHashed`,
and a 30-sample VM router-planner benchmark with measured p95 output in
`benchmarks/citus-patches/results/0004-router-planner-hotpath.json`. The bounded
production claim is the source-integrated placement-intersection algorithm,
legacy semantic preservation, and measured local planner-hot-path microbench. It
does not claim fleet-wide multi-worker planner CPU or release-performance
latency until those release harnesses run against a production-sized cluster.

**References**:

- In-source: `FEATURE: T4` in `companion/src/advanced_planner.rs`
- In-source: `FEATURE: T4` in `companion/src/router_assist.rs`
- Patch: `patches/0004-hashtable-on-planner-hotpath.patch`
- Benchmark smoke: `benchmarks/router-planner/bench.py`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-runtime-canonical`
- CI: `ci/ai-blaise/router-patch-smoke.sh`
- CI: `ci/ai-blaise/companion-advanced-planner-smoke.sh`

### T6: PG18 io_uring Default

**Overlay**: `companion/src/ops_contracts.rs`, `images/citus-pg-overlay/Dockerfile`, `ci/ai-blaise/sql-extension-smoke.sh`, and Helm values
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Tracks the Postgres I/O method policy for the PG18 io_method
contract, paired with the PG version matrix in the overlay image and smoke
harness, and proves a real-kernel `io_method=io_uring` runtime against a
live postgres:18-bookworm container with PGDG PG18 bundled extensions.

**Current boundary**: The T6 production-ready claim is bounded to the
runtime io_method=io_uring policy and the PG18 PGDG bundled-extension subset
that has PGDG packages (vector, pg_cron, pgaudit, postgis, pg_uuidv7, age,
plus core pgcrypto, pg_trgm, citext, pg_walinspect). It does not claim PG18
source-built extension parity with Bundle1 PG17 (citus, pgsodium, topn,
pg_jsonschema, pg_graphql, pg_search, plv8, pg_warm remain PG17-only in the
Bundle1 image until their PG18 source-build paths are verified), nor full
PG18 production Citus distributed plane.

Production evidence: `REQUIRE_DOCKER=1 bash ci/ai-blaise/t6-pg18-io-uring-live-smoke.sh`
boots a `postgres:18-bookworm` container with `-c io_method=io_uring`,
verifies `SHOW io_method` returns `io_uring`, installs the available PG18
PGDG bundled extensions, creates them in best-effort order, runs a 10000-row
workload, and captures `pg_stat_io` reads/writes that confirm io_uring
backend IO is actually occurring at runtime. The evidence row is appended to
`artifacts/t6-pg18-io-uring-evidence.tsv` with the host kernel version,
io_method GUC, extensions-created count, workload row count, and pg_stat_io
reads/writes.

**Citus comparison**: Vanilla Citus does not set ai-blaise PG18 I/O policy or
emit a multi-PG-major operand image from a single overlay contract.

**References**:

- In-source: `FEATURE: T6` in `companion/src/ops_contracts.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-multiregion-contracts-canonical`
- CI: `ci/ai-blaise/operator-multiregion-contracts-smoke.sh`
- Executable: `make -f Makefile.ai-blaise sql-extension-smoke` (runs PG17 and PG18 matrix)
- Executable: `make -f Makefile.ai-blaise build-image-matrix` (builds PG17 and PG18 overlay images)

### T7: Pipelined Client Protocol In Pool

**Overlay**: `pool/src/runtime.rs`, `pool/src/proxy.rs`, `pool/src/main.rs`,
and `companion/src/ops_contracts.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: none

**Summary**: Proves the pool `serve` data plane preserves pipelined
PostgreSQL simple-query frames as a byte-transparent TCP proxy while keeping
extended-query batching and shard-aware routing out of the production-ready
boundary.

Production evidence: `ci/ai-blaise/pool-proxy-smoke.sh` runs the real pool
against a `postgres:17` container, opens a raw PostgreSQL client through the
pool data port, sends two simple-query frames without waiting for the first
result, verifies ordered `pipeline_one` and `pipeline_two` rows from the real
backend, and keeps the live SQL plus pool admin metrics checks. The same
Docker-backed smoke also proves active-connection overload rejection, tenant
quota fail-closed denial, and upstream-unreachable fail-closed routing on the
real data port. Extended-query `Parse`/`Bind`/`Execute` buffering,
transaction-aware batching, and shard-aware routing remain alpha contract
surfaces until they have equivalent raw-wire data-plane evidence.

**Citus comparison**: Vanilla Citus does not ship the ai-blaise pool pipeline.

**References**:

- In-source: `FEATURE: T7` in `pool/src/runtime.rs`
- In-source: `FEATURE: T7` in `pool/src/pipeline.rs`
- In-source: `FEATURE: T7` in `pool/src/proxy.rs`
- In-source: `FEATURE: T7` in `pool/src/main.rs`
- In-source: `FEATURE: T7` in `companion/src/ops_contracts.rs`
- Executable: `cargo run -p ai_blaise_citus_pool -- run-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical`
- Executable: `cargo run -p ai_blaise_citus_operator -- run-multiregion-contracts-canonical`
- CI: `ci/ai-blaise/operator-multiregion-contracts-smoke.sh`
- CI: `ci/ai-blaise/pool-proxy-smoke.sh`


### T10: Bulk Protocol Fetch Path

**Overlay**: `companion/src/advanced_planner.rs`, `companion/src/bulk_distsql.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: Citus distributed table execution

**Summary**: Verifies a bounded bulk-fetch SQL path over a live Citus
distributed table with the canonical 4096-row batch budget.

**Current production-ready boundary**: T10 is production-ready for the
companion-rendered SQL cursor fetch budget only. `BulkDistSqlPlan` renders a
`NO SCROLL` cursor and `FETCH 4096`, and
`ci/ai-blaise/bulk-distsql-live-smoke.sh` proves a live Citus distributed-table
query returns exactly `bulk_fetch_rows_returned=4096` ordered rows. It does not
implement a custom PostgreSQL wire-protocol fetch layer, adaptive backpressure,
client flow control, cross-worker streaming fanout, or Kubernetes traffic.

Production evidence:

- In-source: `FEATURE: T10` in `companion/src/advanced_planner.rs` and
  `companion/src/bulk_distsql.rs`
- Executable: `cargo test -p ai_blaise_citus_companion bulk_distsql -- --nocapture`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-bulk-distsql-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-bulk-distsql-sql-canonical`
- Executable: `REQUIRE_DOCKER=1 bash ci/ai-blaise/bulk-distsql-live-smoke.sh`
- Evidence markers: `bulk_distsql_live=passed`,
  `bulk_fetch_rows_requested=4096`, `bulk_fetch_rows_returned=4096`,
  `wire_protocol_implementation=false`, and
  `backpressure_scheduler_exercised=false`

**Citus comparison**: Citus executes cursor-backed distributed queries for
supported SQL; this overlay adds a deterministic batch-budget guard around the
bounded 4096-row fetch path.

**References**:

- In-source: `FEATURE: T10` in `companion/src/advanced_planner.rs`
- In-source: `FEATURE: T10` in `companion/src/bulk_distsql.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-bulk-distsql-canonical`
- CI: `ci/ai-blaise/companion-advanced-planner-smoke.sh`
- CI: `ci/ai-blaise/bulk-distsql-live-smoke.sh`


### T11: DistSQL Physical Pushdown

**Overlay**: `companion/src/advanced_planner.rs`, `companion/src/bulk_distsql.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: Citus distributed table execution

**Summary**: Verifies bounded Citus physical-pushdown evidence for a distributed
aggregate/filter query and enforces the canonical worker-task budget.

**Current production-ready boundary**: T11 is production-ready for the bounded
live Citus EXPLAIN guard only. `BulkDistSqlPlan` renders an `EXPLAIN (COSTS
OFF)` aggregate over a distributed table, and
`ci/ai-blaise/bulk-distsql-live-smoke.sh` proves the real plan contains
`Custom Scan (Citus Adaptive)`, observes `citus_task_count_observed=1`, and
keeps that inside `worker_task_budget=16`. It does not implement a physical plan
rewrite engine, worker-plan injection, multi-worker fanout, custom executor
nodes, or Kubernetes traffic.

Production evidence:

- In-source: `FEATURE: T11` in `companion/src/advanced_planner.rs` and
  `companion/src/bulk_distsql.rs`
- Executable: `cargo test -p ai_blaise_citus_companion bulk_distsql -- --nocapture`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-bulk-distsql-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-bulk-distsql-sql-canonical`
- Executable: `REQUIRE_DOCKER=1 bash ci/ai-blaise/bulk-distsql-live-smoke.sh`
- Evidence markers: `distsql_physical_pushdown_explain=true`,
  `citus_adaptive_plan_observed=true`, `citus_task_count_observed=1`,
  `worker_task_budget=16`, `worker_task_budget_exceeded=false`,
  `physical_plan_rewrite_exercised=false`, and
  `multi_worker_fanout_exercised=false`

**Citus comparison**: Citus already pushes supported distributed SQL into worker
tasks; this overlay adds a deterministic guard that the bounded DistSQL contract
maps to real Citus adaptive-plan evidence without overclaiming a new optimizer.

**References**:

- In-source: `FEATURE: T11` in `companion/src/advanced_planner.rs`
- In-source: `FEATURE: T11` in `companion/src/bulk_distsql.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-bulk-distsql-canonical`
- CI: `ci/ai-blaise/companion-advanced-planner-smoke.sh`
- CI: `ci/ai-blaise/bulk-distsql-live-smoke.sh`

### T13: Distributed Cursors

**Overlay**: `companion/src/transaction_state.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: Citus distributed table execution

**Summary**: Verifies bounded cursor lifecycle behavior over a live Citus
distributed table, including ordered batched fetches and Citus adaptive-plan
evidence for the table being queried.

**Current production-ready boundary**: T13 is production-ready for the bounded
single-node Citus distributed-table cursor smoke only. The companion-rendered
SQL declares a `NO SCROLL` cursor, fetches the result in two batches, verifies
all five ordered rows are returned, and records that EXPLAIN uses `Custom Scan
(Citus Adaptive)` with `Task Count: 1`. It does not implement a PostgreSQL wire
protocol portal layer, multi-worker cursor cleanup, cursor failover, cursor
holdability across transactions, or coordinator restart recovery.

Production evidence:

- In-source: `FEATURE: T13` in `companion/src/transaction_state.rs` and
  `companion/src/advanced_planner.rs`
- Executable: `cargo test -p ai_blaise_citus_companion transaction_state -- --nocapture`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-transaction-state-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-transaction-state-sql-canonical`
- Executable: `REQUIRE_DOCKER=1 bash ci/ai-blaise/transaction-state-live-smoke.sh`
- Evidence markers: `transaction_state_live=passed`,
  `distributed_cursor_declared=true`, `cursor_fetch_batches=2`,
  `cursor_rows_fetched=5`, `citus_adaptive_plan_observed=true`, and
  `wire_protocol_portal_exercised=false`

**Citus comparison**: Citus supports cursor behavior for supported distributed
queries; this overlay adds a deterministic live smoke and state-budget evidence
for the bounded distributed-table path.

**References**:

- In-source: `FEATURE: T13` in `companion/src/transaction_state.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-transaction-state-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-transaction-state-sql-canonical`
- Executable: `REQUIRE_DOCKER=1 bash ci/ai-blaise/transaction-state-live-smoke.sh`

### T14: Distributed Savepoints

**Overlay**: `companion/src/transaction_state.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: Citus distributed table execution

**Summary**: Verifies bounded savepoint rollback behavior inside a live Citus
distributed-table transaction, paired with cursor continuity after rollback.

**Current production-ready boundary**: T14 is production-ready for the bounded
single-node Citus distributed-table savepoint smoke only. The
companion-rendered SQL creates a savepoint, inserts a sentinel row, observes
`count_after_insert=6`, rolls back to the savepoint, verifies
`count_after_rollback=5` and `final_count=5`, and continues fetching from the
open cursor. It does not prove multi-worker rollback cleanup, prepared
transaction recovery, distributed deadlock handling, coordinator failover, or
Kubernetes transaction-drain behavior.

Production evidence:

- In-source: `FEATURE: T14` in `companion/src/transaction_state.rs` and
  `companion/src/advanced_planner.rs`
- Executable: `cargo test -p ai_blaise_citus_companion transaction_state -- --nocapture`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-transaction-state-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-transaction-state-sql-canonical`
- Executable: `REQUIRE_DOCKER=1 bash ci/ai-blaise/transaction-state-live-smoke.sh`
- Evidence markers: `savepoint_rollback_verified=true`,
  `count_after_insert=6`, `count_after_rollback=5`, `final_count=5`,
  `coordinator_failover_exercised=false`, and
  `multi_worker_cleanup_exercised=false`

**Citus comparison**: Citus supports some transaction semantics; this overlay
adds a deterministic live savepoint rollback smoke and state-budget evidence for
the bounded distributed-table path.

**References**:

- In-source: `FEATURE: T14` in `companion/src/transaction_state.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-transaction-state-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-transaction-state-sql-canonical`
- Executable: `REQUIRE_DOCKER=1 bash ci/ai-blaise/transaction-state-live-smoke.sh`


### TS10: Hierarchical CAGGs Distributed

**Overlay**: `companion/src/advanced_planner.rs`, `companion/src/timescale_advanced.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: partial
**Bundled extension dep**: `timescaledb`, Citus distributed table execution

**Summary**: Verifies a bounded two-level Timescale continuous aggregate
hierarchy over a live Citus-distributed hypertable.

**Current production-ready boundary**: TS10 is production-ready for the bounded
live Citus+Timescale hierarchy guard only. `TimescaleAdvancedPlan` renders a
source hourly CAGG, a target daily CAGG over that source, and explicit
`refresh_continuous_aggregate` calls. `ci/ai-blaise/timescale-advanced-live-smoke.sh`
proves `hierarchical_cagg_count=2` and `hierarchical_cagg_daily_rows=4` against
the real cohabitation image. It does not claim automated refresh scheduling,
continuous aggregate invalidation tuning, multi-worker fanout, production
retention/compression lifecycle management, or Kubernetes traffic.

Production evidence:

- In-source: `FEATURE: TS10` in `companion/src/advanced_planner.rs` and
  `companion/src/timescale_advanced.rs`
- Executable: `cargo test -p ai_blaise_citus_companion timescale_advanced -- --nocapture`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-timescale-advanced-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-timescale-advanced-sql-canonical`
- Executable: `REQUIRE_DOCKER=1 bash ci/ai-blaise/timescale-advanced-live-smoke.sh`
- Evidence markers: `timescale_advanced_live=passed`,
  `hierarchical_cagg_count=2`, `hierarchical_cagg_daily_rows=4`,
  `multi_worker_fanout_exercised=false`, and `kubernetes_traffic_exercised=false`

**Citus comparison**: Citus can cohabit with TimescaleDB in this fork; this
overlay adds a deterministic live guard that the bounded hierarchical CAGG
shape works against the cohabitation image.

**References**:

- In-source: `FEATURE: TS10` in `companion/src/advanced_planner.rs`
- In-source: `FEATURE: TS10` in `companion/src/timescale_advanced.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-timescale-advanced-canonical`
- CI: `ci/ai-blaise/timescale-cohabitation-smoke.sh`
- CI: `ci/ai-blaise/timescale-advanced-live-smoke.sh`


### TS11: Bloom Filters On segmentby

**Overlay**: `companion/src/advanced_planner.rs`, `companion/src/timescale_advanced.rs`
**Status**: production-ready
**Since**: unreleased
**Upstream Citus equivalent**: none
**Bundled extension dep**: `timescaledb`, Citus distributed table execution

**Summary**: Materializes bounded companion SQL bloom-filter rows for Timescale
compression `segmentby` keys over a live Citus-distributed hypertable.

**Current production-ready boundary**: TS11 is production-ready for the bounded
companion SQL bloom-materialization guard only. The live smoke sets
`timescaledb.compress_segmentby='tenant_id,device_id'`, proves
`compression_segmentby_columns=2` with `compression_segmentby_detail=tenant_id,device_id`,
and materializes `segmentby_bloom_rows=16` rows into
`public.ts11_segmentby_bloom_filters` with `segmentby_bloom_bit_count=2048` and
`segmentby_bloom_hash_count=3`. It does not claim native Timescale bloom
filters, planner integration, compressed-chunk scan pruning, multi-worker
fanout, false-positive-rate calibration, or Kubernetes traffic.

Production evidence:

- In-source: `FEATURE: TS11` in `companion/src/advanced_planner.rs` and
  `companion/src/timescale_advanced.rs`
- Executable: `cargo test -p ai_blaise_citus_companion timescale_advanced -- --nocapture`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-timescale-advanced-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-timescale-advanced-sql-canonical`
- Executable: `REQUIRE_DOCKER=1 bash ci/ai-blaise/timescale-advanced-live-smoke.sh`
- Evidence markers: `compression_segmentby_columns=2`,
  `segmentby_bloom_rows=16`, `segmentby_bloom_bit_count=2048`,
  `segmentby_bloom_hash_count=3`, `native_timescale_bloom_filter=false`,
  `planner_integration_exercised=false`, and
  `compressed_chunk_scan_pruning_exercised=false`

**Citus comparison**: Vanilla Citus does not define segmentby bloom materialized
metadata; this overlay adds a deterministic companion SQL guard without claiming
native Timescale or planner-level bloom integration.

**References**:

- In-source: `FEATURE: TS11` in `companion/src/advanced_planner.rs`
- In-source: `FEATURE: TS11` in `companion/src/timescale_advanced.rs`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical`
- Executable: `cargo run -p ai_blaise_citus_companion --bin companion_contracts -- run-timescale-advanced-canonical`
- CI: `ci/ai-blaise/timescale-cohabitation-smoke.sh`
- CI: `ci/ai-blaise/timescale-advanced-live-smoke.sh`


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
