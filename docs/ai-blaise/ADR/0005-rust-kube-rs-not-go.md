# ADR 0005: Rust + kube-rs for the Operator, Not Go + operator-sdk

## Status

Accepted (2026-05-21)

## Context

The operator carries roughly 15 CRDs — `CitusCluster`, `ShardGroup`,
`Hypertable`, `Branch`, `Vectorizer`, `Sidecar`, `Migration`,
`ConflictPolicy`, `Tenant`, `Region`, `SurvivalGoal`, `Backup`,
`Federation`, `SearchIndex`, `Webhook`, `Function`, `ScheduledRepack`
— each with a reconciler that talks to Postgres via `sqlx`, to other
sidecars via `tonic` gRPC, and to the Kubernetes API. The two practical
language choices are Go (operator-sdk / controller-runtime, the
ecosystem default) or Rust (`kube-rs`, the Rust-native client).

## Decision

The operator is written in Rust with `kube-rs v3.x`, `k8s-openapi`,
`schemars` for CRD generation, `sqlx` for the Postgres control plane,
and `tonic` for gRPC to sidecars. The operator binary is the crate
`ai_blaise_citus_operator`, and it joins the same Cargo workspace as
`companion/`, `sidecar/`, `pool/`, and `tools/`.

## Alternatives considered

- Go + operator-sdk + controller-runtime. Rejected — the ecosystem
  is more mature, but adopting it would split the overlay into two
  language stacks. Sharing types between the operator and the
  sidecars (which are Rust) would require code generation or
  protobuf-only contracts. Internal velocity is higher when the
  operator can directly import the same crates that define CRD types,
  Postgres clients, and metrics emitters.
- KUDO or Crossplane. Rejected — too much abstraction over a domain
  (Citus topology, shard placement, sidecar fleet) that requires
  imperative reconciliation logic.
- Bash + kubectl in a controller pod. Rejected on the obvious grounds.

## Consequences

- Positive: one Cargo workspace covers companion, sidecars, pool,
  operator, and tools. Crates like `pg_query`, `tokio-postgres`,
  `tracing`, `opentelemetry`, and the internal CRD types are shared
  end to end.
- Positive: `kube-rs`'s `Controller` builder and the `runtime::watcher`
  stream model produce reconcilers that are short, testable, and
  panic-safe by construction.
- Positive: cross-binary type sharing — a CRD struct generated with
  `schemars` is the same type the sidecar gRPC server consumes.
- Negative: smaller community than Go controller-runtime; some patterns
  (admission webhooks, conversion webhooks, finalizer machinery) have
  fewer worked examples. Mitigation: contribute back upstream as we
  build them.
- Negative: leader election in `kube-rs` is less ergonomic than in
  controller-runtime. We use the documented Lease-CR pattern.
- Risks: `kube-rs` major versions bump occasionally (we are on v3.x).
  Mitigation: pin the version in the workspace and update on a
  scheduled cadence with a separate sync PR.

## References

- Plan §6.5 (`operator/` — Rust kube-rs operator)
- Plan §2.7 (K8s operator landscape — Rust + kube-rs unoccupied)
- `kube-rs` reconciler API (`Controller`, `runtime::watcher`)
- CNPG `Cluster` CR — the substrate we layer on (see ADR 0006)
