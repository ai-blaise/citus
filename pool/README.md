# pool

Shard-aware pooler. The implementation will fork pgcat and replace generic
routing with Citus shard-map routing.

Current implemented surface:

- `ShardMap`
- `Placement`
- `CachedPlanGeneration`
- `PlanCache`
- `ShardRoute`
- `PoolRuntimeContract`
- `PoolExecutionReport`
- `SessionSetting`

These types are the first local model for `FEATURE: T2` placement-generation
partial plan-cache invalidation and `FEATURE: T3` single-shard route
selection.
`PoolRuntimeContract` adds validation for `FEATURE: T1`, `FEATURE: T3`,
`FEATURE: T9`, `FEATURE: T12`, `FEATURE: T15`, `FEATURE: R10`,
`FEATURE: Sec12`, `FEATURE: Auth3`, and `FEATURE: MR5`.
`cargo run -p ai_blaise_citus_pool -- run-canonical` emits the deterministic
execution summary for the pool runtime and shard-map contracts used by CI.
