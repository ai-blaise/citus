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
    info!("operator serving CitusCluster, Migration, Tenant, Hypertable controllers");

    let cluster = tokio::spawn(citus_cluster::run(ctx.clone()));
    let migration = tokio::spawn(migration::run(ctx.clone()));
    let tenant = tokio::spawn(tenant::run(ctx.clone()));
    let hypertable = tokio::spawn(hypertable::run(ctx.clone()));

    tokio::select! {
        result = cluster => log_exit("citus_cluster", result),
        result = migration => log_exit("migration", result),
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
