# sidecar/shared

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Shared sidecar library for health, readiness, metrics, drain handling, and
sidecar contract validation.

Tracing and OpenTelemetry export are not implemented in this shared runtime.
`FEATURE: O5` is production-ready only for the operator `Sidecar` CR to
digest-pinned Deployment/Service/status apply path proven by
`ci/ai-blaise/sidecar-controller-live-smoke.sh`. Trace emission, collector
wiring, configuration loading, PostgreSQL connection helpers, autoscaling,
rollout policy, and broader sidecar application behavior remain outside this
shared runtime surface unless another feature entry claims them with live
evidence.

Current implemented surface:

- `HealthReport`
- `RetargetConfig`
- `EndpointRegistry`
- `ComponentState`
- `DrainState`
- `SidecarRuntimeContracts`
- `SidecarRuntime`
- HTTP probe handling for `/healthz`, `/readyz`, `/drain`, and `/metrics`
- Unix-socket one-shot serving for local sidecar probes

These primitives are the local foundation for `FEATURE: O4` sidecar health and
metrics emission.
`SidecarRuntimeContracts` adds validation contracts for CDC, realtime, auth,
storage, backup/restore, repack, and analytical mirror sidecars.
`RetargetConfig` and `EndpointRegistry` add the narrow `FEATURE: SC7` HA
primitive: fail-closed endpoint config parsing, deterministic health-aware
endpoint selection, failure-driven retargeting, drain-aware exclusion, and
generation-tracked config reloads. They do not create Kubernetes objects,
watch EndpointSlices, or execute cross-region failover; those orchestration
surfaces remain owned by the operator/chart layer and stay alpha until live
evidence is recorded.

`cargo run -p ai_blaise_citus_sidecar_shared -- probe-canonical` emits a
tab-separated canonical probe sequence that covers readiness, Prometheus metrics,
drain transition, and post-drain readiness rejection. CI checks all targets so
the binary probe surface cannot drift from the shared runtime library.
`cargo run -p ai_blaise_citus_sidecar_shared -- ha-canonical` emits a canonical
retarget sequence covering initial selection, primary failure, drain exclusion,
and generation-incrementing reload.
