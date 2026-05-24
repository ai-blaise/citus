//! kube-rs controllers for the V2 operator catalog.
//!
//! Each CRD owns a sub-module that registers a typed kube `CustomResource`
//! wrapping the corresponding `*Spec` type defined in `crate::crds`, plus a
//! `reconcile`/`error_policy` pair driven by `kube::runtime::Controller`.
//!
//! `controllers::serve_all(client)` spawns every controller on the supplied
//! tokio runtime and returns once any controller exits.

pub mod backup;
pub mod boundary;
pub mod citus_cluster;
pub mod conflict_policy;
pub mod federation;
pub mod function;
pub mod hypertable;
pub mod migration;
pub mod region;
pub mod scheduled_repack;
pub mod search_index;
pub mod sidecar;
pub mod survival_goal;
pub mod tenant;
pub mod webhook;

use boundary::{execution_mode_from_env, BoundaryError, ExecutionMode};
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
    pub execution_mode: ExecutionMode,
}

impl Context {
    pub fn new(client: Client) -> Result<Arc<Self>, ControllerError> {
        Ok(Arc::new(Self {
            client,
            default_requeue: Duration::from_secs(30),
            execution_mode: execution_mode_from_env()?,
        }))
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
    #[error("controller boundary error: {0}")]
    Boundary(#[from] BoundaryError),
}

/// Spawn each catalog controller concurrently. Returns when any controller's
/// future exits (kube `Controller::run` is itself infinite under normal
/// conditions).
pub async fn serve_all(client: Client) -> Result<(), ControllerError> {
    let ctx = Context::new(client)?;
    info!(
        mode = ctx.execution_mode.as_str(),
        "operator serving CitusCluster, Migration, Tenant, Region, SurvivalGoal, Backup, Hypertable, Federation, SearchIndex, Webhook, Function, ScheduledRepack, ConflictPolicy, Sidecar controllers"
    );

    let backup = tokio::spawn(backup::run(ctx.clone()));
    let cluster = tokio::spawn(citus_cluster::run(ctx.clone()));
    let conflict_policy = tokio::spawn(conflict_policy::run(ctx.clone()));
    let migration = tokio::spawn(migration::run(ctx.clone()));
    let region = tokio::spawn(region::run(ctx.clone()));
    let survival_goal = tokio::spawn(survival_goal::run(ctx.clone()));
    let tenant = tokio::spawn(tenant::run(ctx.clone()));
    let hypertable = tokio::spawn(hypertable::run(ctx.clone()));
    let federation = tokio::spawn(federation::run(ctx.clone()));
    let search_index = tokio::spawn(search_index::run(ctx.clone()));
    let scheduled_repack = tokio::spawn(scheduled_repack::run(ctx.clone()));
    let sidecar = tokio::spawn(sidecar::run(ctx.clone()));
    let webhook = tokio::spawn(webhook::run(ctx.clone()));
    let function = tokio::spawn(function::run(ctx));

    tokio::select! {
        result = backup => log_exit("backup", result),
        result = cluster => log_exit("citus_cluster", result),
        result = conflict_policy => log_exit("conflict_policy", result),
        result = migration => log_exit("migration", result),
        result = region => log_exit("region", result),
        result = survival_goal => log_exit("survival_goal", result),
        result = tenant => log_exit("tenant", result),
        result = hypertable => log_exit("hypertable", result),
        result = federation => log_exit("federation", result),
        result = search_index => log_exit("search_index", result),
        result = scheduled_repack => log_exit("scheduled_repack", result),
        result = sidecar => log_exit("sidecar", result),
        result = webhook => log_exit("webhook", result),
        result = function => log_exit("function", result),
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
