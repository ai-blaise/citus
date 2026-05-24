# operator Modifications

## 2026-05-22 — kube-rs controllers + Migration phase state machine

Added under `serve`:

- `src/controllers/` — `Controller` reconcilers for `CitusCluster`,
  `Migration`, `Tenant`, and `Hypertable`. Each module declares a
  `kube::CustomResource`-derived CR spec that mirrors the validated `*Spec`
  type in `crate::crds`. The reconciler converts the CR view into the
  authoritative spec, calls `validate()`, and (for Hypertable) drives
  `HypertableReconcilePlan::try_from` to materialize the companion SQL apply
  plan. `controllers::serve_all` spawns all four reconcilers on a single
  tokio runtime and returns when any exits.
- `src/crds/migration/` — Promoted to a directory module.
  - `mod.rs` retains the `MigrationSpec`, `MigrationType`, and
    `MigrationConflictAction` types plus the new `MigrationPhase` lifecycle
    enum (`DeleteOnly → WriteOnly → Backfill → Public → Complete`).
  - `state_machine.rs` implements gh-ost-style `transition(current, evidence)`
    with per-phase guards:
    - `DeleteOnly → WriteOnly` requires `shadow_table_built`.
    - `WriteOnly → Backfill` requires `write_triggers_installed`.
    - `Backfill → Public` requires `backfill_complete &&
      row_diff_verified`.
    - `Public → Complete` is unconditional.
    Evidence regression at any guard returns
    `StateMachineError::EvidenceRegressed`.
- `src/main.rs` — `operator serve` now boots a dedicated probe thread plus a
  multi-thread tokio runtime that calls `controllers::serve_all`. If no
  in-cluster kube config is available, the operator surfaces NotReady via the
  probe rather than crash-looping. `run-canonical` is unchanged.

`Cargo.toml` switches `k8s-openapi` off the `latest` feature and onto
`v1_30 + schemars` to keep the binary linker artifacts under control while
still pulling the `kube::CustomResource` derive surface.

Regression coverage: unit tests in `controllers::citus_cluster::tests` cover
CR-spec round-tripping, and `crds::migration::state_machine::tests` cover
every phase guard + the evidence-regression error.

## 2026-05-23 — Reconcilers batch B

Added production-ready operator plan-builders and kube-rs controller mirrors for
`Federation`, `SearchIndex`, `Webhook`, and `Function`. The plan-builders live
under `src/reconcile/` and render deterministic apply steps for FDW/Iceberg
federation intent, distributed pg_search metadata, companion webhook trigger
registration, and edge-function sidecar/Kubernetes trigger registration.

`src/controllers/` now includes matching controller modules that parse the
Kubernetes CR shape into the authoritative CRD specs, validate them, and build
the same reconcile plans used by the canonical runner. `controllers::serve_all`
spawns these four controllers alongside the existing CitusCluster, Migration,
Tenant, and Hypertable controllers.

`operator run-reconcilers-batch-b` emits a canonical TSV proof row for the batch:
`4 true 5 true 6 2 6 1 2` for federation steps/iceberg, search steps/hybrid,
webhook steps/events, and function steps by target kind. SQL mutation execution
and CRD `.status` writes remain outside this batch unless a feature entry claims
them explicitly.
