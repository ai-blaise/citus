# sidecar/realtime

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

WebSocket broadcast layer driven by CDC events.

Current implemented surface:

- `RealtimeSidecarPlan`
- `RealtimeSubscription`
- `RealtimeFilter`
- `PresencePlan`
- `RealtimeBroadcastPlan`
- `RealtimeRuntime`
- `RealtimeRuntimeState`
- `RealtimeRuntimeBroadcast`
- `canonical_broadcast_plan()`
- `canonical_realtime_runtime_report()`
- `cargo run -p ai_blaise_citus_sidecar_realtime -- run-canonical`
- `cargo run -p ai_blaise_citus_sidecar_realtime -- run-runtime-canonical`

These contracts cover `FEATURE: RT1`, `FEATURE: RT2`, `FEATURE: RT3`, and
`FEATURE: RT4`.

The runtime surface deterministically models active WebSocket connections,
CDC fan-out, filtered connections, frame sizing, and presence snapshot
accounting for canonical tests.
