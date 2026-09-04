// FEATURE: S2
// FEATURE: S4
// FEATURE: S11

use std::error::Error;
use std::fmt;

use serde_json::{json, Value};

use crate::crds::citus_cluster::{
    CitusClusterSpec, CitusClusterSpecError, CitusProductionSpec, CitusTopology, PoolSpec,
    SidecarSpec, SidecarType,
};
use crate::reconcile::security::{WorkloadSecurityError, WorkloadSecurityPlan};

pub const CNPG_COORDINATOR_NAME_SUFFIX: &str = "-coordinator";
pub const CNPG_WORKER_NAME_INFIX: &str = "-worker-";
pub const CNPG_CLUSTER_NAME_SUFFIX: &str = CNPG_COORDINATOR_NAME_SUFFIX;
pub const POOL_DEPLOYMENT_NAME_SUFFIX: &str = "-pool";
pub const BOOTSTRAP_CONFIG_MAP_NAME_SUFFIX: &str = "-bootstrap";
pub const CNPG_IMAGE_CATALOG_NAME_SUFFIX: &str = "-images";
pub const POSTGRES_SHARED_PRELOAD_LIBRARIES: &[&str] = &["citus", "timescaledb"];

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ClusterTopologyPlan {
    pub topology: CitusTopology,
    pub coordinators: u32,
    pub workers: u32,
    pub worker_replicas: u32,
}

