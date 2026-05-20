# Disaster Recovery Runbook

`FEATURE: MR9`

Use this drill for region-loss readiness before every production release and
after any change to `Region`, `SurvivalGoal`, `Branch`, `Backup`, pool routing,
or `pgactive` conflict policy contracts.

## Inputs

- Target cluster and release version.
- Current `Region` and `SurvivalGoal` manifests.
- Latest successful encrypted backup and WAL archive checkpoint.
- Current pool GeoIP and CIDR access-control policy.
- Current `pgactive` reference-table conflict-policy report.

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

## Exit Criteria

- Region failover completes within the declared survival objective.
- No tenant crosses its declared region-affinity policy.
- Reference-table conflicts are either absent or classified by policy.
- Backup restore produces a read-only branch with verified PITR timestamp.
- The incident record links metrics, commands, manifests, and approvals.
