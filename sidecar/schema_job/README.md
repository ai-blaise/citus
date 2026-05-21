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
- `cargo run -p ai_blaise_citus_sidecar_schema_job -- run-canonical`

These contracts cover `FEATURE: C10` and `FEATURE: M2`.
