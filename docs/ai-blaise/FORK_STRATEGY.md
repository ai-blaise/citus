# Fork Strategy

`ai-blaise/citus` is a hard fork of `citusdata/citus` with an upstream-minimal
maintenance model.

## Invariants

- Upstream Citus files are modified only through ordered patches in `patches/`.
- New functionality lives in non-overlapping overlay directories.
- Each patch must be small enough to review and upstream independently.
- The long-term target is to reduce carried patches as upstream accepts them.
- Every feature beyond vanilla Citus is registered in `NEW_FEATURES.md`.

## Branches

- `main` is the release branch.
- `ai-blaise/dev` is the integration branch.
- Feature work lands on `feature/*` branches and merges through review.

## Sync

The upstream sync workflow will fetch `citusdata/citus`, test patch
applicability with `make -f Makefile.ai-blaise patches-check`, and open a
review PR when Citus moves.
