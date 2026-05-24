//! `Migration` controller.
//!
//! Runs each `MigrationSpec` through the schema-job handoff model and records
//! the controller execution boundary for Kubernetes/status side effects.

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
use crate::reconcile::migration::{MigrationCommand, MigrationReconcilePlan};
use ai_blaise_citus_companion::{SchemaJobOperation, SchemaJobState};
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
    #[serde(default)]
    pub table: Option<String>,
    #[serde(default = "default_current_state")]
    pub current_state: String,
    #[serde(default = "default_lease_seconds")]
    pub lease_seconds: u32,
    #[serde(default)]
    pub expected_workers: Vec<String>,
    #[serde(default)]
    pub operations: Vec<MigrationOperationCr>,
}

fn default_type() -> String {
    "Pgroll".to_string()
}

fn default_conflict() -> String {
    "ManualReview".to_string()
}

fn default_current_state() -> String {
    "delete_only".to_string()
}

fn default_lease_seconds() -> u32 {
    60
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MigrationOperationCr {
    #[serde(rename = "type")]
    pub operation_type: String,
    #[serde(default)]
    pub column: Option<String>,
    #[serde(default)]
    pub sql_type: Option<String>,
    #[serde(default)]
    pub statement: Option<String>,
    #[serde(default)]
    pub old_column: Option<String>,
    #[serde(default)]
    pub new_column: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MigrationStatus {
    pub phase: String,
    pub target_phase: String,
    pub apply_steps: usize,
}

impl MigrationCrSpec {
    pub fn to_authoritative(&self) -> MigrationSpec {
        MigrationSpec {
            migration_type: match normalize_token(&self.migration_type).as_str() {
                "ghost" => MigrationType::GhOst,
                _ => MigrationType::Pgroll,
            },
            yaml: self.yaml.clone(),
            on_conflict: match normalize_token(&self.on_conflict).as_str() {
                "fail" => MigrationConflictAction::Fail,
                "skip" => MigrationConflictAction::Skip,
                "replace" => MigrationConflictAction::Replace,
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

    pub fn command_for_resource(
        &self,
        resource_name: &str,
        status: &Option<MigrationStatus>,
    ) -> Result<MigrationCommand, String> {
        let table = required_option("table", &self.table)?;
        let current_state = parse_state(
            status
                .as_ref()
                .map(|status| status.phase.as_str())
                .filter(|phase| !phase.trim().is_empty())
                .unwrap_or(&self.current_state),
        )?;
        let operations = self
            .operations
            .iter()
            .map(MigrationOperationCr::to_schema_job_operation)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(MigrationCommand {
            spec: self.to_authoritative(),
            job_name: resource_name.to_string(),
            table,
            current_state,
            operations,
            lease_seconds: self.lease_seconds,
            workers: self.expected_workers.clone(),
        })
    }
}

impl MigrationOperationCr {
    fn to_schema_job_operation(&self) -> Result<SchemaJobOperation, String> {
        match normalize_token(&self.operation_type).as_str() {
            "addcolumn" => Ok(SchemaJobOperation::AddColumn {
                column: required_option("operations.column", &self.column)?,
                sql_type: required_option("operations.sqlType", &self.sql_type)?,
            }),
            "backfill" => Ok(SchemaJobOperation::Backfill {
                statement: required_option("operations.statement", &self.statement)?,
            }),
            "swapcolumn" => Ok(SchemaJobOperation::SwapColumn {
                old_column: required_option("operations.oldColumn", &self.old_column)?,
                new_column: required_option("operations.newColumn", &self.new_column)?,
            }),
            "dropcolumn" => Ok(SchemaJobOperation::DropColumn {
                column: required_option("operations.column", &self.column)?,
            }),
            other => Err(format!("unsupported migration operation: {other}")),
        }
    }
}

fn required_option(field: &str, value: &Option<String>) -> Result<String, String> {
    match value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) => Ok(value.to_string()),
        None => Err(format!("{field} must not be empty")),
    }
}

fn parse_state(value: &str) -> Result<SchemaJobState, String> {
    SchemaJobState::from_canonical(&normalize_phase(value)).map_err(|error| error.to_string())
}

fn normalize_phase(value: &str) -> String {
    let lower = value.trim().to_ascii_lowercase();
    match lower.as_str() {
        "deleteonly" | "delete-only" => "delete_only".to_string(),
        "writeonly" | "write-only" => "write_only".to_string(),
        other => other.replace('-', "_"),
    }
}

fn normalize_token(value: &str) -> String {
    value
        .trim()
        .chars()
        .filter(|character| !matches!(character, '-' | '_'))
        .collect::<String>()
        .to_ascii_lowercase()
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
        .unwrap_or_else(|| "migration".to_string());
    let command = migration
        .spec
        .command_for_resource(&resource_name, &migration.status)
        .map_err(ControllerError::InvalidSpec)?;
    let plan = MigrationReconcilePlan::try_from(&command)
        .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
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
        table = %plan.schema_job.table,
        current_state = %plan.schema_job.state.as_canonical(),
        target_state = %plan.target_state.as_canonical(),
        from = ?current_phase,
        to = ?next,
        operations = plan.schema_job.operations.len(),
        workers = plan.expected_workers.len(),
        apply_steps = plan.apply_plan().steps.len(),
        boundary = %boundary.render_tsv(),
        "Migration reconcile plan built within bounded dry-run/apply contract"
    );
    Ok(Action::requeue(ctx.default_requeue))
}

fn current_phase(status: &Option<MigrationStatus>) -> MigrationPhase {
    match status.as_ref().map(|status| normalize_phase(&status.phase)) {
        Some(phase) if phase == "write_only" => MigrationPhase::WriteOnly,
        Some(phase) if phase == "backfill" => MigrationPhase::Backfill,
        Some(phase) if phase == "public" => MigrationPhase::Public,
        Some(phase) if phase == "complete" => MigrationPhase::Complete,
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cr_spec_round_trips_into_reconcile_plan() {
        let cr = MigrationCrSpec {
            migration_type: "Pgroll".to_string(),
            yaml: "operations:\n  - add_column".to_string(),
            on_conflict: "ManualReview".to_string(),
            table: Some("public.users".to_string()),
            current_state: "delete_only".to_string(),
            lease_seconds: 60,
            expected_workers: vec!["worker-a".to_string(), "worker-b".to_string()],
            operations: vec![MigrationOperationCr {
                operation_type: "AddColumn".to_string(),
                column: Some("display_name".to_string()),
                sql_type: Some("text".to_string()),
                statement: None,
                old_column: None,
                new_column: None,
            }],
        };
        let command = cr
            .command_for_resource("users-display-name", &None)
            .expect("command");
        let plan = MigrationReconcilePlan::try_from(&command).expect("plan");
        assert_eq!(plan.target_state, SchemaJobState::WriteOnly);
        assert_eq!(plan.apply_plan().steps.len(), 5);
    }

    #[test]
    fn status_phase_takes_precedence_over_spec_phase() {
        let mut cr = baseline_cr();
        cr.current_state = "delete_only".to_string();
        let status = Some(MigrationStatus {
            phase: "write_only".to_string(),
            target_phase: String::new(),
            apply_steps: 0,
        });
        let command = cr.command_for_resource("job", &status).expect("command");
        assert_eq!(command.current_state, SchemaJobState::WriteOnly);
    }

    #[test]
    fn missing_table_is_rejected() {
        let mut cr = baseline_cr();
        cr.table = None;
        assert_eq!(
            cr.command_for_resource("job", &None),
            Err("table must not be empty".to_string())
        );
    }

    #[test]
    fn unsupported_operation_is_rejected() {
        let mut cr = baseline_cr();
        cr.operations[0].operation_type = "rename-table".to_string();
        assert_eq!(
            cr.command_for_resource("job", &None),
            Err("unsupported migration operation: renametable".to_string())
        );
    }

    fn baseline_cr() -> MigrationCrSpec {
        MigrationCrSpec {
            migration_type: "Pgroll".to_string(),
            yaml: "operations:\n  - add_column".to_string(),
            on_conflict: "ManualReview".to_string(),
            table: Some("public.users".to_string()),
            current_state: "delete_only".to_string(),
            lease_seconds: 60,
            expected_workers: vec!["worker-a".to_string()],
            operations: vec![MigrationOperationCr {
                operation_type: "AddColumn".to_string(),
                column: Some("display_name".to_string()),
                sql_type: Some("text".to_string()),
                statement: None,
                old_column: None,
                new_column: None,
            }],
        }
    }
}
