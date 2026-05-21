# ADR 0002: Overlay Directories, Not Inline Patches

## Status

Accepted (2026-05-21)

## Context

`ai-blaise/citus` extends Citus along many axes — companion extension,
sidecar daemons, pooler, operator, and tooling. Each new capability could
be added by editing `src/` in place, by maintaining a long patch series,
or by living in a separate top-level directory that does not collide
with upstream paths. Rule 2 of the project (upstream-minimalist) requires
that we modify upstream files only when no other path achieves the goal,
because every inline edit becomes merge friction on the 14-day
`upstream-sync` cron and a perpetual diff with `citusdata/citus`.

## Decision

All new code lives in non-overlapping overlay directories:
`companion/`, `sidecar/`, `pool/`, `operator/`, `tools/`, `deploy/`,
`images/`, `benchmarks/`, `tests/`, and `docs/ai-blaise/`. The only edits
that touch upstream files are kept in `patches/`, a quilt-style ordered
series with one rationale per patch and a target upstream PR URL where
applicable. The non-overlapping invariant: an upstream merge can conflict
in `patches/` and nowhere else.

## Alternatives considered

- Inline edits to `src/backend/distributed/`. Rejected — every upstream
  merge becomes a manual reconciliation, and the diff against
  `citusdata/citus` grows without bound. Defeats Rule 2.
- A single sprawling patch queue covering every behavior change.
  Rejected — review burden grows with feature count, and patches that
  cannot be upstreamed accumulate indefinitely.
- A separate repository that vendors Citus as a submodule. Rejected —
  loses git history continuity with upstream, breaks `git blame` across
  the boundary, and complicates the upstream-sync automation.

## Consequences

- Positive: upstream merges are mechanical for everything outside
  `patches/`. The patch queue stays small and each entry has a clear
  upstream-PR target, so patches drop as upstream accepts them.
- Positive: each overlay directory has its own owners, CI workflow, and
  release cadence without coupling to upstream Citus internals.
- Negative: cross-cutting changes that need to touch both upstream and
  the overlay require coordination across two surfaces (a patch plus
  overlay code).
- Risks: an overlay component may reach for upstream internals through
  unstable C ABI. We constrain `companion/` to the documented hook
  surface; deeper hooks ship as numbered patches with a stated upstream
  target.

## References

- Plan §5.1 (repo layout)
- Plan §5.3 (`patches/` discipline)
- Plan §6 (component plans)
- Rule 2 — upstream-minimalist
- `docs/ai-blaise/FORK_STRATEGY.md`
