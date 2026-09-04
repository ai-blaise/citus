# operator

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Rust operator contract model for Citus topology, CRDs, sidecars, and ai-blaise
feature orchestration. The `serve` path starts the shared
health/readiness/metrics runtime and, when Kubernetes client configuration is
available, starts kube-rs watch loops for `CitusCluster`, `Migration`,
`Tenant`, `Region`, `SurvivalGoal`, `Backup`, `Hypertable`, `Federation`,
`SearchIndex`, `Webhook`, `Function`, `ScheduledRepack`, `ConflictPolicy`, and
`Sidecar`. Without a cluster client it keeps the probe runtime up and logs the
controller startup failure instead of claiming a live reconciliation surface.

Implemented kube-rs controller modules currently cover `CitusCluster`,
`Migration`, `Tenant`, `Region`, `SurvivalGoal`, `Backup`, `Hypertable`,
`Federation`, `SearchIndex`, `Webhook`, `Function`, `ScheduledRepack`,
`ConflictPolicy`, and `Sidecar`. Most build typed reconcile/apply plans and log
the intended work. `CitusCluster`, `Hypertable`, and `Sidecar` have bounded live
apply paths with status mutation. SQL execution remains delegated to a bounded
Job or companion/sidecar boundary called out in the feature docs.

The production `CitusCluster` path is deliberately coordinator-worker only. It
requires a digest-pinned operand image, exact Citus and companion extension
versions, explicit PostgreSQL UID/GID and storage, at least two worker groups,
and pre-provisioned CA, CA-signed server TLS, and superuser Secrets. It server-side
manages one CNPG coordinator group, one CNPG cluster per logical worker group,
and a namespaced CNPG `ImageCatalog`. After every CNPG group reports all desired
instances ready on the requested image in CNPG's healthy/Ready state with no
PVC resize pending, the controller directly proves that every exact
owner-UID instance Pod serves the current server Secret leaf under verify-full
TLS. An immutable, hash-named, bounded Job then first proves the live
password, verify-full TLS, and exact node configuration, then installs and verifies
the exact extensions, configures `pg_dist_authinfo`, registers workers, checks
TLS transport, and proves byte-equal `pg_dist_node` topology. `.status` records
the reconcile revision (including CR generation, rendered apply contracts, and
referenced Secret resource versions),
`observedGeneration`, CNPG readiness, bootstrap Job, expected versions, exact
`citus.node_conninfo`, errors, and conditions. CNPG-group removal fails closed
until a separate shard-evacuation workflow supplies evidence; both status and
the live owner-UID CNPG inventory participate in that guard. Owner UID and
optimistic preconditions prevent force-adoption, and older runnable bootstrap
generations are foreground-quiesced before a replacement starts. See
`operator/CITUS_CLUSTER_PRODUCTION.md` for the CR, certificate, RBAC, and
promotion-evidence contracts. This bounded path accepts exactly `citus` and
`ai_blaise_citus`; TimescaleDB cohabitation remains on its separately evidenced
controller/runtime path. CNPG v1 does not publish observed generation for its
Cluster status, so storage/class, PostgreSQL major/UID/GID, and initdb-database
changes are fail-closed after create and require reviewed reprovision/migration.
Extension-version changes likewise require the reviewed upgrade path rather
than being inferred from an idempotent bootstrap verification Job.

The implemented specs are `CitusCluster` for `FEATURE: S4` topology
selection, `ShardGroup` for `FEATURE: S2` placement policy, `Hypertable` for
`FEATURE: TS7`, `Branch` for `FEATURE: R2` / `FEATURE: C6` / `FEATURE: C7` /
`FEATURE: C8`, `Tenant` for `FEATURE: S10` / `FEATURE: TO1` / `FEATURE: TO2` /
`FEATURE: TO5`, `Region` for `FEATURE: MR1` / `FEATURE: MR4` /
`FEATURE: MR8`, `SurvivalGoal` for `FEATURE: S11` / `FEATURE: MR2`, and
`Backup` for `FEATURE: B2` / `FEATURE: B6`, plus the remaining V2 operator
catalog: `Vectorizer`, `Sidecar`, `Migration`, `ConflictPolicy`,
`Federation`, `SearchIndex`, `Webhook`, `Function`, and `ScheduledRepack`.
They validate the declarative
inputs needed to plan coordinator-worker versus coordinator-less layouts,
topology-aware placement, extension cohabitation, `TS1` distributed
hypertables, schema tenants, region-aware storage, survival targets, branch
storage, scale-to-zero intent, encrypted backup policy, vector destinations,
sidecar deployment resources, online migration DSL, conflict resolution, FDW
federation, hybrid search, outbound webhooks, edge functions, and online
repack scheduling.

