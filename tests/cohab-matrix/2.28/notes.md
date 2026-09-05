# TimescaleDB 2.28.x Cohabitation Notes

TimescaleDB 2.28 is tracked here as a required forward-compatibility row, not
production evidence. `image-tag.txt` and the shared fixture lock select the
published `docker.io/timescale/timescaledb-ha:pg17-ts2.28` operand by the exact
reviewed manifest digest. The source-bound builder must install the selected
Citus checkout and companion into that base before the matrix can execute it.

## Static Hook Inventory

The `expected-hook-claims.tsv` file records a static 2.28 inventory. Individual
rows state whether they are source-measured or carry-forward expectations:

- `planner_hook`, `ExecutorRun_hook`: not claimed (carry-forward from 2.27).
- `ProcessUtility_hook`: claimed for DDL interception.
- `ExplainOneQuery_hook`: not claimed in the measured 2.28 source.
- `ExecutorStart_hook`: not claimed. TimescaleDB 2.22 freed this hook when the
  hypercore TAM was removed and the measured 2.28 source keeps it free.

`compare-hook-claims.sh` fails on any `unknown` row in the load-bearing matrix.

## Promotion Checklist

1. Build or verify the exact source-bound fixture and require the installed
   extension to report the canonical `timescaledb` extension name.
2. Run `REQUIRE_DOCKER=1 TS_VERSION_MATRIX=2.28 bash ci/ai-blaise/ts-version-matrix-smoke.sh`.
3. If a hook differs from the measured table, audit the matching
   capture/chaining path in `src/backend/distributed/shared_library_init.c`.
4. Only a fresh exact-source native receipt may qualify this fixture revision;
   the source contract alone is not release evidence.
