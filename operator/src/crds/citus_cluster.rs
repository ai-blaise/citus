// FEATURE: D9
// FEATURE: S4

use std::error::Error;
use std::fmt;

pub const TIMESCALEDB_EXTENSION: &str = "timescaledb";
pub const CITUS_EXTENSION: &str = "citus";
pub const COMPANION_EXTENSION: &str = "ai_blaise_citus";
#[cfg(test)]
pub(crate) const SHIPPED_COMPANION_EXTENSION_VERSION: &str = "0.1.2";
pub const CNPG_SERVER_CA_PATH: &str = "/controller/certificates/server-ca.crt";
pub const MAX_PRODUCTION_WORKER_GROUPS: u32 = 32;
pub const MAX_PRODUCTION_GROUP_REPLICAS: u32 = 9;
pub const MAX_PRODUCTION_DATABASES: usize = 32;

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
    /// Fail-closed inputs required by the live Kubernetes reconcile path.
    /// Planning-only callers may omit this block; apply mode may not.
    pub production: Option<CitusProductionSpec>,
}

impl CitusClusterSpec {
    pub fn validate(&self) -> Result<(), CitusClusterSpecError> {
        validate_required("image", &self.image)?;
        validate_optional("storage_class", &self.storage_class)?;

        if self.workers == 0 {
            return Err(CitusClusterSpecError::InvalidWorkerCount);
        }
        if self.workers > MAX_PRODUCTION_WORKER_GROUPS {
            return Err(CitusClusterSpecError::ExcessiveProductionWorkers);
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
        if matches!(self.topology, CitusTopology::CoordinatorLess) && self.pool.is_none() {
            return Err(CitusClusterSpecError::MissingRequiredComponent("pool"));
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

        if let Some(production) = &self.production {
            production.validate()?;
        }

        Ok(())
    }

    /// Validate the stronger contract used before the controller mutates the
    /// cluster. This deliberately rejects legacy plan-only specifications.
    pub fn validate_apply_ready(&self) -> Result<(), CitusClusterSpecError> {
        self.validate()?;
        if !is_digest_pinned_image(&self.image) {
            return Err(CitusClusterSpecError::MutableImage);
        }
        if !contains_extension(&self.extensions, CITUS_EXTENSION) {
            return Err(CitusClusterSpecError::MissingExtension(CITUS_EXTENSION));
        }
        if !contains_extension(&self.extensions, COMPANION_EXTENSION) {
            return Err(CitusClusterSpecError::MissingExtension(COMPANION_EXTENSION));
        }
        if self.timescale_enabled {
            return Err(CitusClusterSpecError::UnsupportedApplyCohabitation);
        }
        if self.extensions.len() != 2
            || self.extensions[0] != CITUS_EXTENSION
            || self.extensions[1] != COMPANION_EXTENSION
        {
            return Err(CitusClusterSpecError::UnsupportedApplyExtensions);
        }
        if !matches!(self.topology, CitusTopology::CoordinatorWorker) {
            return Err(CitusClusterSpecError::UnsupportedApplyTopology);
        }
        if self.workers < 2 {
            return Err(CitusClusterSpecError::InsufficientProductionWorkers);
        }
        if self.coordinators > MAX_PRODUCTION_GROUP_REPLICAS {
            return Err(CitusClusterSpecError::ExcessiveCoordinatorReplicas);
        }
        if self.pool.is_some() || !self.sidecars.is_empty() {
            return Err(CitusClusterSpecError::UnsupportedApplyChildren);
        }
        self.production
            .as_ref()
            .ok_or(CitusClusterSpecError::MissingRequiredComponent(
                "production",
            ))?
            .validate()
    }
}

/// Exact, independently reviewed values consumed by the production
/// `CitusCluster` reconciler.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CitusProductionSpec {
    pub postgres_major: u32,
    pub postgres_uid: u32,
    pub postgres_gid: u32,
    /// Number of CNPG instances in every worker group.
    pub worker_replicas: u32,
    pub storage_size: String,
    pub cluster_domain: String,
    pub databases: Vec<String>,
    pub extension_versions: ExactExtensionVersions,
    pub node_tls: NodeTlsSpec,
    pub bootstrap: BootstrapJobSpec,
}

impl CitusProductionSpec {
    fn validate(&self) -> Result<(), CitusClusterSpecError> {
        if !(13..=18).contains(&self.postgres_major) {
            return Err(CitusClusterSpecError::InvalidPostgresMajor);
        }
        if self.postgres_uid == 0 || self.postgres_gid == 0 {
            return Err(CitusClusterSpecError::InvalidPostgresIdentity);
        }
        if self.worker_replicas == 0 {
            return Err(CitusClusterSpecError::InvalidReplicaCount("worker"));
        }
        if self.worker_replicas > MAX_PRODUCTION_GROUP_REPLICAS {
            return Err(CitusClusterSpecError::ExcessiveWorkerReplicas);
        }
        if !is_storage_quantity(&self.storage_size) {
            return Err(CitusClusterSpecError::InvalidStorageSize);
        }
        validate_dns_subdomain("production.cluster_domain", &self.cluster_domain)?;
        if self.databases.is_empty() {
            return Err(CitusClusterSpecError::MissingRequiredComponent(
                "production.databases",
            ));
        }
        if self.databases.len() > MAX_PRODUCTION_DATABASES {
            return Err(CitusClusterSpecError::ExcessiveProductionDatabases);
        }
        let mut seen = std::collections::BTreeSet::new();
        for database in &self.databases {
            validate_postgres_identifier("production.databases", database)?;
            if !seen.insert(database) {
                return Err(CitusClusterSpecError::DuplicateDatabase);
            }
        }
        self.extension_versions.validate()?;
        self.node_tls.validate()?;
        self.bootstrap.validate()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExactExtensionVersions {
    pub citus: String,
    pub companion: String,
}

impl ExactExtensionVersions {
    fn validate(&self) -> Result<(), CitusClusterSpecError> {
        validate_exact_version("production.extension_versions.citus", &self.citus)?;
        validate_exact_version("production.extension_versions.companion", &self.companion)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NodeTlsSpec {
    pub server_ca_secret: String,
    pub server_tls_secret: String,
    pub superuser_secret: String,
    pub ssl_mode: String,
    pub ssl_root_cert: String,
    pub connect_timeout_seconds: u32,
}

impl NodeTlsSpec {
    fn validate(&self) -> Result<(), CitusClusterSpecError> {
        validate_dns_subdomain(
            "production.node_tls.server_ca_secret",
            &self.server_ca_secret,
        )?;
        validate_dns_subdomain(
            "production.node_tls.server_tls_secret",
            &self.server_tls_secret,
        )?;
        validate_dns_subdomain(
            "production.node_tls.superuser_secret",
            &self.superuser_secret,
        )?;
        if self.ssl_mode != "verify-full" {
            return Err(CitusClusterSpecError::InvalidNodeTlsMode);
        }
        if self.ssl_root_cert != CNPG_SERVER_CA_PATH {
            return Err(CitusClusterSpecError::InvalidNodeTlsRootCert);
        }
        if !(1..=60).contains(&self.connect_timeout_seconds) {
            return Err(CitusClusterSpecError::InvalidConnectTimeout);
        }
        Ok(())
    }

    pub fn node_conninfo(&self) -> String {
        format!(
            "sslmode={} sslrootcert={} connect_timeout={}",
            self.ssl_mode, self.ssl_root_cert, self.connect_timeout_seconds
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BootstrapJobSpec {
    pub backoff_limit: u32,
    pub active_deadline_seconds: u32,
}

impl BootstrapJobSpec {
    fn validate(&self) -> Result<(), CitusClusterSpecError> {
        if self.backoff_limit > 6 {
            return Err(CitusClusterSpecError::InvalidBootstrapBackoff);
        }
        if !(60..=1800).contains(&self.active_deadline_seconds) {
            return Err(CitusClusterSpecError::InvalidBootstrapDeadline);
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
    MissingRequiredComponent(&'static str),
    MissingRequiredField(&'static str),
    MutableImage,
    UnsupportedApplyTopology,
    UnsupportedApplyChildren,
    UnsupportedApplyCohabitation,
    UnsupportedApplyExtensions,
    InsufficientProductionWorkers,
    ExcessiveProductionWorkers,
    ExcessiveCoordinatorReplicas,
    ExcessiveWorkerReplicas,
    ExcessiveProductionDatabases,
    InvalidStorageSize,
    InvalidDnsSubdomain(&'static str),
    InvalidPostgresIdentifier(&'static str),
    InvalidExactVersion(&'static str),
    DuplicateDatabase,
    InvalidNodeTlsMode,
    InvalidNodeTlsRootCert,
    InvalidConnectTimeout,
    InvalidBootstrapBackoff,
    InvalidBootstrapDeadline,
    InvalidPostgresMajor,
    InvalidPostgresIdentity,
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
            Self::MissingRequiredComponent(component) if *component == "pool" => {
                write!(formatter, "coordinator-less topology requires {component}")
            }
            Self::MissingRequiredComponent(component) => {
                write!(formatter, "spec requires {component}")
            }
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
            Self::MutableImage => write!(
                formatter,
                "image must be an immutable @sha256 reference with a lowercase 64-character digest"
            ),
            Self::UnsupportedApplyTopology => write!(
                formatter,
                "live apply currently requires coordinator-worker topology"
            ),
            Self::UnsupportedApplyChildren => write!(
                formatter,
                "live CitusCluster apply does not accept inline pool or sidecar children"
            ),
            Self::UnsupportedApplyCohabitation => write!(
                formatter,
                "live CitusCluster apply does not claim TimescaleDB cohabitation"
            ),
            Self::UnsupportedApplyExtensions => write!(
                formatter,
                "live CitusCluster apply requires extensions exactly [citus, ai_blaise_citus]"
            ),
            Self::InsufficientProductionWorkers => {
                write!(formatter, "live apply requires at least two worker groups")
            }
            Self::ExcessiveProductionWorkers => write!(
                formatter,
                "CitusCluster supports at most {MAX_PRODUCTION_WORKER_GROUPS} worker groups"
            ),
            Self::ExcessiveCoordinatorReplicas => write!(
                formatter,
                "live apply supports at most {MAX_PRODUCTION_GROUP_REPLICAS} coordinator replicas"
            ),
            Self::ExcessiveWorkerReplicas => write!(
                formatter,
                "live apply supports at most {MAX_PRODUCTION_GROUP_REPLICAS} replicas per worker group"
            ),
            Self::ExcessiveProductionDatabases => write!(
                formatter,
                "live apply supports at most {MAX_PRODUCTION_DATABASES} databases"
            ),
            Self::InvalidStorageSize => write!(
                formatter,
                "production.storage_size must be a positive binary storage quantity such as 10Gi"
            ),
            Self::InvalidDnsSubdomain(field) => {
                write!(formatter, "{field} must be a valid lowercase DNS subdomain")
            }
            Self::InvalidPostgresIdentifier(field) => write!(
                formatter,
                "{field} entries must be lowercase PostgreSQL identifiers of at most 63 bytes"
            ),
            Self::InvalidExactVersion(field) => write!(
                formatter,
                "{field} must be a single exact extension version token"
            ),
            Self::DuplicateDatabase => {
                write!(formatter, "production.databases contains duplicates")
            }
            Self::InvalidNodeTlsMode => {
                write!(
                    formatter,
                    "production.node_tls.ssl_mode must equal verify-full"
                )
            }
            Self::InvalidNodeTlsRootCert => write!(
                formatter,
                "production.node_tls.ssl_root_cert must equal {CNPG_SERVER_CA_PATH}"
            ),
            Self::InvalidConnectTimeout => write!(
                formatter,
                "production.node_tls.connect_timeout_seconds must be between 1 and 60"
            ),
            Self::InvalidBootstrapBackoff => write!(
                formatter,
                "production.bootstrap.backoff_limit must be at most 6"
            ),
            Self::InvalidBootstrapDeadline => write!(
                formatter,
                "production.bootstrap.active_deadline_seconds must be between 60 and 1800"
            ),
            Self::InvalidPostgresMajor => write!(
                formatter,
                "production.postgres_major must be a supported major between 13 and 18"
            ),
            Self::InvalidPostgresIdentity => write!(
                formatter,
                "production postgres_uid and postgres_gid must be non-zero"
            ),
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

fn is_digest_pinned_image(image: &str) -> bool {
    let Some((repository, digest)) = image.split_once("@sha256:") else {
        return false;
    };
    !repository.is_empty()
        && repository.len() <= 255
        && !repository.contains('@')
        && repository.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'/' | b':' | b'_' | b'-')
        })
        && digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_storage_quantity(value: &str) -> bool {
    let suffix = ["Ki", "Mi", "Gi", "Ti", "Pi"]
        .into_iter()
        .find(|suffix| value.ends_with(suffix));
    let Some(suffix) = suffix else {
        return false;
    };
    value
        .strip_suffix(suffix)
        .and_then(|number| number.parse::<u64>().ok())
        .is_some_and(|number| number > 0)
}

fn validate_dns_subdomain(field: &'static str, value: &str) -> Result<(), CitusClusterSpecError> {
    let valid = !value.is_empty()
        && value.len() <= 253
        && value.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
                && label
                    .as_bytes()
                    .first()
                    .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
                && label
                    .as_bytes()
                    .last()
                    .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        });
    if valid {
        Ok(())
    } else {
        Err(CitusClusterSpecError::InvalidDnsSubdomain(field))
    }
}

fn validate_postgres_identifier(
    field: &'static str,
    value: &str,
) -> Result<(), CitusClusterSpecError> {
    let mut bytes = value.bytes();
    let valid = value.len() <= 63
        && bytes
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(CitusClusterSpecError::InvalidPostgresIdentifier(field))
    }
}

fn validate_exact_version(field: &'static str, value: &str) -> Result<(), CitusClusterSpecError> {
    let valid = !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-'));
    if valid {
        Ok(())
    } else {
        Err(CitusClusterSpecError::InvalidExactVersion(field))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_companion_version_matches_both_control_files() {
        let overlay_control =
            include_str!("../../../images/citus-pg-overlay/extensions/ai_blaise_citus.control");
        let companion_control = include_str!("../../../companion/ai_blaise_citus.control");

        for control in [overlay_control, companion_control] {
            let default_version = control
                .lines()
                .find_map(|line| line.strip_prefix("default_version = '")?.strip_suffix('\''))
                .expect("control default_version");
            assert_eq!(default_version, SHIPPED_COMPANION_EXTENSION_VERSION);
        }
    }

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
            production: None,
        };

        assert_eq!(spec.validate(), Ok(()));
    }

    #[test]
    fn coordinator_less_rejects_dedicated_coordinators() {
        let mut spec = minimal_spec();
        spec.topology = CitusTopology::CoordinatorLess;
        spec.coordinators = 1;
        spec.pool = Some(PoolSpec {
            replicas: 2,
            geoip_db: None,
        });

        assert_eq!(
            spec.validate(),
            Err(CitusClusterSpecError::InvalidCoordinatorCount)
        );
    }

    #[test]
    fn coordinator_less_requires_pool_entrypoint() {
        let mut spec = minimal_spec();
        spec.topology = CitusTopology::CoordinatorLess;
        spec.coordinators = 0;

        assert_eq!(
            spec.validate(),
            Err(CitusClusterSpecError::MissingRequiredComponent("pool"))
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
            production: None,
        }
    }
}
