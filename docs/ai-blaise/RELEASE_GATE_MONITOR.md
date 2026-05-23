# Release Gate Monitor

`ci/ai-blaise/release-gate-monitor.sh` is the bounded release/integration
monitor for this fork. It is intentionally smaller than the broad upstream
Citus matrix, so it can run during normal implementation slices while matrix
jobs continue elsewhere.

The local monitor fails closed on:

- production-ready feature entries without `Production evidence:` and an
  executable CI, cargo, VM, Docker, or SQL-runtime evidence marker
- alpha feature entries that carry production evidence or production-release
  overclaim wording
- stale V2 domain-contract command counts; the current baseline is 49 commands
- release docs that imply V2 acceptance or canonical model data is production
  evidence by itself
- missing benchmark Black-formatting enforcement for
  `benchmarks/timescale-ingest/ingest.py`
- missing custom HTTP probe coverage in `ci/ai-blaise/image-check.sh`
- weakened Docker/Postgres readiness guardrails such as detached `docker exec`
  SQL smokes or missing `PostgreSQL init process complete` checks

For PR monitoring, use:

```bash
ci/ai-blaise/release-gate-monitor.sh --pr <number-or-url>
ci/ai-blaise/release-gate-monitor.sh --pr <number-or-url> --watch --interval 60
```

The PR mode prints pass, pending, and failed check counts without starting the
full matrix itself. This supports parallel matrix monitoring while work
continues in other isolated worktrees. It is a monitor and release gate, not a
self-merge mechanism.

A production release still requires
`ci/ai-blaise/production-readiness-check.sh production-release`. The repository
remains not production-ready as a whole while alpha, model-only, or
contract-only feature entries remain.
