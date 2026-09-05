# tests/cohab-matrix

## Plan status

The September 5 **CHIMERA — FINAL PRODUCTION PLAN** selects Apache-only
TimescaleDB and excludes toolkit/TSL artifacts. The existing Community-image
fixtures in this directory are historical fork diagnostics, not qualifying
W2/B4/B6 inputs for that plan. Preserve their actual source and image identities;
future qualifying lanes must use the selected Apache-only build and the
plan's revised capability surface. The matrix description and status below
describe the existing fork harness, not completion of that conversion.

TS-version cohabitation matrix for ai-blaise/citus. Each subdirectory pins the
TimescaleDB Docker image reference by immutable manifest digest, the expected
PostgreSQL hook claims for that minor release line, and version-specific
cohabitation notes.

This harness exercises the historical TS6 (trusted-hook allowlist) and
TS18 (bridge SQL) contracts. The existing cohabitation evidence path is
`ci/ai-blaise/timescale-cohabitation-smoke.sh`, which builds and exercises a
real TimescaleDB image with this Citus fork. The matrix smoke reuses that path
for every pinned TimescaleDB minor line.

## Layout

```
tests/cohab-matrix/
  README.md                    -- this file
  compare-hook-claims.sh       -- runtime admission + static inventory validator
  <TS_VERSION>/
    image-tag.txt              -- exact Docker base image reference and digest
    expected-hook-claims.tsv   -- hooks this TS line is expected to claim
    notes.md                   -- per-version cohabitation seam notes
```

`expected-hook-claims.tsv` is a tab-separated file with one row per known TS
hook seam:

```
hook_symbol	claim_status	notes
```

`claim_status` is one of `claimed`, `not_claimed`, or `unknown`. PostgreSQL SQL
introspection does not expose live C hook pointers, so this table is a
source-measured static inventory rather than a runtime observation. Unknown rows
are not allowed in either load-bearing row. The comparator rejects them unless
`TS_VERSION_MATRIX_ALLOW_UNKNOWN=1` is set for an explicit exploratory local
probe. CI and release gates must not set that escape hatch.

## Status As Of 2026-09-05

- `2.27/` and `2.28/` are both required source-fixture lanes. Their
  `image-tag.txt` files match the two exact manifest-digest rows in
  `images/citus-timescale-cohabitation/base-image.lock.tsv`; the matrix asks
  the shared builder for the selected minor and passes only its verified
  immutable image ID to the cohabitation smoke and runtime-admission probe.
- The former 2.28 `unknown` ExecutorStart_hook forecast row is resolved by source
  measurement: the timescale/timescaledb 2.28.0 tag tarball contains zero
  ExecutorStart_hook references, so the hook freed by the 2.22 hypercore TAM
  removal remains unclaimed. The same measurement pass corrected the stale
  ExplainOneQuery_hook rows (zero references in 2.27.2 and 2.28.0). The
  table labels its other rows as source-measured or carry-forward expectations;
  it is not uniformly source-measured. The source-fixture
  conversion invalidates older image receipts; fresh native execution of both
  exact source-built lanes remains required and neither row alone is release
  qualification.

Any future TS version directory must land with measured `claimed` or
`not_claimed` rows before the gate passes; `unknown` rows are rejected for
every live image.
