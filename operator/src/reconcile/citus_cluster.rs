// FEATURE: S2
// FEATURE: S4
// FEATURE: S11

use std::error::Error;
use std::fmt;

use crate::crds::citus_cluster::{
    CitusClusterSpec, CitusClusterSpecError, CitusTopology, PoolSpec, SidecarSpec, SidecarType,
};

/// Default name suffix used for the CloudNativePG cluster generated for a
/// `CitusCluster`. The owning CRD's metadata.name is concatenated with this
/// suffix to keep names deterministic across reconciles.
pub const CNPG_CLUSTER_NAME_SUFFIX: &str = "-postgres";

/// Default name suffix used for the pool Deployment generated for a
/// `CitusCluster`.
pub const POOL_DEPLOYMENT_NAME_SUFFIX: &str = "-pool";

/// Default extension reload command applied to the CNPG cluster so the bundled
/// citus + timescaledb shared libraries load in the correct order.
pub const POSTGRES_SHARED_PRELOAD_LIBRARIES: &[&str] = &["citus", "timescaledb"];

/// Resolved cluster topology fingerprint suitable for status reporting.
///
/// This is a flattened view of the CRD-side `CitusTopology` plus the derived
/// node counts, so downstream controllers can reason about the cluster
/// without re-reading the original spec.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ClusterTopologyPlan {
    pub topology: CitusTopology,
    pub coordinators: u32,
    pub workers: u32,
}

impl ClusterTopologyPlan {
    pub fn total_postgres_instances(&self) -> u32 {
        self.coordinators.saturating_add(self.workers)
    }
}

/// Plan to apply to the CloudNativePG cluster managed by a `CitusCluster`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CnpgClusterPlan {
    pub name: String,
    pub instances: u32,
    pub image: String,
    pub storage_class: Option<String>,
    pub shared_preload_libraries: Vec<String>,
    pub extensions: Vec<String>,
    pub timescale_enabled: bool,
}

impl CnpgClusterPlan {
    pub fn manifest_yaml(&self) -> String {
        let storage = self
            .storage_class
            .as_ref()
            .map(|class| format!("\n  storage:\n    storageClass: {class}"))
            .unwrap_or_default();
        let preload = self.shared_preload_libraries.join(",");
        let extensions = if self.extensions.is_empty() {
            String::new()
        } else {
            let lines: Vec<String> = self
                .extensions
                .iter()
                .map(|extension| format!("      - name: {extension}"))
                .collect();
            format!("\n  postgresql:\n    extensions:\n{}", lines.join("\n"))
        };

        format!(
            "apiVersion: postgresql.cnpg.io/v1\nkind: Cluster\nmetadata:\n  name: {name}\nspec:\n  instances: {instances}\n  imageName: {image}\n  postgresql:\n    shared_preload_libraries: \"{preload}\"{extensions}{storage}\n",
            name = self.name,
            instances = self.instances,
            image = self.image,
        )
    }
}

/// Plan for the connection pool Deployment generated alongside a
/// `CitusCluster`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PoolDeploymentPlan {
    pub name: String,
    pub replicas: u32,
    pub geoip_db: Option<String>,
}

impl PoolDeploymentPlan {
    fn from_spec(cluster_name: &str, pool: &PoolSpec) -> Self {
        Self {
            name: format!("{cluster_name}{POOL_DEPLOYMENT_NAME_SUFFIX}"),
            replicas: pool.replicas,
            geoip_db: pool.geoip_db.clone(),
        }
    }
}

/// Plan for a single sidecar Deployment generated for a `CitusCluster`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SidecarDeploymentPlan {
    pub sidecar_type: SidecarType,
    pub replicas: u32,
    pub deployment_name: String,
}

impl SidecarDeploymentPlan {
    fn from_spec(cluster_name: &str, sidecar: &SidecarSpec) -> Self {
        let suffix = sidecar_name_suffix(&sidecar.sidecar_type);
        Self {
            sidecar_type: sidecar.sidecar_type.clone(),
            replicas: sidecar.replicas,
            deployment_name: format!("{cluster_name}-sidecar-{suffix}"),
        }
    }
}

fn sidecar_name_suffix(sidecar_type: &SidecarType) -> String {
    match sidecar_type {
        SidecarType::Analytical => "analytical".to_string(),
        SidecarType::Vectorizer => "vectorizer".to_string(),
        SidecarType::Cdc => "cdc".to_string(),
        SidecarType::ColdTier => "coldtier".to_string(),
        SidecarType::Raft => "raft".to_string(),
        SidecarType::Hlc => "hlc".to_string(),
        SidecarType::TxnStatus => "txn-status".to_string(),
        SidecarType::SchemaJob => "schema-job".to_string(),
        SidecarType::Realtime => "realtime".to_string(),
        SidecarType::Auth => "auth".to_string(),
        SidecarType::Storage => "storage".to_string(),
        SidecarType::Postgrest => "postgrest".to_string(),
        SidecarType::Graphql => "graphql".to_string(),
        SidecarType::EdgeFunctions => "edge-functions".to_string(),
        SidecarType::Backup => "backup".to_string(),
        SidecarType::Repack => "repack".to_string(),
        SidecarType::Mcp => "mcp".to_string(),
        SidecarType::Custom(name) => sanitize_custom_sidecar_name(name),
    }
}

