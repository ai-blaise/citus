//! kube-rs controllers for the V2 operator catalog.
//!
//! Each CRD owns a sub-module that registers a typed kube `CustomResource`
//! wrapping the corresponding `*Spec` type defined in `crate::crds`, plus a
//! `reconcile`/`error_policy` pair driven by `kube::runtime::Controller`.
//!
//! `controllers::serve_all(client)` spawns every controller on the supplied
//! tokio runtime and returns once any controller exits.

pub mod citus_cluster;
pub mod hypertable;
pub mod migration;
pub mod tenant;

pub mod conflict_policy;
pub mod scheduled_repack;
pub mod sidecar;
use kube::Client;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tracing::{info, warn};

/// Shared context handed to every reconciler.
#[derive(Clone)]
pub struct Context {
    pub client: Client,
    pub default_requeue: Duration,
}

impl Context {
    pub fn new(client: Client) -> Arc<Self> {
        Arc::new(Self {
            client,
            default_requeue: Duration::from_secs(30),
        })
    }
}

/// Errors common to every reconciler.
#[derive(Debug, Error)]
pub enum ControllerError {
    #[error("kube-rs client error: {0}")]
    Kube(#[from] kube::Error),
    #[error("companion error: {0}")]
    Companion(String),
    #[error("invalid spec: {0}")]
    InvalidSpec(String),
}

/// Spawn each catalog controller concurrently. Returns when any controller's
/// future exits (kube `Controller::run` is itself infinite under normal
/// conditions).
pub async fn serve_all(client: Client) -> Result<(), ControllerError> {
    let ctx = Context::new(client);
    info!("operator serving CitusCluster, Migration, Tenant, Hypertable, ScheduledRepack, ConflictPolicy, Sidecar controllers");

    let cluster = tokio::spawn(citus_cluster::run(ctx.clone()));
    let conflict_policy = tokio::spawn(conflict_policy::run(ctx.clone()));
    let migration = tokio::spawn(migration::run(ctx.clone()));
    let scheduled_repack = tokio::spawn(scheduled_repack::run(ctx.clone()));
    let sidecar = tokio::spawn(sidecar::run(ctx.clone()));
    let tenant = tokio::spawn(tenant::run(ctx.clone()));
    let hypertable = tokio::spawn(hypertable::run(ctx.clone()));

    tokio::select! {
        result = cluster => log_exit("citus_cluster", result),
        result = conflict_policy => log_exit("conflict_policy", result),
        result = migration => log_exit("migration", result),
        result = scheduled_repack => log_exit("scheduled_repack", result),
        result = sidecar => log_exit("sidecar", result),
        result = tenant => log_exit("tenant", result),
        result = hypertable => log_exit("hypertable", result),
    }
    Ok(())
}

fn log_exit(name: &str, result: Result<Result<(), ControllerError>, tokio::task::JoinError>) {
    match result {
        Ok(Ok(())) => info!(controller = name, "controller exited cleanly"),
        Ok(Err(error)) => warn!(controller = name, ?error, "controller errored"),
        Err(error) => warn!(controller = name, ?error, "controller panicked"),
    }
}
