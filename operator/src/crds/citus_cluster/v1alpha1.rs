// FEATURE: S4

use std::error::Error;
use std::fmt;

pub const TIMESCALEDB_EXTENSION: &str = "timescaledb";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CitusClusterSpec {
    pub topology: CitusTopology,
    pub image: String,
    pub workers: u32,
    pub coordinators: u32,
    pub storage_class: Option<String>,
    pub timescale_enabled: bool,
    pub extensions: Vec<String>,
    pub pool: Option<PoolSpec>,
    pub sidecars: Vec<SidecarSpec>,
}

impl CitusClusterSpec {
    pub fn validate(&self) -> Result<(), CitusClusterSpecError> {
        validate_required("image", &self.image)?;
        validate_optional("storage_class", &self.storage_class)?;

        if self.workers == 0 {
            return Err(CitusClusterSpecError::InvalidWorkerCount);
        }

        match self.topology {
            CitusTopology::CoordinatorWorker if self.coordinators == 0 => {
                return Err(CitusClusterSpecError::InvalidCoordinatorCount);
            }
            CitusTopology::CoordinatorLess if self.coordinators > 0 => {
                return Err(CitusClusterSpecError::InvalidCoordinatorCount);
            }
            _ => {}
        }

        validate_optional_list("extensions", &self.extensions)?;
        if self.timescale_enabled && !contains_extension(&self.extensions, TIMESCALEDB_EXTENSION) {
            return Err(CitusClusterSpecError::MissingExtension(
                TIMESCALEDB_EXTENSION,
            ));
        }

        if let Some(pool) = &self.pool {
            pool.validate()?;
        }

        for sidecar in &self.sidecars {
            sidecar.validate()?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CitusTopology {
    CoordinatorWorker,
    CoordinatorLess,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PoolSpec {
    pub replicas: u32,
    pub geoip_db: Option<String>,
}

impl PoolSpec {
    fn validate(&self) -> Result<(), CitusClusterSpecError> {
        if self.replicas == 0 {
            return Err(CitusClusterSpecError::InvalidReplicaCount("pool"));
        }
        validate_optional("pool.geoip_db", &self.geoip_db)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SidecarSpec {
    pub sidecar_type: SidecarType,
    pub replicas: u32,
}

impl SidecarSpec {
    fn validate(&self) -> Result<(), CitusClusterSpecError> {
        if self.replicas == 0 {
            return Err(CitusClusterSpecError::InvalidReplicaCount("sidecar"));
        }
        if let SidecarType::Custom(name) = &self.sidecar_type {
            validate_required("sidecar.custom.name", name)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SidecarType {
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
pub enum CitusClusterSpecError {
    InvalidWorkerCount,
    InvalidCoordinatorCount,
    InvalidReplicaCount(&'static str),
    MissingExtension(&'static str),
    MissingRequiredField(&'static str),
}

impl fmt::Display for CitusClusterSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorkerCount => write!(formatter, "workers must be greater than zero"),
            Self::InvalidCoordinatorCount => {
                write!(formatter, "coordinators are inconsistent with topology")
            }
            Self::InvalidReplicaCount(component) => {
                write!(formatter, "{component} replicas must be greater than zero")
            }
            Self::MissingExtension(extension) => {
                write!(formatter, "extensions must include {extension}")
            }
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for CitusClusterSpecError {}

fn contains_extension(extensions: &[String], expected: &str) -> bool {
    extensions
        .iter()
        .any(|extension| extension.trim().eq_ignore_ascii_case(expected))
}

fn validate_required(field: &'static str, value: &str) -> Result<(), CitusClusterSpecError> {
    if value.trim().is_empty() {
        return Err(CitusClusterSpecError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_optional(
    field: &'static str,
    value: &Option<String>,
) -> Result<(), CitusClusterSpecError> {
    if matches!(value, Some(value) if value.trim().is_empty()) {
        return Err(CitusClusterSpecError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_optional_list(
    field: &'static str,
    values: &[String],
) -> Result<(), CitusClusterSpecError> {
    if values.iter().any(|value| value.trim().is_empty()) {
        return Err(CitusClusterSpecError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinator_worker_cluster_with_timescale_passes() {
        let spec = CitusClusterSpec {
            topology: CitusTopology::CoordinatorWorker,
            image: "ghcr.io/ai-blaise/citus:pg18-v2".to_string(),
            workers: 3,
            coordinators: 1,
            storage_class: Some("fast-ssd".to_string()),
            timescale_enabled: true,
            extensions: vec!["citus".to_string(), TIMESCALEDB_EXTENSION.to_string()],
            pool: Some(PoolSpec {
                replicas: 2,
                geoip_db: None,
            }),
            sidecars: vec![SidecarSpec {
                sidecar_type: SidecarType::Vectorizer,
                replicas: 1,
            }],
        };

        assert_eq!(spec.validate(), Ok(()));
    }

    #[test]
    fn coordinator_less_rejects_dedicated_coordinators() {
        let mut spec = minimal_spec();
        spec.topology = CitusTopology::CoordinatorLess;
        spec.coordinators = 1;

        assert_eq!(
            spec.validate(),
            Err(CitusClusterSpecError::InvalidCoordinatorCount)
        );
    }

    #[test]
    fn timescale_enabled_requires_timescaledb_extension() {
        let mut spec = minimal_spec();
        spec.timescale_enabled = true;
        spec.extensions = vec!["citus".to_string()];

        assert_eq!(
            spec.validate(),
            Err(CitusClusterSpecError::MissingExtension(
                TIMESCALEDB_EXTENSION
            ))
        );
    }

    fn minimal_spec() -> CitusClusterSpec {
        CitusClusterSpec {
            topology: CitusTopology::CoordinatorWorker,
            image: "ghcr.io/ai-blaise/citus:pg18-v2".to_string(),
            workers: 3,
            coordinators: 1,
            storage_class: None,
            timescale_enabled: false,
            extensions: vec!["citus".to_string()],
            pool: None,
            sidecars: Vec::new(),
        }
    }
}
