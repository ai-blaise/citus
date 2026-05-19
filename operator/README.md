# operator

Rust `kube-rs` operator for Citus topology, CRDs, sidecars, and ai-blaise
feature orchestration.

The first implemented specs are `CitusCluster` for `FEATURE: S4` topology
selection, `ShardGroup` for `FEATURE: S2` placement policy, `Hypertable` for
`FEATURE: TS7`, `Branch` for `FEATURE: R2` / `FEATURE: C6` / `FEATURE: C7` /
`FEATURE: C8`, `Tenant` for `FEATURE: S10` / `FEATURE: TO1` / `FEATURE: TO2` /
`FEATURE: TO5`, `Region` for `FEATURE: MR1` / `FEATURE: MR4` /
`FEATURE: MR8`, `SurvivalGoal` for `FEATURE: S11` / `FEATURE: MR2`, and
`Backup` for `FEATURE: B2` / `FEATURE: B6`, plus the remaining V2 operator
catalog: `Vectorizer`, `Sidecar`, `Migration`, `ConflictPolicy`,
`Federation`, `SearchIndex`, `Webhook`, `Function`, and `ScheduledRepack`.
They validate the declarative
inputs needed to reconcile coordinator-worker versus coordinator-less layouts,
topology-aware placement, extension cohabitation, `TS1` distributed
hypertables, schema tenants, region-aware storage, survival targets, branch
storage, scale-to-zero intent, encrypted backup policy, vector destinations,
sidecar deployment resources, online migration DSL, conflict resolution, FDW
federation, hybrid search, outbound webhooks, edge functions, and online
repack scheduling.

`operator/src/reconcile/hypertable.rs` converts that spec into companion
planning types so the future reconciler has one typed execution plan to apply.