`operator/src/reconcile/hypertable.rs` converts that spec into a typed apply
plan. The plan creates the `ai_blaise_citus` companion extension, checks
`companion_feature_status()` for executable features, verifies
`citus.cohabit_extensions` contains `timescaledb`, then runs the ordered
companion SQL for distributed hypertables, policies, continuous aggregates, and
time-range shard pruning.

Reconcilers Batch A adds `operator/src/reconcile/{tenant,region,survival_goal,backup}.rs`.
Those modules produce the tenant schema/quota/config/archive plan, region
tablespace/affinity/leader-pin plan, survival-goal topology/replication policy
plan, and backup sidecar/config/KMS/status plan. The companion smoke is
`ci/ai-blaise/operator-reconcilers-batch-a-smoke.sh`.

Batch B reconciler plan-builders live in `operator/src/reconcile/federation.rs`,
`operator/src/reconcile/search_index.rs`, `operator/src/reconcile/webhook.rs`,
and `operator/src/reconcile/function.rs`. They render deterministic apply steps
for FDW/Iceberg federation intent, pg_search/hybrid-search metadata, companion
webhook trigger registration, and edge-function sidecar/Kubernetes trigger
registration. Their kube-rs controllers mirror live CRs into the same
authoritative specs and plan-builders during `serve`.

`cargo run -p ai_blaise_citus_operator -- run-canonical` validates the
canonical V2 operator surface and emits the stable deterministic TSV summary
for repository closure checks. `cargo run -p ai_blaise_citus_operator --
run-reconcilers-batch-a` emits the deterministic Reconcilers Batch A evidence
row without changing the closure-contract TSV. `cargo run -p
ai_blaise_citus_operator -- run-reconcilers-batch-b` emits the canonical
Federation/SearchIndex/Webhook/Function reconciler TSV row.

`cargo run -p ai_blaise_citus_operator -- run-reconcile-plans-batch-c` emits a
deterministic TSV summary for the Batch C reconcile plans: scheduled repack,
online migration/schema job handoff, replication conflict policy, and sidecar
deployment/deletion planning. `ci/ai-blaise/operator-reconcilers-batch-c-smoke.sh`
guards that contract in addition to the Rust tests.

`cargo run -p ai_blaise_citus_operator -- run-controller-boundary` emits typed
controller boundary Conditions for dry-run mode, and
`ci/ai-blaise/operator-boundary-smoke.sh` guards which controller operations are
implemented. The `CitusCluster` row includes Kubernetes apply and status
mutation; unsupported direct SQL remains fail-closed behind the bootstrap Job
boundary.

`ci/ai-blaise/sidecar-controller-live-smoke.sh` is the live O5 apply-mode gate.
It builds real operator and realtime sidecar containers, pushes digest-pinned
image references to a local OCI registry, applies the generated `Sidecar` CRD,
runs only the Sidecar controller with scoped RBAC, and verifies generated
Deployment/Service/status resources plus `/healthz`, `/readyz`, and `/metrics`
through the generated Service. Apply mode requires `spec.image` to be an
immutable `@sha256` reference and rejects mutable tags before creating a
Deployment.

`cargo run -p ai_blaise_citus_operator -- run-security-canonical` validates the
operator-owned security boundary for generated operator, pool, built-in
sidecar, and custom sidecar workloads. The runner fails closed on inline
secrets, Secret API listing or mutation, wildcard RBAC, weak TLS settings, auth policies that
do not fail closed, root or privilege-escalating containers, writable root
filesystems, missing `RuntimeDefault` seccomp, and retained Linux capabilities.
This is model/smoke evidence only; live certificate issuance, mounts, and mTLS
traffic rotation remain release-gated elsewhere.
