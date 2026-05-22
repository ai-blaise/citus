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
