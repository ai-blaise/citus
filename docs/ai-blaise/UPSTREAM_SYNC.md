# Upstream Sync

The fork checks patch applicability against `citusdata/citus` every 14 days and
on every V2 acceptance run. The default upstream target is `release-14.0`, which
matches the V2 upstream-merge gate; override it with `UPSTREAM_REF` only for
explicit backport or forward-port drills.

The sync job intentionally starts as a dry-run gate. Once the first overlay
release stabilizes, it should be extended to open a PR on
`chore/upstream-sync-YYYY-MM-DD` with:

- upstream Citus changes
- refreshed patch applicability output
- any patch rebases needed to keep `patches/series` clean

## Pending upstream PRs

These quilt patches are upstream-PR candidates -- small, single-purpose, and
designed to apply cleanly against `citusdata/citus` `release-14.0`. Each entry
records the target branch, the rationale, and the gate that must turn green
before the PR opens.

### 0003-guc-report-citus-userset.patch -- citus.* USERSET GUCs report to clients

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

### 0005-placement-generation-counter.patch -- pg_dist_placement generation counter

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

The submission cadence is: open the upstream PR only after the patch passes
both ai-blaise CI and `kind-smoke`, and only after the corresponding
`NEW_FEATURES.md` entry has flipped from `alpha` to `production-ready`.

## PostgreSQL core quilt patches (`patches/postgres/`)

These patches target PostgreSQL core, not Citus. They are applied at the
PG-build layer (see `images/citus-pg-overlay/Dockerfile` and its
`ai-blaise.citus.pg-core-patches` label) before Citus is compiled. They live
in their own series file (`patches/postgres/series`) so the citus quilt and
the postgres-core quilt rebase independently.

The upstream contributor in both cases is pgEdge via the Spock multi-master
replication extension. The diffs we ship are the canonical pgEdge/Spock
contributions to pgsql-hackers, rebased to PostgreSQL 17 by Spock upstream
and re-wrapped here with mailbox headers and `FEATURE:` markers.

| Patch | Upstream | Status |
|---|---|---|
| `0001-logical-commit-clock.patch` | pgEdge/spock `patches/17/pg17-025-logical_commit_clock.diff` | alpha; runtime gate stays alpha until the custom-PG-compile pipeline lands |
| `0002-per-subtrans-commit-ts.patch` | pgEdge/spock `patches/17/pg17-030-per-subtrans-commit-ts.diff` | alpha; runtime gate stays alpha until the custom-PG-compile pipeline lands |

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
