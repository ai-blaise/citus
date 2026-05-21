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
