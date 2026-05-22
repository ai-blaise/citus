//! `CitusCluster` controller.

use super::{Context, ControllerError};
use crate::crds::citus_cluster::{
    CitusClusterSpec, CitusTopology, PoolSpec, SidecarSpec, SidecarType,
};
use futures::StreamExt;
use kube::{
    api::Api,
    runtime::{controller::Action, watcher, Controller},
    CustomResource,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};

/// Kube-rs typed resource for the CitusCluster CRD.
///
/// The kube derive macro requires a serde-friendly spec; this type mirrors the
/// declarative fields the controller acts on. Values are validated against the
/// authoritative [`CitusClusterSpec`] in `reconcile`.
#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "citus.ai-blaise.io",
    version = "v2",
    kind = "CitusCluster",
    namespaced
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
}

fn default_coordinators() -> u32 {
    1
}

fn default_extensions() -> Vec<String> {
    vec!["citus".to_string()]
}

/// Sidecar entry mirrored from the spec model.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SidecarSpecCr {
    pub kind: String,
    #[serde(default = "default_one")]
    pub replicas: u32,
}

fn default_one() -> u32 {
    1
}

impl CitusClusterCrSpec {
    /// Translate the CR view into the validated authoritative spec.
    pub fn to_authoritative(&self) -> CitusClusterSpec {
        let topology = if self.coordinator_less {
            CitusTopology::CoordinatorLess
        } else {
            CitusTopology::CoordinatorWorker
        };
        let coordinators = if self.coordinator_less {
            0
        } else {
            self.coordinators
        };
        CitusClusterSpec {
            topology,
            image: self.image.clone(),
            workers: self.workers,
            coordinators,
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
        .for_each(|res| async move {
            match res {
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
    let authoritative = cluster.spec.to_authoritative();
    authoritative
        .validate()
        .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
    info!(
        cluster = ?cluster.metadata.name,
        workers = authoritative.workers,
        sidecars = authoritative.sidecars.len(),
        "CitusCluster reconciled"
    );
    Ok(Action::requeue(ctx.default_requeue))
}

fn error_policy(_cluster: Arc<CitusCluster>, error: &ControllerError, ctx: Arc<Context>) -> Action {
    error!(?error, "CitusCluster controller backoff");
    Action::requeue(ctx.default_requeue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cr_spec_round_trips_into_authoritative_spec() {
        let cr = CitusClusterCrSpec {
            image: "ghcr.io/ai-blaise/citus:pg18-v2".to_string(),
            workers: 3,
            coordinators: 1,
            coordinator_less: false,
            timescale_enabled: true,
            extensions: vec!["citus".to_string(), "timescaledb".to_string()],
            storage_class: Some("fast-ssd".to_string()),
            pool_replicas: Some(2),
            sidecars: vec![SidecarSpecCr {
                kind: "Cdc".to_string(),
                replicas: 1,
            }],
        };
        let authoritative = cr.to_authoritative();
        authoritative.validate().expect("spec valid");
        assert_eq!(authoritative.workers, 3);
        assert!(matches!(
            authoritative.topology,
            CitusTopology::CoordinatorWorker
        ));
        assert_eq!(authoritative.sidecars.len(), 1);
    }

    #[test]
    fn cr_spec_coordinator_less_clears_coordinators() {
        let cr = CitusClusterCrSpec {
            image: "ghcr.io/ai-blaise/citus:pg18-v2".to_string(),
            workers: 2,
            coordinators: 1,
            coordinator_less: true,
            timescale_enabled: false,
            extensions: vec!["citus".to_string()],
            storage_class: None,
            pool_replicas: None,
            sidecars: Vec::new(),
        };
        let authoritative = cr.to_authoritative();
        authoritative.validate().expect("spec valid");
        assert_eq!(authoritative.coordinators, 0);
        assert!(matches!(
            authoritative.topology,
            CitusTopology::CoordinatorLess
        ));
    }
}
