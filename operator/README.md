# operator

Rust `kube-rs` operator for Citus topology, CRDs, sidecars, and ai-blaise
feature orchestration.

The first implemented specs are `CitusCluster` for `FEATURE: S4` topology
selection, `ShardGroup` for `FEATURE: S2` placement policy, and `Hypertable`
for `FEATURE: TS7`. They validate the declarative inputs needed to reconcile
coordinator-worker versus coordinator-less layouts, topology-aware placement,
extension cohabitation, `TS1` distributed hypertables, and TimescaleDB policies.

`operator/src/reconcile/hypertable.rs` converts that spec into companion
planning types so the future reconciler has one typed execution plan to apply.
