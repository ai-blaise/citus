# sidecar/repack

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Online repack coordinator using `pg_repack` and later PostgreSQL 19
`REPACK CONCURRENTLY`.

Current implemented surface:

- `RepackJobPlan`
- `ShardRepackTarget`
- `RepackCommandPlan`
- fail-closed `RepackRuntimeEnvironment` strategy selection
- dry-run `RepackExecutionReport` with `executed=false` and `evidence_boundary=dry-run-plan-only`
- `cargo run -p ai_blaise_citus_sidecar_repack -- run-canonical`
- live `pg_repack` execution through `run-live-pg-repack`
- `bash ci/ai-blaise/sidecar-repack-smoke.sh`
- `REQUIRE_DOCKER=1 bash ci/ai-blaise/sidecar-repack-smoke.sh`

`run-live-pg-repack` requires `AI_BLAISE_REPACK_DATABASE_URL` or `DATABASE_URL`
and invokes the real `pg_repack` binary with a validated qualified table,
bounded job count, and explicit wait timeout. It emits a TSV report with
`dry_run=false`, `executed=true`, and
`evidence_boundary=live-pg-repack-execution` only after the command exits
successfully.

The Docker live smoke builds the repack sidecar, boots PostgreSQL 17 with the
packaged `postgresql-17-repack` extension, creates and sparsely deletes from a
real `public.orders` table, installs `pg_repack`, executes the sidecar-owned
`pg_repack` command, and verifies the table remains readable afterward.

Production boundary: this is production-ready evidence for a sidecar-owned live
`pg_repack` invocation against a single local PostgreSQL target plus the
existing operator plan rendering. It is not production evidence for PostgreSQL
19 `REPACK CONCURRENTLY`, Kubernetes-scheduled repack execution, or Citus shard
fanout across workers.
