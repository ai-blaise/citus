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

These contracts cover `FEATURE: C1`, `FEATURE: C2`, `FEATURE: C3`,
`FEATURE: C14`, `FEATURE: C15`, `FEATURE: L8`, and `FEATURE: WH3`.
