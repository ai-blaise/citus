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

## Status As Of 2026-05-24

- `2.27/` is load-bearing. The pinned image is
  `timescale/timescaledb-ha:pg17-ts2.27`, and the VM registry probe confirmed it
  exists. This is also the line currently covered by the single-version
  `timescale/timescaledb-ha:pg17-ts2.27` cohabitation smoke.
- `2.28/` is a forward-compatibility row, not production evidence. The pinned
  image is `timescale/timescaledb-ha:pg17-ts2.28`; the VM registry probe on
  2026-05-24 confirmed that `timescale/timescaledb-ha:pg17-ts2.28`, `timescale/timescaledb-ha:pg17-ts2.28.0`, and `timescale/timescaledb-ha:pg17-ts2.28.1` are
  not published. The matrix records `skip-with-note` while the tag is absent
  and does not promote TS 2.28 to production-ready.

When the 2.28 image appears, the matrix will run the same non-stubbed
Citus+TimescaleDB cohabitation smoke against that image. Any remaining
`unknown` hook rows must be resolved to measured `claimed` or `not_claimed`
rows before the gate passes.
