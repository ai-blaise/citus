# TimescaleDB 2.28.x Cohabitation Notes

TimescaleDB 2.28 is tracked here as a forward-compatibility row, not production
evidence. A VM registry probe on 2026-05-23 found no published PG17 image for
`timescale/timescaledb:2.28-pg17`, `timescale/timescaledb:2.28.0-pg17`, or
`timescale/timescaledb:2.28.1-pg17`. The pinned candidate is recorded in
`image-tag.txt` as `timescale/timescaledb:2.28.0-pg17` so CI can detect the
line as soon as Docker publishes it.

## Forecasted Hook Seam

The `expected-hook-claims.tsv` file is a forecast based on TimescaleDB
2.22-2.27 hook usage history:

- `planner_hook`, `ExecutorRun_hook`: not claimed (carry-forward from 2.27).
- `ProcessUtility_hook`, `ExplainOneQuery_hook`: claimed (carry-forward).
- `ExecutorStart_hook`: `unknown`. TimescaleDB 2.22 freed this hook when the
  hypercore TAM was removed; TS 2.28 may keep it free or reclaim it.

The `unknown` row is intentional while the image is absent. Once the image is
published, `compare-hook-claims.sh` fails on any remaining `unknown` rows so TS
2.28 cannot be treated as production-ready until the hook claim is measured.

## Promotion Checklist

1. Pull the pinned image and verify the installed extension still reports the
   canonical `timescaledb` extension name.
2. Run `REQUIRE_DOCKER=1 TS_VERSION_MATRIX=2.28 bash ci/ai-blaise/ts-version-matrix-smoke.sh`.
3. Update `expected-hook-claims.tsv` from `unknown` to measured `claimed` or
   `not_claimed` rows. If a previously free hook is now claimed, audit the
   matching capture/chaining path in `src/backend/distributed/shared_library_init.c`.
4. Only then may docs cite TS 2.28 as live cohabitation evidence.
