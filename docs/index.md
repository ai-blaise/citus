# ai-blaise/citus

`ai-blaise/citus` is a Rust-first Postgres extension overlay that forks
[`citusdata/citus`](https://github.com/citusdata/citus) and ships the V2 release stack: a
3-worker Citus + TimescaleDB cohabitation, the distributed plan cache, parallel commit,
branch/suspend/resume, the vectorizer, distributed BM25 search, HTAP across hot/cold tiers,
and multi-region survival.

The overlay is AGPL-3.0 and merges upstream cleanly via a documented patch series; see
[Fork strategy](ai-blaise/FORK_STRATEGY.md) and [Upstream sync](ai-blaise/UPSTREAM_SYNC.md).

## Quick links

| Topic                                   | Doc                                                                 |
| --------------------------------------- | ------------------------------------------------------------------- |
| System architecture                     | [Architecture](ai-blaise/ARCHITECTURE.md)                           |
| Co-existing with upstream Citus         | [Cohabitation](ai-blaise/COHABITATION.md)                           |
| All V2 features and their gate status   | [New features](ai-blaise/NEW_FEATURES.md)                           |
| Sidecar + extension catalogue           | [Bundled extensions](ai-blaise/BUNDLED_EXTENSIONS.md)               |
| Performance evidence + thresholds       | [Benchmarks](ai-blaise/BENCHMARKS.md)                               |
| Operator tuning runbook                 | [Performance tuning](ai-blaise/PERFORMANCE_TUNING.md)               |
| License audit (AGPL-3.0 + dependencies) | [License audit](ai-blaise/LICENSE_AUDIT.md)                         |
| Release process + GitHub Pages source   | [Releasing](ai-blaise/RELEASING.md)                                 |
| End-to-end harness                      | [End-to-end](ai-blaise/E2E.md)                                      |
| Production readiness evidence           | [Production readiness audit](ai-blaise/PRODUCTION_READINESS_AUDIT.md) |

## Status

The release gates are tracked in
[New features](ai-blaise/NEW_FEATURES.md); production-ready features carry executable
evidence and a `Status: production-ready` line, alpha features only carry a model and a
harness path. The V2 acceptance gates (10 performance, 11 chaos) draw on the harnesses in
the `benchmarks/` tree of the repository.

## Where the source lives

The published site is built from the [`docs/` tree of `ai-blaise/citus`](https://github.com/ai-blaise/citus/tree/main/docs).
Every page on this site has an "edit on GitHub" link in the header; PRs are welcome.
