# Internal Modifications

This bootstrap commit records the fork policy and rebase anchor. Subsequent
bootstrap-v2 changes are summarized here; each modified subsystem also carries
its own adjacent `MODIFICATION.md`.

## 2026-05-22 — CDC sidecar + operator + gh-ost state machine runtime wiring

- `sidecar/cdc/` — New tokio-postgres logical-replication consumer with an
  async-nats sink and an axum probe server. Subjects published under
  `citus.cdc.<schema>.<table>` with NATS headers carrying `tx_xid`, `lsn`,
  `op_type`, and source label. `cdc serve` boots the probe server, optionally
  connects NATS + replication targets from env, and shuts down on SIGTERM.
- `operator/` — New `kube-rs` `Controller` reconcilers for `CitusCluster`,
  `Migration`, `Tenant`, and `Hypertable` running concurrently under a single
  tokio runtime, with a dedicated probe thread for readiness. CR specs mirror
  the validated `*Spec` types and are translated into the authoritative
  validator before action.
- `operator/src/crds/migration/` — Promoted to a directory module exposing the
  `MigrationPhase` lifecycle enum and the gh-ost-style `state_machine` with
  per-phase evidence guards.
- `companion/src/migration.rs` — Replaced the stub `companion_internal.*`
  emitters with real `citus_admin.*` invocations covering every gh-ost phase
  (`shadow_table_create`, `install_write_triggers`, `backfill_run`,
  `row_diff_verify`, `shadow_table_publish`, plus the
  `migrate_start`/`migrate_complete` envelopes).

Unblocks PLAN §2.4 (sidecar runtime) and §11.7 (gh-ost cut-over executor).

Future bootstrap-v2 changes must document the modified subsystem here or in a
nearer `MODIFICATION.md`, include regression coverage, and preserve the
platform rule that `ai-blaise/platform` never imports Citus source code.
