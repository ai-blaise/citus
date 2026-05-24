# Disaster Recovery Runbook

`FEATURE: MR9`

Use this drill for region-loss readiness before every production release and
after any change to `Region`, `SurvivalGoal`, `Branch`, `Backup`, pool routing,
or `pgactive` conflict policy contracts.

This runbook is a release prerequisite and operational checklist, not
production evidence by itself. `FEATURE: MR9` remains alpha until a live
multi-region failover drill, PITR restore, backup artifact restore, sidecar
readiness check, and conflict-policy report are executed against real runtime
infrastructure and recorded with measured evidence.

## Inputs

- Target cluster and release version.
- Current `Region` and `SurvivalGoal` manifests.
- Latest successful encrypted backup and WAL archive checkpoint.
- Current pool GeoIP and CIDR access-control policy.
- Current `pgactive` reference-table conflict-policy report.

## Dry-run Command Checks

Before scheduling the live failover or restore drill, run the bounded checks
this repository can execute without touching production data:

```bash
bash ci/ai-blaise/runbook-command-check.sh
cargo run -q -p ai_blaise_citus_sidecar_hlc -- run-canonical
cargo run -q -p ai_blaise_citus_sidecar_raft -- run-canonical
cargo run -q -p ai_blaise_citus_sidecar_backup -- run-runtime-canonical
cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-operations-canonical
```

These commands validate runbook syntax and offline contracts only. They do not
replace measured regional failover, PITR, backup artifact restore, sidecar
readiness, or conflict-policy evidence from the live drill.

## Regional Failover Drill

1. Freeze schema changes and canary traffic.
2. Confirm `SurvivalGoal` status for the affected shard groups.
3. Verify closed-timestamp follower-read lag is within the configured budget.
4. Promote the surviving region leader pinning policy.
5. Rotate pool routing to the surviving region.
6. Reconcile tenant region affinity and tablespace mappings.
7. Re-enable canary traffic, then mirror production reads before writes.
8. Run conflict-policy reports for reference tables.
9. Verify realtime, storage, auth, API, vectorizer, and backup sidecars are
   ready in the surviving region.
10. Record the failover window, data-loss window, and follow-up actions.

## Restore Drill

1. Create a read-only branch from the latest backup-as-data-source checkpoint.
2. Run PITR to the requested timestamp.
3. Compare tenant counts, shard placements, ledger hash chains, and search
   index freshness against the source cluster.
4. Keep the restored branch read-only until application owners sign off.

## Machine-Verifiable Restore Depth Gate

Before a release can cite this runbook as DR readiness evidence, run the
restore-depth gate from the repo root:

```bash
REQUIRE_DOCKER=1 ci/ai-blaise/dr-restore-depth-check.sh
```

The gate fails closed unless the model covers a read-only branch before any
destructive restore, a destructive plan id for in-place restore, two distinct
operator approvals, KMS evidence, WAL archive continuity, PITR evidence, and
validation-query evidence. With Docker required, the same gate runs a real
PostgreSQL PITR smoke: it takes a `pg_basebackup`, archives WAL with
`archive_command`, restores to `recovery_target_time`, promotes the restored
cluster, and emits `dr_restore_depth_postgres_smoke` proving the before-target
row is present while the after-target row is absent. This is not production
evidence by itself; it is the executable minimum that keeps the checklist from
becoming only prose.

## Exit Criteria

- Region failover completes within the declared survival objective.
- No tenant crosses its declared region-affinity policy.
- Reference-table conflicts are either absent or classified by policy.
- Backup restore produces a read-only branch with verified PITR timestamp.
- The incident record links metrics, commands, manifests, and approvals.
- These exit criteria are required release evidence only after they are backed
  by real drill logs; completing the document checklist alone does not promote
  any disaster-recovery feature out of alpha.
