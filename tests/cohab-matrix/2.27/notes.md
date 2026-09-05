# TimescaleDB 2.27.x cohabitation notes

This is the load-bearing TS line for the ai-blaise/citus cohabitation seam at
the time this matrix lands. `image-tag.txt` and the shared fixture lock select
`docker.io/timescale/timescaledb-ha:pg17-ts2.27` by the exact reviewed manifest
digest. The source-bound builder installs the selected Citus checkout and
companion into that base; older cohabitation receipts do not qualify the new
fixture bytes.

## Hook seam summary

- `planner_hook`: not claimed by TimescaleDB; Citus installs `distributed_planner`.
- `ProcessUtility_hook`: claimed by TimescaleDB for DDL interception. Citus
  captures the prior hook and chains through it via `PrevProcessUtility`.
- `ExecutorStart_hook`: freed by TimescaleDB 2.22 when the hypercore TAM was
  removed; remains free in 2.27.x. Citus installs `CitusExecutorStart`
  cleanly; `PreviousExecutorStartHook` is `NULL`.
- `ExecutorRun_hook`: not claimed by TimescaleDB; Citus uses the standard path.
- `ExplainOneQuery_hook`: not claimed. Source measurement found no references
  in the 2.27.2 source, correcting the former chunk-aware-EXPLAIN assumption.

## GUC interactions

- `citus.cohabit_extensions=timescaledb` must be set at postmaster start. The
  trusted-coextension allowlist recognizes only `timescaledb` in this Citus
  fork (see `IsTrustedHookCoextension` in `src/backend/distributed/shared_library_init.c`).
- `shared_preload_libraries=timescaledb,citus` is the cohabitation load order.
  TimescaleDB must precede Citus so its hook claims are visible when the Citus
  `_PG_init` runs the trusted-coextension check.

## Known regression risks for 2.28+

- If TimescaleDB reclaims `ExecutorStart_hook` in 2.28 (it was freed in 2.22
  when the hypercore TAM was removed), Citus must capture and chain through
  the prior hook in the cohabitation path. The current
  `PreviousExecutorStartHook` capture in `shared_library_init.c` already
  handles this case; the matrix is the regression net.
