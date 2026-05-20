# citus-watch

Ratatui dashboard for cluster, pool, and sidecar status.

The current Rust contract covers `FEATURE: O13` and `FEATURE: D12` by
validating data sources and panels for cluster topology, shards, hypertables,
Citus-aware EXPLAIN, rebalance status, vectorizer backlog, search indexes,
tenants, and branches.