fn sanitize_custom_sidecar_name(raw: &str) -> String {
    raw.trim()
        .chars()
        .map(|character| match character {
            'A'..='Z' => character.to_ascii_lowercase(),
            'a'..='z' | '0'..='9' | '-' => character,
            _ => '-',
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Resolved reconcile plan derived from a `CitusClusterSpec`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CitusClusterReconcilePlan {
    pub cluster_name: String,
    pub topology: ClusterTopologyPlan,
    pub cnpg_cluster: CnpgClusterPlan,
    pub pool: Option<PoolDeploymentPlan>,
    pub sidecars: Vec<SidecarDeploymentPlan>,
    pub timescale_enabled: bool,
    pub coordinator_less: bool,
}

impl CitusClusterReconcilePlan {
    /// Build a reconcile plan from a `CitusClusterSpec` belonging to the named
    /// resource. The name is the `metadata.name` of the owning CRD so that
    /// child resources stay deterministic across reconciliation passes.
    pub fn from_spec(
        cluster_name: &str,
        spec: &CitusClusterSpec,
    ) -> Result<Self, CitusClusterReconcileError> {
        let trimmed = cluster_name.trim();
        if trimmed.is_empty() {
            return Err(CitusClusterReconcileError::MissingClusterName);
        }
        spec.validate()?;

        let topology = ClusterTopologyPlan {
            topology: spec.topology,
            coordinators: spec.coordinators,
            workers: spec.workers,
        };

        let cnpg_cluster = CnpgClusterPlan {
            name: format!("{trimmed}{CNPG_CLUSTER_NAME_SUFFIX}"),
            instances: topology.total_postgres_instances(),
            image: spec.image.clone(),
            storage_class: spec.storage_class.clone(),
            shared_preload_libraries: shared_preload_libraries(spec),
            extensions: spec.extensions.clone(),
            timescale_enabled: spec.timescale_enabled,
        };

        let pool = spec
            .pool
            .as_ref()
            .map(|pool_spec| PoolDeploymentPlan::from_spec(trimmed, pool_spec));

        let sidecars = spec
            .sidecars
            .iter()
            .map(|sidecar| SidecarDeploymentPlan::from_spec(trimmed, sidecar))
            .collect::<Vec<_>>();

        Ok(Self {
            cluster_name: trimmed.to_string(),
            timescale_enabled: spec.timescale_enabled,
            coordinator_less: matches!(spec.topology, CitusTopology::CoordinatorLess),
            topology,
            cnpg_cluster,
            pool,
            sidecars,
        })
    }

    /// Number of Kubernetes Deployment objects produced by the plan, including
    /// pool and every sidecar. The CNPG `Cluster` resource is counted
    /// separately because CNPG owns the underlying StatefulSet/Pods.
    pub fn total_deployments(&self) -> usize {
        let pool = usize::from(self.pool.is_some());
        pool + self.sidecars.len()
    }

    /// Total managed PostgreSQL instances across coordinator and workers.
    pub fn total_postgres_instances(&self) -> u32 {
        self.topology.total_postgres_instances()
    }
}

fn shared_preload_libraries(spec: &CitusClusterSpec) -> Vec<String> {
    let mut libraries = vec!["citus".to_string()];
    if spec.timescale_enabled {
        libraries.push("timescaledb".to_string());
    }
    libraries
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CitusClusterReconcileError {
    InvalidSpec(CitusClusterSpecError),
    MissingClusterName,
}

impl fmt::Display for CitusClusterReconcileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSpec(error) => write!(formatter, "{error}"),
            Self::MissingClusterName => {
                write!(formatter, "cluster_name must not be empty")
            }
        }
    }
}

impl Error for CitusClusterReconcileError {}

impl From<CitusClusterSpecError> for CitusClusterReconcileError {
    fn from(error: CitusClusterSpecError) -> Self {
        Self::InvalidSpec(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crds::citus_cluster::TIMESCALEDB_EXTENSION;

    #[test]
    fn coordinator_worker_plan_renders_cnpg_pool_and_sidecars() {
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
                geoip_db: Some("maxmind-city".to_string()),
            }),
            sidecars: vec![
                SidecarSpec {
                    sidecar_type: SidecarType::Vectorizer,
                    replicas: 1,
                },
                SidecarSpec {
                    sidecar_type: SidecarType::Mcp,
                    replicas: 2,
                },
            ],
        };

        let plan =
            CitusClusterReconcilePlan::from_spec("ai-blaise-citus", &spec).expect("valid plan");

        assert_eq!(plan.cluster_name, "ai-blaise-citus");
        assert!(!plan.coordinator_less);
        assert!(plan.timescale_enabled);
        assert_eq!(plan.topology.total_postgres_instances(), 4);
        assert_eq!(plan.cnpg_cluster.name, "ai-blaise-citus-postgres");
        assert_eq!(plan.cnpg_cluster.instances, 4);
        assert_eq!(
            plan.cnpg_cluster.shared_preload_libraries,
            vec!["citus".to_string(), "timescaledb".to_string()]
        );
        assert_eq!(plan.cnpg_cluster.storage_class.as_deref(), Some("fast-ssd"));
        let pool = plan.pool.as_ref().expect("pool plan present");
        assert_eq!(pool.name, "ai-blaise-citus-pool");
        assert_eq!(pool.replicas, 2);
        assert_eq!(pool.geoip_db.as_deref(), Some("maxmind-city"));
        assert_eq!(plan.sidecars.len(), 2);
        assert_eq!(
            plan.sidecars[0].deployment_name,
            "ai-blaise-citus-sidecar-vectorizer"
        );
        assert_eq!(
            plan.sidecars[1].deployment_name,
            "ai-blaise-citus-sidecar-mcp"
        );
        assert_eq!(plan.total_deployments(), 3);

        let manifest = plan.cnpg_cluster.manifest_yaml();
        assert!(manifest.contains("apiVersion: postgresql.cnpg.io/v1"));
        assert!(manifest.contains("kind: Cluster"));
        assert!(manifest.contains("name: ai-blaise-citus-postgres"));
        assert!(manifest.contains("instances: 4"));
        assert!(manifest.contains("shared_preload_libraries: \"citus,timescaledb\""));
        assert!(manifest.contains("storageClass: fast-ssd"));
        assert!(manifest.contains("- name: timescaledb"));
    }

    #[test]
    fn coordinator_less_plan_omits_dedicated_coordinator_instances() {
        let spec = CitusClusterSpec {
            topology: CitusTopology::CoordinatorLess,
            image: "ghcr.io/ai-blaise/citus:pg18-v2".to_string(),
            workers: 4,
            coordinators: 0,
            storage_class: None,
            timescale_enabled: false,
            extensions: vec!["citus".to_string()],
            pool: None,
            sidecars: Vec::new(),
        };

        let plan = CitusClusterReconcilePlan::from_spec("acme", &spec).expect("valid plan");

        assert!(plan.coordinator_less);
        assert!(!plan.timescale_enabled);
        assert_eq!(plan.topology.total_postgres_instances(), 4);
        assert_eq!(plan.cnpg_cluster.instances, 4);
        assert_eq!(
            plan.cnpg_cluster.shared_preload_libraries,
            vec!["citus".to_string()]
        );
        assert!(plan.pool.is_none());
        assert_eq!(plan.sidecars.len(), 0);
        assert_eq!(plan.total_deployments(), 0);
    }

    #[test]
    fn empty_cluster_name_is_rejected() {
        let spec = CitusClusterSpec {
            topology: CitusTopology::CoordinatorWorker,
            image: "ghcr.io/ai-blaise/citus:pg18-v2".to_string(),
            workers: 1,
            coordinators: 1,
            storage_class: None,
            timescale_enabled: false,
            extensions: vec!["citus".to_string()],
            pool: None,
            sidecars: Vec::new(),
        };

        assert_eq!(
            CitusClusterReconcilePlan::from_spec("   ", &spec),
            Err(CitusClusterReconcileError::MissingClusterName)
        );
    }

    #[test]
    fn invalid_spec_propagates_validation_error() {
        let spec = CitusClusterSpec {
            topology: CitusTopology::CoordinatorWorker,
            image: "ghcr.io/ai-blaise/citus:pg18-v2".to_string(),
            workers: 0,
            coordinators: 1,
            storage_class: None,
            timescale_enabled: false,
            extensions: vec!["citus".to_string()],
            pool: None,
            sidecars: Vec::new(),
        };

        assert_eq!(
            CitusClusterReconcilePlan::from_spec("acme", &spec),
            Err(CitusClusterReconcileError::InvalidSpec(
                CitusClusterSpecError::InvalidWorkerCount
            ))
        );
    }

    #[test]
    fn custom_sidecar_name_is_sanitized_for_kubernetes() {
        let spec = CitusClusterSpec {
            topology: CitusTopology::CoordinatorWorker,
            image: "ghcr.io/ai-blaise/citus:pg18-v2".to_string(),
            workers: 1,
            coordinators: 1,
            storage_class: None,
            timescale_enabled: false,
            extensions: vec!["citus".to_string()],
            pool: None,
            sidecars: vec![SidecarSpec {
                sidecar_type: SidecarType::Custom("Custom Analytics_v2".to_string()),
                replicas: 1,
            }],
        };

        let plan = CitusClusterReconcilePlan::from_spec("acme", &spec).expect("valid plan");

        assert_eq!(
            plan.sidecars[0].deployment_name,
            "acme-sidecar-custom-analytics-v2"
        );
    }
}
