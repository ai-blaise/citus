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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_job_plan_validates_base_and_wal_archive() {
        assert_eq!(valid_backup_job().validate(), Ok(()));
    }

    #[test]
    fn pitr_restore_requires_utc_timestamp() {
        let restore = PitrRestorePlan {
            cluster: "prod".to_string(),
            source_archive_uri: "s3://backups/prod".to_string(),
            target_time: "2026-05-19 12:00:00".to_string(),
            target_cluster: "restore-prod".to_string(),
        };

        assert_eq!(
            restore.validate(),
            Err(BackupSidecarError::InvalidTimestamp)
        );
    }

    #[test]
    fn queryable_branch_must_be_read_only() {
        let branch = QueryableBackupBranchPlan {
            branch_name: "prod-at-noon".to_string(),
            source_archive_uri: "s3://backups/prod".to_string(),
            target_time: "2026-05-19T12:00:00Z".to_string(),
            read_only: false,
        };

        assert_eq!(
            branch.validate(),
            Err(BackupSidecarError::QueryableBranchMustBeReadOnly)
        );
    }

    #[test]
    fn backup_job_rejects_missing_kms_key() {
        let mut job = valid_backup_job();
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

    fn valid_backup_job() -> BackupJobPlan {
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
}
