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
Citus stub. The script records the Git SHA, image identity, and command path in
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

## TS-version Forward-compatibility Matrix

`ci/ai-blaise/ts-version-matrix-smoke.sh` is the forward-compatibility gate for
TS6 and TS18. It iterates the TS minor lines pinned under `tests/cohab-matrix/`,
checks each `image-tag.txt` with `docker manifest inspect`, and runs the
existing non-stubbed cohabitation smoke for every published image. The matrix
writes `artifacts/ts-version-matrix-smoke.tsv`; each live row also gets a
per-version cohabitation evidence file under `artifacts/`.

| TS version | Pinned base image                         | Status as of 2026-05-24                | Matrix entry               |
| ---------- | ----------------------------------------- | -------------------------------------- | -------------------------- |
| 2.27       | `timescale/timescaledb:2.27.1-pg17`       | load-bearing, VM registry tag present  | `tests/cohab-matrix/2.27/` |
| 2.28       | `timescale/timescaledb:2.28.0-pg17`       | skip-with-note, VM registry tag absent | `tests/cohab-matrix/2.28/` |

The 2.28 row does not promote TS 2.28 to production-ready. It is a guardrail:
while the image tag is absent the matrix records `skip-with-note`; once the tag
exists, the same gate runs live and fails if any `expected-hook-claims.tsv` row
still says `unknown`.

## PostgreSQL Version Matrix

The cohabitation contract spans the ai-blaise/citus PG-version matrix:

| PG major | Cohabitation status                                                                               |
| -------- | ------------------------------------------------------------------------------------------------- |
| 17       | TS6 and TS18 production-ready against `timescale/timescaledb:latest-pg17`.                        |
| 18       | TS6 source-path verified by `ci/ai-blaise/sql-extension-smoke.sh` PG18 matrix entry. TS18 real    |
|          | Timescale+Citus image evidence stays alpha until a `latest-pg18` Timescale base is published.     |
| 16       | Suppressed pending the `background_rebalance_parallel_reference_tables` upstream flake fix.       |

`ci/ai-blaise/sql-extension-smoke.sh` runs the companion SQL extension contract
against both PG17 and PG18 on every PR. The PG18 leg also asserts the new
`io_method` GUC accepts its contract value without breaking Citus or any
bundled extension surface; this guards TS6 cohabitation under the PG18
io_method default (T6).
