# sidecar/hlc

Hybrid logical clock service for closed timestamps and bounded-staleness reads.

Current implemented surface:

- `HlcTimestamp`
- `HlcClock`
- `ClosedTimestampPlan`
- `FollowerReadPlan`

These contracts cover `FEATURE: S9`.
