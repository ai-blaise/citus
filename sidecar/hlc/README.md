# sidecar/hlc

Hybrid logical clock service for closed timestamps and bounded-staleness reads.

Current implemented surface:

- `HlcTimestamp`
- `HlcClock`
- `ClosedTimestampPlan`
- `FollowerReadPlan`
- `cargo run -p ai_blaise_citus_sidecar_hlc -- run-canonical`

These contracts cover `FEATURE: S9`.
