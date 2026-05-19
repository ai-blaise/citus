// FEATURE: S14
// FEATURE: TO3
// FEATURE: TO4
// FEATURE: TO5

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantMovePlan {
    pub tenant_name: String,
    pub source_worker: String,
    pub target_worker: String,
    pub region_affinity: Option<String>,
}

impl TenantMovePlan {
    pub fn validate(&self) -> Result<(), TenantOperationError> {
        validate_required("tenant_name", &self.tenant_name)?;
        validate_required("source_worker", &self.source_worker)?;
        validate_required("target_worker", &self.target_worker)?;
        if self.source_worker.trim() == self.target_worker.trim() {
            return Err(TenantOperationError::SameWorkerMove);
        }
        validate_optional("region_affinity", &self.region_affinity)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantQuotaPlan {
    pub tenant_name: String,
    pub max_connections: u32,
    pub max_qps: u32,
}

impl TenantQuotaPlan {
    pub fn validate(&self) -> Result<(), TenantOperationError> {
        validate_required("tenant_name", &self.tenant_name)?;
        if self.max_connections == 0 {
            return Err(TenantOperationError::InvalidQuota("max_connections"));
        }
        if self.max_qps == 0 {
            return Err(TenantOperationError::InvalidQuota("max_qps"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantArchivePlan {
    pub tenant_name: String,
    pub destination_uri: String,
    pub retention_days: u32,
}

impl TenantArchivePlan {
    pub fn validate(&self) -> Result<(), TenantOperationError> {
        validate_required("tenant_name", &self.tenant_name)?;
        validate_required("destination_uri", &self.destination_uri)?;
        if self.retention_days == 0 {
            return Err(TenantOperationError::InvalidRetention);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TenantOperationError {
    InvalidQuota(&'static str),
    InvalidRetention,
    SameWorkerMove,
    MissingRequiredField(&'static str),
}

impl fmt::Display for TenantOperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidQuota(field) => write!(formatter, "{field} must be greater than zero"),
            Self::InvalidRetention => {
                write!(formatter, "retention_days must be greater than zero")
            }
            Self::SameWorkerMove => {
                write!(formatter, "source_worker and target_worker must differ")
            }
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for TenantOperationError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), TenantOperationError> {
    if value.trim().is_empty() {
        return Err(TenantOperationError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_optional(
    field: &'static str,
    value: &Option<String>,
) -> Result<(), TenantOperationError> {
    if matches!(value, Some(value) if value.trim().is_empty()) {
        return Err(TenantOperationError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_tenant_move_passes() {
        let plan = TenantMovePlan {
            tenant_name: "tenant-a".to_string(),
            source_worker: "worker-1".to_string(),
            target_worker: "worker-2".to_string(),
            region_affinity: Some("us-east-1".to_string()),
        };

        assert_eq!(plan.validate(), Ok(()));
    }

    #[test]
    fn tenant_move_rejects_same_worker() {
        let plan = TenantMovePlan {
            tenant_name: "tenant-a".to_string(),
            source_worker: "worker-1".to_string(),
            target_worker: "worker-1".to_string(),
            region_affinity: None,
        };

        assert_eq!(plan.validate(), Err(TenantOperationError::SameWorkerMove));
    }

    #[test]
    fn tenant_archive_requires_retention() {
        let plan = TenantArchivePlan {
            tenant_name: "tenant-a".to_string(),
            destination_uri: "s3://archives/tenant-a".to_string(),
            retention_days: 0,
        };

        assert_eq!(plan.validate(), Err(TenantOperationError::InvalidRetention));
    }
}