impl ClusterTopologyPlan {
    pub fn total_postgres_instances(&self) -> u32 {
        self.coordinators
            .saturating_add(self.workers.saturating_mul(self.worker_replicas))
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CnpgClusterRole {
    Coordinator,
    Worker(u32),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CnpgClusterPlan {
    pub name: String,
    pub role: CnpgClusterRole,
    pub instances: u32,
    pub image: String,
    pub image_catalog_name: Option<String>,
    pub postgres_major: Option<u32>,
    pub postgres_uid: Option<u32>,
    pub postgres_gid: Option<u32>,
    pub storage_class: Option<String>,
    pub storage_size: Option<String>,
    pub shared_preload_libraries: Vec<String>,
    pub local_hostname: Option<String>,
    pub node_conninfo: Option<String>,
    pub server_ca_secret: Option<String>,
    pub server_tls_secret: Option<String>,
    pub superuser_secret: Option<String>,
    pub bootstrap_database: Option<String>,
    pub server_alt_dns_names: Vec<String>,
}

impl CnpgClusterPlan {
    pub fn endpoint(&self, namespace: &str, cluster_domain: &str) -> String {
        format!("{}-rw.{namespace}.svc.{cluster_domain}", self.name)
    }

    /// Render only fields owned by this controller. CNPG owns generated Pods,
    /// Services, PVCs, and status.
    pub fn manifest_value(&self) -> Value {
        let mut spec = json!({
            "instances": self.instances,
            "imagePullPolicy": "IfNotPresent",
            "enablePDB": true,
            "postgresql": {
                "shared_preload_libraries": self.shared_preload_libraries,
            },
        });
        if let (Some(catalog), Some(major), Some(uid), Some(gid)) = (
            &self.image_catalog_name,
            self.postgres_major,
            self.postgres_uid,
            self.postgres_gid,
        ) {
            spec["imageCatalogRef"] = json!({
                "apiGroup": "postgresql.cnpg.io",
                "kind": "ImageCatalog",
                "name": catalog,
                "major": major,
            });
            spec["postgresUID"] = json!(uid);
            spec["postgresGID"] = json!(gid);
        } else {
            spec["imageName"] = json!(self.image);
        }

        if let Some(storage_size) = &self.storage_size {
            spec["storage"] = json!({ "size": storage_size });
            if let Some(storage_class) = &self.storage_class {
                spec["storage"]["storageClass"] = json!(storage_class);
            }
        } else if let Some(storage_class) = &self.storage_class {
            spec["storage"] = json!({ "storageClass": storage_class });
        }

        if let (Some(node_conninfo), Some(local_hostname)) =
            (&self.node_conninfo, &self.local_hostname)
        {
            spec["postgresql"]["parameters"] = json!({
                "citus.node_conninfo": node_conninfo,
                "citus.local_hostname": local_hostname,
            });
        }

        if let (Some(server_ca_secret), Some(server_tls_secret)) =
            (&self.server_ca_secret, &self.server_tls_secret)
        {
            spec["certificates"] = json!({
                "serverCASecret": server_ca_secret,
                "serverTLSSecret": server_tls_secret,
                "serverAltDNSNames": self.server_alt_dns_names,
            });
        }

        if let Some(superuser_secret) = &self.superuser_secret {
            spec["enableSuperuserAccess"] = json!(true);
            spec["superuserSecret"] = json!({ "name": superuser_secret });
        }
        if let Some(database) = &self.bootstrap_database {
            spec["bootstrap"] = json!({ "initdb": { "database": database } });
        }

        json!({
            "apiVersion": "postgresql.cnpg.io/v1",
            "kind": "Cluster",
            "metadata": { "name": self.name },
            "spec": spec,
        })
    }

    pub fn manifest_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(&self.manifest_value())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CitusBootstrapPlan {
    pub config_map_name: String,
    pub coordinator_host: String,
    pub worker_hosts: Vec<String>,
    pub databases: Vec<String>,
    pub citus_version: String,
    pub companion_version: String,
    pub superuser_secret: String,
    pub server_ca_secret: String,
    pub connect_timeout_seconds: u32,
    pub backoff_limit: u32,
    pub active_deadline_seconds: u32,
    pub node_conninfo: String,
    pub postgres_uid: u32,
    pub postgres_gid: u32,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CnpgImageCatalogPlan {
    pub name: String,
    pub postgres_major: u32,
    pub image: String,
}

impl CnpgImageCatalogPlan {
    pub fn manifest_value(&self) -> Value {
        json!({
            "apiVersion": "postgresql.cnpg.io/v1",
            "kind": "ImageCatalog",
            "metadata": { "name": self.name },
            "spec": { "images": [{
                "major": self.postgres_major,
                "image": self.image,
            }]},
        })
    }
}

impl CitusBootstrapPlan {
    /// Script run by a digest-pinned operand image after every CNPG primary is
    /// ready. All interpolated tokens have already passed strict allow-list
    /// validation in `CitusProductionSpec`.
    pub fn script(&self) -> String {
        let hosts = std::iter::once(&self.coordinator_host)
            .chain(self.worker_hosts.iter())
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        let workers = self.worker_hosts.join(" ");
        let databases = self.databases.join(" ");
        let worker_sql_literals = self
            .worker_hosts
            .iter()
            .map(|worker| format!("'{worker}'"))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            r#"#!/bin/sh
set -eu

if [ "${{DB_USER}}" != "postgres" ]; then
  echo "superuser secret username must be postgres" >&2
  exit 64
fi
export PGPASSWORD="${{DB_PASSWORD}}"

connect() {{
  host="$1"
  database="$2"
  shift 2
  psql "host=${{host}} port=5432 dbname=${{database}} user=${{DB_USER}} sslmode=verify-full sslrootcert=/tls/ca.crt connect_timeout={connect_timeout}" -X -v ON_ERROR_STOP=1 "$@"
}}

# CNPG v1 does not publish an observedGeneration for Cluster status. Its
# Ready=True condition can therefore briefly describe the previous Cluster
# spec immediately after our SSA update. Do not mutate any database until the
# live endpoints themselves prove that the TLS/password/configuration inputs
# for this immutable bootstrap revision have been loaded.
preflight_attempt=1
while :; do
  preflight_ready=t
  for host in {hosts}; do
    if configured="$(connect "${{host}}" postgres -Atc "SELECT current_setting('citus.node_conninfo')" 2>/dev/null)"; then
      if [ "${{configured}}" != "{node_conninfo}" ]; then
        preflight_ready=f
        break
      fi
    else
      preflight_ready=f
      break
    fi
    if tls="$(connect "${{host}}" postgres -Atc "SELECT ssl::text FROM pg_stat_ssl WHERE pid = pg_backend_pid()" 2>/dev/null)"; then
      if [ "${{tls}}" != "true" ]; then
        preflight_ready=f
        break
      fi
    else
      preflight_ready=f
      break
    fi
  done
  if [ "${{preflight_ready}}" = "t" ]; then
    break
  fi
  if [ "${{preflight_attempt}}" -ge {preflight_attempts} ]; then
    echo "CNPG endpoints did not load the requested TLS/password/node_conninfo revision" >&2
    exit 75
  fi
  preflight_attempt=$((preflight_attempt + 1))
  sleep 5
done

for host in {hosts}; do
  for database in {databases}; do
    connect "${{host}}" postgres -v "database=${{database}}" <<'SQL'
SELECT format('CREATE DATABASE %I', :'database')
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = :'database') \gexec
SQL
    connect "${{host}}" "${{database}}" <<'SQL'
CREATE EXTENSION IF NOT EXISTS citus WITH VERSION '{citus_version}';
CREATE EXTENSION IF NOT EXISTS ai_blaise_citus WITH VERSION '{companion_version}';
SQL
    versions="$(connect "${{host}}" "${{database}}" -Atc "SELECT string_agg(extname || '=' || extversion, ',' ORDER BY extname) FROM pg_extension WHERE extname IN ('ai_blaise_citus','citus')")"
    test "${{versions}}" = "ai_blaise_citus={companion_version},citus={citus_version}"
    configured="$(connect "${{host}}" "${{database}}" -Atc "SELECT current_setting('citus.node_conninfo')")"
    test "${{configured}}" = "{node_conninfo}"
    tls="$(connect "${{host}}" "${{database}}" -Atc "SELECT ssl::text FROM pg_stat_ssl WHERE pid = pg_backend_pid()")"
    test "${{tls}}" = "true"
    connect "${{host}}" "${{database}}" -v "node_password=${{DB_PASSWORD}}" <<'SQL'
WITH escaped AS (
  SELECT replace(replace(:'node_password', chr(92), chr(92) || chr(92)), chr(39), chr(92) || chr(39)) AS value
)
INSERT INTO pg_dist_authinfo(nodeid, rolename, authinfo)
SELECT 0, current_user, 'password=' || chr(39) || value || chr(39) FROM escaped
ON CONFLICT (nodeid, rolename) DO UPDATE SET authinfo = EXCLUDED.authinfo;
SQL
  done
done

for database in {databases}; do
  connect "{coordinator}" "${{database}}" -c "SELECT citus_set_coordinator_host('{coordinator}', 5432);"
  for worker in {workers}; do
    connect "{coordinator}" "${{database}}" -v "worker=${{worker}}" <<'SQL'
SELECT citus_add_node(:'worker', 5432)
WHERE NOT EXISTS (
  SELECT FROM pg_dist_node
  WHERE nodename = :'worker' AND nodeport = 5432 AND noderole = 'primary'
);
SELECT start_metadata_sync_to_node(:'worker', 5432)
WHERE NOT EXISTS (
  SELECT FROM pg_dist_node
  WHERE nodename = :'worker' AND nodeport = 5432 AND metadatasynced
);
SQL
  done
  worker_count="$(connect "{coordinator}" "${{database}}" -Atc "SELECT count(*) FROM pg_dist_node WHERE isactive AND noderole = 'primary' AND shouldhaveshards")"
  test "${{worker_count}}" = "{worker_count}"
  topology_exact="$(connect "{coordinator}" "${{database}}" -Atc "SELECT count(*) = {node_count} AND count(DISTINCT nodename) = {node_count} AND count(DISTINCT groupid) FILTER (WHERE groupid > 0) = {worker_count} AND bool_and(nodeport = 5432 AND nodecluster = 'default' AND isactive AND noderole = 'primary' AND hasmetadata AND metadatasynced AND ((nodename = '{coordinator}' AND groupid = 0 AND NOT shouldhaveshards) OR (nodename IN ({worker_sql_literals}) AND groupid > 0 AND shouldhaveshards))) FROM pg_dist_node")"
  test "${{topology_exact}}" = "t"
  transport_tls="$(connect "{coordinator}" "${{database}}" -Atc "SELECT bool_and(result::boolean) FROM run_command_on_workers('SELECT ssl FROM pg_stat_ssl WHERE pid = pg_backend_pid()'))"
  test "${{transport_tls}}" = "t"
  expected_topology="$(connect "{coordinator}" "${{database}}" -Atc "SELECT coalesce(jsonb_agg(to_jsonb(n) - 'nodeid' ORDER BY nodename, nodeport)::text, '[]') FROM pg_dist_node n")"
  for host in {hosts}; do
    actual_topology="$(connect "${{host}}" "${{database}}" -Atc "SELECT coalesce(jsonb_agg(to_jsonb(n) - 'nodeid' ORDER BY nodename, nodeport)::text, '[]') FROM pg_dist_node n")"
    test "${{actual_topology}}" = "${{expected_topology}}"
  done
done

echo "citus bootstrap and exact-version verification passed"
"#,
            connect_timeout = self.connect_timeout_seconds,
            preflight_attempts = (self.active_deadline_seconds / 10).max(1),
            hosts = hosts,
            databases = databases,
            citus_version = self.citus_version,
            companion_version = self.companion_version,
            node_conninfo = self.node_conninfo,
            coordinator = self.coordinator_host,
            workers = workers,
            worker_count = self.worker_hosts.len(),
            worker_sql_literals = worker_sql_literals,
            node_count = self.worker_hosts.len() + 1,
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PoolDeploymentPlan {
    pub name: String,
    pub replicas: u32,
    pub geoip_db: Option<String>,
    pub security: WorkloadSecurityPlan,
}

impl PoolDeploymentPlan {
    fn from_spec(cluster_name: &str, pool: &PoolSpec) -> Self {
        Self {
            name: format!("{cluster_name}{POOL_DEPLOYMENT_NAME_SUFFIX}"),
            replicas: pool.replicas,
            geoip_db: pool.geoip_db.clone(),
            security: WorkloadSecurityPlan::pool(cluster_name),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SidecarDeploymentPlan {
    pub sidecar_type: SidecarType,
    pub replicas: u32,
    pub deployment_name: String,
    pub security: WorkloadSecurityPlan,
}

impl SidecarDeploymentPlan {
    fn from_spec(cluster_name: &str, sidecar: &SidecarSpec) -> Self {
        let suffix = sidecar_name_suffix(&sidecar.sidecar_type);
        Self {
            sidecar_type: sidecar.sidecar_type.clone(),
            replicas: sidecar.replicas,
            deployment_name: format!("{cluster_name}-sidecar-{suffix}"),
            security: WorkloadSecurityPlan::sidecar(cluster_name, &suffix),
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
        SidecarType::Custom(name) => name
            .trim()
            .chars()
            .map(|character| match character {
                'A'..='Z' => character.to_ascii_lowercase(),
                'a'..='z' | '0'..='9' | '-' => character,
                _ => '-',
            })
            .collect::<String>()
            .trim_matches('-')
            .to_string(),
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CitusClusterReconcilePlan {
    pub cluster_name: String,
    pub topology: ClusterTopologyPlan,
    pub cnpg_clusters: Vec<CnpgClusterPlan>,
    pub image_catalog: Option<CnpgImageCatalogPlan>,
    pub bootstrap: Option<CitusBootstrapPlan>,
    pub pool: Option<PoolDeploymentPlan>,
    pub sidecars: Vec<SidecarDeploymentPlan>,
    pub timescale_enabled: bool,
    pub coordinator_less: bool,
}

impl CitusClusterReconcilePlan {
    pub fn from_spec(
        cluster_name: &str,
        spec: &CitusClusterSpec,
    ) -> Result<Self, CitusClusterReconcileError> {
        Self::from_spec_in_namespace(cluster_name, "default", spec)
    }

    pub fn from_spec_in_namespace(
        cluster_name: &str,
        namespace: &str,
        spec: &CitusClusterSpec,
    ) -> Result<Self, CitusClusterReconcileError> {
        let trimmed = cluster_name.trim();
        if trimmed.is_empty() {
            return Err(CitusClusterReconcileError::MissingClusterName);
        }
        if trimmed.len() > 40 {
            return Err(CitusClusterReconcileError::ClusterNameTooLong);
        }
        if !is_dns_label(trimmed) {
            return Err(CitusClusterReconcileError::InvalidClusterName);
        }
        if !is_dns_label(namespace) {
            return Err(CitusClusterReconcileError::InvalidNamespace);
        }
        spec.validate()?;

        let worker_replicas = spec
            .production
            .as_ref()
            .map_or(1, |production| production.worker_replicas);
        let topology = ClusterTopologyPlan {
            topology: spec.topology,
            coordinators: spec.coordinators,
            workers: spec.workers,
            worker_replicas,
        };
        let libraries = shared_preload_libraries(spec);
        let all_names = std::iter::once(format!("{trimmed}{CNPG_COORDINATOR_NAME_SUFFIX}"))
            .chain(
                (0..spec.workers).map(|index| format!("{trimmed}{CNPG_WORKER_NAME_INFIX}{index}")),
            )
            .collect::<Vec<_>>();
        if let Some(production) = &spec.production {
            for name in &all_names {
                let endpoint = format!("{name}-rw.{namespace}.svc.{}", production.cluster_domain);
                if endpoint.len() > 253 {
                    return Err(CitusClusterReconcileError::DerivedDnsNameTooLong);
                }
            }
        }
        let mut cnpg_clusters = Vec::with_capacity(all_names.len());
        if spec.coordinators > 0 {
            cnpg_clusters.push(cnpg_plan(
                &all_names[0],
                CnpgClusterRole::Coordinator,
                spec.coordinators,
                spec,
                &libraries,
                &all_names,
                namespace,
            ));
        }
        for index in 0..spec.workers {
            cnpg_clusters.push(cnpg_plan(
                &format!("{trimmed}{CNPG_WORKER_NAME_INFIX}{index}"),
                CnpgClusterRole::Worker(index),
                worker_replicas,
                spec,
                &libraries,
                &all_names,
                namespace,
            ));
        }

        let pool = spec
            .pool
            .as_ref()
            .map(|pool_spec| PoolDeploymentPlan::from_spec(trimmed, pool_spec));
        if let Some(pool) = &pool {
            pool.security.validate()?;
        }
        let sidecars = spec
            .sidecars
            .iter()
            .map(|sidecar| SidecarDeploymentPlan::from_spec(trimmed, sidecar))
            .collect::<Vec<_>>();
        for sidecar in &sidecars {
            sidecar.security.validate()?;
        }

        Ok(Self {
            cluster_name: trimmed.to_string(),
            topology,
            image_catalog: spec
                .production
                .as_ref()
                .map(|production| CnpgImageCatalogPlan {
                    name: format!("{trimmed}{CNPG_IMAGE_CATALOG_NAME_SUFFIX}"),
                    postgres_major: production.postgres_major,
                    image: spec.image.clone(),
                }),
            bootstrap: spec
                .production
                .as_ref()
                .map(|production| bootstrap_plan(trimmed, namespace, &cnpg_clusters, production))
                .transpose()?,
            cnpg_clusters,
            pool,
            sidecars,
            timescale_enabled: spec.timescale_enabled,
            coordinator_less: matches!(spec.topology, CitusTopology::CoordinatorLess),
        })
    }

    pub fn total_deployments(&self) -> usize {
        usize::from(self.pool.is_some()) + self.sidecars.len()
    }

    pub fn total_postgres_instances(&self) -> u32 {
        self.topology.total_postgres_instances()
    }
}

fn is_dns_label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 63
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && value
            .as_bytes()
            .first()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && value
            .as_bytes()
            .last()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
}

fn cnpg_plan(
    name: &str,
    role: CnpgClusterRole,
    instances: u32,
    spec: &CitusClusterSpec,
    libraries: &[String],
    all_names: &[String],
    namespace: &str,
) -> CnpgClusterPlan {
    let production = spec.production.as_ref();
    let endpoint = production
        .map(|production| format!("{name}-rw.{namespace}.svc.{}", production.cluster_domain));
    let alt_names = production.map_or_else(Vec::new, |production| {
        all_names
            .iter()
            .flat_map(|cluster| {
                [
                    format!("{cluster}-rw"),
                    format!("{cluster}-rw.{namespace}"),
                    format!("{cluster}-rw.{namespace}.svc"),
                    format!("{cluster}-rw.{namespace}.svc.{}", production.cluster_domain),
                ]
            })
            .collect()
    });
    CnpgClusterPlan {
        name: name.to_string(),
        role,
        instances,
        image: spec.image.clone(),
        image_catalog_name: production.map(|_| {
            format!(
                "{}{}",
                spec_name_prefix(name),
                CNPG_IMAGE_CATALOG_NAME_SUFFIX
            )
        }),
        postgres_major: production.map(|production| production.postgres_major),
        postgres_uid: production.map(|production| production.postgres_uid),
        postgres_gid: production.map(|production| production.postgres_gid),
        storage_class: spec.storage_class.clone(),
        storage_size: production.map(|production| production.storage_size.clone()),
        shared_preload_libraries: libraries.to_vec(),
        local_hostname: endpoint,
        node_conninfo: production.map(|production| production.node_tls.node_conninfo()),
        server_ca_secret: production.map(|production| production.node_tls.server_ca_secret.clone()),
        server_tls_secret: production
            .map(|production| production.node_tls.server_tls_secret.clone()),
        superuser_secret: production.map(|production| production.node_tls.superuser_secret.clone()),
        bootstrap_database: production.and_then(|production| production.databases.first().cloned()),
        server_alt_dns_names: alt_names,
    }
}

fn bootstrap_plan(
    cluster_name: &str,
    namespace: &str,
    clusters: &[CnpgClusterPlan],
    production: &CitusProductionSpec,
) -> Result<CitusBootstrapPlan, CitusClusterReconcileError> {
    let coordinator = clusters
        .iter()
        .find(|cluster| matches!(cluster.role, CnpgClusterRole::Coordinator))
        .ok_or(CitusClusterReconcileError::ProductionBootstrapRequiresCoordinator)?;
    let coordinator_host = coordinator.endpoint(namespace, &production.cluster_domain);
    let worker_hosts = clusters
        .iter()
        .filter(|cluster| matches!(cluster.role, CnpgClusterRole::Worker(_)))
        .map(|cluster| cluster.endpoint(namespace, &production.cluster_domain))
        .collect();
    Ok(CitusBootstrapPlan {
        config_map_name: format!("{cluster_name}{BOOTSTRAP_CONFIG_MAP_NAME_SUFFIX}"),
        coordinator_host,
        worker_hosts,
        databases: production.databases.clone(),
        citus_version: production.extension_versions.citus.clone(),
        companion_version: production.extension_versions.companion.clone(),
        superuser_secret: production.node_tls.superuser_secret.clone(),
        server_ca_secret: production.node_tls.server_ca_secret.clone(),
        connect_timeout_seconds: production.node_tls.connect_timeout_seconds,
        backoff_limit: production.bootstrap.backoff_limit,
        active_deadline_seconds: production.bootstrap.active_deadline_seconds,
        node_conninfo: production.node_tls.node_conninfo(),
        postgres_uid: production.postgres_uid,
        postgres_gid: production.postgres_gid,
    })
}

fn spec_name_prefix(cluster_name: &str) -> &str {
    cluster_name
        .strip_suffix(CNPG_COORDINATOR_NAME_SUFFIX)
        .or_else(|| {
            cluster_name
                .rsplit_once(CNPG_WORKER_NAME_INFIX)
                .map(|(prefix, _)| prefix)
        })
        .unwrap_or(cluster_name)
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
    InvalidWorkloadSecurity(WorkloadSecurityError),
    MissingClusterName,
    ClusterNameTooLong,
    InvalidClusterName,
    InvalidNamespace,
    DerivedDnsNameTooLong,
    ProductionBootstrapRequiresCoordinator,
}

impl fmt::Display for CitusClusterReconcileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSpec(error) => write!(formatter, "{error}"),
            Self::InvalidWorkloadSecurity(error) => write!(formatter, "{error}"),
            Self::MissingClusterName => write!(formatter, "cluster_name must not be empty"),
            Self::ClusterNameTooLong => write!(
                formatter,
                "cluster_name must be at most 40 characters so derived resources remain DNS labels"
            ),
            Self::InvalidClusterName => write!(
                formatter,
                "cluster_name must be a lowercase Kubernetes DNS label because CNPG derives Services from it"
            ),
            Self::InvalidNamespace => write!(
                formatter,
                "namespace must be a lowercase Kubernetes DNS label"
            ),
            Self::DerivedDnsNameTooLong => write!(
                formatter,
                "a derived CNPG read-write Service FQDN exceeds the 253-byte DNS limit"
            ),
            Self::ProductionBootstrapRequiresCoordinator => write!(
                formatter,
                "production bootstrap requires a coordinator-worker topology"
            ),
        }
    }
}

impl Error for CitusClusterReconcileError {}

impl From<CitusClusterSpecError> for CitusClusterReconcileError {
    fn from(error: CitusClusterSpecError) -> Self {
        Self::InvalidSpec(error)
    }
}

impl From<WorkloadSecurityError> for CitusClusterReconcileError {
    fn from(error: WorkloadSecurityError) -> Self {
        Self::InvalidWorkloadSecurity(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crds::citus_cluster::{
        BootstrapJobSpec, ExactExtensionVersions, NodeTlsSpec, CNPG_SERVER_CA_PATH,
    };

    fn production() -> CitusProductionSpec {
        CitusProductionSpec {
            postgres_major: 17,
            postgres_uid: 999,
            postgres_gid: 999,
            worker_replicas: 2,
            storage_size: "10Gi".to_string(),
            cluster_domain: "cluster.local".to_string(),
            databases: vec!["app".to_string(), "analytics".to_string()],
            extension_versions: ExactExtensionVersions {
                citus: "13.2-1".to_string(),
                companion: "1.0".to_string(),
            },
            node_tls: NodeTlsSpec {
                server_ca_secret: "citus-ca".to_string(),
                server_tls_secret: "citus-server".to_string(),
                superuser_secret: "citus-superuser".to_string(),
                ssl_mode: "verify-full".to_string(),
                ssl_root_cert: CNPG_SERVER_CA_PATH.to_string(),
                connect_timeout_seconds: 5,
            },
            bootstrap: BootstrapJobSpec {
                backoff_limit: 3,
                active_deadline_seconds: 600,
            },
        }
    }

    fn production_spec() -> CitusClusterSpec {
        CitusClusterSpec {
            topology: CitusTopology::CoordinatorWorker,
            image: format!("ghcr.io/ai-blaise/citus@sha256:{}", "a".repeat(64)),
            workers: 3,
            coordinators: 3,
            storage_class: Some("fast-ssd".to_string()),
            timescale_enabled: false,
            extensions: vec!["citus".to_string(), "ai_blaise_citus".to_string()],
            pool: None,
            sidecars: Vec::new(),
            production: Some(production()),
        }
    }

    #[test]
    fn production_plan_renders_distinct_cnpg_groups_and_exact_tls() {
        let spec = production_spec();
        spec.validate_apply_ready().expect("apply ready");
        let plan = CitusClusterReconcilePlan::from_spec("primary", &spec).expect("valid plan");

        assert_eq!(plan.cnpg_clusters.len(), 4);
        assert_eq!(plan.total_postgres_instances(), 9);
        assert_eq!(plan.cnpg_clusters[0].name, "primary-coordinator");
        assert_eq!(plan.cnpg_clusters[1].name, "primary-worker-0");
        assert_eq!(
            plan.image_catalog.as_ref().expect("catalog").name,
            "primary-images"
        );
        let manifest = plan.cnpg_clusters[0].manifest_value();
        assert_eq!(manifest["spec"]["instances"], 3);
        assert_eq!(manifest["spec"]["imageCatalogRef"]["major"], 17);
        assert_eq!(manifest["spec"]["postgresUID"], 999);
        assert!(plan.cnpg_clusters[0].manifest_yaml().is_ok());
        assert_eq!(
            manifest["spec"]["postgresql"]["parameters"]["citus.node_conninfo"],
            "sslmode=verify-full sslrootcert=/controller/certificates/server-ca.crt connect_timeout=5"
        );
        assert_eq!(
            manifest["spec"]["certificates"]["serverCASecret"],
            "citus-ca"
        );
        assert_eq!(
            manifest["spec"]["certificates"]["serverTLSSecret"],
            "citus-server"
        );
        assert!(manifest["spec"]["postgresql"]["shared_preload_libraries"].is_array());
    }

    #[test]
    fn bootstrap_script_binds_exact_versions_tls_topology_and_bounded_retry_inputs() {
        let plan = CitusClusterReconcilePlan::from_spec("primary", &production_spec())
            .expect("valid plan");
        let bootstrap = plan.bootstrap.expect("bootstrap plan");
        let script = bootstrap.script();

        assert!(script.contains("CREATE EXTENSION IF NOT EXISTS citus WITH VERSION '13.2-1'"));
        assert!(script.contains("ai_blaise_citus WITH VERSION '1.0'"));
        assert!(script.contains("current_setting('citus.node_conninfo')"));
        assert!(script.contains("sslmode=verify-full sslrootcert=/tls/ca.crt"));
        assert!(script.contains("pg_dist_authinfo"));
        assert!(script.contains("run_command_on_workers"));
        assert!(script.contains("start_metadata_sync_to_node"));
        assert!(script.contains("topology_exact"));
        assert!(script.contains(
            "count(*) = 4 AND count(DISTINCT nodename) = 4 AND count(DISTINCT groupid) FILTER (WHERE groupid > 0) = 3 AND bool_and"
        ));
        assert!(script.contains("groupid = 0 AND NOT shouldhaveshards"));
        assert!(script.contains("hasmetadata AND metadatasynced"));
        assert!(script.contains("nodecluster = 'default'"));
        assert!(script.contains("preflight_attempt"));
        assert!(script.contains("exit 75"));
        assert_eq!(bootstrap.backoff_limit, 3);
        assert_eq!(bootstrap.active_deadline_seconds, 600);
    }

    #[test]
    fn legacy_plan_keeps_deterministic_counts_without_becoming_apply_ready() {
        let spec = CitusClusterSpec {
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
        };
        let plan = CitusClusterReconcilePlan::from_spec("legacy", &spec).expect("legacy plan");
        assert_eq!(plan.total_postgres_instances(), 4);
        assert_eq!(plan.cnpg_clusters.len(), 4);
        assert_eq!(
            spec.validate_apply_ready(),
            Err(CitusClusterSpecError::MutableImage)
        );
    }

    #[test]
    fn coordinator_less_plan_omits_dedicated_coordinator_instances() {
        let spec = CitusClusterSpec {
            topology: CitusTopology::CoordinatorLess,
            image: "ghcr.io/ai-blaise/citus:pg18-v2".to_string(),
            workers: 3,
            coordinators: 0,
            storage_class: None,
            timescale_enabled: false,
            extensions: vec!["citus".to_string()],
            pool: Some(PoolSpec {
                replicas: 2,
                geoip_db: None,
            }),
            sidecars: Vec::new(),
            production: None,
        };

        let plan = CitusClusterReconcilePlan::from_spec("mx", &spec).expect("coordinator-less");
        assert_eq!(plan.total_postgres_instances(), 3);
        assert_eq!(plan.cnpg_clusters.len(), 3);
        assert!(plan
            .cnpg_clusters
            .iter()
            .all(|cluster| matches!(cluster.role, CnpgClusterRole::Worker(_))));
        assert!(plan.coordinator_less);
    }

    #[test]
    fn production_coordinator_less_plan_returns_typed_error_instead_of_panicking() {
        let mut spec = production_spec();
        spec.topology = CitusTopology::CoordinatorLess;
        spec.coordinators = 0;
        spec.pool = Some(PoolSpec {
            replicas: 2,
            geoip_db: None,
        });

        assert_eq!(spec.validate(), Ok(()));
        assert_eq!(
            CitusClusterReconcilePlan::from_spec("mx", &spec),
            Err(CitusClusterReconcileError::ProductionBootstrapRequiresCoordinator)
        );
    }

    #[test]
    fn empty_and_overlong_cluster_names_are_rejected() {
        let spec = production_spec();
        assert_eq!(
            CitusClusterReconcilePlan::from_spec("   ", &spec),
            Err(CitusClusterReconcileError::MissingClusterName)
        );
        assert_eq!(
            CitusClusterReconcilePlan::from_spec(&"a".repeat(41), &spec),
            Err(CitusClusterReconcileError::ClusterNameTooLong)
        );
        assert_eq!(
            CitusClusterReconcilePlan::from_spec("primary.db", &spec),
            Err(CitusClusterReconcileError::InvalidClusterName)
        );
        assert_eq!(
            CitusClusterReconcilePlan::from_spec_in_namespace("primary", "Database", &spec),
            Err(CitusClusterReconcileError::InvalidNamespace)
        );
    }

    #[test]
    fn derived_service_fqdn_must_fit_the_dns_wire_contract() {
        let mut spec = production_spec();
        spec.production.as_mut().expect("production").cluster_domain = [
            "a".repeat(63),
            "b".repeat(63),
            "c".repeat(63),
            "d".repeat(61),
        ]
        .join(".");
        assert_eq!(
            CitusClusterReconcilePlan::from_spec_in_namespace("primary", "database", &spec),
            Err(CitusClusterReconcileError::DerivedDnsNameTooLong)
        );
    }
}
