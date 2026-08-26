# Upstream Sync

The fork checks patch applicability against `citusdata/citus` every 14 days and
on every V2 acceptance run. The default upstream target is `release-14.0`, which
matches the V2 upstream-merge gate; override it with `UPSTREAM_REF` only for
explicit backport or forward-port drills.

The sync job intentionally starts as a dry-run gate. Once the first overlay
release stabilizes, it should be extended to open a PR on
`chore/upstream-sync-YYYY-MM-DD` with:

Status snapshot: 2026-08-26. The patch and PR tables below are a dated planning
snapshot, not live PR evidence. Refresh this date before a release and whenever
an upstream or fork PR opens, closes, merges, or changes target branch. The
2026-05-26 refresh folds in five upstream `citusdata/citus` `main` commits on
top of the 2026-05-20 base (`4d54b11bbab52f71b76c316432e878a1bc38206c` ->
`dee8ec140aff84d8769bdcd859c39c379180fe06`): #8587 object-ownership
enforcement, #8497 NOT (x IS DISTINCT FROM y) recursive-planning fix, #8498
COLLATE-with-type-cast distributed-query fix, #8593 SortList short-circuit,
#8592 README grammar; the live integration smoke is
`ci/ai-blaise/upstream-rebase-2026-05-25-integration-smoke.sh`. The 2026-06-19 refresh then integrates the three upstream `main` commits that landed after `dee8ec140...` (`dee8ec140aff84d8769bdcd859c39c379180fe06` -> `efa65fc4d580...`): #5195 batched adaptive executor (re-entrant `AdaptiveExecutorStart`/`Run`/`End`, `citus.executor_batch_size` + `citus.executor_chunk_size` GUCs, PG17+ chunked libpq rows, SCROLL-cursor Material wrap), #8594 `GetLatestSnapshot()` in `CleanupRecordExists` (closes the cleanup-worker double-drop race), and #8596 mitmproxy fork bump (pyOpenSSL CVE, test-dep). The three compose with the integrated fork patches 0001-0008: #5195's `distributed_planner.c`/`FinalizePlan` changes are disjoint from the fast-path-router patches 0004/0006 (which touch only `multi_router_planner.c`), its new GUCs register at a different anchor than the patch-0006 GUC and stay `GUC_STANDARD` (not router state, so patch-0003 client GUC reporting is intentionally not extended to them), and `adaptive_executor.c`/`citus_custom_scan.c` carry no fork modifications. Verified by full Citus C build + a batched distributed-query runtime smoke + `router-patch-smoke` (skip-coordinator round-trips still 0) + `pg-cron-cohabitation-smoke` (clock patch 0007). The 2026-08-26 refresh merges the 23 upstream `main` commits `efa65fc4d..008b391a75` as a true-ancestry merge (fork commit for the sync branch: `2c83ddbc4`), closing the whole 31-commit gap behind `citusdata/citus` when combined with the two earlier folds. Behavior commits: #8692 constant-false router-modify fix, #8625 `citus.allow_unsafe_insert_select_pushdown`, #8642 `citus.enable_or_clause_arm_pruning`, #8566 PLpgSQL-plugin 2PC-skip redesign, #8621/#8686/#8677 `citus_internal.distribute_object` + upgrade-path/version persistence, #8679 lock-assertion fix, #8676/#8651/#8638 orphan-cleanup and clone-node fixes; the rest are build/test/CI infra (GCC-15, cassert nightlies, Dependabot, citus-backport skill). One conflict (`citus--14.0-1--15.0-1.sql`) resolved as ordered union with the fork's five FEATURE UDF includes trailing. The pre-merge patch series 0001-0008 forward-applies cleanly to `008b391a75` and `patches-check` passes against the merged tree unchanged, so no patch rebase was needed. All four new upstream GUCs are `PGC_USERSET`, so patch 0003's blanket `GUC_REPORT` loop covers them; `pool/README.md` lists them as tracked-GUC candidates. `UPSTREAM_REBASE_BASE` now pins `008b391a75fc46464450a6ff4b5d34132e63f410` (capturedAt 2026-08-26).

- upstream Citus changes
- refreshed patch applicability output
- any patch rebases needed to keep `patches/series` clean

