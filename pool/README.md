# pool

Shard-aware pooler. The implementation will fork pgcat and replace generic
routing with Citus shard-map routing.

Current implemented surface:

- `ShardMap`
- `Placement`
- `CachedPlanGeneration`

These types are the first local model for `FEATURE: T2` placement-generation
partial plan-cache invalidation.
