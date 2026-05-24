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
`ConflictPolicy`, and `Sidecar`. They build typed reconcile/apply plans and log the intended work;
they do not yet mutate Kubernetes status or execute companion SQL directly from
the operator. SQL execution remains delegated to the companion/sidecar boundary
called out in the feature docs.

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
`ci/ai-blaise/operator-boundary-smoke.sh` proves apply mode fails closed while
Kubernetes apply, direct SQL execution, and `.status` mutation are still alpha.

`cargo run -p ai_blaise_citus_operator -- run-security-canonical` validates the
operator-owned security boundary for generated operator, pool, built-in
sidecar, and custom sidecar workloads. The runner fails closed on inline
secrets, Secret API RBAC, wildcard RBAC, weak TLS settings, auth policies that
do not fail closed, root or privilege-escalating containers, writable root
filesystems, missing `RuntimeDefault` seccomp, and retained Linux capabilities.
This is model/smoke evidence only; live certificate issuance, mounts, and mTLS
traffic rotation remain release-gated elsewhere.
