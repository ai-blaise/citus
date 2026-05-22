// FEATURE: S10
// FEATURE: TO1
// FEATURE: TO2
// FEATURE: TO5

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantSpec {
    pub name: String,
    pub schema_name: String,
    pub quotas: TenantQuotas,
    pub region_affinity: Option<String>,
}

impl TenantSpec {
    pub fn validate(&self) -> Result<(), TenantSpecError> {
        validate_required("name", &self.name)?;
        validate_required("schema_name", &self.schema_name)?;
        validate_optional("region_affinity", &self.region_affinity)?;
        self.quotas.validate()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantQuotas {
    pub max_connections: u32,
    pub max_qps: u32,
    pub max_storage_bytes: u64,
}

impl TenantQuotas {
    fn validate(&self) -> Result<(), TenantSpecError> {
        if self.max_connections == 0 {
            return Err(TenantSpecError::InvalidQuota("max_connections"));
        }
        if self.max_qps == 0 {
            return Err(TenantSpecError::InvalidQuota("max_qps"));
        }
        if self.max_storage_bytes == 0 {
            return Err(TenantSpecError::InvalidQuota("max_storage_bytes"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TenantSpecError {
    InvalidQuota(&'static str),
    MissingRequiredField(&'static str),
}

impl fmt::Display for TenantSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidQuota(field) => write!(formatter, "{field} must be greater than zero"),
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for TenantSpecError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), TenantSpecError> {
    if value.trim().is_empty() {
        return Err(TenantSpecError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_optional(field: &'static str, value: &Option<String>) -> Result<(), TenantSpecError> {
    if matches!(value, Some(value) if value.trim().is_empty()) {
        return Err(TenantSpecError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_tenant_with_region_affinity_passes() {
        let spec = TenantSpec {
            name: "tenant-a".to_string(),
            schema_name: "tenant_a".to_string(),
            quotas: TenantQuotas {
                max_connections: 32,
                max_qps: 5_000,
                max_storage_bytes: 1_099_511_627_776,
            },
            region_affinity: Some("us-east-1".to_string()),
        };

        assert_eq!(spec.validate(), Ok(()));
    }

    #[test]
    fn tenant_rejects_empty_schema_name() {
        let mut spec = minimal_spec();
        spec.schema_name = String::new();

        assert_eq!(
            spec.validate(),
            Err(TenantSpecError::MissingRequiredField("schema_name"))
        );
    }

    #[test]
    fn tenant_rejects_zero_storage_quota() {
        let mut spec = minimal_spec();
        spec.quotas.max_storage_bytes = 0;

        assert_eq!(
            spec.validate(),
            Err(TenantSpecError::InvalidQuota("max_storage_bytes"))
        );
    }

    fn minimal_spec() -> TenantSpec {
        TenantSpec {
            name: "tenant-a".to_string(),
            schema_name: "tenant_a".to_string(),
            quotas: TenantQuotas {
                max_connections: 16,
                max_qps: 1_000,
                max_storage_bytes: 536_870_912,
            },
            region_affinity: None,
        }
    }
}
