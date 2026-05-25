# sidecar/hlc

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Hybrid logical clock service for closed timestamps and bounded-staleness reads.

Current implemented surface:

- `HlcTimestamp`
- `HlcClock`
- `ClosedTimestampPlan`
- `FollowerReadPlan`
- `EdgeReadPlan`
- `cargo run -p ai_blaise_citus_sidecar_hlc -- run-canonical`
- `cargo run -p ai_blaise_citus_sidecar_hlc -- run-runtime-canonical`
- `cargo run -p ai_blaise_citus_sidecar_hlc -- serve`
- `ci/ai-blaise/sidecar-hlc-smoke.sh`

The `serve` mode exposes the bounded HTTP gate used by the live smoke:
`/clock/tick`, `/clock/observe`, `/closed_ts`, and `/follower_read`, plus the
Edge1 `/edge_read` gate and the shared `/healthz`, `/readyz`, `/drain`, and
`/metrics` probe surface. `sidecar-hlc-smoke.sh` starts the real sidecar
process, advances the local clock, observes a peer clock exchange, verifies
closed timestamp advancement, serves follower and edge reads at the closed
timestamp, and rejects AS OF timestamps that are newer than closed, too stale
for the configured edge budget, mapped to the wrong edge replica, or mapped to
an unknown edge region.

These contracts cover `FEATURE: S9`, `FEATURE: MR6`, and `FEATURE: Edge1` for
the bounded sidecar gates described in `docs/ai-blaise/NEW_FEATURES.md`.
