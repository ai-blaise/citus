# Upstream Sync

The fork checks patch applicability against `citusdata/citus` every 14 days and
on every V2 acceptance run. The default upstream target is `release-14.0`, which
matches the V2 upstream-merge gate; override it with `UPSTREAM_REF` only for
explicit backport or forward-port drills.

The sync job intentionally starts as a dry-run gate. Once the first overlay
release stabilizes, it should be extended to open a PR on
`chore/upstream-sync-YYYY-MM-DD` with:

Status snapshot: 2026-05-24. The patch and PR tables below are a dated planning
snapshot, not live PR evidence. Refresh this date before a release and whenever
an upstream or fork PR opens, closes, merges, or changes target branch.

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
| 0004 | `patches/0004-hashtable-on-planner-hotpath.patch` | pending submission | fork artifact ready; upstream PR after measured live Citus planner benchmark evidence | landed in fork | gates on `patches-check`, `router-patch-smoke`, `citus-patch-production-audit`, and measured `benchmarks/citus-patches/results/0004-router-planner-hotpath.json` before any production-ready claim |
| 0005 | `patches/0005-placement-generation-counter.patch` | pending submission | draft prepared | landed in fork PR50 | gates on companion-side subscriber tests (`cargo test -p ai_blaise_citus_companion --lib router_assist`) plus a real Citus build that exercises the counter through a rebalance |
| 0006 | `patches/0006-fast-path-router-no-coord-rt.patch` | pending submission | fork artifact ready; upstream PR after coord-less pool live evidence | landed in fork | gates on `patches-check`, companion router-assist tests, `router-patch-smoke`, `citus-patch-production-audit`, and measured `benchmarks/citus-patches/results/0006-fast-path-router-skip.json` before any production-ready claim |
| 0007 | `patches/0007-citus-clock-cohabit-pg-cron.patch` | pending submission | draft prepared; live pg_cron boot evidence still required before upstream PR | landed in fork | gates on `patches-check`, `citus-patch-production-audit`, and measured `benchmarks/citus-patches/results/0007-pg-cron-cohabit.json` from a live Citus + `pg_cron` boot with zero registration conflicts |
| 0008 | `patches/0008-cohabit-extensions-detection-api.patch` | pending submission | draft prepared; C API live-build proof still required before upstream PR | landed in fork | gates on companion-side `cohabit-detection-smoke`, `citus-patch-production-audit`, and measured `benchmarks/citus-patches/results/0008-detection-matrix.json` covering TimescaleDB, `pg_cron`, and `pg_partman` |
| 0009 | distSQL physical plan distribution | pending submission | in flight (large scope; will require multiple sub-PRs) | not yet landed in fork | gates on `companion-advanced-planner` canonical row plus an end-to-end physical-plan distribution smoke |
| 0010 | distributed cursors | pending submission | in flight | not yet landed in fork | gates on a cursor-correctness smoke that fetches across coordinator failover |
| 0011 | distributed savepoints | pending submission | in flight | not yet landed in fork | gates on a savepoint-correctness smoke covering nested-transaction rollback across shards |

Patches 0001-0002 are landed-in-fork but not yet submitted upstream because the
trusted-coextension contract is a deployment-layer feature that Citus
maintainers have historically declined to absorb. Patches 0003 and 0005 are
the next two candidates we expect to submit; their draft mailbox diffs are
already mailbox-header-clean and have on-disk `patches/series` entries.

Patches 0004, 0006, 0007, and 0008 now have fork patch artifacts and
`patches/series` entries, but they remain not production-ready until the
fail-closed manifest in `benchmarks/citus-patches/production-gates.json` has
measured non-scaffold results for each declared gate. The audit rejects stale
roster-only status for landed artifacts, skipped/scaffold results, docs that
overstate maturity, and any production-ready claim before measured evidence
exists. Patches 0009-0011 remain roster entries with tracked designs but no
landed fork patch artifact yet.

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
  a live Citus+pg_cron cohabitation boot that records the patched image and
  verifies no clock registration conflict.

#### 0008-cohabit-extensions-detection-api.patch -- role-aware cohabit detection

- **Target branch**: `release-14.0`
- **Type**: internal API
- **Rationale**: Exposes a role-aware classifier for supported cohabitants so
  the fork can distinguish trusted hook-chain extensions from clock-side and
  partition-management neighbors. The first roles are TimescaleDB, pg_cron, and
  pg_partman.
- **Gate before upstream PR**: `make -f Makefile.ai-blaise patches-check`,
  `cargo test -p ai_blaise_citus_companion`, and
  `ci/ai-blaise/cohabit-detection-smoke.sh`, followed by live-build proof for
  the C API before submission upstream.

## Pending pgsql-hackers PRs (`postgres/postgres`, via pgEdge/Spock contribution path)

These patches target PostgreSQL core, not Citus. They are applied at the
PG-build layer (see `images/citus-pg-overlay/Dockerfile` and its
`ai-blaise.citus.pg-core-patches` label) before Citus is compiled. They live
in their own series file (`patches/postgres/series`) so the citus quilt and
the postgres-core quilt rebase independently.

The upstream contributor in both cases is pgEdge via the Spock multi-master
replication extension. The diffs shipped here are the canonical pgEdge/Spock
contributions to pgsql-hackers, rebased to PostgreSQL 17 by Spock upstream and
re-wrapped here with mailbox headers and `FEATURE:` markers. The pgsql-hackers
threads owned by Spock are the canonical submission record; the fork tracks
them by the Spock patch identifier in the upstream column below.

| # | Patch | Spock upstream | pgsql-hackers thread | Status | Our fork PR |
|---|---|---|---|---|---|
| PG-0001 | `patches/postgres/0001-logical-commit-clock.patch` | pgEdge/spock `patches/17/pg17-025-logical_commit_clock.diff` | tracked via Spock's `XLogReserveInsertHook` proposal trail on pgsql-hackers (search `XLogReserveInsertHook logical commit clock`) | landed in fork PR48; alpha runtime gate (runtime flips once custom-PG-compile pipeline lands) | PR48 |
| PG-0002 | `patches/postgres/0002-per-subtrans-commit-ts.patch` | pgEdge/spock `patches/17/pg17-030-per-subtrans-commit-ts.diff` | tracked via Spock's `SubTransactionCommitTsEntry` proposal trail on pgsql-hackers | landed in fork PR48; alpha runtime gate (same custom-PG pipeline dependency) | PR48 |

Both PG-core diffs ship today as alpha because the custom-PG-compile pipeline
that links them into the runtime image is not yet wired into
`images/citus-pg-overlay/Dockerfile`. The runtime gate for FEATURE: PGC1 and
FEATURE: PGC2 stays alpha until that pipeline lands and a real
PostgreSQL+patches build records its image identity in
`docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`.

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
