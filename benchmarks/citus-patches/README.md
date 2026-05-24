# Citus Patch Production Gates

This directory records the production-evidence contract for custom Citus
patches `0004`, `0006`, `0007`, and `0008`.

`production-gates.json` is intentionally fail-closed: a patch cannot be treated
as production-ready unless its patch artifact exists under `patches/`, is listed
in `patches/series`, passes the patch applicability gate, and has a measured
non-scaffold benchmark or smoke result at the declared result path. Missing
results, skipped/scaffold results, and prose-only gate descriptions do not count
as production evidence.

Current `bootstrap-v2` status:

| Patch | Current state | Required production evidence |
| ----- | ------------- | ---------------------------- |
| `0004` | roster-only; no `patches/*.patch` artifact in `bootstrap-v2` | router planner hot-path benchmark with fail-closed p95 and sample thresholds |
| `0006` | roster-only; no `patches/*.patch` artifact in `bootstrap-v2` | fast-path-router coordinator-skip result with measured round-trip threshold |
| `0007` | roster-only; no `patches/*.patch` artifact in `bootstrap-v2` | real Citus + `pg_cron` cohabitation smoke with zero registration conflicts |
| `0008` | roster-only; no `patches/*.patch` artifact in `bootstrap-v2` | companion detection matrix covering TimescaleDB, `pg_cron`, and `pg_partman` |

The CI entry point is:

```sh
make -f Makefile.ai-blaise citus-patch-production-audit
```

Do not add placeholder JSON under `results/`; the audit rejects any existing
result that is not `mode: "measured"`.
