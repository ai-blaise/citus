# sidecar/repack

Online repack coordinator using `pg_repack` and later PostgreSQL 19
`REPACK CONCURRENTLY`.

Current implemented surface:

- `RepackJobPlan`
- `ShardRepackTarget`
- `RepackCommandPlan`

These contracts cover `FEATURE: R7`.
