# sidecar/shared

Shared sidecar library for health, readiness, metrics, tracing, configuration,
and PostgreSQL connection helpers.

Current implemented surface:

- `HealthReport`
- `ComponentState`
- `DrainState`

These primitives are the local foundation for `FEATURE: O4` sidecar health and
metrics emission.