## Pending Citus upstream PRs (`citusdata/citus`)

These quilt patches are upstream-PR candidates -- small, single-purpose, and
designed to apply cleanly against `citusdata/citus` `release-14.0`. Each entry
records the patch path, the matched upstream-PR URL (`pending submission` if no
PR has opened yet), maintainer status, and the gate that must turn green before
the PR opens.

| # | Patch | Upstream PR | Maintainer status | Our fork PR | Gating notes |
|---|---|---|---|---|---|
| 0001 | `patches/0001-allow-trusted-hook-coextensions.patch` | pending submission | not submitted | landed in fork PR1 | gates on `make -f Makefile.ai-blaise patches-check` plus the `timescale-cohabitation-smoke` run that exercises a real Citus+TimescaleDB cohabit boot |
| 0002 | `patches/0002-preserve-trusted-hook-chain-state.patch` | pending submission | not submitted | landed in fork PR1 | gates on `timescale-cohabitation-smoke` covering planner, executor, and explain hook chains under a trusted coextension |
| 0003 | `patches/0003-guc-report-citus-userset.patch` | pending submission | draft prepared (one-place diff to `OverridePostgresConfigProperties()`) | landed in fork PR50 | gates on `patches-check` plus a `kind-production-smoke` run that demonstrates a pooler receiving `ParameterStatus` for `citus.enable_router_execution` |
| 0004 | `patches/0004-hashtable-on-planner-hotpath.patch` | pending submission | fork artifact and measured VM gate ready; upstream PR after maintainer-focused benchmark packaging | landed in fork | gates on `patches-check`, `router-patch-smoke`, `citus-patch-production-audit`, and measured `benchmarks/citus-patches/results/0004-router-planner-hotpath.json` |
| 0005 | `patches/0005-placement-generation-counter.patch` | pending submission | draft prepared | landed in fork PR50 | gates on companion-side subscriber tests (`cargo test -p ai_blaise_citus_companion --lib router_assist`) plus a real Citus build that exercises the counter through a rebalance |
| 0006 | `patches/0006-fast-path-router-no-coord-rt.patch` | pending submission | fork artifact and measured live SQL gate ready; upstream PR after coord-less pool packaging | landed in fork | gates on `patches-check`, companion router-assist tests, `router-patch-smoke`, `citus-patch-production-audit`, and measured `benchmarks/citus-patches/results/0006-fast-path-router-skip.json` |
| 0007 | `patches/0007-citus-clock-cohabit-pg-cron.patch` | pending submission | draft prepared; live pg_cron boot evidence and measured patch gate exist | landed in fork | gates on `patches-check`, `citus-patch-production-audit`, and measured `benchmarks/citus-patches/results/0007-pg-cron-cohabit.json` from a live Citus + `pg_cron` boot with zero registration conflicts |
| 0008 | `patches/0008-cohabit-extensions-detection-api.patch` | pending submission | draft prepared; SQL-visible C API live proof and measured patch gate exist | landed in fork | gates on companion-side `cohabit-detection-smoke`, `citus-patch-production-audit`, and measured `benchmarks/citus-patches/results/0008-detection-matrix.json` covering TimescaleDB, `pg_cron`, and `pg_partman` |
| 0009 | distSQL physical plan distribution | pending submission | in flight (large scope; will require multiple sub-PRs) | not yet landed in fork | gates on `companion-advanced-planner` canonical row plus an end-to-end physical-plan distribution smoke |
| 0010 | distributed cursors | pending submission | in flight | not yet landed in fork | gates on a cursor-correctness smoke that fetches across coordinator failover |
| 0011 | distributed savepoints | pending submission | in flight | not yet landed in fork | gates on a savepoint-correctness smoke covering nested-transaction rollback across shards |

Patches 0001-0002 are landed-in-fork but not yet submitted upstream because the
trusted-coextension contract is a deployment-layer feature that Citus
maintainers have historically declined to absorb. Patches 0003 and 0005 are
the next two candidates we expect to submit; their draft mailbox diffs are
already mailbox-header-clean and have on-disk `patches/series` entries.

