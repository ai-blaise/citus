# tests/cohab-matrix

TS-version cohabitation matrix for ai-blaise/citus. Each subdirectory pins the
TimescaleDB Docker image tag, the expected PostgreSQL hook claims for that
minor release line, and version-specific cohabitation notes.

This is the forward-compatibility gate for TS6 (trusted-hook allowlist) and
TS18 (bridge SQL). The production cohabitation evidence path remains
`ci/ai-blaise/timescale-cohabitation-smoke.sh`, which builds and exercises a
real TimescaleDB image with this Citus fork. The matrix smoke reuses that path
for every pinned TimescaleDB minor line.

## Layout

```
tests/cohab-matrix/
  README.md                    -- this file
  compare-hook-claims.sh       -- runtime comparator (per-version)
  <TS_VERSION>/
    image-tag.txt              -- exact Docker base image tag
    expected-hook-claims.tsv   -- hooks this TS line is expected to claim
    notes.md                   -- per-version cohabitation seam notes
```

`expected-hook-claims.tsv` is a tab-separated file with one row per known TS
hook seam:

```
hook_symbol	claim_status	notes
```

`claim_status` is one of `claimed`, `not_claimed`, or `unknown`. Unknown rows
are allowed only while the pinned image tag is absent. If Docker publishes the
image and the row still says `unknown`, `compare-hook-claims.sh` fails the
matrix gate unless `TS_VERSION_MATRIX_ALLOW_UNKNOWN=1` is set for an explicit
exploratory local probe. CI and release gates must not set that escape hatch.

## Status As Of 2026-08-26

- `2.27/` is load-bearing. The pinned image is
  `timescale/timescaledb-ha:pg17-ts2.27`, and the VM registry probe confirmed it
  exists. This is also the line currently covered by the single-version
  `timescale/timescaledb-ha:pg17-ts2.27` cohabitation smoke.
- `2.28/` runs live: `timescale/timescaledb-ha:pg17-ts2.28` is published and
  the matrix executes the same non-stubbed Citus+TimescaleDB cohabitation
  smoke against it (both legs passed on the 2026-08-26 upstream-sync run).
  The former `unknown` ExecutorStart_hook forecast row is resolved by source
  measurement: the timescale/timescaledb 2.28.0 tag tarball contains zero
  ExecutorStart_hook references, so the hook freed by the 2.22 hypercore TAM
  removal remains unclaimed. The same measurement pass corrected the stale
  ExplainOneQuery_hook rows (zero references in 2.27.2 and 2.28.0). The
  cohabitation-smoke pass plus measured hook rows keep the matrix green but
  this alone does not promote TS 2.28 to production-ready.

Any future TS version directory must land with measured `claimed` or
`not_claimed` rows before the gate passes; `unknown` rows are rejected for
every live image.
