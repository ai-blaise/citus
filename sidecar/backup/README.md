# sidecar/backup

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Backup, WAL archive, PITR restore, and backup-as-data-source runtime.

Production boundary: the sidecar owns WAL-G command orchestration, HTTP
status/control endpoints, retention pruning, scheduled backup cycles, PITR job
tracking, queryable read-only branch materialization, and backup-specific
metrics. Cloud object-store credentials, External Secrets, Backup CR
reconciliation, and full Kubernetes restore drills are outside this crate and
must be proven separately before those broader workflows are promoted.

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
- `BackupEngine`
- `WalgRunner`
- `BackupSchedule`
- `QueryableBranchRunner`
- HTTP: `/backups`, `/backups/run`, `/backups/status`, `/backups/delete-old`
- HTTP: `/wal/status`, `/pitr/restore`, `/pitr/status/<job_id>`
- HTTP: `/branches/queryable`, `/healthz`, `/readyz`, `/metrics`
- `canonical_backup_report()`
- `canonical_backup_runtime_report()`
- `cargo run -p ai_blaise_citus_sidecar_backup -- run-canonical`
- `cargo run -p ai_blaise_citus_sidecar_backup -- run-runtime-canonical`

These contracts cover `FEATURE: B1`, `FEATURE: B3`, `FEATURE: B4`, and
`FEATURE: B6`.

Backup and PITR coordinator built around WAL-G. The runtime executes WAL-G for
base backups, WAL archive status, PITR restore, retention pruning, and backup
listing. Queryable branches are restored into sidecar-owned data directories,
written with recovery/read-only PostgreSQL configuration, started through
`pg_ctl`, and probed through `psql` before they are recorded as mounted.

Verification:

- `cargo test -p ai_blaise_citus_sidecar_backup`
- `bash ci/ai-blaise/sidecar-backup-smoke.sh`
- `cargo run -q -p ai_blaise_citus_sidecar_backup -- serve`