Patches 0004, 0006, 0007, and 0008 now have fork patch artifacts,
`patches/series` entries, and measured result JSON under
`benchmarks/citus-patches/results/`. The audit rejects stale roster-only status
for landed artifacts, skipped/scaffold results, docs that overstate maturity,
and any production-ready claim without measured evidence. Patches 0009-0011
remain roster entries with tracked designs but no landed fork patch artifact yet.

Each roster entry will land in the fork first (per the cadence below) and then
be submitted upstream after the runtime gate flips from `alpha` to
`production-ready` in `docs/ai-blaise/NEW_FEATURES.md`.

### Citus upstream submission cadence

The submission cadence is: open the upstream PR only after the patch passes
both ai-blaise CI and `kind-production-smoke`, and only after the corresponding
`NEW_FEATURES.md` entry has flipped from `alpha` to `production-ready`. The
two gating runs are recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
under the same FEATURE id.

### Per-patch details

#### 0003-guc-report-citus-userset.patch -- citus.* USERSET GUCs report to clients

- **Target branch**: `release-14.0` (and `main` once accepted)
- **Type**: behavior fix
- **Rationale**: Vanilla Citus does not tag its USERSET GUCs with `GUC_REPORT`,
  which means a session pooler in front of Citus cannot observe planner-
  affecting `SET` commands via ParameterStatus packets. Transaction pooling
  therefore inherits stale router/execution settings across multiplexed
  sessions. The patch extends the existing `OverridePostgresConfigProperties()`
  loop -- one place, no new functions -- and is independent of any ai-blaise
  feature.
- **Gate before upstream PR**: `make -f Makefile.ai-blaise patches-check` plus
  a kind-smoke run that demonstrates a pooler receiving ParameterStatus for
  `citus.enable_router_execution`.

#### 0005-placement-generation-counter.patch -- pg_dist_placement generation counter

- **Target branch**: `release-14.0`
- **Type**: new internal API
- **Rationale**: Adds a process-local monotonic counter that the
  pg_dist_placement trigger and `CitusInvalidateRelcacheByShardId` advance, plus
  a `pg_catalog.citus_placement_generation()` SQL UDF for poolers to read.
  Enables partial plan-cache invalidation across rebalances instead of dropping
  every cached plan.
- **Gate before upstream PR**: companion-side subscriber tests
  (`cargo test -p ai_blaise_citus_companion --lib router_assist`) plus a real
  Citus build that exercises the counter through a rebalance.

#### 0007-citus-clock-cohabit-pg-cron.patch -- pg_cron clock cohabitation

- **Target branch**: `release-14.0`
- **Type**: cohabitation safety hook
- **Rationale**: Records an explicit logical-clock reservation when operators
  configure `pg_cron` as a clock-side cohabitant. The patch does not grant
  pg_cron trusted hook-chain status; it only reserves and exposes the clock flag
  needed by pg_cron scheduled-job callers.
- **Gate before upstream PR**: `make -f Makefile.ai-blaise patches-check` plus
  `REQUIRE_DOCKER=1 bash ci/ai-blaise/pg-cron-cohabitation-smoke.sh`, which
  records the patched image, verifies `citus_cohabit_clock_tick_reserved()`,
  and waits for a scheduled pg_cron worker to call Citus clock UDFs.

#### 0008-cohabit-extensions-detection-api.patch -- role-aware cohabit detection

- **Target branch**: `release-14.0`
- **Type**: internal API
- **Rationale**: Exposes a role-aware classifier for supported cohabitants so
  the fork can distinguish trusted hook-chain extensions from clock-side and
  partition-management neighbors. The first roles are TimescaleDB, pg_cron, and
  pg_partman.
- **Gate before upstream PR**: `make -f Makefile.ai-blaise patches-check`,
  `cargo test -p ai_blaise_citus_companion`, and
  `ci/ai-blaise/cohabit-detection-smoke.sh`, plus
  `REQUIRE_DOCKER=1 bash ci/ai-blaise/pg-cron-cohabitation-smoke.sh` for the
  SQL-visible C API proof before submission upstream.

## Pending pgsql-hackers PRs (`postgres/postgres`, via pgEdge/Spock contribution path)

