# sidecar/shared

Shared sidecar library for health, readiness, metrics, tracing, configuration,
and PostgreSQL connection helpers.

Current implemented surface:

- `HealthReport`
- `ComponentState`
- `DrainState`
- `SidecarRuntimeContracts`

These primitives are the local foundation for `FEATURE: O4` sidecar health and
metrics emission.
`SidecarRuntimeContracts` adds validation contracts for CDC, realtime, auth,
storage, backup/restore, repack, and analytical mirror sidecars.
