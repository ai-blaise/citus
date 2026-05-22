// FEATURE: B2
// FEATURE: B6

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupSpec {
    pub schedule: String,
    pub retention_days: u32,
    pub target: BackupTarget,
    pub encryption: Option<BackupEncryption>,
}

impl BackupSpec {
    pub fn validate(&self) -> Result<(), BackupSpecError> {
        validate_required("schedule", &self.schedule)?;
        if self.retention_days == 0 {
            return Err(BackupSpecError::InvalidRetention);
        }
        self.target.validate()?;
        if let Some(encryption) = &self.encryption {
            encryption.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupTarget {
    pub provider: BackupProvider,
    pub bucket: String,
    pub prefix: String,
}

impl BackupTarget {
    fn validate(&self) -> Result<(), BackupSpecError> {
        validate_required("target.bucket", &self.bucket)?;
        validate_required("target.prefix", &self.prefix)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BackupProvider {
    S3,
    Gcs,
    Azure,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupEncryption {
    pub kms_key_ref: String,
}

impl BackupEncryption {
    fn validate(&self) -> Result<(), BackupSpecError> {
        validate_required("encryption.kms_key_ref", &self.kms_key_ref)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BackupSpecError {
    InvalidRetention,
    MissingRequiredField(&'static str),
}

impl fmt::Display for BackupSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRetention => {
                write!(formatter, "retention_days must be greater than zero")
            }
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for BackupSpecError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), BackupSpecError> {
    if value.trim().is_empty() {
        return Err(BackupSpecError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_encrypted_backup_passes() {
        let spec = BackupSpec {
            schedule: "0 */6 * * *".to_string(),
            retention_days: 30,
            target: BackupTarget {
                provider: BackupProvider::S3,
                bucket: "ai-blaise-citus-backups".to_string(),
                prefix: "prod/us-east-1".to_string(),
            },
            encryption: Some(BackupEncryption {
                kms_key_ref: "aws-kms-prod".to_string(),
            }),
        };

        assert_eq!(spec.validate(), Ok(()));
    }

    #[test]
    fn backup_rejects_zero_retention() {
        let mut spec = minimal_spec();
        spec.retention_days = 0;

        assert_eq!(spec.validate(), Err(BackupSpecError::InvalidRetention));
    }

    #[test]
    fn backup_rejects_empty_kms_ref() {
        let mut spec = minimal_spec();
        spec.encryption = Some(BackupEncryption {
            kms_key_ref: " ".to_string(),
        });

        assert_eq!(
            spec.validate(),
            Err(BackupSpecError::MissingRequiredField(
                "encryption.kms_key_ref"
            ))
        );
    }

    fn minimal_spec() -> BackupSpec {
        BackupSpec {
            schedule: "0 0 * * *".to_string(),
            retention_days: 7,
            target: BackupTarget {
                provider: BackupProvider::Gcs,
                bucket: "citus-backups".to_string(),
                prefix: "dev".to_string(),
            },
            encryption: None,
        }
    }
}
