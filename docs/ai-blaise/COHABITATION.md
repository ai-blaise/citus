# Extension Cohabitation

Citus normally requires its hook registrations to be first in
`shared_preload_libraries`. The ai-blaise fork carries
`patches/0001-allow-trusted-hook-coextensions.patch` to make this guard
explicitly configurable for validated co-extensions and
`patches/0002-preserve-trusted-hook-chain-state.patch` to preserve the captured
hook chain once Citus takes the outer hook position.

## Configuration

```conf
shared_preload_libraries = 'timescaledb,citus'
citus.cohabit_extensions = 'timescaledb'
```

`citus.cohabit_extensions` is a postmaster-level allowlist. It should contain
only extensions whose hook chain with Citus has been tested in this fork's
cohabitation suite.

When the allowlist is non-empty, Citus stores any preexisting planner,
executor-start, executor-run, and EXPLAIN hooks before installing its own hooks.
Planner and executor calls continue through the stored hook when present. For
EXPLAIN, Citus delegates non-distributed statements to the stored hook and keeps
distributed statements on Citus' EXPLAIN path so worker-task output is still
produced.

The initial required cohabitation target is TimescaleDB. The bridge shape is a
PostgreSQL declarative-partitioned parent distributed by Citus with Timescale
hypertable partitions on workers.
