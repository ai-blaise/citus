# operator

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Rust operator contract model for Citus topology, CRDs, sidecars, and ai-blaise
feature orchestration. The current production `serve` path exposes only the
shared health/readiness/metrics runtime; live Kubernetes watches, CRD status
updates, and controller reconciliation remain alpha until a real controller is
implemented and live-gated.

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

`cargo run -p ai_blaise_citus_operator -- run-canonical` validates the
canonical V2 operator surface and emits a deterministic TSV summary covering
`CitusCluster`, `ShardGroup`, `Hypertable`, the hypertable apply plan, and the
operator catalog CRDs.

`cargo run -p ai_blaise_citus_operator -- run-controller-boundary` renders the
typed controller execution boundary. The default mode is `dry-run`: specs are
validated and plans are rendered, while Kubernetes apply, direct SQL execution,
and `.status` mutation are emitted as `AlphaNotImplemented` conditions. Setting
`AI_BLAISE_OPERATOR_EXECUTION_MODE=apply` fails closed until those mutating
paths have real runners, RBAC/status-subresource support, idempotency, and live
evidence. `ci/ai-blaise/operator-boundary-smoke.sh` locks the dry-run TSV and
the apply-mode failure.
