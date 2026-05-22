# sidecar/backup

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Backup, WAL archive, PITR restore, and backup-as-data-source contracts.

Current implemented surface:

- `BackupJobPlan`
- `BaseBackupPlan`
- `WalArchivePlan`
- `PitrRestorePlan`
- `QueryableBackupBranchPlan`
- `BackupEncryptionPlan`
- `BackupRuntime`
- `BackupRuntimeState`
- `BackupArtifact`
- `canonical_backup_report()`
- `canonical_backup_runtime_report()`
- `cargo run -p ai_blaise_citus_sidecar_backup -- run-canonical`
- `cargo run -p ai_blaise_citus_sidecar_backup -- run-runtime-canonical`

These contracts cover `FEATURE: B1`, `FEATURE: B3`, `FEATURE: B4`, and
`FEATURE: B6`.

Backup and PITR coordinator built around WAL-G or pgBackRest. The runtime
surface deterministically models encrypted base backup execution, WAL segment
archival, PITR replay, and read-only backup branch mounting for canonical
tests.
