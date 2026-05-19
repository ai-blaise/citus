# End-to-End Acceptance

The critical-path acceptance harness starts in `e2e/`. It is a pure Rust model
until the kind/PostgreSQL runner is available, but it already exercises the same
contract the real cluster test must satisfy:

1. validate the `CitusCluster` topology and extension surface
2. validate the `ShardGroup` topology-aware placement policy
3. preload Citus and TimescaleDB together
4. configure `citus.cohabit_extensions = 'timescaledb'`
5. create a `Hypertable` spec with compression, retention, and CAGG policy
6. map that spec through the operator reconciler into companion plans
7. record the gates the database-backed runner must prove

The first scenario is `TimescaleOnCitusAcceptance::canonical_metrics()`.
