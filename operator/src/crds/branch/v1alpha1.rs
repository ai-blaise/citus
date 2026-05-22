// FEATURE: R2
// FEATURE: C6
// FEATURE: C7
// FEATURE: C8

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BranchSpec {
    pub source_cluster: String,
    pub branch_type: BranchType,
    pub storage: BranchStorageSpec,
    pub suspend: bool,
    pub retention_days: Option<u32>,
}

impl BranchSpec {
    pub fn validate(&self) -> Result<(), BranchSpecError> {
        validate_required("source_cluster", &self.source_cluster)?;
        self.storage.validate()?;

        if matches!(self.retention_days, Some(0)) {
            return Err(BranchSpecError::InvalidRetention);
        }

        Ok(())
    }

    pub fn is_scale_to_zero_enabled(&self) -> bool {
        self.suspend
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BranchType {
    CopyOnWrite,
    Snapshot,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BranchStorageSpec {
    pub size: String,
    pub storage_class: Option<String>,
    pub snapshot_class: Option<String>,
}

impl BranchStorageSpec {
    fn validate(&self) -> Result<(), BranchSpecError> {
        validate_required("storage.size", &self.size)?;
        validate_optional("storage.storage_class", &self.storage_class)?;
        validate_optional("storage.snapshot_class", &self.snapshot_class)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BranchSpecError {
    InvalidRetention,
    MissingRequiredField(&'static str),
}

impl fmt::Display for BranchSpecError {
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

impl Error for BranchSpecError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), BranchSpecError> {
    if value.trim().is_empty() {
        return Err(BranchSpecError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_optional(field: &'static str, value: &Option<String>) -> Result<(), BranchSpecError> {
    if matches!(value, Some(value) if value.trim().is_empty()) {
        return Err(BranchSpecError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_copy_on_write_branch_passes() {
        let spec = BranchSpec {
            source_cluster: "prod-us-east".to_string(),
            branch_type: BranchType::CopyOnWrite,
            storage: BranchStorageSpec {
                size: "256Gi".to_string(),
                storage_class: Some("fast-ssd".to_string()),
                snapshot_class: Some("csi-rbdplugin-snapclass".to_string()),
            },
            suspend: true,
            retention_days: Some(7),
        };

        assert_eq!(spec.validate(), Ok(()));
        assert!(spec.is_scale_to_zero_enabled());
    }

    #[test]
    fn branch_rejects_missing_source_cluster() {
        let mut spec = minimal_spec();
        spec.source_cluster = " ".to_string();

        assert_eq!(
            spec.validate(),
            Err(BranchSpecError::MissingRequiredField("source_cluster"))
        );
    }

    #[test]
    fn branch_rejects_zero_retention() {
        let mut spec = minimal_spec();
        spec.retention_days = Some(0);

        assert_eq!(spec.validate(), Err(BranchSpecError::InvalidRetention));
    }

    fn minimal_spec() -> BranchSpec {
        BranchSpec {
            source_cluster: "prod-us-east".to_string(),
            branch_type: BranchType::Snapshot,
            storage: BranchStorageSpec {
                size: "128Gi".to_string(),
                storage_class: None,
                snapshot_class: None,
            },
            suspend: false,
            retention_days: None,
        }
    }
}