These patches target PostgreSQL core, not Citus. They are applied at the
PG-build layer (see `images/citus-pg-overlay/Dockerfile` for the operand
contract label and `images/citus-pg-overlay/Dockerfile.pgcore-patches` for the
VM-verified patched PostgreSQL + Citus runtime) before Citus is compiled. They
live in their own series file (`patches/postgres/series`) so the citus quilt
and the postgres-core quilt rebase independently.

The upstream contributor in both cases is pgEdge via the Spock multi-master
replication extension. The diffs shipped here are the canonical pgEdge/Spock
contributions to pgsql-hackers, rebased to PostgreSQL 17 by Spock upstream and
re-wrapped here with mailbox headers and `FEATURE:` markers. The pgsql-hackers
threads owned by Spock are the canonical submission record; the fork tracks
them by the Spock patch identifier in the upstream column below.

| # | Patch | Spock upstream | pgsql-hackers thread | Status | Our fork PR |
|---|---|---|---|---|---|
| PG-0001 | `patches/postgres/0001-logical-commit-clock.patch` | pgEdge/spock `patches/17/pg17-025-logical_commit_clock.diff` | tracked via Spock's `XLogReserveInsertHook` proposal trail on pgsql-hackers (search `XLogReserveInsertHook logical commit clock`) | landed in fork PR48; runtime gate live: patched PostgreSQL 17 builds, starts, and runs Citus plus probe smoke | PR48 |
| PG-0002 | `patches/postgres/0002-per-subtrans-commit-ts.patch` | pgEdge/spock `patches/17/pg17-030-per-subtrans-commit-ts.diff` | tracked via Spock's `SubTransactionCommitTsEntry` proposal trail on pgsql-hackers | landed in fork PR48; runtime gate live: commit-ts override and WAL SUBTRANS_TS proof smoke | PR48 |

Both PG-core diffs now have bounded runtime evidence from
`REQUIRE_DOCKER=1 bash ci/ai-blaise/postgres-core-patches-live-smoke.sh`. The
smoke builds PostgreSQL `REL_17_10` from source, applies
`patches/postgres/series`, builds Citus against the patched `pg_config`, starts
that runtime, and verifies PGC1/PGC2 behavior through the smoke-only
`ai_blaise_pgc_probe` extension. The gate is intentionally PG17-only; pgactive
or Spock apply traffic, multi-node active-active conflict replay, PG18, and the
full Bundle1 operand image remain outside this evidence boundary.

References:

- pgEdge/spock repo: <https://github.com/pgEdge/spock> (PostgreSQL License,
  compatible with the PostgreSQL fork). The Spock patches directory layout is
  `patches/<pg_major>/pg<pg_major>-<NNN>-<feature>.diff`.
- Logical commit clock: pgsql-hackers discussion of monotonic commit
  timestamps via Lamport clock for multi-master, see the Spock contribution
  trail starting at the `XLogReserveInsertHook` proposal. Tracked locally by
  FEATURE: PGC1 in `docs/ai-blaise/NEW_FEATURES.md`.
- Per-subtransaction commit timestamps: pgsql-hackers discussion of
  per-subxid commit-ts overrides for delta-apply origin attribution, see the
  Spock contribution trail starting at the `SubTransactionCommitTsEntry`
  proposal. Tracked locally by FEATURE: PGC2 in
  `docs/ai-blaise/NEW_FEATURES.md`.

Patch refresh procedure when bumping the PostgreSQL major target:

1. `git -C /tmp/spock-investigate/spock pull` (or re-clone)
2. Diff `patches/<old_major>/` against `patches/<new_major>/` to confirm the
   shape of any rebase Spock applied.
3. Re-wrap the new Spock diff with the local mailbox header and `FEATURE:`
   marker (preserve the `Spock:` comment prefix so the upstream trail is
   recoverable).
4. Update `patches/postgres/series` if the filename changed.
5. Bump `PG_MAJOR` in `images/citus-pg-overlay/Dockerfile` and re-run
   `make -f Makefile.ai-blaise patches-check`.

The upstream-sync job tracks only Citus today. Tracking pgEdge/spock in the
same job is a follow-up once `patches/postgres/` carries more than the two
seeded entries.
