# Extension Cohabitation

Citus normally requires its hook registrations to be first in
`shared_preload_libraries`. The ai-blaise fork carries
`patches/0001-allow-trusted-hook-coextensions.patch` to make this guard
explicitly configurable for operator-trusted co-extensions and
`patches/0002-preserve-trusted-hook-chain-state.patch` to preserve the captured
hook chain once Citus takes the outer hook position.

## Configuration

```conf
shared_preload_libraries = 'timescaledb,citus'
citus.cohabit_extensions = 'timescaledb'
```

`citus.cohabit_extensions` is a postmaster-level allowlist. It should contain
only extensions whose hook chain with Citus has been explicitly covered by the
current cohabitation evidence boundary.

The current suite boundary is intentionally conservative. Static patch
application, pure Rust acceptance models, and the default contract mode of
`tests/e2e/kind-timescale-citus-smoke.sh` are not production evidence for
hook-chain safety. The live kind smoke is opt-in because it requires a real
operand image containing Postgres, Citus, TimescaleDB, and `ai_blaise_citus`;
its output counts as production evidence only when that exact image digest,
command log, and CI or VM run are recorded in
`docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md` and the relevant feature entry
is promoted in `docs/ai-blaise/NEW_FEATURES.md`.

When the allowlist is non-empty, Citus stores any preexisting planner,
executor-start, executor-run, and EXPLAIN hooks before installing its own hooks.
Planner and executor calls continue through the stored hook when present. For
EXPLAIN, Citus delegates non-distributed statements to the stored hook and keeps
distributed statements on Citus' EXPLAIN path so worker-task output is still
produced.

The initial required cohabitation target is TimescaleDB. The bridge shape is a
PostgreSQL declarative-partitioned parent distributed by Citus with Timescale
hypertable partitions on workers.
