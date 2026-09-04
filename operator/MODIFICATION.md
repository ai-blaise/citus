# operator Modifications

## 2026-09-04 — Production-bounded CitusCluster reconciliation

Implemented the coordinator-worker `CitusCluster` apply path behind `FEATURE:
S4`. Apply mode now fails closed unless the operand image is digest-pinned and
the CR supplies exact extension versions, PostgreSQL identity/storage values,
at least two logical worker groups, bounded bootstrap retry settings, and exact
`sslmode=verify-full` node transport configuration. The reconciler validates
the referenced CA, server-certificate, private-key, and superuser Secret shape
before applying children; Secret resource-version rotation creates a new
immutable reconcile revision.

The production hardening pass additionally makes immutable revisions depend on
the CR generation and rendered CNPG, script, and Job contracts; verifies CNPG's
healthy/Ready condition, running image digest, and PVC-resize completion before
bootstrap; rejects DNS-unsafe derived names; and prevents force-adoption with
owner-UID plus UID/resource-version preconditions. Older runnable bootstrap
Jobs are foreground-deleted before a replacement can start, while successful
cleanup inventories every owned hash-named Job and ConfigMap so crashes cannot
forget superseded generations. CNPG contraction checks also inventory the live
owner-UID children, so a crash before a status write cannot hide an existing
group. Production planning, Secret extraction, and YAML rendering return typed
errors rather than relying on validated-input panics, including the otherwise
valid planning shape of coordinator-less topology with production inputs.
Because the published CNPG v1 Cluster status has no observed-generation
acknowledgement, post-create changes to storage size/class, PostgreSQL major,
PostgreSQL UID/GID, and initdb database fail closed before any child mutation;
they require the reviewed reprovision/data-migration path. Any future CNPG
observed-generation field is checked when present. Secret hashing also requires
a real API `resourceVersion` rather than collapsing malformed input into a
placeholder revision. The deletion finalizer also remains installed until all
still-runnable bootstrap Jobs have been foreground-deleted, preventing a writer
from outliving its CitusCluster deletion request.
Exact extension-version changes also fail before CNPG mutation: this bootstrap
is an exact install/verification contract, not an implicit `ALTER EXTENSION`
upgrade mechanism.
Because CNPG also exposes no certificate-reload acknowledgement, readiness now
enumerates every Ready instance Pod by the exact applied Cluster owner UID and
performs a direct PostgreSQL TLS negotiation to its Pod IP. CA/SNI validation
and an exact peer-leaf SHA-256 match against the current server Secret are both
required, so a same-CA leaf rotation cannot leave a primary or failover
candidate stale while bootstrap or Ready proceeds. The operator RBAC contract
therefore includes namespace-scoped Pod `get`/`list` access and requires
operator-to-instance-Pod TCP 5432 egress. Only literal parsed Pod IPs are used
as socket targets; API-provided status text cannot trigger DNS resolution.

The controller server-side applies an owner-referenced CNPG `ImageCatalog`, a
distinct coordinator CNPG cluster, and one CNPG cluster per worker group. It
waits for every desired CNPG instance, then runs a digest-pinned, non-root,
read-only, bounded bootstrap Job from an immutable ConfigMap. The Job installs
and verifies exact extension versions on every node/database, writes the
password-only Citus auth boundary, registers workers, starts metadata sync,
proves TLS on administrator and worker transport, and compares normalized
`pg_dist_node` catalogs. Before writing, it polls every endpoint for the exact
password/TLS/node-conninfo revision. Its exact-topology guard also requires
every expected node name and positive worker group ID to occur distinctly and
requires `hasmetadata` plus `metadatasynced`, so duplicate or stale rows cannot
conceal a missing worker. Status records `observedGeneration`, reconcile hash,
per-cluster readiness, expected versions, exact node conninfo, errors, and
conditions. Unsafe worker-group removal fails closed pending shard evacuation;
superseded Jobs and ConfigMaps are removed only after replacement success and
owner-reference verification.

Regression coverage includes strict spec validation, exact CNPG JSON, bootstrap
script and Job security/bounds, status readiness gating, Secret-rotation hash,
unsafe group-removal rejection, CRD rendering, boundary smoke, and the full
operator test suite. `operator/CITUS_CLUSTER_PRODUCTION.md` records the
deployment/RBAC contract and the external live promotion evidence that source
tests cannot supply.

The same hardening pass upgraded the operator to `kube`/`kube-runtime` 4.2,
`k8s-openapi` 0.28 (Kubernetes 1.35 bindings), and `schemars` 1.2. The resolved
operator graph also carries fixed `event-listener` 5.4.2 and
`chacha20`/`rand` 0.10.2 releases. None of the workspace audit advisories or
unmaintained-package warnings are reachable from the operator package. The
analytical sidecar moved to DataFusion 55 to remove `paste`; the single narrow
workspace exception is the pgrx extension's upstream-only `serde_cbor` warning,
documented and mechanically contained by the warnings-denied audit policy.


## 2026-05-24 — Sidecar controller live apply contract

Promoted the bounded O5 sidecar deployment controller path from deterministic
planning to live Kubernetes apply. `Sidecar` CRs now accept an optional
`spec.image`; apply mode requires that image to be immutable and digest-pinned,
then server-side applies the generated Deployment and Service with owner
references and patches `sidecars/status`. The `print-sidecar-crd` command emits
the live CRD for kind smoke tests, and `AI_BLAISE_OPERATOR_CONTROLLERS=sidecar`
lets the operator run only the Sidecar controller under narrow RBAC for focused
production evidence.

Regression coverage: unit tests cover digest-pinned image validation and apply
metadata injection. `ci/ai-blaise/sidecar-controller-live-smoke.sh` builds real
operator and realtime sidecar containers, pushes digest-pinned images to a
local registry, runs a kind cluster, verifies generated Deployment/Service
resources and status, probes `/healthz`, `/readyz`, and `/metrics` through the
Service, and proves mutable tags are rejected before Deployment creation.

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
