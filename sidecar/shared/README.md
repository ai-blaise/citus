# sidecar/shared

Shared sidecar library for health, readiness, metrics, tracing, configuration,
and PostgreSQL connection helpers.

Current implemented surface:

- `HealthReport`
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

`cargo run -p ai_blaise_citus_sidecar_shared -- probe-canonical` emits a
tab-separated canonical probe sequence that covers readiness, Prometheus metrics,
drain transition, and post-drain readiness rejection. CI checks all targets so
the binary probe surface cannot drift from the shared runtime library.
