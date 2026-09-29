# Release Gate Monitor

`ci/ai-blaise/release-gate-monitor.sh` is the bounded release/integration
monitor for this fork. It is intentionally smaller than the broad upstream
Citus matrix, so it can run during normal implementation slices while matrix
jobs continue elsewhere.

The local monitor validates `docs/features.tsv` through
`ai_blaise_feature_register check` and consumes its deterministic `summary`
only for structural inventory. It does not infer readiness from implementation
maturity, historical `claimed_status`, `FEATURE:` markers, source paths, check
paths, or evidence paths. Modeled release gates and fixed canonical-output
counts are package contracts, not release certification, and are not promotion
inputs to this monitor.

The monitor also invokes `ai_blaise_feature_register release-gaps` and requires
the current fail-closed contract: exit 1, `feature_release_gaps\tblocked`, an
unimplemented trusted release-evidence verifier, and one blocked record for
every structurally valid register row. The monitor itself succeeds when that
rejection contract is intact. It cannot qualify a production release.

The remaining local checks cover:

- release documentation that must reject whole-product and V2-model
  overclaims
- benchmark Black-formatting enforcement for
  `benchmarks/timescale-ingest/ingest.py`
- custom HTTP probe coverage and Docker/Postgres readiness guardrails in
  `ci/ai-blaise/image-check.sh`
- workflow and Makefile wiring for the monitor
- shell syntax for the release, image, and V2 audit entry points

For PR monitoring, use:

```bash
ci/ai-blaise/release-gate-monitor.sh --pr <number-or-url>
ci/ai-blaise/release-gate-monitor.sh --pr <number-or-url> --watch --interval 60
```

The PR mode requests only the installed GitHub CLI's supported JSON fields:
`name`, `state`, `bucket`, `workflow`, and `link`. It accepts the CLI's normal,
failed-check, and pending-check exits (0, 1, and 8) only when they carry a
nonempty, strictly valid JSON check inventory whose terminal or pending
buckets are consistent with a nonzero exit. The monitor validates each state
against the CLI-defined `pass`, `fail`, `pending`, `skipping`, or `cancel`
bucket and has no human-table fallback. Malformed or empty output, unknown
states or buckets, inconsistent state/bucket pairs, and authentication or
command errors fail closed.

The PR mode prints pass, pending, skipped, failed, and cancelled check counts
without starting the full matrix itself. This preserves parallel matrix
monitoring while work continues in other isolated worktrees. Failed and
cancelled checks make the monitor fail. Pending checks remain pending: they
are reported and nonblocking unless `--watch` is requested. Skipped checks are
reported explicitly and are never counted as passes. The monitor does not
merge, approve, or promote anything.

The repository is not production-ready as a whole. A future positive release
path requires a separately governed verifier for trusted, current-source,
full-scope runtime and operational receipts. The V2 acceptance model must not
be cited as production evidence.
