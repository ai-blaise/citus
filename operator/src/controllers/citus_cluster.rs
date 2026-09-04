// FEATURE: S4
//! Production `CitusCluster` controller backed by CloudNativePG.

use super::{boundary::ExecutionMode, Context, ControllerError};
use crate::crds::citus_cluster::{
    BootstrapJobSpec, CitusClusterSpec, CitusProductionSpec, CitusTopology, ExactExtensionVersions,
    NodeTlsSpec, PoolSpec, SidecarSpec, SidecarType, CNPG_SERVER_CA_PATH,
};
use crate::reconcile::citus_cluster::{
    CitusBootstrapPlan, CitusClusterReconcilePlan, CnpgClusterPlan,
};
use futures::StreamExt;
use k8s_openapi::api::{
    batch::v1::Job,
    core::v1::{ConfigMap, Pod, Secret},
};
use kube::{
    api::{Api, DeleteParams, ListParams, Patch, PatchParams, PostParams},
    core::{ApiResource, DynamicObject, GroupVersionKind},
    runtime::{
        controller::Action,
        finalizer::{finalizer, Event},
        watcher, Controller,
    },
    CustomResource, Resource, ResourceExt,
};
use rustls::{
    client::{danger::ServerCertVerifier, WebPkiServerVerifier},
    pki_types::{CertificateDer, ServerName, UnixTime},
    RootCertStore,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tracing::{debug, error, info};
use x509_parser::{extensions::GeneralName, pem::parse_x509_pem};

pub const CITUS_CLUSTER_FINALIZER: &str = "citusclusters.citus.ai-blaise.io/finalizer";
const FIELD_MANAGER: &str = "ai-blaise-citus-cluster-controller";
const PROGRESS_REQUEUE: Duration = Duration::from_secs(15);
const RECONCILE_CONTRACT_VERSION: &str = "citus-cluster-production-v1";
const CNPG_RELOAD_LABEL: &str = "cnpg.io/reload";
const MANAGED_BY_LABEL: &str = "app.kubernetes.io/managed-by";
const MANAGED_BY_VALUE: &str = "ai-blaise-citus-operator";
const CLUSTER_LABEL: &str = "citus.ai-blaise.io/cluster";
const CNPG_HEALTHY_PHASE: &str = "Cluster in healthy state";
const CNPG_CLUSTER_LABEL: &str = "cnpg.io/cluster";
const CNPG_POD_ROLE_LABEL: &str = "cnpg.io/podRole";
const CNPG_INSTANCE_POD_ROLE: &str = "instance";
const POSTGRES_PORT: u16 = 5432;
const POSTGRES_SSL_REQUEST: [u8; 8] = [0, 0, 0, 8, 4, 210, 22, 47];

#[derive(Debug)]
struct ValidatedSecretRevision {
    reconcile_hash: String,
    server_ca_der: Vec<u8>,
    server_leaf_sha256: [u8; 32],
    secret_revisions: Vec<SecretRevision>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SecretRevision {
    name: String,
    uid: String,
    resource_version: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CnpgInstanceTlsTarget {
    cluster_name: String,
    pod_name: String,
    pod_ip: IpAddr,
    server_name: String,
}

#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "citus.ai-blaise.io",
    version = "v2",
    kind = "CitusCluster",
    namespaced,
    status = "CitusClusterStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct CitusClusterCrSpec {
    pub image: String,
    pub workers: u32,
    #[serde(default = "default_coordinators")]
    pub coordinators: u32,
    #[serde(default)]
    pub coordinator_less: bool,
    #[serde(default)]
    pub timescale_enabled: bool,
    #[serde(default = "default_extensions")]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub storage_class: Option<String>,
    #[serde(default)]
    pub pool_replicas: Option<u32>,
    #[serde(default)]
    pub sidecars: Vec<SidecarSpecCr>,
    #[serde(default)]
    pub production: Option<CitusProductionSpecCr>,
}

fn default_coordinators() -> u32 {
    1
}

fn default_extensions() -> Vec<String> {
    vec!["citus".to_string()]
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SidecarSpecCr {
    pub kind: String,
    #[serde(default = "default_one")]
    pub replicas: u32,
}

fn default_one() -> u32 {
    1
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CitusProductionSpecCr {
    pub postgres_major: u32,
    pub postgres_uid: u32,
    pub postgres_gid: u32,
    #[serde(default = "default_one")]
    pub worker_replicas: u32,
    pub storage_size: String,
    #[serde(default = "default_cluster_domain")]
    pub cluster_domain: String,
    pub databases: Vec<String>,
    pub extension_versions: ExactExtensionVersionsCr,
    pub node_tls: NodeTlsSpecCr,
    #[serde(default)]
    pub bootstrap: BootstrapJobSpecCr,
}

fn default_cluster_domain() -> String {
    "cluster.local".to_string()
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExactExtensionVersionsCr {
    pub citus: String,
    pub companion: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NodeTlsSpecCr {
    pub server_ca_secret: String,
    pub server_tls_secret: String,
    pub superuser_secret: String,
    #[serde(default = "default_ssl_mode")]
    pub ssl_mode: String,
    #[serde(default = "default_ssl_root_cert")]
    pub ssl_root_cert: String,
    #[serde(default = "default_connect_timeout_seconds")]
    pub connect_timeout_seconds: u32,
}

fn default_ssl_mode() -> String {
    "verify-full".to_string()
}

fn default_ssl_root_cert() -> String {
    CNPG_SERVER_CA_PATH.to_string()
}

fn default_connect_timeout_seconds() -> u32 {
    5
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapJobSpecCr {
    #[serde(default = "default_backoff_limit")]
    pub backoff_limit: u32,
    #[serde(default = "default_active_deadline_seconds")]
    pub active_deadline_seconds: u32,
}

impl Default for BootstrapJobSpecCr {
    fn default() -> Self {
        Self {
            backoff_limit: default_backoff_limit(),
            active_deadline_seconds: default_active_deadline_seconds(),
        }
    }
}

fn default_backoff_limit() -> u32 {
    3
}

fn default_active_deadline_seconds() -> u32 {
    600
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CitusClusterStatus {
    pub phase: String,
    pub observed_generation: Option<i64>,
    pub applied_spec_hash: String,
    pub cnpg_clusters: Vec<CnpgResourceStatus>,
    pub bootstrap_job: Option<String>,
    pub expected_extensions: Option<ExactExtensionVersionsCr>,
    pub node_conninfo: Option<String>,
    pub last_error: Option<String>,
    pub conditions: Vec<CitusClusterCondition>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CnpgResourceStatus {
    pub name: String,
    pub desired_instances: u32,
    pub ready_instances: u32,
    pub phase: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CitusClusterCondition {
    #[serde(rename = "type")]
    pub condition_type: String,
    pub status: String,
    pub reason: String,
    pub message: String,
    pub observed_generation: Option<i64>,
    pub last_transition_time: String,
}

impl CitusClusterCrSpec {
    pub fn to_authoritative(&self) -> CitusClusterSpec {
        let topology = if self.coordinator_less {
            CitusTopology::CoordinatorLess
        } else {
            CitusTopology::CoordinatorWorker
        };
        CitusClusterSpec {
            topology,
            image: self.image.clone(),
            workers: self.workers,
            coordinators: if self.coordinator_less {
                0
            } else {
                self.coordinators
            },
            storage_class: self.storage_class.clone(),
            timescale_enabled: self.timescale_enabled,
            extensions: self.extensions.clone(),
            pool: self.pool_replicas.map(|replicas| PoolSpec {
                replicas,
                geoip_db: None,
            }),
            sidecars: self
                .sidecars
                .iter()
                .map(|sidecar| SidecarSpec {
                    sidecar_type: parse_sidecar_kind(&sidecar.kind),
                    replicas: sidecar.replicas,
                })
                .collect(),
            production: self
                .production
                .as_ref()
                .map(|production| CitusProductionSpec {
                    postgres_major: production.postgres_major,
                    postgres_uid: production.postgres_uid,
                    postgres_gid: production.postgres_gid,
                    worker_replicas: production.worker_replicas,
                    storage_size: production.storage_size.clone(),
                    cluster_domain: production.cluster_domain.clone(),
                    databases: production.databases.clone(),
                    extension_versions: ExactExtensionVersions {
                        citus: production.extension_versions.citus.clone(),
                        companion: production.extension_versions.companion.clone(),
                    },
                    node_tls: NodeTlsSpec {
                        server_ca_secret: production.node_tls.server_ca_secret.clone(),
                        server_tls_secret: production.node_tls.server_tls_secret.clone(),
                        superuser_secret: production.node_tls.superuser_secret.clone(),
                        ssl_mode: production.node_tls.ssl_mode.clone(),
                        ssl_root_cert: production.node_tls.ssl_root_cert.clone(),
                        connect_timeout_seconds: production.node_tls.connect_timeout_seconds,
                    },
                    bootstrap: BootstrapJobSpec {
                        backoff_limit: production.bootstrap.backoff_limit,
                        active_deadline_seconds: production.bootstrap.active_deadline_seconds,
                    },
                }),
        }
    }
}

fn parse_sidecar_kind(kind: &str) -> SidecarType {
    match kind {
        "Analytical" => SidecarType::Analytical,
        "Vectorizer" => SidecarType::Vectorizer,
        "Cdc" => SidecarType::Cdc,
        "ColdTier" => SidecarType::ColdTier,
        "Raft" => SidecarType::Raft,
        "Hlc" => SidecarType::Hlc,
        "TxnStatus" => SidecarType::TxnStatus,
        "SchemaJob" => SidecarType::SchemaJob,
        "Realtime" => SidecarType::Realtime,
        "Auth" => SidecarType::Auth,
        "Storage" => SidecarType::Storage,
        "Postgrest" => SidecarType::Postgrest,
        "Graphql" => SidecarType::Graphql,
        "EdgeFunctions" => SidecarType::EdgeFunctions,
        "Backup" => SidecarType::Backup,
        "Repack" => SidecarType::Repack,
        "Mcp" => SidecarType::Mcp,
        other => SidecarType::Custom(other.to_string()),
    }
}

pub async fn run(ctx: Arc<Context>) -> Result<(), ControllerError> {
    let api: Api<CitusCluster> = Api::default_namespaced(ctx.client.clone());
    info!("CitusCluster controller starting");
    Controller::new(api, watcher::Config::default())
        .run(reconcile, error_policy, ctx)
        .for_each(|result| async move {
            match result {
                Ok((object, _action)) => debug!(?object, "reconciled CitusCluster"),
                Err(error) => error!(?error, "CitusCluster reconcile error"),
            }
        })
        .await;
    Ok(())
}

async fn reconcile(
    cluster: Arc<CitusCluster>,
    ctx: Arc<Context>,
) -> Result<Action, ControllerError> {
    if matches!(ctx.execution_mode, ExecutionMode::DryRun) {
        let namespace = cluster.namespace().unwrap_or_else(|| "default".to_string());
        let authoritative = cluster.spec.to_authoritative();
        authoritative
            .validate()
            .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
        let plan = CitusClusterReconcilePlan::from_spec_in_namespace(
            &cluster.name_any(),
            &namespace,
            &authoritative,
        )
        .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
        info!(
            cluster = %cluster.name_any(),
            cnpg_clusters = plan.cnpg_clusters.len(),
            cnpg_instances = plan.total_postgres_instances(),
            mode = "dry-run",
            "CitusCluster reconcile plan built without mutation"
        );
        return Ok(Action::requeue(ctx.default_requeue));
    }

    let api: Api<CitusCluster> = Api::default_namespaced(ctx.client.clone());
    finalizer(&api, CITUS_CLUSTER_FINALIZER, cluster, |event| {
        let ctx = ctx.clone();
        async move {
            match event {
                Event::Apply(cluster) => reconcile_apply(cluster, ctx).await,
                Event::Cleanup(cluster) => reconcile_cleanup(cluster, ctx).await,
            }
        }
    })
    .await
    .map_err(|error| ControllerError::Finalizer(error.to_string()))
}

async fn reconcile_apply(
    cluster: Arc<CitusCluster>,
    ctx: Arc<Context>,
) -> Result<Action, ControllerError> {
    let namespace = cluster.namespace().unwrap_or_else(|| "default".to_string());
    let generation = cluster.metadata.generation;
    let spec_hash = spec_hash(&cluster.spec)?;
    let authoritative = cluster.spec.to_authoritative();
    if let Err(error) = authoritative.validate_apply_ready() {
        patch_cluster_status(
            &cluster,
            &ctx,
            invalid_status(
                generation,
                &spec_hash,
                error.to_string(),
                cluster.status.as_ref(),
            ),
        )
        .await?;
        return Ok(Action::requeue(ctx.default_requeue));
    }
    let plan = match CitusClusterReconcilePlan::from_spec_in_namespace(
        &cluster.name_any(),
        &namespace,
        &authoritative,
    ) {
        Ok(plan) => plan,
        Err(error) => {
            patch_cluster_status(
                &cluster,
                &ctx,
                invalid_status(
                    generation,
                    &spec_hash,
                    error.to_string(),
                    cluster.status.as_ref(),
                ),
            )
            .await?;
            return Ok(Action::requeue(ctx.default_requeue));
        }
    };
    if let Err(error) = validate_no_cnpg_group_removal(cluster.status.as_ref(), &plan) {
        patch_cluster_status(
            &cluster,
            &ctx,
            invalid_status(
                generation,
                &spec_hash,
                error.to_string(),
                cluster.status.as_ref(),
            ),
        )
        .await?;
        return Ok(Action::requeue(ctx.default_requeue));
    }
    let production = authoritative.production.as_ref().ok_or_else(|| {
        ControllerError::InvalidSpec(
            "validated production apply spec did not contain production settings".to_string(),
        )
    })?;

    let validated_secrets = match validate_referenced_secrets(
        &ctx, &namespace, production, &plan, &spec_hash, generation,
    )
    .await
    {
        Ok(validated_secrets) => validated_secrets,
        Err(error @ ControllerError::InvalidSpec(_)) => {
            patch_cluster_status(
                &cluster,
                &ctx,
                invalid_status(
                    generation,
                    &spec_hash,
                    error.to_string(),
                    cluster.status.as_ref(),
                ),
            )
            .await?;
            return Ok(Action::requeue(ctx.default_requeue));
        }
        Err(error) => return Err(error),
    };
    let reconcile_hash = validated_secrets.reconcile_hash.as_str();

    let owner_uid = cluster.metadata.uid.as_deref().ok_or_else(|| {
        ControllerError::InvalidSpec("CitusCluster owner reference requires metadata.uid".into())
    })?;
    let owner = cluster.controller_owner_ref(&()).ok_or_else(|| {
        ControllerError::InvalidSpec("CitusCluster owner reference requires metadata.uid".into())
    })?;
    let owner = serde_json::to_value(owner)
        .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
    if !reconcile_input_is_current(&ctx, &namespace, &cluster).await? {
        return Ok(Action::requeue(PROGRESS_REQUEUE));
    }
    match validate_live_cnpg_changes(&ctx, &namespace, owner_uid, &plan, production).await {
        Ok(()) => {}
        Err(error @ ControllerError::InvalidSpec(_)) => {
            patch_cluster_status(
                &cluster,
                &ctx,
                invalid_status(
                    generation,
                    &spec_hash,
                    error.to_string(),
                    cluster.status.as_ref(),
                ),
            )
            .await?;
            return Ok(Action::requeue(ctx.default_requeue));
        }
        Err(error) => return Err(error),
    }
    let desired_bootstrap_job =
        format!("{}-bootstrap-{}", cluster.name_any(), &reconcile_hash[..12]);
    if !quiesce_superseded_bootstrap_jobs(&ctx, &namespace, &cluster, &desired_bootstrap_job)
        .await?
    {
        info!(
            cluster = %cluster.name_any(),
            desired_bootstrap_job,
            "waiting for superseded bootstrap Pods to terminate before mutating the next revision"
        );
        return Ok(Action::requeue(PROGRESS_REQUEUE));
    }
    let apply = PatchParams::apply(FIELD_MANAGER).force();
    if let Some(catalog) = &plan.image_catalog {
        let manifest = manifest_with_metadata(
            catalog.manifest_value(),
            &namespace,
            owner.clone(),
            &cluster.name_any(),
            reconcile_hash,
            production,
        )?;
        let api = image_catalog_api(ctx.client.clone(), &namespace);
        create_or_apply_dynamic(&api, &catalog.name, manifest, owner_uid, &apply).await?;
    }
    let cnpg_api = cnpg_api(ctx.client.clone(), &namespace);
    let mut cnpg_statuses = Vec::with_capacity(plan.cnpg_clusters.len());
    let mut applied_cnpg_resources = Vec::with_capacity(plan.cnpg_clusters.len());
    let mut all_cnpg_resources_converged = true;
    for child in &plan.cnpg_clusters {
        let manifest = manifest_with_metadata(
            child.manifest_value(),
            &namespace,
            owner.clone(),
            &cluster.name_any(),
            reconcile_hash,
            production,
        )?;
        let applied =
            create_or_apply_dynamic(&cnpg_api, &child.name, manifest, owner_uid, &apply).await?;
        all_cnpg_resources_converged &= cnpg_resource_converged(child, &applied);
        cnpg_statuses.push(cnpg_resource_status(child.instances, &applied));
        applied_cnpg_resources.push(applied);
    }

    let mut postgres_ready = cnpg_statuses
        .iter()
        .all(|status| status.ready_instances == status.desired_instances)
        && all_cnpg_resources_converged;
    if postgres_ready {
        postgres_ready = cnpg_server_leaf_certificates_current(
            &ctx,
            &namespace,
            production,
            &plan.cnpg_clusters,
            &applied_cnpg_resources,
            &validated_secrets.server_ca_der,
            validated_secrets.server_leaf_sha256,
        )
        .await?;
    }
    // Retain the last successful/attempted immutable Job name while replacement
    // CNPG resources converge so it can be owner-checked and pruned only after
    // the new revision succeeds.
    let mut bootstrap_job = cluster
        .status
        .as_ref()
        .and_then(|status| status.bootstrap_job.clone());
    let mut verification_state = BootstrapState::Pending;
    if postgres_ready {
        if !reconcile_input_is_current(&ctx, &namespace, &cluster).await? {
            return Ok(Action::requeue(PROGRESS_REQUEUE));
        }
        if !secret_revisions_are_current(&ctx, &namespace, &validated_secrets.secret_revisions)
            .await?
        {
            return Ok(Action::requeue(PROGRESS_REQUEUE));
        }
        let bootstrap = plan.bootstrap.as_ref().ok_or_else(|| {
            ControllerError::InvalidSpec(
                "validated production apply plan did not contain bootstrap intent".to_string(),
            )
        })?;
        apply_bootstrap_config_map(
            &ctx,
            &namespace,
            &owner,
            owner_uid,
            &cluster.name_any(),
            reconcile_hash,
            bootstrap,
        )
        .await?;
        let job = apply_bootstrap_job(
            &ctx,
            &namespace,
            &owner,
            owner_uid,
            &cluster.name_any(),
            reconcile_hash,
            &desired_bootstrap_job,
            &authoritative.image,
            bootstrap,
        )
        .await?;
        verification_state = bootstrap_state(&job);
        bootstrap_job = Some(desired_bootstrap_job);
    }

    let status = ready_status(
        generation,
        reconcile_hash,
        cnpg_statuses,
        bootstrap_job.clone(),
        postgres_ready,
        verification_state,
        production,
    );
    let ready = status.phase == "Ready";
    if ready
        && !secret_revisions_are_current(&ctx, &namespace, &validated_secrets.secret_revisions)
            .await?
    {
        return Ok(Action::requeue(PROGRESS_REQUEUE));
    }
    if ready {
        cleanup_superseded_bootstrap(&ctx, &namespace, &cluster, bootstrap_job.as_deref()).await?;
    }
    patch_cluster_status(&cluster, &ctx, status).await?;
    info!(
        cluster = %cluster.name_any(),
        observed_generation = ?generation,
        postgres_ready,
        ready,
        "CitusCluster desired state server-side applied"
    );
    if ready {
        Ok(Action::requeue(ctx.default_requeue))
    } else {
        Ok(Action::requeue(PROGRESS_REQUEUE))
    }
}

async fn reconcile_input_is_current(
    ctx: &Context,
    namespace: &str,
    cluster: &CitusCluster,
) -> Result<bool, ControllerError> {
    let api: Api<CitusCluster> = Api::namespaced(ctx.client.clone(), namespace);
    let current = api.get(&cluster.name_any()).await?;
    Ok(same_object_revision(&cluster.metadata, &current.metadata))
}

fn same_object_revision(
    candidate: &kube::core::ObjectMeta,
    current: &kube::core::ObjectMeta,
) -> bool {
    candidate.uid.is_some()
        && candidate.uid == current.uid
        && candidate.resource_version.is_some()
        && candidate.resource_version == current.resource_version
        && candidate.generation == current.generation
}

fn validate_no_cnpg_group_removal(
    previous: Option<&CitusClusterStatus>,
    plan: &CitusClusterReconcilePlan,
) -> Result<(), ControllerError> {
    let Some(previous) = previous else {
        return Ok(());
    };
    validate_no_removed_cnpg_groups(
        previous
            .cnpg_clusters
            .iter()
            .map(|cluster| cluster.name.as_str()),
        plan,
    )
}

async fn validate_live_cnpg_changes(
    ctx: &Context,
    namespace: &str,
    owner_uid: &str,
    plan: &CitusClusterReconcilePlan,
    production: &CitusProductionSpec,
) -> Result<(), ControllerError> {
    let api = cnpg_api(ctx.client.clone(), namespace);
    let resources = api.list(&ListParams::default()).await?;
    validate_no_removed_cnpg_groups(
        resources
            .iter()
            .filter(|resource| controller_owned_by(&resource.metadata.owner_references, owner_uid))
            .map(ResourceExt::name_any),
        plan,
    )?;

    let desired = plan
        .cnpg_clusters
        .iter()
        .map(|cluster| (cluster.name.as_str(), cluster))
        .collect::<std::collections::BTreeMap<_, _>>();
    for resource in &resources.items {
        let name = resource.name_any();
        let Some(cluster_plan) = desired.get(name.as_str()) else {
            continue;
        };
        ensure_owned_by(&resource.metadata.owner_references, owner_uid, &name)?;
        validate_cnpg_mutation_is_observable(cluster_plan, resource, production)?;
    }
    Ok(())
}

/// CNPG v1 does not expose an observed generation for Cluster status. Fields
/// without an independent live-state fence must consequently remain immutable:
/// accepting them would allow a previous Ready=True status to race our SSA and
/// admit bootstrap before CNPG had processed the new specification.
fn validate_cnpg_mutation_is_observable(
    plan: &CnpgClusterPlan,
    existing: &DynamicObject,
    production: &CitusProductionSpec,
) -> Result<(), ControllerError> {
    let desired = plan.manifest_value();
    let immutable_fields = [
        ("/spec/storage/size", "production.storageSize"),
        ("/spec/storage/storageClass", "spec.storageClass"),
        ("/spec/imageCatalogRef/major", "production.postgresMajor"),
        ("/spec/postgresUID", "production.postgresUid"),
        ("/spec/postgresGID", "production.postgresGid"),
        ("/spec/bootstrap/initdb/database", "production.databases[0]"),
    ];
    for (pointer, field) in immutable_fields {
        if normalized_cnpg_field(existing.data.pointer(pointer))
            != normalized_cnpg_field(desired.pointer(pointer))
        {
            return Err(ControllerError::InvalidSpec(format!(
                "changing {field} on existing CNPG Cluster {} is fail-closed because CNPG v1 has no observed-generation acknowledgement; reprovision through the reviewed data-migration path",
                plan.name
            )));
        }
    }
    for (annotation, desired, field) in [
        (
            "citus.ai-blaise.io/citus-extension-version",
            production.extension_versions.citus.as_str(),
            "production.extensionVersions.citus",
        ),
        (
            "citus.ai-blaise.io/companion-extension-version",
            production.extension_versions.companion.as_str(),
            "production.extensionVersions.companion",
        ),
    ] {
        let existing_version = existing
            .metadata
            .annotations
            .as_ref()
            .and_then(|annotations| annotations.get(annotation))
            .map(String::as_str);
        if existing_version != Some(desired) {
            return Err(ControllerError::InvalidSpec(format!(
                "changing {field} on existing CNPG Cluster {} is fail-closed because the bootstrap contract does not perform extension upgrades; use the reviewed extension-upgrade path",
                plan.name
            )));
        }
    }
    Ok(())
}

/// Kubernetes JSON serialization may represent an unset optional field as
/// either absent or explicit `null`. Treat only those representations as
/// equivalent; concrete API-defaulted values still require exact equality.
fn normalized_cnpg_field(value: Option<&Value>) -> Option<&Value> {
    value.filter(|value| !value.is_null())
}

fn validate_no_removed_cnpg_groups<I, S>(
    existing: I,
    plan: &CitusClusterReconcilePlan,
) -> Result<(), ControllerError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let desired = plan
        .cnpg_clusters
        .iter()
        .map(|cluster| cluster.name.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let removed = existing
        .into_iter()
        .filter_map(|name| {
            let name = name.as_ref();
            (!desired.contains(name)).then(|| name.to_string())
        })
        .collect::<std::collections::BTreeSet<_>>();
    if removed.is_empty() {
        Ok(())
    } else {
        Err(ControllerError::InvalidSpec(format!(
            "removing CNPG groups is fail-closed because shard evacuation evidence is required first: {}",
            removed.into_iter().collect::<Vec<_>>().join(",")
        )))
    }
}

async fn cleanup_superseded_bootstrap(
    ctx: &Context,
    namespace: &str,
    cluster: &CitusCluster,
    current_name: Option<&str>,
) -> Result<(), ControllerError> {
    let Some(owner_uid) = cluster.metadata.uid.as_deref() else {
        return Err(ControllerError::InvalidSpec(
            "CitusCluster cleanup requires metadata.uid".to_string(),
        ));
    };
    let selector = bootstrap_resource_selector(&cluster.name_any());
    let prefix = bootstrap_resource_prefix(&cluster.name_any());

    let jobs: Api<Job> = Api::namespaced(ctx.client.clone(), namespace);
    for job in jobs.list(&ListParams::default().labels(&selector)).await? {
        let name = job.name_any();
        if !is_superseded_bootstrap_resource(&name, &prefix, current_name) {
            continue;
        }
        ensure_owned_by(&job.metadata.owner_references, owner_uid, &name)?;
        jobs.delete(&name, &DeleteParams::foreground()).await?;
    }
    let config_maps: Api<ConfigMap> = Api::namespaced(ctx.client.clone(), namespace);
    for config_map in config_maps
        .list(&ListParams::default().labels(&selector))
        .await?
    {
        let name = config_map.name_any();
        if !is_superseded_bootstrap_resource(&name, &prefix, current_name) {
            continue;
        }
        ensure_owned_by(&config_map.metadata.owner_references, owner_uid, &name)?;
        config_maps.delete(&name, &DeleteParams::default()).await?;
    }
    Ok(())
}

/// Stop every older, still-runnable bootstrap generation before creating a
/// replacement. A foreground deletion keeps the new Job from starting until
/// Kubernetes has terminated the old Job's Pods, so an old credential revision
/// cannot overwrite `pg_dist_authinfo` after the replacement has verified it.
async fn quiesce_superseded_bootstrap_jobs(
    ctx: &Context,
    namespace: &str,
    cluster: &CitusCluster,
    desired_name: &str,
) -> Result<bool, ControllerError> {
    let owner_uid = cluster.metadata.uid.as_deref().ok_or_else(|| {
        ControllerError::InvalidSpec("CitusCluster bootstrap requires metadata.uid".to_string())
    })?;
    let selector = bootstrap_resource_selector(&cluster.name_any());
    let prefix = bootstrap_resource_prefix(&cluster.name_any());
    let jobs: Api<Job> = Api::namespaced(ctx.client.clone(), namespace);
    let mut all_quiesced = true;
    for job in jobs.list(&ListParams::default().labels(&selector)).await? {
        let name = job.name_any();
        let state = bootstrap_state(&job);
        if !bootstrap_job_requires_quiescence(&name, &prefix, desired_name, state) {
            continue;
        }
        ensure_owned_by(&job.metadata.owner_references, owner_uid, &name)?;
        all_quiesced = false;
        if job.metadata.deletion_timestamp.is_none() {
            jobs.delete(&name, &DeleteParams::foreground()).await?;
        }
    }
    Ok(all_quiesced)
}

fn is_superseded_bootstrap_resource(name: &str, prefix: &str, current_name: Option<&str>) -> bool {
    name.starts_with(prefix) && Some(name) != current_name
}

fn bootstrap_job_requires_quiescence(
    name: &str,
    prefix: &str,
    desired_name: &str,
    state: BootstrapState,
) -> bool {
    is_superseded_bootstrap_resource(name, prefix, Some(desired_name))
        && state == BootstrapState::Pending
}

fn bootstrap_resource_selector(cluster_name: &str) -> String {
    format!("{MANAGED_BY_LABEL}={MANAGED_BY_VALUE},{CLUSTER_LABEL}={cluster_name}")
}

fn bootstrap_resource_prefix(cluster_name: &str) -> String {
    format!("{cluster_name}-bootstrap-")
}

fn ensure_owned_by(
    owner_references: &Option<Vec<k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference>>,
    owner_uid: &str,
    resource_name: &str,
) -> Result<(), ControllerError> {
    if controller_owned_by(owner_references, owner_uid) {
        Ok(())
    } else {
        Err(ControllerError::InvalidSpec(format!(
            "refusing to manage resource {resource_name} without the expected controller owner UID"
        )))
    }
}

fn controller_owned_by(
    owner_references: &Option<Vec<k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference>>,
    owner_uid: &str,
) -> bool {
    owner_references.as_ref().is_some_and(|owners| {
        owners
            .iter()
            .any(|owner| owner.uid == owner_uid && owner.controller == Some(true))
    })
}

fn post_params() -> PostParams {
    PostParams {
        field_manager: Some(FIELD_MANAGER.to_string()),
        ..PostParams::default()
    }
}

fn add_update_preconditions(
    manifest: &mut Value,
    metadata: &kube::core::ObjectMeta,
    resource_name: &str,
) -> Result<(), ControllerError> {
    let resource_version = metadata.resource_version.as_deref().ok_or_else(|| {
        ControllerError::InvalidSpec(format!(
            "existing resource {resource_name} has no metadata.resourceVersion"
        ))
    })?;
    let uid = metadata.uid.as_deref().ok_or_else(|| {
        ControllerError::InvalidSpec(format!(
            "existing resource {resource_name} has no metadata.uid"
        ))
    })?;
    let object_metadata = manifest
        .get_mut("metadata")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| {
            ControllerError::InvalidSpec("apply manifest metadata must be an object".to_string())
        })?;
    object_metadata.insert("resourceVersion".to_string(), json!(resource_version));
    object_metadata.insert("uid".to_string(), json!(uid));
    Ok(())
}

/// Create without upsert semantics when the name is absent, then use SSA only
/// after verifying the existing object's controller owner UID. The POST path
/// turns a create race into HTTP 409 instead of force-adopting the winner, while
/// resourceVersion/UID preconditions close the delete-and-recreate race on the
/// update path.
async fn create_or_apply_dynamic(
    api: &Api<DynamicObject>,
    name: &str,
    mut manifest: Value,
    owner_uid: &str,
    apply: &PatchParams,
) -> Result<DynamicObject, ControllerError> {
    if let Some(existing) = api.get_opt(name).await? {
        ensure_owned_by(&existing.metadata.owner_references, owner_uid, name)?;
        add_update_preconditions(&mut manifest, &existing.metadata, name)?;
        return api
            .patch(name, apply, &Patch::Apply(&manifest))
            .await
            .map_err(ControllerError::Kube);
    }
    let object = serde_json::from_value(manifest).map_err(|error| {
        ControllerError::InvalidSpec(format!("invalid dynamic manifest for {name}: {error}"))
    })?;
    api.create(&post_params(), &object)
        .await
        .map_err(ControllerError::Kube)
}

async fn create_or_apply_config_map(
    api: &Api<ConfigMap>,
    name: &str,
    mut manifest: Value,
    owner_uid: &str,
) -> Result<ConfigMap, ControllerError> {
    if let Some(existing) = api.get_opt(name).await? {
        ensure_owned_by(&existing.metadata.owner_references, owner_uid, name)?;
        add_update_preconditions(&mut manifest, &existing.metadata, name)?;
        return api
            .patch(
                name,
                &PatchParams::apply(FIELD_MANAGER).force(),
                &Patch::Apply(&manifest),
            )
            .await
            .map_err(ControllerError::Kube);
    }
    let object = serde_json::from_value(manifest).map_err(|error| {
        ControllerError::InvalidSpec(format!("invalid ConfigMap manifest for {name}: {error}"))
    })?;
    api.create(&post_params(), &object)
        .await
        .map_err(ControllerError::Kube)
}

async fn create_or_apply_job(
    api: &Api<Job>,
    name: &str,
    mut manifest: Value,
    owner_uid: &str,
) -> Result<Job, ControllerError> {
    if let Some(existing) = api.get_opt(name).await? {
        ensure_owned_by(&existing.metadata.owner_references, owner_uid, name)?;
        add_update_preconditions(&mut manifest, &existing.metadata, name)?;
        return api
            .patch(
                name,
                &PatchParams::apply(FIELD_MANAGER).force(),
                &Patch::Apply(&manifest),
            )
            .await
            .map_err(ControllerError::Kube);
    }
    let object = serde_json::from_value(manifest).map_err(|error| {
        ControllerError::InvalidSpec(format!("invalid Job manifest for {name}: {error}"))
    })?;
    api.create(&post_params(), &object)
        .await
        .map_err(ControllerError::Kube)
}

async fn reconcile_cleanup(
    cluster: Arc<CitusCluster>,
    ctx: Arc<Context>,
) -> Result<Action, ControllerError> {
    let namespace = cluster.namespace().unwrap_or_else(|| "default".to_string());
    let owner_uid = cluster.metadata.uid.as_deref().ok_or_else(|| {
        ControllerError::InvalidSpec("CitusCluster cleanup requires metadata.uid".to_string())
    })?;
    let selector = bootstrap_resource_selector(&cluster.name_any());
    let prefix = bootstrap_resource_prefix(&cluster.name_any());
    let jobs: Api<Job> = Api::namespaced(ctx.client.clone(), &namespace);
    let mut waiting_for_job_termination = false;
    for job in jobs.list(&ListParams::default().labels(&selector)).await? {
        let name = job.name_any();
        if !name.starts_with(&prefix) || bootstrap_state(&job) != BootstrapState::Pending {
            continue;
        }
        ensure_owned_by(&job.metadata.owner_references, owner_uid, &name)?;
        waiting_for_job_termination = true;
        if job.metadata.deletion_timestamp.is_none() {
            jobs.delete(&name, &DeleteParams::foreground()).await?;
        }
    }
    if waiting_for_job_termination {
        // Returning an error keeps the finalizer installed. The bounded error
        // policy requeues until foreground deletion has removed every writer.
        return Err(ControllerError::Finalizer(
            "waiting for bootstrap Job Pods to terminate before CitusCluster deletion".to_string(),
        ));
    }
    info!(
        cluster = %cluster.name_any(),
        "CitusCluster writers quiesced; child cleanup delegated to owner-reference garbage collection"
    );
    Ok(Action::await_change())
}

fn cnpg_api(client: kube::Client, namespace: &str) -> Api<DynamicObject> {
    let resource = ApiResource::from_gvk_with_plural(
        &GroupVersionKind::gvk("postgresql.cnpg.io", "v1", "Cluster"),
        "clusters",
    );
    Api::namespaced_with(client, namespace, &resource)
}

fn image_catalog_api(client: kube::Client, namespace: &str) -> Api<DynamicObject> {
    let resource = ApiResource::from_gvk_with_plural(
        &GroupVersionKind::gvk("postgresql.cnpg.io", "v1", "ImageCatalog"),
        "imagecatalogs",
    );
    Api::namespaced_with(client, namespace, &resource)
}

async fn validate_referenced_secrets(
    ctx: &Context,
    namespace: &str,
    production: &CitusProductionSpec,
    plan: &CitusClusterReconcilePlan,
    spec_hash: &str,
    generation: Option<i64>,
) -> Result<ValidatedSecretRevision, ControllerError> {
    let secrets: Api<Secret> = Api::namespaced(ctx.client.clone(), namespace);
    let tls = &production.node_tls;
    let ca = get_secret(&secrets, &tls.server_ca_secret).await?;
    require_secret_label(&ca, CNPG_RELOAD_LABEL, "server CA")?;
    require_secret_key(&ca, "ca.crt", "server CA")?;
    let ca_bytes = secret_bytes(&ca, "ca.crt").ok_or_else(|| {
        ControllerError::InvalidSpec(format!(
            "server CA Secret {} must contain non-empty key ca.crt",
            tls.server_ca_secret
        ))
    })?;
    let (_, ca_pem) = parse_x509_pem(ca_bytes).map_err(|error| {
        ControllerError::InvalidSpec(format!(
            "server CA Secret {} ca.crt is not valid PEM: {error}",
            tls.server_ca_secret
        ))
    })?;
    let ca_certificate = ca_pem.parse_x509().map_err(|error| {
        ControllerError::InvalidSpec(format!(
            "server CA Secret {} ca.crt is not a valid X.509 certificate: {error}",
            tls.server_ca_secret
        ))
    })?;
    if !ca_certificate.validity().is_valid() {
        return Err(ControllerError::InvalidSpec(format!(
            "Secret {} ca.crt is not currently valid",
            tls.server_ca_secret
        )));
    }
    if !ca_certificate
        .basic_constraints()
        .map_err(|error| {
            ControllerError::InvalidSpec(format!(
                "Secret {} ca.crt has invalid basic constraints: {error}",
                tls.server_ca_secret
            ))
        })?
        .is_some_and(|constraints| constraints.value.ca)
    {
        return Err(ControllerError::InvalidSpec(format!(
            "Secret {} ca.crt must be a CA certificate",
            tls.server_ca_secret
        )));
    }

    let server = get_secret(&secrets, &tls.server_tls_secret).await?;
    require_secret_label(&server, CNPG_RELOAD_LABEL, "server TLS")?;
    if server.type_.as_deref() != Some("kubernetes.io/tls") {
        return Err(ControllerError::InvalidSpec(format!(
            "Secret {} must have type kubernetes.io/tls",
            tls.server_tls_secret
        )));
    }
    require_secret_key(&server, "tls.crt", "server TLS")?;
    require_secret_key(&server, "tls.key", "server TLS")?;
    let certificate_bytes = secret_bytes(&server, "tls.crt").ok_or_else(|| {
        ControllerError::InvalidSpec(format!(
            "server TLS Secret {} must contain non-empty key tls.crt",
            tls.server_tls_secret
        ))
    })?;
    let (_, certificate_pem) = parse_x509_pem(certificate_bytes).map_err(|error| {
        ControllerError::InvalidSpec(format!(
            "server TLS Secret {} tls.crt is not valid PEM: {error}",
            tls.server_tls_secret
        ))
    })?;
    let certificate = certificate_pem.parse_x509().map_err(|error| {
        ControllerError::InvalidSpec(format!(
            "server TLS Secret {} tls.crt is not a valid X.509 certificate: {error}",
            tls.server_tls_secret
        ))
    })?;
    if !certificate.validity().is_valid() {
        return Err(ControllerError::InvalidSpec(format!(
            "Secret {} tls.crt is not currently valid",
            tls.server_tls_secret
        )));
    }
    certificate
        .verify_signature(Some(ca_certificate.public_key()))
        .map_err(|error| {
            ControllerError::InvalidSpec(format!(
                "Secret {} tls.crt is not signed by the configured server CA: {error}",
                tls.server_tls_secret
            ))
        })?;
    let private_key_bytes = secret_bytes(&server, "tls.key").ok_or_else(|| {
        ControllerError::InvalidSpec(format!(
            "server TLS Secret {} must contain non-empty key tls.key",
            tls.server_tls_secret
        ))
    })?;
    let (_, private_key) = parse_x509_pem(private_key_bytes).map_err(|error| {
        ControllerError::InvalidSpec(format!(
            "Secret {} tls.key is not valid PEM: {error}",
            tls.server_tls_secret
        ))
    })?;
    if !matches!(
        private_key.label.as_str(),
        "PRIVATE KEY" | "RSA PRIVATE KEY" | "EC PRIVATE KEY"
    ) {
        return Err(ControllerError::InvalidSpec(format!(
            "Secret {} tls.key must contain a private key PEM block",
            tls.server_tls_secret
        )));
    }
    let key_der: rustls::pki_types::PrivateKeyDer<'static> = match private_key.label.as_str() {
        "PRIVATE KEY" => rustls::pki_types::PrivatePkcs8KeyDer::from(private_key.contents).into(),
        "RSA PRIVATE KEY" => {
            rustls::pki_types::PrivatePkcs1KeyDer::from(private_key.contents).into()
        }
        "EC PRIVATE KEY" => rustls::pki_types::PrivateSec1KeyDer::from(private_key.contents).into(),
        _ => {
            return Err(ControllerError::InvalidSpec(format!(
                "Secret {} tls.key must contain a private key PEM block",
                tls.server_tls_secret
            )))
        }
    };
    let signing_key =
        rustls::crypto::ring::sign::any_supported_type(&key_der).map_err(|error| {
            ControllerError::InvalidSpec(format!(
                "Secret {} tls.key is not a supported signing key: {error}",
                tls.server_tls_secret
            ))
        })?;
    let public_key = signing_key.public_key().ok_or_else(|| {
        ControllerError::InvalidSpec(format!(
            "Secret {} tls.key does not expose a comparable public key",
            tls.server_tls_secret
        ))
    })?;
    if public_key.as_ref() != certificate.public_key().raw {
        return Err(ControllerError::InvalidSpec(format!(
            "Secret {} tls.key does not match tls.crt",
            tls.server_tls_secret
        )));
    }
    let san = certificate
        .subject_alternative_name()
        .map_err(|error| {
            ControllerError::InvalidSpec(format!(
                "Secret {} tls.crt has an invalid subjectAltName extension: {error}",
                tls.server_tls_secret
            ))
        })?
        .ok_or_else(|| {
            ControllerError::InvalidSpec(format!(
                "Secret {} tls.crt must contain DNS subjectAltName entries",
                tls.server_tls_secret
            ))
        })?;
    let dns_names = san
        .value
        .general_names
        .iter()
        .filter_map(|name| match name {
            GeneralName::DNSName(name) => Some(*name),
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    let mut roots = RootCertStore::empty();
    roots
        .add(CertificateDer::from(ca_pem.contents.clone()))
        .map_err(|error| {
            ControllerError::InvalidSpec(format!(
                "Secret {} ca.crt is not a usable TLS trust anchor: {error}",
                tls.server_ca_secret
            ))
        })?;
    let verifier = WebPkiServerVerifier::builder(Arc::new(roots))
        .build()
        .map_err(|error| {
            ControllerError::InvalidSpec(format!(
                "Secret {} ca.crt cannot construct a TLS server verifier: {error}",
                tls.server_ca_secret
            ))
        })?;
    let certificate_der = CertificateDer::from(certificate_pem.contents.clone());
    let expected_dns_names = plan
        .cnpg_clusters
        .iter()
        .flat_map(|cluster| cluster.server_alt_dns_names.iter())
        .collect::<std::collections::BTreeSet<_>>();
    if expected_dns_names.is_empty() {
        return Err(ControllerError::InvalidSpec(
            "production reconcile plan produced no server DNS identities".to_string(),
        ));
    }
    for expected in expected_dns_names {
        if !dns_names.contains(expected.as_str()) {
            return Err(ControllerError::InvalidSpec(format!(
                "Secret {} tls.crt is missing exact DNS SAN {expected}",
                tls.server_tls_secret
            )));
        }
        let server_name = ServerName::try_from(expected.clone()).map_err(|error| {
            ControllerError::InvalidSpec(format!(
                "derived server name {expected} is invalid for TLS verification: {error}"
            ))
        })?;
        verifier
            .verify_server_cert(&certificate_der, &[], &server_name, &[], UnixTime::now())
            .map_err(|error| {
                ControllerError::InvalidSpec(format!(
                    "Secret {} tls.crt is not a valid server certificate for {expected}: {error}",
                    tls.server_tls_secret
                ))
            })?;
    }

    let superuser = get_secret(&secrets, &tls.superuser_secret).await?;
    if superuser.type_.as_deref() != Some("kubernetes.io/basic-auth") {
        return Err(ControllerError::InvalidSpec(format!(
            "Secret {} must have type kubernetes.io/basic-auth",
            tls.superuser_secret
        )));
    }
    require_secret_key(&superuser, "username", "superuser")?;
    require_secret_key(&superuser, "password", "superuser")?;
    if secret_bytes(&superuser, "username") != Some(b"postgres".as_slice()) {
        return Err(ControllerError::InvalidSpec(format!(
            "Secret {} username must equal postgres",
            tls.superuser_secret
        )));
    }
    let password = secret_bytes(&superuser, "password").ok_or_else(|| {
        ControllerError::InvalidSpec(format!(
            "superuser Secret {} must contain non-empty key password",
            tls.superuser_secret
        ))
    })?;
    if !(16..=256).contains(&password.len())
        || password
            .iter()
            .any(|byte| matches!(byte, b'\0' | b'\n' | b'\r'))
    {
        return Err(ControllerError::InvalidSpec(format!(
            "Secret {} password must be 16 to 256 bytes without NUL or newline characters",
            tls.superuser_secret
        )));
    }
    let referenced_secrets = [&ca, &server, &superuser];
    let secret_revisions = referenced_secrets
        .iter()
        .map(|secret| secret_revision(secret))
        .collect::<Result<Vec<_>, _>>()?;
    let reconcile_hash =
        reconcile_hash_for_secrets(spec_hash, generation, namespace, plan, referenced_secrets)?;
    Ok(ValidatedSecretRevision {
        reconcile_hash,
        server_ca_der: ca_pem.contents,
        server_leaf_sha256: Sha256::digest(&certificate_pem.contents).into(),
        secret_revisions,
    })
}

fn secret_revision(secret: &Secret) -> Result<SecretRevision, ControllerError> {
    let name = secret.metadata.name.clone().ok_or_else(|| {
        ControllerError::InvalidSpec("referenced Secret has no metadata.name".to_string())
    })?;
    let uid = secret.metadata.uid.clone().ok_or_else(|| {
        ControllerError::InvalidSpec(format!("referenced Secret {name} has no metadata.uid"))
    })?;
    let resource_version = secret.metadata.resource_version.clone().ok_or_else(|| {
        ControllerError::InvalidSpec(format!(
            "referenced Secret {name} has no metadata.resourceVersion"
        ))
    })?;
    Ok(SecretRevision {
        name,
        uid,
        resource_version,
    })
}

async fn secret_revisions_are_current(
    ctx: &Context,
    namespace: &str,
    expected: &[SecretRevision],
) -> Result<bool, ControllerError> {
    let secrets: Api<Secret> = Api::namespaced(ctx.client.clone(), namespace);
    for revision in expected {
        let Some(current) = secrets.get_opt(&revision.name).await? else {
            return Ok(false);
        };
        if current.metadata.uid.as_deref() != Some(revision.uid.as_str())
            || current.metadata.resource_version.as_deref()
                != Some(revision.resource_version.as_str())
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn reconcile_hash_for_secrets<'a>(
    spec_hash: &str,
    generation: Option<i64>,
    namespace: &str,
    plan: &CitusClusterReconcilePlan,
    secrets: impl IntoIterator<Item = &'a Secret>,
) -> Result<String, ControllerError> {
    let mut hasher = Sha256::new();
    hash_reconcile_part(&mut hasher, b"spec", spec_hash.as_bytes());
    hash_reconcile_part(
        &mut hasher,
        b"generation",
        generation.unwrap_or_default().to_string().as_bytes(),
    );
    if let Some(catalog) = &plan.image_catalog {
        let encoded = serde_json::to_vec(&catalog.manifest_value()).map_err(|error| {
            ControllerError::InvalidSpec(format!("ImageCatalog hash encoding failed: {error}"))
        })?;
        hash_reconcile_part(&mut hasher, b"image-catalog", &encoded);
    }
    for child in &plan.cnpg_clusters {
        let encoded = serde_json::to_vec(&child.manifest_value()).map_err(|error| {
            ControllerError::InvalidSpec(format!("CNPG Cluster hash encoding failed: {error}"))
        })?;
        hash_reconcile_part(&mut hasher, b"cnpg-cluster", &encoded);
    }
    if let Some(bootstrap) = &plan.bootstrap {
        hash_reconcile_part(
            &mut hasher,
            b"bootstrap-script",
            bootstrap.script().as_bytes(),
        );
        let placeholder_hash = "0".repeat(64);
        let placeholder_name = format!("{}-bootstrap-contract", plan.cluster_name);
        let owner = json!({
            "apiVersion": "citus.ai-blaise.io/v2",
            "kind": "CitusCluster",
            "name": plan.cluster_name,
            "uid": "reconcile-contract",
        });
        let image = plan
            .cnpg_clusters
            .first()
            .map(|cluster| cluster.image.as_str())
            .ok_or_else(|| {
                ControllerError::InvalidSpec(
                    "production reconcile plan must contain a CNPG cluster".to_string(),
                )
            })?;
        let job = bootstrap_job_manifest(
            namespace,
            &owner,
            &plan.cluster_name,
            &placeholder_hash,
            &placeholder_name,
            image,
            &placeholder_name,
            bootstrap,
        );
        let encoded = serde_json::to_vec(&job).map_err(|error| {
            ControllerError::InvalidSpec(format!("bootstrap Job hash encoding failed: {error}"))
        })?;
        hash_reconcile_part(&mut hasher, b"bootstrap-job", &encoded);
    }
    for secret in secrets {
        let revision = secret_revision(secret)?;
        let mut identity = revision.name.into_bytes();
        identity.push(0);
        identity.extend_from_slice(revision.uid.as_bytes());
        identity.push(0);
        identity.extend_from_slice(revision.resource_version.as_bytes());
        hash_reconcile_part(&mut hasher, b"secret", &identity);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn hash_reconcile_part(hasher: &mut Sha256, domain: &[u8], value: &[u8]) {
    hasher.update((domain.len() as u64).to_be_bytes());
    hasher.update(domain);
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
}

fn secret_bytes<'a>(secret: &'a Secret, key: &str) -> Option<&'a [u8]> {
    secret
        .data
        .as_ref()
        .and_then(|data| data.get(key))
        .map(|value| value.0.as_slice())
}

async fn get_secret(secrets: &Api<Secret>, name: &str) -> Result<Secret, ControllerError> {
    secrets.get(name).await.map_err(|error| match &error {
        kube::Error::Api(response) if response.code == 404 => {
            ControllerError::InvalidSpec(format!("referenced Secret {name} does not exist"))
        }
        _ => ControllerError::Kube(error),
    })
}

fn require_secret_key(secret: &Secret, key: &str, purpose: &str) -> Result<(), ControllerError> {
    let present = secret
        .data
        .as_ref()
        .and_then(|data| data.get(key))
        .is_some_and(|value| !value.0.is_empty());
    if present {
        Ok(())
    } else {
        Err(ControllerError::InvalidSpec(format!(
            "{purpose} Secret {} must contain non-empty key {key}",
            secret.name_any()
        )))
    }
}

fn require_secret_label(secret: &Secret, key: &str, purpose: &str) -> Result<(), ControllerError> {
    if secret
        .metadata
        .labels
        .as_ref()
        .is_some_and(|labels| labels.contains_key(key))
    {
        Ok(())
    } else {
        Err(ControllerError::InvalidSpec(format!(
            "{purpose} Secret {} must carry the {key} label for CNPG reload",
            secret.name_any()
        )))
    }
}

fn manifest_with_metadata(
    mut manifest: Value,
    namespace: &str,
    owner: Value,
    cluster_name: &str,
    spec_hash: &str,
    production: &CitusProductionSpec,
) -> Result<Value, ControllerError> {
    let metadata = manifest
        .get_mut("metadata")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| {
            ControllerError::InvalidSpec("manifest metadata must be an object".into())
        })?;
    metadata.insert("namespace".into(), json!(namespace));
    metadata.insert("ownerReferences".into(), json!([owner]));
    metadata.insert("labels".into(), managed_labels(cluster_name, spec_hash));
    metadata.insert(
        "annotations".into(),
        json!({
            "citus.ai-blaise.io/citus-extension-version": production.extension_versions.citus,
            "citus.ai-blaise.io/companion-extension-version": production.extension_versions.companion,
            "citus.ai-blaise.io/node-conninfo": production.node_tls.node_conninfo(),
        }),
    );
    Ok(manifest)
}

fn managed_labels(cluster_name: &str, spec_hash: &str) -> Value {
    json!({
        "app.kubernetes.io/managed-by": MANAGED_BY_VALUE,
        "citus.ai-blaise.io/cluster": cluster_name,
        "citus.ai-blaise.io/spec-hash": &spec_hash[..12],
    })
}

async fn apply_bootstrap_config_map(
    ctx: &Context,
    namespace: &str,
    owner: &Value,
    owner_uid: &str,
    cluster_name: &str,
    spec_hash: &str,
    bootstrap: &CitusBootstrapPlan,
) -> Result<(), ControllerError> {
    let name = format!("{}-{}", bootstrap.config_map_name, &spec_hash[..12]);
    let manifest = json!({
        "apiVersion": "v1",
        "kind": "ConfigMap",
        "metadata": {
            "name": name.clone(),
            "namespace": namespace,
            "ownerReferences": [owner.clone()],
            "labels": managed_labels(cluster_name, spec_hash),
        },
        "immutable": true,
        "data": { "bootstrap.sh": bootstrap.script() },
    });
    let config_maps: Api<ConfigMap> = Api::namespaced(ctx.client.clone(), namespace);
    create_or_apply_config_map(&config_maps, &name, manifest, owner_uid).await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn apply_bootstrap_job(
    ctx: &Context,
    namespace: &str,
    owner: &Value,
    owner_uid: &str,
    cluster_name: &str,
    spec_hash: &str,
    job_name: &str,
    image: &str,
    bootstrap: &CitusBootstrapPlan,
) -> Result<Job, ControllerError> {
    let config_map_name = format!("{}-{}", bootstrap.config_map_name, &spec_hash[..12]);
    let manifest = bootstrap_job_manifest(
        namespace,
        owner,
        cluster_name,
        spec_hash,
        job_name,
        image,
        &config_map_name,
        bootstrap,
    );
    let jobs: Api<Job> = Api::namespaced(ctx.client.clone(), namespace);
    create_or_apply_job(&jobs, job_name, manifest, owner_uid).await
}

#[allow(clippy::too_many_arguments)]
fn bootstrap_job_manifest(
    namespace: &str,
    owner: &Value,
    cluster_name: &str,
    spec_hash: &str,
    job_name: &str,
    image: &str,
    config_map_name: &str,
    bootstrap: &CitusBootstrapPlan,
) -> Value {
    json!({
        "apiVersion": "batch/v1",
        "kind": "Job",
        "metadata": {
            "name": job_name,
            "namespace": namespace,
            "ownerReferences": [owner.clone()],
            "labels": managed_labels(cluster_name, spec_hash),
            "annotations": {
                "citus.ai-blaise.io/citus-extension-version": bootstrap.citus_version,
                "citus.ai-blaise.io/companion-extension-version": bootstrap.companion_version,
            },
        },
        "spec": {
            "backoffLimit": bootstrap.backoff_limit,
            "activeDeadlineSeconds": bootstrap.active_deadline_seconds,
            "template": {
                "metadata": { "labels": managed_labels(cluster_name, spec_hash) },
                "spec": {
                    "restartPolicy": "Never",
                    "terminationGracePeriodSeconds": 30,
                    "automountServiceAccountToken": false,
                    "enableServiceLinks": false,
                    "securityContext": {
                        "runAsUser": bootstrap.postgres_uid,
                        "runAsGroup": bootstrap.postgres_gid,
                        "runAsNonRoot": true,
                        "seccompProfile": { "type": "RuntimeDefault" },
                    },
                    "containers": [{
                        "name": "bootstrap",
                        "image": image,
                        "imagePullPolicy": "IfNotPresent",
                        "command": ["/bin/sh", "/bootstrap/bootstrap.sh"],
                        "env": [
                            { "name": "HOME", "value": "/tmp" },
                            {
                                "name": "DB_USER",
                                "valueFrom": { "secretKeyRef": {
                                    "name": bootstrap.superuser_secret,
                                    "key": "username",
                                }},
                            },
                            {
                                "name": "DB_PASSWORD",
                                "valueFrom": { "secretKeyRef": {
                                    "name": bootstrap.superuser_secret,
                                    "key": "password",
                                }},
                            },
                        ],
                        "securityContext": {
                            "allowPrivilegeEscalation": false,
                            "readOnlyRootFilesystem": true,
                            "capabilities": { "drop": ["ALL"] },
                        },
                        "resources": {
                            "requests": {
                                "cpu": "50m",
                                "memory": "64Mi",
                                "ephemeral-storage": "64Mi",
                            },
                            "limits": {
                                "cpu": "500m",
                                "memory": "256Mi",
                                "ephemeral-storage": "256Mi",
                            },
                        },
                        "volumeMounts": [
                            { "name": "bootstrap", "mountPath": "/bootstrap", "readOnly": true },
                            { "name": "server-ca", "mountPath": "/tls", "readOnly": true },
                            { "name": "tmp", "mountPath": "/tmp" },
                        ],
                    }],
                    "volumes": [
                        { "name": "bootstrap", "configMap": {
                            "name": config_map_name,
                            "defaultMode": 365,
                        }},
                        { "name": "server-ca", "secret": {
                            "secretName": bootstrap.server_ca_secret,
                            "defaultMode": 292,
                            "items": [{ "key": "ca.crt", "path": "ca.crt" }],
                        }},
                        { "name": "tmp", "emptyDir": { "sizeLimit": "256Mi" } },
                    ],
                },
            },
        },
    })
}

fn cnpg_resource_converged(plan: &CnpgClusterPlan, object: &DynamicObject) -> bool {
    // CNPG's current v1 ClusterStatus does not expose a top-level
    // observedGeneration, and its Ready condition currently omits the standard
    // condition field too. Honor either field when a newer CNPG supplies it so
    // stale status cannot pass, while retaining compatibility with the current
    // published contract. The bootstrap script independently gates all writes
    // on live endpoint TLS/password/node_conninfo evidence.
    let current_generation = object
        .metadata
        .generation
        .filter(|generation| *generation > 0);
    let ready_instances = object
        .data
        .pointer("/status/readyInstances")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
    let healthy_phase =
        object.data.pointer("/status/phase").and_then(Value::as_str) == Some(CNPG_HEALTHY_PHASE);
    let desired_image_running =
        object.data.pointer("/status/image").and_then(Value::as_str) == Some(plan.image.as_str());
    let ready_condition = object
        .data
        .pointer("/status/conditions")
        .and_then(Value::as_array)
        .is_some_and(|conditions| {
            conditions.iter().any(|condition| {
                condition.get("type").and_then(Value::as_str) == Some("Ready")
                    && condition.get("status").and_then(Value::as_str) == Some("True")
                    && condition
                        .get("observedGeneration")
                        .and_then(Value::as_i64)
                        .is_none_or(|observed| Some(observed) == current_generation)
            })
        });
    let top_level_generation_current = object
        .data
        .pointer("/status/observedGeneration")
        .and_then(Value::as_i64)
        .is_none_or(|observed| Some(observed) == current_generation);
    let pvc_resize_complete = object
        .data
        .pointer("/status/resizingPVC")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty);

    current_generation.is_some()
        && top_level_generation_current
        && ready_instances == Some(plan.instances)
        && healthy_phase
        && desired_image_running
        && ready_condition
        && pvc_resize_complete
}

/// Return exact live instance targets only when every desired CNPG instance is
/// present, controller-owned by the just-applied Cluster UID, non-terminating,
/// Ready, and has a Pod IP. Foreign and stale-generation Pods never contribute
/// to readiness.
fn cnpg_instance_tls_targets(
    namespace: &str,
    cluster_domain: &str,
    plans: &[CnpgClusterPlan],
    applied_clusters: &[DynamicObject],
    pods: &[Pod],
) -> Result<Option<Vec<CnpgInstanceTlsTarget>>, ControllerError> {
    let applied_by_name = applied_clusters
        .iter()
        .map(|cluster| (cluster.name_any(), cluster))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut targets = Vec::new();
    for plan in plans {
        let applied = applied_by_name.get(&plan.name).ok_or_else(|| {
            ControllerError::InvalidSpec(format!(
                "applied CNPG Cluster inventory omitted {}",
                plan.name
            ))
        })?;
        let cluster_uid = applied.metadata.uid.as_deref().ok_or_else(|| {
            ControllerError::InvalidSpec(format!(
                "applied CNPG Cluster {} has no metadata.uid",
                plan.name
            ))
        })?;
        let mut cluster_pods = pods
            .iter()
            .filter(|pod| {
                pod.metadata.deletion_timestamp.is_none()
                    && pod
                        .metadata
                        .labels
                        .as_ref()
                        .and_then(|labels| labels.get(CNPG_CLUSTER_LABEL))
                        .is_some_and(|name| name == &plan.name)
                    && pod
                        .metadata
                        .labels
                        .as_ref()
                        .and_then(|labels| labels.get(CNPG_POD_ROLE_LABEL))
                        .is_some_and(|role| role == CNPG_INSTANCE_POD_ROLE)
                    && controller_owned_by(&pod.metadata.owner_references, cluster_uid)
            })
            .collect::<Vec<_>>();
        cluster_pods.sort_by_key(|pod| pod.name_any());
        if cluster_pods.len() != plan.instances as usize {
            return Ok(None);
        }
        for pod in cluster_pods {
            let ready = pod
                .status
                .as_ref()
                .and_then(|status| status.conditions.as_ref())
                .is_some_and(|conditions| {
                    conditions
                        .iter()
                        .any(|condition| condition.type_ == "Ready" && condition.status == "True")
                });
            let Some(pod_ip) = pod
                .status
                .as_ref()
                .and_then(|status| status.pod_ip.clone())
                .filter(|ip| !ip.is_empty())
                .and_then(|ip| ip.parse::<IpAddr>().ok())
            else {
                return Ok(None);
            };
            if !ready {
                return Ok(None);
            }
            targets.push(CnpgInstanceTlsTarget {
                cluster_name: plan.name.clone(),
                pod_name: pod.name_any(),
                pod_ip,
                server_name: plan.endpoint(namespace, cluster_domain),
            });
        }
    }
    Ok(Some(targets))
}

/// CNPG v1 does not acknowledge certificate reloads in Cluster status. Probe
/// each Ready instance directly and require both normal CA/SNI validation and
/// an exact leaf fingerprint match with the current server Secret before the
/// bootstrap Job (or Ready status) is admitted.
async fn cnpg_server_leaf_certificates_current(
    ctx: &Context,
    namespace: &str,
    production: &CitusProductionSpec,
    plans: &[CnpgClusterPlan],
    applied_clusters: &[DynamicObject],
    server_ca_der: &[u8],
    expected_leaf_sha256: [u8; 32],
) -> Result<bool, ControllerError> {
    let pod_api: Api<Pod> = Api::namespaced(ctx.client.clone(), namespace);
    let mut instance_pods = Vec::new();
    for plan in plans {
        let page = pod_api
            .list(
                &ListParams::default()
                    .labels(&format!(
                        "{CNPG_CLUSTER_LABEL}={},{CNPG_POD_ROLE_LABEL}={CNPG_INSTANCE_POD_ROLE}",
                        plan.name
                    ))
                    // One extra item is enough to prove a non-exact inventory;
                    // never download an unbounded namespace-wide Pod set.
                    .limit(plan.instances.saturating_add(1)),
            )
            .await?;
        if page.metadata.continue_.is_some() || page.items.len() > plan.instances as usize {
            debug!(
                cluster = %plan.name,
                desired_instances = plan.instances,
                "bounded CNPG instance Pod list exceeded the exact desired inventory"
            );
            return Ok(false);
        }
        instance_pods.extend(page.items);
    }
    let Some(targets) = cnpg_instance_tls_targets(
        namespace,
        &production.cluster_domain,
        plans,
        applied_clusters,
        &instance_pods,
    )?
    else {
        debug!("waiting for the exact controller-owned CNPG instance Pod inventory");
        return Ok(false);
    };

    let mut roots = RootCertStore::empty();
    roots
        .add(CertificateDer::from(server_ca_der.to_vec()))
        .map_err(|error| {
            ControllerError::InvalidSpec(format!(
                "validated server CA could not build live TLS probe trust store: {error}"
            ))
        })?;
    let connector = TlsConnector::from(Arc::new(
        rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth(),
    ));
    let probe_timeout = Duration::from_secs(
        production
            .node_tls
            .connect_timeout_seconds
            .clamp(1, 30)
            .into(),
    );
    let results = futures::stream::iter(targets.into_iter().map(|target| {
        let connector = connector.clone();
        async move {
            let outcome = tokio::time::timeout(
                probe_timeout,
                probe_cnpg_instance_leaf(&connector, &target, expected_leaf_sha256),
            )
            .await;
            (target, outcome)
        }
    }))
    .buffer_unordered(32)
    .collect::<Vec<_>>()
    .await;
    let mut all_current = true;
    for (target, outcome) in results {
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(reason)) => {
                all_current = false;
                debug!(
                    cluster = %target.cluster_name,
                    pod = %target.pod_name,
                    reason,
                    "waiting for CNPG instance to serve the current TLS leaf"
                );
            }
            Err(_) => {
                all_current = false;
                debug!(
                    cluster = %target.cluster_name,
                    pod = %target.pod_name,
                    timeout_seconds = probe_timeout.as_secs(),
                    "timed out waiting for CNPG instance TLS leaf"
                );
            }
        }
    }
    Ok(all_current)
}

async fn probe_cnpg_instance_leaf(
    connector: &TlsConnector,
    target: &CnpgInstanceTlsTarget,
    expected_leaf_sha256: [u8; 32],
) -> Result<(), String> {
    let mut tcp = TcpStream::connect(SocketAddr::new(target.pod_ip, POSTGRES_PORT))
        .await
        .map_err(|error| format!("TCP connect failed: {error}"))?;
    tcp.write_all(&POSTGRES_SSL_REQUEST)
        .await
        .map_err(|error| format!("PostgreSQL SSLRequest write failed: {error}"))?;
    let mut response = [0_u8; 1];
    tcp.read_exact(&mut response)
        .await
        .map_err(|error| format!("PostgreSQL SSLRequest response failed: {error}"))?;
    if response[0] != b'S' {
        return Err("PostgreSQL endpoint refused TLS".to_string());
    }
    let server_name = ServerName::try_from(target.server_name.clone())
        .map_err(|error| format!("invalid TLS server name: {error}"))?;
    let tls = connector
        .connect(server_name, tcp)
        .await
        .map_err(|error| format!("verify-full TLS handshake failed: {error}"))?;
    let peer_leaf = tls
        .get_ref()
        .1
        .peer_certificates()
        .and_then(|chain| chain.first())
        .ok_or_else(|| "TLS peer returned no certificate".to_string())?;
    let actual_leaf_sha256: [u8; 32] = Sha256::digest(peer_leaf.as_ref()).into();
    if actual_leaf_sha256 != expected_leaf_sha256 {
        return Err("TLS peer is still serving a superseded server leaf certificate".to_string());
    }
    Ok(())
}

fn cnpg_resource_status(desired_instances: u32, object: &DynamicObject) -> CnpgResourceStatus {
    let ready_instances = object
        .data
        .pointer("/status/readyInstances")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(0);
    let phase = object
        .data
        .pointer("/status/phase")
        .and_then(Value::as_str)
        .unwrap_or("Pending")
        .to_string();
    CnpgResourceStatus {
        name: object.name_any(),
        desired_instances,
        ready_instances,
        phase,
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum BootstrapState {
    Pending,
    Complete,
    Failed,
}

fn bootstrap_state(job: &Job) -> BootstrapState {
    let conditions = job
        .status
        .as_ref()
        .and_then(|status| status.conditions.as_ref());
    if conditions.is_some_and(|conditions| {
        conditions
            .iter()
            .any(|condition| condition.type_ == "Complete" && condition.status == "True")
    }) {
        BootstrapState::Complete
    } else if conditions.is_some_and(|conditions| {
        conditions
            .iter()
            .any(|condition| condition.type_ == "Failed" && condition.status == "True")
    }) {
        BootstrapState::Failed
    } else {
        BootstrapState::Pending
    }
}

fn invalid_status(
    generation: Option<i64>,
    desired_spec_hash: &str,
    message: String,
    previous: Option<&CitusClusterStatus>,
) -> CitusClusterStatus {
    CitusClusterStatus {
        phase: "Invalid".to_string(),
        observed_generation: generation,
        applied_spec_hash: previous
            .map(|status| status.applied_spec_hash.clone())
            .unwrap_or_default(),
        cnpg_clusters: previous
            .map(|status| status.cnpg_clusters.clone())
            .unwrap_or_default(),
        bootstrap_job: previous.and_then(|status| status.bootstrap_job.clone()),
        expected_extensions: previous.and_then(|status| status.expected_extensions.clone()),
        node_conninfo: previous.and_then(|status| status.node_conninfo.clone()),
        last_error: Some(message.clone()),
        conditions: vec![condition(
            "SpecAccepted",
            "False",
            "ValidationFailed",
            &format!("desired spec {desired_spec_hash} rejected: {message}"),
            generation,
        )],
    }
}

#[allow(clippy::too_many_arguments)]
fn ready_status(
    generation: Option<i64>,
    spec_hash: &str,
    cnpg_clusters: Vec<CnpgResourceStatus>,
    bootstrap_job: Option<String>,
    postgres_ready: bool,
    bootstrap_state: BootstrapState,
    production: &CitusProductionSpec,
) -> CitusClusterStatus {
    let (phase, last_error) = match (postgres_ready, bootstrap_state) {
        (false, _) => ("WaitingForPostgres", None),
        (true, BootstrapState::Pending) => ("VerifyingExtensions", None),
        (true, BootstrapState::Complete) => ("Ready", None),
        (true, BootstrapState::Failed) => (
            "Failed",
            Some("bootstrap Job exhausted its bounded retry budget".to_string()),
        ),
    };
    let extension_status = if bootstrap_state == BootstrapState::Complete {
        "True"
    } else if bootstrap_state == BootstrapState::Failed {
        "False"
    } else {
        "Unknown"
    };
    let extension_reason = match bootstrap_state {
        BootstrapState::Complete => "ExactVersionsVerified",
        BootstrapState::Failed => "BootstrapFailed",
        BootstrapState::Pending => "VerificationPending",
    };
    let ready = postgres_ready && bootstrap_state == BootstrapState::Complete;
    CitusClusterStatus {
        phase: phase.to_string(),
        observed_generation: generation,
        applied_spec_hash: spec_hash.to_string(),
        cnpg_clusters,
        bootstrap_job,
        expected_extensions: Some(ExactExtensionVersionsCr {
            citus: production.extension_versions.citus.clone(),
            companion: production.extension_versions.companion.clone(),
        }),
        node_conninfo: Some(production.node_tls.node_conninfo()),
        last_error,
        conditions: vec![
            condition(
                "SpecAccepted",
                "True",
                "Validated",
                "production apply contract validated",
                generation,
            ),
            condition(
                "ResourcesApplied",
                "True",
                "ServerSideApplied",
                "all desired CNPG resources were server-side applied",
                generation,
            ),
            condition(
                "PostgresReady",
                if postgres_ready { "True" } else { "False" },
                if postgres_ready {
                    "AllInstancesReady"
                } else {
                    "InstancesProgressing"
                },
                "CNPG status and exact owner-UID instance Pod TLS leaf must match the desired revision",
                generation,
            ),
            condition(
                "ExtensionVersionsVerified",
                extension_status,
                extension_reason,
                "bootstrap Job verifies exact Citus and companion versions on every node and database",
                generation,
            ),
            condition(
                "Ready",
                if ready { "True" } else { "False" },
                if ready {
                    "ReconcileComplete"
                } else if bootstrap_state == BootstrapState::Failed {
                    "BootstrapFailed"
                } else {
                    "ReconcileProgressing"
                },
                "Ready requires CNPG readiness, the current TLS leaf on every instance, and successful exact-version/TLS verification",
                generation,
            ),
        ],
    }
}

fn condition(
    condition_type: &str,
    status: &str,
    reason: &str,
    message: &str,
    generation: Option<i64>,
) -> CitusClusterCondition {
    CitusClusterCondition {
        condition_type: condition_type.to_string(),
        status: status.to_string(),
        reason: reason.to_string(),
        message: message.to_string(),
        observed_generation: generation,
        last_transition_time: now_rfc3339(),
    }
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

async fn patch_cluster_status(
    cluster: &CitusCluster,
    ctx: &Context,
    mut status: CitusClusterStatus,
) -> Result<(), ControllerError> {
    if let Some(previous) = &cluster.status {
        for condition in &mut status.conditions {
            if let Some(old) = previous.conditions.iter().find(|old| {
                old.condition_type == condition.condition_type
                    && old.status == condition.status
                    && old.reason == condition.reason
                    && old.message == condition.message
            }) {
                condition.last_transition_time = old.last_transition_time.clone();
            }
        }
        if previous == &status {
            return Ok(());
        }
    }
    let namespace = cluster.namespace().unwrap_or_else(|| "default".to_string());
    let uid = cluster.metadata.uid.as_deref().ok_or_else(|| {
        ControllerError::InvalidSpec("status patch requires metadata.uid".to_string())
    })?;
    let resource_version = cluster
        .metadata
        .resource_version
        .as_deref()
        .ok_or_else(|| {
            ControllerError::InvalidSpec(
                "status patch requires metadata.resourceVersion".to_string(),
            )
        })?;
    let patch = json!({
        "apiVersion": "citus.ai-blaise.io/v2",
        "kind": "CitusCluster",
        "metadata": {
            "name": cluster.name_any(),
            "namespace": namespace,
            "uid": uid,
            "resourceVersion": resource_version,
        },
        "status": status,
    });
    let api: Api<CitusCluster> = Api::namespaced(ctx.client.clone(), &namespace);
    api.patch_status(
        &cluster.name_any(),
        &PatchParams::apply(FIELD_MANAGER).force(),
        &Patch::Apply(&patch),
    )
    .await?;
    Ok(())
}

fn spec_hash(spec: &CitusClusterCrSpec) -> Result<String, ControllerError> {
    let encoded = serde_json::to_vec(spec)
        .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(RECONCILE_CONTRACT_VERSION.as_bytes());
    hasher.update([0]);
    hasher.update(encoded);
    Ok(format!("{:x}", hasher.finalize()))
}

fn error_policy(_cluster: Arc<CitusCluster>, error: &ControllerError, ctx: Arc<Context>) -> Action {
    error!(?error, "CitusCluster controller bounded backoff");
    match error {
        ControllerError::InvalidSpec(_) | ControllerError::Boundary(_) => Action::await_change(),
        ControllerError::Kube(_)
        | ControllerError::Companion(_)
        | ControllerError::Finalizer(_) => Action::requeue(ctx.default_requeue),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn production_cr() -> CitusClusterCrSpec {
        CitusClusterCrSpec {
            image: format!("ghcr.io/ai-blaise/citus@sha256:{}", "a".repeat(64)),
            workers: 2,
            coordinators: 3,
            coordinator_less: false,
            timescale_enabled: false,
            extensions: vec!["citus".to_string(), "ai_blaise_citus".to_string()],
            storage_class: Some("fast".to_string()),
            pool_replicas: None,
            sidecars: Vec::new(),
            production: Some(CitusProductionSpecCr {
                postgres_major: 17,
                postgres_uid: 999,
                postgres_gid: 999,
                worker_replicas: 2,
                storage_size: "10Gi".to_string(),
                cluster_domain: "cluster.local".to_string(),
                databases: vec!["app".to_string()],
                extension_versions: ExactExtensionVersionsCr {
                    citus: "13.2-1".to_string(),
                    companion: "1.0".to_string(),
                },
                node_tls: NodeTlsSpecCr {
                    server_ca_secret: "citus-ca".to_string(),
                    server_tls_secret: "citus-tls".to_string(),
                    superuser_secret: "citus-superuser".to_string(),
                    ssl_mode: "verify-full".to_string(),
                    ssl_root_cert: CNPG_SERVER_CA_PATH.to_string(),
                    connect_timeout_seconds: 5,
                },
                bootstrap: BootstrapJobSpecCr::default(),
            }),
        }
    }

    #[test]
    fn cr_spec_round_trips_into_apply_ready_authoritative_spec() {
        let authoritative = production_cr().to_authoritative();
        authoritative.validate_apply_ready().expect("apply ready");
        assert_eq!(authoritative.workers, 2);
        assert_eq!(
            authoritative
                .production
                .as_ref()
                .expect("production")
                .node_tls
                .node_conninfo(),
            "sslmode=verify-full sslrootcert=/controller/certificates/server-ca.crt connect_timeout=5"
        );
    }

    #[test]
    fn mutable_image_and_weaker_tls_fail_closed() {
        let mut cr = production_cr();
        cr.image = "ghcr.io/ai-blaise/citus:latest".to_string();
        assert!(matches!(
            cr.to_authoritative().validate_apply_ready(),
            Err(crate::crds::citus_cluster::CitusClusterSpecError::MutableImage)
        ));
        let mut cr = production_cr();
        cr.production
            .as_mut()
            .expect("production")
            .node_tls
            .ssl_mode = "require".to_string();
        assert!(matches!(
            cr.to_authoritative().validate_apply_ready(),
            Err(crate::crds::citus_cluster::CitusClusterSpecError::InvalidNodeTlsMode)
        ));
    }

    #[test]
    fn bootstrap_job_is_digest_pinned_non_root_and_bounded() {
        let cr = production_cr();
        let authoritative = cr.to_authoritative();
        let plan = CitusClusterReconcilePlan::from_spec_in_namespace(
            "primary",
            "database",
            &authoritative,
        )
        .expect("plan");
        let bootstrap = plan.bootstrap.as_ref().expect("bootstrap");
        let owner = json!({"apiVersion":"v2","kind":"CitusCluster","name":"primary","uid":"1"});
        let manifest = bootstrap_job_manifest(
            "database",
            &owner,
            "primary",
            &"b".repeat(64),
            "primary-bootstrap-bbbbbbbbbbbb",
            &authoritative.image,
            "primary-bootstrap-bbbbbbbbbbbb",
            bootstrap,
        );
        assert_eq!(manifest["spec"]["backoffLimit"], 3);
        assert_eq!(manifest["spec"]["activeDeadlineSeconds"], 600);
        assert_eq!(
            manifest["spec"]["template"]["spec"]["automountServiceAccountToken"],
            false
        );
        assert_eq!(
            manifest["spec"]["template"]["spec"]["containers"][0]["securityContext"]
                ["readOnlyRootFilesystem"],
            true
        );
    }

    #[test]
    fn status_requires_both_cnpg_and_exact_version_evidence() {
        let production = production_cr()
            .to_authoritative()
            .production
            .expect("production");
        let pending = ready_status(
            Some(7),
            &"c".repeat(64),
            vec![CnpgResourceStatus {
                name: "primary-coordinator".to_string(),
                desired_instances: 3,
                ready_instances: 3,
                phase: "Cluster in healthy state".to_string(),
            }],
            Some("bootstrap".to_string()),
            true,
            BootstrapState::Pending,
            &production,
        );
        assert_eq!(pending.phase, "VerifyingExtensions");
        let ready = ready_status(
            Some(7),
            &"c".repeat(64),
            pending.cnpg_clusters,
            Some("bootstrap".to_string()),
            true,
            BootstrapState::Complete,
            &production,
        );
        assert_eq!(ready.phase, "Ready");
        assert_eq!(ready.observed_generation, Some(7));
        assert!(ready.conditions.iter().any(|condition| {
            condition.condition_type == "ExtensionVersionsVerified" && condition.status == "True"
        }));
    }

    #[test]
    fn cnpg_convergence_rejects_stale_image_phase_condition_and_pvc_resize() {
        let authoritative = production_cr().to_authoritative();
        let plan = CitusClusterReconcilePlan::from_spec_in_namespace(
            "primary",
            "database",
            &authoritative,
        )
        .expect("plan");
        let child = &plan.cnpg_clusters[0];
        let object = |image: &str,
                      phase: &str,
                      ready_status: &str,
                      observed_generation: Option<i64>,
                      resizing: Value| {
            serde_json::from_value::<DynamicObject>(json!({
                "apiVersion": "postgresql.cnpg.io/v1",
                "kind": "Cluster",
                "metadata": { "name": child.name, "generation": 7 },
                "status": {
                    "readyInstances": child.instances,
                    "phase": phase,
                    "image": image,
                    "resizingPVC": resizing,
                    "conditions": [{
                        "type": "Ready",
                        "status": ready_status,
                        "observedGeneration": observed_generation,
                    }],
                },
            }))
            .expect("dynamic CNPG object")
        };

        assert!(!cnpg_resource_converged(
            child,
            &object(
                "ghcr.io/ai-blaise/citus@sha256:old",
                CNPG_HEALTHY_PHASE,
                "True",
                None,
                json!([]),
            ),
        ));
        assert!(!cnpg_resource_converged(
            child,
            &object(&child.image, "Upgrading cluster", "False", None, json!([])),
        ));
        assert!(!cnpg_resource_converged(
            child,
            &object(
                &child.image,
                CNPG_HEALTHY_PHASE,
                "True",
                None,
                json!(["pvc-1"]),
            ),
        ));
        assert!(cnpg_resource_converged(
            child,
            &object(&child.image, CNPG_HEALTHY_PHASE, "True", None, json!([]),),
        ));
        assert!(!cnpg_resource_converged(
            child,
            &object(&child.image, CNPG_HEALTHY_PHASE, "True", Some(6), json!([]),),
        ));
        assert!(cnpg_resource_converged(
            child,
            &object(&child.image, CNPG_HEALTHY_PHASE, "True", Some(7), json!([]),),
        ));
    }

    #[test]
    fn update_preconditions_and_owner_checks_prevent_force_adoption() {
        let owner = k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference {
            api_version: "citus.ai-blaise.io/v2".to_string(),
            kind: "CitusCluster".to_string(),
            name: "primary".to_string(),
            uid: "expected-uid".to_string(),
            block_owner_deletion: Some(true),
            controller: Some(true),
        };
        assert!(ensure_owned_by(&Some(vec![owner.clone()]), "expected-uid", "child").is_ok());
        assert!(ensure_owned_by(&Some(vec![owner]), "foreign-uid", "child").is_err());
        assert!(ensure_owned_by(&None, "expected-uid", "child").is_err());

        let metadata = kube::core::ObjectMeta {
            uid: Some("child-uid".to_string()),
            resource_version: Some("42".to_string()),
            ..kube::core::ObjectMeta::default()
        };
        let mut manifest = json!({ "metadata": { "name": "child" } });
        add_update_preconditions(&mut manifest, &metadata, "child").expect("preconditions");
        assert_eq!(manifest["metadata"]["uid"], "child-uid");
        assert_eq!(manifest["metadata"]["resourceVersion"], "42");

        let candidate = kube::core::ObjectMeta {
            uid: Some("cluster-uid".to_string()),
            resource_version: Some("7".to_string()),
            generation: Some(3),
            ..kube::core::ObjectMeta::default()
        };
        assert!(same_object_revision(&candidate, &candidate));
        let mut newer = candidate.clone();
        newer.resource_version = Some("8".to_string());
        assert!(!same_object_revision(&candidate, &newer));
        newer = candidate.clone();
        newer.generation = Some(4);
        assert!(!same_object_revision(&candidate, &newer));
    }

    #[test]
    fn bootstrap_revision_inventory_is_label_scoped_and_hash_named() {
        assert_eq!(
            bootstrap_resource_selector("primary"),
            "app.kubernetes.io/managed-by=ai-blaise-citus-operator,citus.ai-blaise.io/cluster=primary"
        );
        assert_eq!(bootstrap_resource_prefix("primary"), "primary-bootstrap-");
        let prefix = bootstrap_resource_prefix("primary");
        let desired = "primary-bootstrap-new";
        assert!(is_superseded_bootstrap_resource(
            "primary-bootstrap-old",
            &prefix,
            Some(desired),
        ));
        assert!(!is_superseded_bootstrap_resource(
            desired,
            &prefix,
            Some(desired),
        ));
        assert!(!is_superseded_bootstrap_resource(
            "another-bootstrap-old",
            &prefix,
            Some(desired),
        ));
        assert!(bootstrap_job_requires_quiescence(
            "primary-bootstrap-old",
            &prefix,
            desired,
            BootstrapState::Pending,
        ));
        assert!(!bootstrap_job_requires_quiescence(
            "primary-bootstrap-old",
            &prefix,
            desired,
            BootstrapState::Complete,
        ));
        assert!(!bootstrap_job_requires_quiescence(
            desired,
            &prefix,
            desired,
            BootstrapState::Pending,
        ));
        let mut pending = Job::default();
        assert_eq!(bootstrap_state(&pending), BootstrapState::Pending);
        pending.status = Some(k8s_openapi::api::batch::v1::JobStatus {
            conditions: Some(vec![k8s_openapi::api::batch::v1::JobCondition {
                status: "True".to_string(),
                type_: "Complete".to_string(),
                ..k8s_openapi::api::batch::v1::JobCondition::default()
            }]),
            ..k8s_openapi::api::batch::v1::JobStatus::default()
        });
        assert_eq!(bootstrap_state(&pending), BootstrapState::Complete);
    }

    #[test]
    fn cnpg_group_removal_fails_closed() {
        let authoritative = production_cr().to_authoritative();
        let plan = CitusClusterReconcilePlan::from_spec_in_namespace(
            "primary",
            "database",
            &authoritative,
        )
        .expect("plan");
        let previous = CitusClusterStatus {
            cnpg_clusters: vec![CnpgResourceStatus {
                name: "primary-worker-9".to_string(),
                desired_instances: 1,
                ready_instances: 1,
                phase: "Cluster in healthy state".to_string(),
            }],
            ..CitusClusterStatus::default()
        };

        let error = validate_no_cnpg_group_removal(Some(&previous), &plan)
            .expect_err("unsafe group removal must be rejected");
        assert!(error.to_string().contains("shard evacuation evidence"));

        let invalid = invalid_status(Some(8), &"d".repeat(64), error.to_string(), Some(&previous));
        assert_eq!(invalid.cnpg_clusters, previous.cnpg_clusters);
        assert_eq!(invalid.applied_spec_hash, previous.applied_spec_hash);

        let live_inventory_error = validate_no_removed_cnpg_groups(
            [
                "primary-coordinator",
                "primary-worker-0",
                "primary-worker-9",
            ],
            &plan,
        )
        .expect_err("live owned inventory must close the pre-status crash window");
        assert!(live_inventory_error
            .to_string()
            .contains("primary-worker-9"));
        validate_no_removed_cnpg_groups(
            plan.cnpg_clusters
                .iter()
                .map(|cluster| cluster.name.as_str()),
            &plan,
        )
        .expect("the exact live inventory is accepted");
    }

    #[test]
    fn cnpg_fields_without_live_acknowledgements_are_immutable_after_create() {
        let authoritative = production_cr().to_authoritative();
        let production = authoritative.production.as_ref().expect("production");
        let plan = CitusClusterReconcilePlan::from_spec_in_namespace(
            "primary",
            "database",
            &authoritative,
        )
        .expect("plan");
        let child = &plan.cnpg_clusters[0];
        let mut existing = serde_json::from_value::<DynamicObject>(child.manifest_value())
            .expect("dynamic CNPG object");
        existing.metadata.annotations = Some(std::collections::BTreeMap::from([
            (
                "citus.ai-blaise.io/citus-extension-version".to_string(),
                production.extension_versions.citus.clone(),
            ),
            (
                "citus.ai-blaise.io/companion-extension-version".to_string(),
                production.extension_versions.companion.clone(),
            ),
        ]));
        validate_cnpg_mutation_is_observable(child, &existing, production)
            .expect("an unchanged child remains reconcilable");

        for (pointer, replacement, field) in [
            ("/spec/storage/size", json!("200Gi"), "storageSize"),
            (
                "/spec/storage/storageClass",
                json!("different-class"),
                "storageClass",
            ),
            ("/spec/imageCatalogRef/major", json!(18), "postgresMajor"),
            ("/spec/postgresUID", json!(1234), "postgresUid"),
            ("/spec/postgresGID", json!(1234), "postgresGid"),
            (
                "/spec/bootstrap/initdb/database",
                json!("replacement"),
                "databases[0]",
            ),
        ] {
            let mut changed = existing.clone();
            *changed
                .data
                .pointer_mut(pointer)
                .expect("field exists in production child") = replacement;
            let error = validate_cnpg_mutation_is_observable(child, &changed, production)
                .expect_err("unobservable mutation must fail closed");
            assert!(error.to_string().contains(field));
            assert!(error.to_string().contains("data-migration path"));
        }

        let mut no_storage_class_cr = production_cr();
        no_storage_class_cr.storage_class = None;
        let no_storage_class = no_storage_class_cr.to_authoritative();
        let no_storage_production = no_storage_class
            .production
            .as_ref()
            .expect("production without storage class");
        let no_storage_class_plan = CitusClusterReconcilePlan::from_spec_in_namespace(
            "primary",
            "database",
            &no_storage_class,
        )
        .expect("plan without storage class");
        let child = &no_storage_class_plan.cnpg_clusters[0];
        let mut explicit_null = serde_json::from_value::<DynamicObject>(child.manifest_value())
            .expect("dynamic CNPG object");
        explicit_null.metadata.annotations = Some(std::collections::BTreeMap::from([
            (
                "citus.ai-blaise.io/citus-extension-version".to_string(),
                no_storage_production.extension_versions.citus.clone(),
            ),
            (
                "citus.ai-blaise.io/companion-extension-version".to_string(),
                no_storage_production.extension_versions.companion.clone(),
            ),
        ]));
        explicit_null.data["spec"]["storage"]["storageClass"] = Value::Null;
        validate_cnpg_mutation_is_observable(child, &explicit_null, no_storage_production)
            .expect("explicit null and omitted optional field are equivalent");

        let mut upgraded = production.clone();
        upgraded.extension_versions.citus = "14.0-1".to_string();
        let error =
            validate_cnpg_mutation_is_observable(&plan.cnpg_clusters[0], &existing, &upgraded)
                .expect_err("implicit extension upgrade must fail before child mutation");
        assert!(error
            .to_string()
            .contains("reviewed extension-upgrade path"));
    }

    #[test]
    fn live_leaf_targets_require_exact_owned_ready_instance_inventory() {
        fn instance_pod(cluster: &str, uid: &str, ordinal: usize) -> Pod {
            Pod {
                metadata: kube::core::ObjectMeta {
                    name: Some(format!("{cluster}-{ordinal}")),
                    labels: Some(std::collections::BTreeMap::from([
                        (CNPG_CLUSTER_LABEL.to_string(), cluster.to_string()),
                        (
                            CNPG_POD_ROLE_LABEL.to_string(),
                            CNPG_INSTANCE_POD_ROLE.to_string(),
                        ),
                    ])),
                    owner_references: Some(vec![
                        k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference {
                            api_version: "postgresql.cnpg.io/v1".to_string(),
                            block_owner_deletion: Some(true),
                            controller: Some(true),
                            kind: "Cluster".to_string(),
                            name: cluster.to_string(),
                            uid: uid.to_string(),
                        },
                    ]),
                    ..kube::core::ObjectMeta::default()
                },
                status: Some(k8s_openapi::api::core::v1::PodStatus {
                    conditions: Some(vec![k8s_openapi::api::core::v1::PodCondition {
                        last_probe_time: None,
                        last_transition_time: None,
                        message: None,
                        observed_generation: None,
                        reason: None,
                        status: "True".to_string(),
                        type_: "Ready".to_string(),
                    }]),
                    pod_ip: Some(format!("10.0.0.{}", ordinal + 1)),
                    ..k8s_openapi::api::core::v1::PodStatus::default()
                }),
                ..Pod::default()
            }
        }

        let authoritative = production_cr().to_authoritative();
        let plan = CitusClusterReconcilePlan::from_spec_in_namespace(
            "primary",
            "database",
            &authoritative,
        )
        .expect("plan");
        let mut applied = Vec::new();
        let mut pods = Vec::new();
        let mut ordinal = 0;
        for child in &plan.cnpg_clusters {
            let uid = format!("{}-uid", child.name);
            let mut object = serde_json::from_value::<DynamicObject>(child.manifest_value())
                .expect("dynamic CNPG object");
            object.metadata.uid = Some(uid.clone());
            applied.push(object);
            for _ in 0..child.instances {
                pods.push(instance_pod(&child.name, &uid, ordinal));
                ordinal += 1;
            }
        }
        let targets = cnpg_instance_tls_targets(
            "database",
            "cluster.local",
            &plan.cnpg_clusters,
            &applied,
            &pods,
        )
        .expect("target selection")
        .expect("complete target inventory");
        assert_eq!(targets.len(), plan.total_postgres_instances() as usize);
        assert!(targets
            .iter()
            .all(|target| target.server_name.ends_with(".database.svc.cluster.local")));

        pods[0]
            .metadata
            .owner_references
            .as_mut()
            .expect("owner reference")[0]
            .uid = "stale-child-uid".to_string();
        assert!(cnpg_instance_tls_targets(
            "database",
            "cluster.local",
            &plan.cnpg_clusters,
            &applied,
            &pods,
        )
        .expect("target selection")
        .is_none());

        pods[0]
            .metadata
            .owner_references
            .as_mut()
            .expect("owner reference")[0]
            .uid = format!("{}-uid", plan.cnpg_clusters[0].name);
        pods[0]
            .status
            .as_mut()
            .expect("status")
            .conditions
            .as_mut()
            .expect("conditions")[0]
            .status = "False".to_string();
        assert!(cnpg_instance_tls_targets(
            "database",
            "cluster.local",
            &plan.cnpg_clusters,
            &applied,
            &pods,
        )
        .expect("target selection")
        .is_none());

        pods[0]
            .status
            .as_mut()
            .expect("status")
            .conditions
            .as_mut()
            .expect("conditions")[0]
            .status = "True".to_string();
        pods[0].status.as_mut().expect("status").pod_ip =
            Some("attacker-controlled.example".to_string());
        assert!(cnpg_instance_tls_targets(
            "database",
            "cluster.local",
            &plan.cnpg_clusters,
            &applied,
            &pods,
        )
        .expect("target selection")
        .is_none());
    }

    #[test]
    fn secret_resource_version_rotates_reconcile_hash_without_hashing_secret_data() {
        fn secret(name: &str, resource_version: &str) -> Secret {
            Secret {
                metadata: kube::core::ObjectMeta {
                    name: Some(name.to_string()),
                    uid: Some(format!("{name}-uid")),
                    resource_version: Some(resource_version.to_string()),
                    ..kube::core::ObjectMeta::default()
                },
                ..Secret::default()
            }
        }
        let first = secret("ca", "10");
        let rotated = secret("ca", "11");
        let authoritative = production_cr().to_authoritative();
        let plan = CitusClusterReconcilePlan::from_spec_in_namespace(
            "primary",
            "database",
            &authoritative,
        )
        .expect("plan");

        assert_ne!(
            reconcile_hash_for_secrets(&"a".repeat(64), Some(1), "database", &plan, [&first])
                .expect("first hash"),
            reconcile_hash_for_secrets(&"a".repeat(64), Some(1), "database", &plan, [&rotated])
                .expect("rotated hash")
        );

        let mut changed_contract = plan.clone();
        changed_contract
            .bootstrap
            .as_mut()
            .expect("bootstrap")
            .active_deadline_seconds += 1;
        assert_ne!(
            reconcile_hash_for_secrets(&"a".repeat(64), Some(1), "database", &plan, [&first])
                .expect("first contract hash"),
            reconcile_hash_for_secrets(
                &"a".repeat(64),
                Some(1),
                "database",
                &changed_contract,
                [&first],
            )
            .expect("changed contract hash")
        );
        assert_ne!(
            reconcile_hash_for_secrets(&"a".repeat(64), Some(1), "database", &plan, [&first])
                .expect("generation one hash"),
            reconcile_hash_for_secrets(&"a".repeat(64), Some(2), "database", &plan, [&first])
                .expect("generation two hash")
        );

        let missing_resource_version = Secret {
            metadata: kube::core::ObjectMeta {
                name: Some("missing-version".to_string()),
                uid: Some("missing-version-uid".to_string()),
                ..kube::core::ObjectMeta::default()
            },
            ..Secret::default()
        };
        let error = reconcile_hash_for_secrets(
            &"a".repeat(64),
            Some(1),
            "database",
            &plan,
            [&missing_resource_version],
        )
        .expect_err("API Secrets without resourceVersion must fail closed");
        assert!(error
            .to_string()
            .contains("has no metadata.resourceVersion"));
    }
}
