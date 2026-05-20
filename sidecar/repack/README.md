# sidecar/repack

Online repack coordinator using `pg_repack` and later PostgreSQL 19
`REPACK CONCURRENTLY`.

Current implemented surface:

- `RepackJobPlan`
- `ShardRepackTarget`
- `RepackCommandPlan`
- `cargo run -p ai_blaise_citus_sidecar_repack -- run-canonical`

These contracts cover `FEATURE: R7`.
