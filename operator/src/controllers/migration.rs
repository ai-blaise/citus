//! `Migration` controller.
//!
//! Runs each `MigrationSpec` through the gh-ost-style state machine defined in
//! [`crate::crds::migration::state_machine`].

use super::{
    boundary::{
        retry_class_for_error, BoundaryOperation, BoundaryOperationKind, ControllerBoundaryPlan,
    },
    Context, ControllerError,
};
use crate::crds::migration::{
    state_machine::{transition, PhaseEvidence},
    MigrationConflictAction, MigrationPhase, MigrationSpec, MigrationType,
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

/// Kube-rs typed resource for the Migration CRD.
#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "citus.ai-blaise.io",
    version = "v2",
    kind = "Migration",
    namespaced,
    status = "MigrationStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct MigrationCrSpec {
    #[serde(default = "default_type")]
    pub migration_type: String,
    pub yaml: String,
    #[serde(default = "default_conflict")]
    pub on_conflict: String,
    #[serde(default)]
    pub shadow_table_built: bool,
    #[serde(default)]
    pub write_triggers_installed: bool,
    #[serde(default)]
    pub backfill_complete: bool,
    #[serde(default)]
    pub row_diff_verified: bool,
}

fn default_type() -> String {
    "Pgroll".to_string()
}

fn default_conflict() -> String {
    "ManualReview".to_string()
}

/// Status surface reported per Migration object.
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MigrationStatus {
    pub phase: String,
}

impl MigrationCrSpec {
    pub fn to_authoritative(&self) -> MigrationSpec {
        MigrationSpec {
            migration_type: match self.migration_type.as_str() {
                "GhOst" => MigrationType::GhOst,
                _ => MigrationType::Pgroll,
            },
            yaml: self.yaml.clone(),
            on_conflict: match self.on_conflict.as_str() {
                "Fail" => MigrationConflictAction::Fail,
                "Skip" => MigrationConflictAction::Skip,
                "Replace" => MigrationConflictAction::Replace,
                _ => MigrationConflictAction::ManualReview,
            },
        }
    }

    pub fn evidence(&self) -> PhaseEvidence {
        PhaseEvidence {
            shadow_table_built: self.shadow_table_built,
            write_triggers_installed: self.write_triggers_installed,
            backfill_complete: self.backfill_complete,
            row_diff_verified: self.row_diff_verified,
        }
    }
}

pub async fn run(ctx: Arc<Context>) -> Result<(), ControllerError> {
    let api: Api<Migration> = Api::default_namespaced(ctx.client.clone());
    info!("Migration controller starting");
    Controller::new(api, watcher::Config::default())
        .run(reconcile, error_policy, ctx)
        .for_each(|res| async move {
            match res {
                Ok((object, _action)) => debug!(?object, "reconciled Migration"),
                Err(error) => error!(?error, "Migration reconcile error"),
            }
        })
        .await;
    Ok(())
}

async fn reconcile(
    migration: Arc<Migration>,
    ctx: Arc<Context>,
) -> Result<Action, ControllerError> {
    let resource_name = migration
        .metadata
        .name
        .clone()
        .unwrap_or_else(|| "<unnamed>".to_string());
    let authoritative = migration.spec.to_authoritative();
    authoritative
        .validate()
        .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
    let evidence = migration.spec.evidence();
    let current_phase = current_phase(&migration.status);
    let next = transition(current_phase, &evidence)
        .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
    let boundary = ControllerBoundaryPlan::try_new(
        "Migration",
        &resource_name,
        ctx.execution_mode,
        vec![
            BoundaryOperation::render_plan("render_migration_state_transition"),
            BoundaryOperation::alpha(
                "invoke_schema_job_sidecar",
                BoundaryOperationKind::KubernetesApply,
            ),
            BoundaryOperation::alpha(
                "patch_migration_status",
                BoundaryOperationKind::StatusMutation,
            ),
        ],
        ctx.default_requeue,
    )?;
    info!(
        migration = %resource_name,
        from = ?current_phase,
        to = ?next,
        boundary = %boundary.render_tsv(),
        "Migration state machine reconciled in bounded dry-run/apply contract"
    );
    Ok(Action::requeue(ctx.default_requeue))
}

fn current_phase(status: &Option<MigrationStatus>) -> MigrationPhase {
    match status.as_ref().map(|status| status.phase.as_str()) {
        Some("WriteOnly") => MigrationPhase::WriteOnly,
        Some("Backfill") => MigrationPhase::Backfill,
        Some("Public") => MigrationPhase::Public,
        Some("Complete") => MigrationPhase::Complete,
        _ => MigrationPhase::DeleteOnly,
    }
}

fn error_policy(_migration: Arc<Migration>, error: &ControllerError, ctx: Arc<Context>) -> Action {
    let retry_class = retry_class_for_error(error);
    error!(
        ?error,
        retry_class = retry_class.as_str(),
        "Migration controller classified reconcile error"
    );
    retry_class.action(ctx.default_requeue)
}
