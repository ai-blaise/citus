# sidecar/schema_job

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Durable worker for online schema-change state machines.

Current implemented surface:

- `SchemaJobWorkerPlan`
- `SchemaJobLease`
- `BackfillPlan`
- `OnlineDdlSafetyPlan`
- `GhOstShadowPlan`
- `SchemaJobAction`
- JSON manifest validation with fail-closed SQL/apply-boundary checks
- controller canonical advance/wait/rollback reports
- `cargo run -p ai_blaise_citus_sidecar_schema_job -- run-canonical`
- `cargo run -p ai_blaise_citus_sidecar_schema_job -- run-controller-canonical`
- `cargo run -p ai_blaise_citus_sidecar_schema_job -- validate-manifest <path>`
- `ci/ai-blaise/schema-txn-runtime-smoke.sh`

These contracts cover `FEATURE: C10` and `FEATURE: M2`.
