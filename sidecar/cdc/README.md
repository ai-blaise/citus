# sidecar/cdc

Logical replication consumer for CDC sinks, webhooks, realtime broadcasts, and
analytical mirrors.

Current implemented surface:

- `CdcSidecarPlan`
- `LogicalSlotPlan`
- `CdcSinkPlan`
- `SchemaCapturePlan`
- `AnonymizationRule`
- `CdcEventEnvelope`
- `CdcDeliveryPlan`
- `LogicalReplicationFrame`
- `CdcRuntime`
- `CdcRuntimeState`
- `decode_wal2json_frame()`
- `canonical_delivery_plan()`
- `cargo run -p ai_blaise_citus_sidecar_cdc -- run-canonical`
- `cargo run -p ai_blaise_citus_sidecar_cdc -- run-runtime-canonical`

These contracts cover `FEATURE: C1`, `FEATURE: C2`, `FEATURE: C3`,
`FEATURE: C14`, `FEATURE: C15`, `FEATURE: L8`, and `FEATURE: WH3`.
