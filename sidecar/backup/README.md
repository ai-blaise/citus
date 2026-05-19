# sidecar/backup

Backup, WAL archive, PITR restore, and backup-as-data-source contracts.

Current implemented surface:

- `BackupJobPlan`
- `BaseBackupPlan`
- `WalArchivePlan`
- `PitrRestorePlan`
- `QueryableBackupBranchPlan`
- `BackupEncryptionPlan`

These contracts cover `FEATURE: B1`, `FEATURE: B3`, `FEATURE: B4`, and
`FEATURE: B6`.

Backup and PITR coordinator built around WAL-G or pgBackRest.
