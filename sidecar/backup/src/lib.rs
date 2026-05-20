//! Backup sidecar contracts.

// FEATURE: B1
// FEATURE: B3
// FEATURE: B4
// FEATURE: B6

use ai_blaise_citus_sidecar_shared::{BackupRestoreContract, SidecarContractError};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupJobPlan {
    pub cluster: String,
    pub contract: BackupRestoreContract,
    pub base_backup: BaseBackupPlan,
    pub wal_archive: WalArchivePlan,
    pub encryption: Option<BackupEncryptionPlan>,
}

impl BackupJobPlan {
    pub fn validate(&self) -> Result<(), BackupSidecarError> {
        validate_required("cluster", &self.cluster)?;
        self.contract.validate()?;
        self.base_backup.validate()?;
        self.wal_archive.validate()?;
        if let Some(encryption) = &self.encryption {
            encryption.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BaseBackupPlan {
    pub destination_uri: String,
    pub retention_days: u32,
    pub concurrency: u32,
}

impl BaseBackupPlan {
    fn validate(&self) -> Result<(), BackupSidecarError> {
        validate_uri("base_backup.destination_uri", &self.destination_uri)?;
        if self.retention_days == 0 {
            return Err(BackupSidecarError::InvalidRetention);
        }
        if self.concurrency == 0 {
            return Err(BackupSidecarError::InvalidConcurrency);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WalArchivePlan {
    pub slot_name: String,
    pub archive_uri: String,
    pub compression: WalCompression,
}

impl WalArchivePlan {
    fn validate(&self) -> Result<(), BackupSidecarError> {
        validate_required("wal.slot_name", &self.slot_name)?;
        validate_uri("wal.archive_uri", &self.archive_uri)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum WalCompression {
    None,
    Gzip,
    Zstd,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupEncryptionPlan {
    pub kms_key_ref: String,
}

impl BackupEncryptionPlan {
    fn validate(&self) -> Result<(), BackupSidecarError> {
        validate_required("encryption.kms_key_ref", &self.kms_key_ref)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PitrRestorePlan {
    pub cluster: String,
    pub source_archive_uri: String,
    pub target_time: String,
    pub target_cluster: String,
}

impl PitrRestorePlan {
    pub fn validate(&self) -> Result<(), BackupSidecarError> {
        validate_required("cluster", &self.cluster)?;
        validate_uri("source_archive_uri", &self.source_archive_uri)?;
        validate_timestamp(&self.target_time)?;
        validate_required("target_cluster", &self.target_cluster)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct QueryableBackupBranchPlan {
    pub branch_name: String,
    pub source_archive_uri: String,
    pub target_time: String,
    pub read_only: bool,
}

impl QueryableBackupBranchPlan {
    pub fn validate(&self) -> Result<(), BackupSidecarError> {
        validate_required("branch_name", &self.branch_name)?;
        validate_uri("source_archive_uri", &self.source_archive_uri)?;
        validate_timestamp(&self.target_time)?;
        if !self.read_only {
            return Err(BackupSidecarError::QueryableBranchMustBeReadOnly);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BackupSidecarError {
    ArchiveMismatch,
    InvalidConcurrency,
    InvalidRetention,
    InvalidTimestamp,
    InvalidUri(&'static str),
    MissingRequiredField(&'static str),
    QueryableBranchMustBeReadOnly,
    SharedContract(String),
}

impl fmt::Display for BackupSidecarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArchiveMismatch => {
                write!(
                    formatter,
                    "restore or branch archive does not match backup job"
                )
            }
            Self::InvalidConcurrency => write!(formatter, "concurrency must be greater than zero"),
            Self::InvalidRetention => write!(formatter, "retention_days must be greater than zero"),
            Self::InvalidTimestamp => {
                write!(formatter, "target_time must be an RFC3339 UTC timestamp")
            }
            Self::InvalidUri(field) => write!(formatter, "{field} must be an object-store URI"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::QueryableBranchMustBeReadOnly => {
                write!(formatter, "queryable backup branches must be read-only")
            }
            Self::SharedContract(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for BackupSidecarError {}

impl From<SidecarContractError> for BackupSidecarError {
    fn from(error: SidecarContractError) -> Self {
        Self::SharedContract(error.to_string())
    }
}

fn validate_required(field: &'static str, value: &str) -> Result<(), BackupSidecarError> {
    if value.trim().is_empty() {
        return Err(BackupSidecarError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_uri(field: &'static str, value: &str) -> Result<(), BackupSidecarError> {
    validate_required(field, value)?;
    if value.starts_with("s3://") || value.starts_with("gs://") || value.starts_with("az://") {
        Ok(())
    } else {
        Err(BackupSidecarError::InvalidUri(field))
    }
}

fn validate_timestamp(value: &str) -> Result<(), BackupSidecarError> {
    validate_required("target_time", value)?;
    if value.len() >= 20 && value.contains('T') && value.ends_with('Z') {
        Ok(())
    } else {
        Err(BackupSidecarError::InvalidTimestamp)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupCanonicalReport {
    pub job: BackupJobPlan,
    pub restore: PitrRestorePlan,
    pub queryable_branch: QueryableBackupBranchPlan,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupArtifact {
    pub cluster: String,
    pub base_destination_uri: String,
    pub wal_archive_uri: String,
    pub base_size_bytes: u64,
    pub wal_segments: u32,
    pub encrypted: bool,
    pub retention_days: u32,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PitrRestoreResult {
    pub source_cluster: String,
    pub target_cluster: String,
    pub target_time: String,
    pub replayed_wal_segments: u32,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct QueryableBranchResult {
    pub branch_name: String,
    pub mounted_archive_uri: String,
    pub target_time: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupRuntimeState {
    pub completed_base_backups: u64,
    pub archived_wal_segments: u64,
    pub pitr_restores: u64,
    pub queryable_branches: u64,
    pub encrypted_artifacts: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupRuntimeReport {
    pub backup: BackupArtifact,
    pub restore: PitrRestoreResult,
    pub queryable_branch: QueryableBranchResult,
    pub state: BackupRuntimeState,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupRuntime {
    job: BackupJobPlan,
    state: BackupRuntimeState,
}

impl BackupRuntime {
    pub fn new(job: BackupJobPlan) -> Result<Self, BackupSidecarError> {
        job.validate()?;

        Ok(Self {
            job,
            state: BackupRuntimeState {
                completed_base_backups: 0,
                archived_wal_segments: 0,
                pitr_restores: 0,
                queryable_branches: 0,
                encrypted_artifacts: 0,
            },
        })
    }

    pub fn state(&self) -> &BackupRuntimeState {
        &self.state
    }

    pub fn run_backup_cycle(&mut self) -> Result<BackupArtifact, BackupSidecarError> {
        self.job.validate()?;
        let wal_segments = deterministic_wal_segments(&self.job);
        let encrypted = self.job.encryption.is_some();
        let encrypted_artifacts = if encrypted {
            1_u64 + u64::from(wal_segments)
        } else {
            0
        };

        self.state.completed_base_backups += 1;
        self.state.archived_wal_segments += u64::from(wal_segments);
        self.state.encrypted_artifacts += encrypted_artifacts;

        Ok(BackupArtifact {
            cluster: self.job.cluster.clone(),
            base_destination_uri: self.job.base_backup.destination_uri.clone(),
            wal_archive_uri: self.job.wal_archive.archive_uri.clone(),
            base_size_bytes: deterministic_base_size_bytes(&self.job),
            wal_segments,
            encrypted,
            retention_days: self.job.base_backup.retention_days,
        })
    }

    pub fn restore_pitr(
        &mut self,
        plan: &PitrRestorePlan,
    ) -> Result<PitrRestoreResult, BackupSidecarError> {
        plan.validate()?;
        self.ensure_archive_matches(&plan.source_archive_uri)?;
        let replayed_wal_segments = deterministic_wal_segments(&self.job);
        self.state.pitr_restores += 1;

        Ok(PitrRestoreResult {
            source_cluster: plan.cluster.clone(),
            target_cluster: plan.target_cluster.clone(),
            target_time: plan.target_time.clone(),
            replayed_wal_segments,
        })
    }

    pub fn mount_queryable_branch(
        &mut self,
        plan: &QueryableBackupBranchPlan,
    ) -> Result<QueryableBranchResult, BackupSidecarError> {
        plan.validate()?;
        self.ensure_archive_matches(&plan.source_archive_uri)?;
        self.state.queryable_branches += 1;

        Ok(QueryableBranchResult {
            branch_name: plan.branch_name.clone(),
            mounted_archive_uri: plan.source_archive_uri.clone(),
            target_time: plan.target_time.clone(),
            read_only: plan.read_only,
        })
    }

    fn ensure_archive_matches(&self, archive_uri: &str) -> Result<(), BackupSidecarError> {
        if archive_uri == self.job.contract.archive_uri {
            Ok(())
        } else {
            Err(BackupSidecarError::ArchiveMismatch)
        }
    }
}

fn deterministic_wal_segments(job: &BackupJobPlan) -> u32 {
    (job.base_backup.retention_days / 10).max(1)
}

fn deterministic_base_size_bytes(job: &BackupJobPlan) -> u64 {
    u64::from(job.base_backup.concurrency) * 1_048_576
}

pub fn canonical_backup_job() -> BackupJobPlan {
    BackupJobPlan {
        cluster: "prod".to_string(),
        contract: BackupRestoreContract {
            schedule: "0 */6 * * *".to_string(),
            archive_uri: "s3://backups/prod".to_string(),
            pitr_target: Some("2026-05-19T12:00:00Z".to_string()),
            queryable_branch_name: Some("prod-at-noon".to_string()),
        },
        base_backup: BaseBackupPlan {
            destination_uri: "s3://backups/prod/base".to_string(),
            retention_days: 30,
            concurrency: 2,
        },
        wal_archive: WalArchivePlan {
            slot_name: "ai_blaise_wal".to_string(),
            archive_uri: "s3://backups/prod/wal".to_string(),
            compression: WalCompression::Zstd,
        },
        encryption: Some(BackupEncryptionPlan {
            kms_key_ref: "aws-kms-prod".to_string(),
        }),
    }
}

pub fn canonical_pitr_restore_plan() -> PitrRestorePlan {
    PitrRestorePlan {
        cluster: "prod".to_string(),
        source_archive_uri: "s3://backups/prod".to_string(),
        target_time: "2026-05-19T12:00:00Z".to_string(),
        target_cluster: "restore-prod".to_string(),
    }
}

pub fn canonical_queryable_branch_plan() -> QueryableBackupBranchPlan {
    QueryableBackupBranchPlan {
        branch_name: "prod-at-noon".to_string(),
        source_archive_uri: "s3://backups/prod".to_string(),
        target_time: "2026-05-19T12:00:00Z".to_string(),
        read_only: true,
    }
}

pub fn canonical_backup_report() -> Result<BackupCanonicalReport, BackupSidecarError> {
    let job = canonical_backup_job();
    let restore = canonical_pitr_restore_plan();
    let queryable_branch = canonical_queryable_branch_plan();

    job.validate()?;
    restore.validate()?;
    queryable_branch.validate()?;

    Ok(BackupCanonicalReport {
        job,
        restore,
        queryable_branch,
    })
}

pub fn canonical_backup_runtime_report() -> Result<BackupRuntimeReport, BackupSidecarError> {
    let mut runtime = BackupRuntime::new(canonical_backup_job())?;
    let backup = runtime.run_backup_cycle()?;
    let restore = runtime.restore_pitr(&canonical_pitr_restore_plan())?;
    let queryable_branch = runtime.mount_queryable_branch(&canonical_queryable_branch_plan())?;

    Ok(BackupRuntimeReport {
        backup,
        restore,
        queryable_branch,
        state: runtime.state().clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_job_plan_validates_base_and_wal_archive() {
        assert_eq!(canonical_backup_job().validate(), Ok(()));
    }

    #[test]
    fn canonical_backup_report_is_deterministic() {
        let report = canonical_backup_report().expect("canonical report");

        assert_eq!(report.job.cluster, "prod");
        assert_eq!(report.restore.target_cluster, "restore-prod");
        assert_eq!(report.queryable_branch.branch_name, "prod-at-noon");
    }

    #[test]
    fn backup_runtime_runs_encrypted_backup_restore_and_branch() {
        let report = canonical_backup_runtime_report().expect("runtime report");

        assert_eq!(report.backup.cluster, "prod");
        assert_eq!(report.backup.base_size_bytes, 2_097_152);
        assert_eq!(report.backup.wal_segments, 3);
        assert!(report.backup.encrypted);
        assert_eq!(report.restore.target_cluster, "restore-prod");
        assert_eq!(report.restore.replayed_wal_segments, 3);
        assert_eq!(report.queryable_branch.branch_name, "prod-at-noon");
        assert!(report.queryable_branch.read_only);
        assert_eq!(report.state.completed_base_backups, 1);
        assert_eq!(report.state.archived_wal_segments, 3);
        assert_eq!(report.state.pitr_restores, 1);
        assert_eq!(report.state.queryable_branches, 1);
        assert_eq!(report.state.encrypted_artifacts, 4);
    }

    #[test]
    fn backup_runtime_rejects_restore_from_wrong_archive() {
        let mut runtime = BackupRuntime::new(canonical_backup_job()).expect("runtime");
        let mut restore = canonical_pitr_restore_plan();
        restore.source_archive_uri = "s3://backups/other".to_string();

        assert_eq!(
            runtime.restore_pitr(&restore),
            Err(BackupSidecarError::ArchiveMismatch)
        );
    }

    #[test]
    fn pitr_restore_requires_utc_timestamp() {
        let mut restore = canonical_pitr_restore_plan();
        restore.target_time = "2026-05-19 12:00:00".to_string();

        assert_eq!(
            restore.validate(),
            Err(BackupSidecarError::InvalidTimestamp)
        );
    }

    #[test]
    fn queryable_branch_must_be_read_only() {
        let mut branch = canonical_queryable_branch_plan();
        branch.read_only = false;

        assert_eq!(
            branch.validate(),
            Err(BackupSidecarError::QueryableBranchMustBeReadOnly)
        );
    }

    #[test]
    fn backup_job_rejects_missing_kms_key() {
        let mut job = canonical_backup_job();
        job.encryption = Some(BackupEncryptionPlan {
            kms_key_ref: " ".to_string(),
        });

        assert_eq!(
            job.validate(),
            Err(BackupSidecarError::MissingRequiredField(
                "encryption.kms_key_ref"
            ))
        );
    }
}
