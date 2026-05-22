// FEATURE: O5

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SidecarDeploymentSpec {
    pub sidecar_type: SidecarDeploymentType,
    pub replicas: u32,
    pub resources: ResourceRequirements,
    pub config_yaml: Option<String>,
}

impl SidecarDeploymentSpec {
    pub fn validate(&self) -> Result<(), SidecarDeploymentSpecError> {
        if self.replicas == 0 {
            return Err(SidecarDeploymentSpecError::InvalidReplicaCount);
        }
        if let SidecarDeploymentType::Custom(name) = &self.sidecar_type {
            validate_required("sidecar_type.custom", name)?;
        }
        self.resources.validate()?;
        validate_optional("config_yaml", &self.config_yaml)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SidecarDeploymentType {
    Analytical,
    Vectorizer,
    Cdc,
    ColdTier,
    Raft,
    Hlc,
    TxnStatus,
    SchemaJob,
    Realtime,
    Auth,
    Storage,
    Postgrest,
    Graphql,
    EdgeFunctions,
    Backup,
    Repack,
    Mcp,
    Custom(String),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ResourceRequirements {
    pub cpu_millis: u32,
    pub memory_mib: u32,
}

impl ResourceRequirements {
    fn validate(&self) -> Result<(), SidecarDeploymentSpecError> {
        if self.cpu_millis == 0 {
            return Err(SidecarDeploymentSpecError::InvalidCpuRequest);
        }
        if self.memory_mib == 0 {
            return Err(SidecarDeploymentSpecError::InvalidMemoryRequest);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SidecarDeploymentSpecError {
    InvalidCpuRequest,
    InvalidMemoryRequest,
    InvalidReplicaCount,
    MissingRequiredField(&'static str),
}

impl fmt::Display for SidecarDeploymentSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCpuRequest => write!(formatter, "cpu_millis must be greater than zero"),
            Self::InvalidMemoryRequest => write!(formatter, "memory_mib must be greater than zero"),
            Self::InvalidReplicaCount => write!(formatter, "replicas must be greater than zero"),
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for SidecarDeploymentSpecError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), SidecarDeploymentSpecError> {
    if value.trim().is_empty() {
        return Err(SidecarDeploymentSpecError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_optional(
    field: &'static str,
    value: &Option<String>,
) -> Result<(), SidecarDeploymentSpecError> {
    if matches!(value, Some(value) if value.trim().is_empty()) {
        return Err(SidecarDeploymentSpecError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_sidecar_deployment_passes() {
        let spec = SidecarDeploymentSpec {
            sidecar_type: SidecarDeploymentType::Realtime,
            replicas: 2,
            resources: ResourceRequirements {
                cpu_millis: 250,
                memory_mib: 512,
            },
            config_yaml: Some("subscriptions:\n  max_per_tenant: 1000".to_string()),
        };

        assert_eq!(spec.validate(), Ok(()));
    }

    #[test]
    fn sidecar_rejects_zero_replicas() {
        let mut spec = minimal_spec();
        spec.replicas = 0;

        assert_eq!(
            spec.validate(),
            Err(SidecarDeploymentSpecError::InvalidReplicaCount)
        );
    }

    #[test]
    fn custom_sidecar_requires_name() {
        let mut spec = minimal_spec();
        spec.sidecar_type = SidecarDeploymentType::Custom(String::new());

        assert_eq!(
            spec.validate(),
            Err(SidecarDeploymentSpecError::MissingRequiredField(
                "sidecar_type.custom"
            ))
        );
    }

    fn minimal_spec() -> SidecarDeploymentSpec {
        SidecarDeploymentSpec {
            sidecar_type: SidecarDeploymentType::Vectorizer,
            replicas: 1,
            resources: ResourceRequirements {
                cpu_millis: 100,
                memory_mib: 256,
            },
            config_yaml: None,
        }
    }
}
