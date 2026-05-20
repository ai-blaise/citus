# sidecar/schema_job

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
