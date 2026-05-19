# operator

Rust `kube-rs` operator for Citus topology, CRDs, sidecars, and ai-blaise
feature orchestration.

The first implemented spec is the `Hypertable` CRD model for `FEATURE: TS7`.
It validates the declarative inputs needed to reconcile `TS1` distributed
hypertables and TimescaleDB policies.

`operator/src/reconcile/hypertable.rs` converts that spec into companion
planning types so the future reconciler has one typed execution plan to apply.
