# sidecar/schema_job

Durable worker for online schema-change state machines.

Current implemented surface:

- `SchemaJobWorkerPlan`
- `SchemaJobLease`
- `BackfillPlan`
- `OnlineDdlSafetyPlan`
- `GhOstShadowPlan`
- `SchemaJobAction`

These contracts cover `FEATURE: C10` and `FEATURE: M2`.
