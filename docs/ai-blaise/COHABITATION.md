# Extension Cohabitation

Citus normally requires its hook registrations to be first in
`shared_preload_libraries`. The ai-blaise fork now integrates the TS6 source
changes that make this guard explicitly configurable for operator-trusted
co-extensions and preserve the captured hook chain once Citus takes the outer
hook position. The matching files under `patches/` remain as reference and
upstream-rebase artifacts.

## Configuration

```conf
shared_preload_libraries = 'timescaledb,citus'
citus.cohabit_extensions = 'timescaledb'
```

`citus.cohabit_extensions` is a postmaster-level allowlist. The production
implementation currently recognizes only `timescaledb`; unsupported names do
not satisfy the trust check and Citus keeps its normal first-hook guard. The
setting should contain only extensions whose hook chain with Citus has been
explicitly covered by live cohabitation evidence.

`ci/ai-blaise/timescale-cohabitation-smoke.sh` is the production evidence path
for TS6 and TS18. It builds a real image from `timescale/timescaledb:latest-pg17`
with this Citus fork and `ai_blaise_citus` installed, starts PostgreSQL with
`shared_preload_libraries=timescaledb,citus` and
`citus.cohabit_extensions=timescaledb`, creates real `citus`, `timescaledb`,
and `ai_blaise_citus` extensions, verifies real Citus distribution metadata in
`pg_dist_partition`, and runs the bridge apply functions without defining a
Citus stub. The script records the image identity in
`artifacts/timescale-cohabitation-evidence.tsv`.

When the allowlist is non-empty, Citus stores any preexisting planner,
executor-start, executor-run, and EXPLAIN hooks before installing its own hooks.
Planner and executor calls continue through the stored hook when present. For
EXPLAIN, Citus delegates non-distributed statements to the stored hook and keeps
distributed statements on Citus' EXPLAIN path so worker-task output is still
produced.

The initial required cohabitation target is TimescaleDB. The current
production-ready claim is intentionally narrow: TS6 covers the trusted Citus
hook-chain source path, and TS18 covers the installable bridge-state SQL under
real Citus+TimescaleDB cohabitation. The broader TS1/TS2/TS3/TS4/TS5/TS12
distributed Timescale features remain alpha until multi-worker fanout,
rebalance, and operator reconciliation are proven end to end.
