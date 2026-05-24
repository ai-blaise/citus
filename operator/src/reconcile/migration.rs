// FEATURE: C9
// FEATURE: M3
// FEATURE: M14

//! MigrationReconciler: drives `Migration` CRs through the F1-style
//! schema-job state machine and the two-version invariant (2VI).

use ai_blaise_citus_companion::{
    assert_migration_data_invariants_sql, verify_two_version_invariant_sql, SchemaJobOperation,
    SchemaJobPlan, SchemaJobState, TransitionGate,
};
use std::error::Error;
use std::fmt;

use crate::crds::migration::{
    MigrationConflictAction, MigrationSpec, MigrationSpecError, MigrationType,
    MIGRATION_2VI_PRECHECK_SQL,
};

pub const SCHEMA_JOB_START_FUNCTION: &str = "companion_internal.schema_job_start";
pub const SCHEMA_JOB_ADD_OPERATION_FUNCTION: &str = "companion_internal.schema_job_add_operation";
pub const SCHEMA_JOB_ADVANCE_FUNCTION: &str = "companion_internal.schema_job_advance";
pub const SCHEMA_JOB_STATUS_VIEW: &str = "companion_schema_jobs";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MigrationReconcilePlan {
    pub migration_type: MigrationType,
    pub conflict_action: MigrationConflictAction,
    pub schema_job: SchemaJobPlan,
    pub target_state: SchemaJobState,
    pub gate: TransitionGate,
    pub expected_workers: Vec<String>,
    pub invariant_check_sql: String,
    pub data_invariants_verified: bool,
    pub data_invariant_check_sql: String,
}

impl TryFrom<&MigrationCommand> for MigrationReconcilePlan {
    type Error = MigrationReconcileError;

    fn try_from(command: &MigrationCommand) -> Result<Self, Self::Error> {
        command.spec.validate()?;
        if command.workers.is_empty() {
            return Err(MigrationReconcileError::NoWorkers);
        }

        let schema_job = SchemaJobPlan {
            name: command.job_name.clone(),
            table: command.table.clone(),
            state: command.current_state,
            operations: command.operations.clone(),
            lease_seconds: command.lease_seconds,
        };
        schema_job
            .validate()
            .map_err(|error| MigrationReconcileError::PlanInvalid(error.to_string()))?;

        let target_state = next_state_after(command.current_state).ok_or(
            MigrationReconcileError::NoForwardTransition(command.current_state),
        )?;
        if !schema_job.can_advance_to(target_state) {
            return Err(MigrationReconcileError::IllegalTransition {
                from: command.current_state,
                to: target_state,
            });
        }

        if target_state == SchemaJobState::Public && !command.data_invariants_verified {
            return Err(MigrationReconcileError::DataInvariantsNotVerified(
                command.job_name.clone(),
            ));
        }

        let gate = gate_from_conflict(command.spec.on_conflict);

        Ok(Self {
            migration_type: command.spec.migration_type,
            conflict_action: command.spec.on_conflict,
            schema_job,
            target_state,
            gate,
            expected_workers: command.workers.clone(),
            invariant_check_sql: verify_two_version_invariant_sql().to_string(),
            data_invariants_verified: command.data_invariants_verified,
            data_invariant_check_sql: assert_migration_data_invariants_sql(&command.job_name)
                .map_err(|error| MigrationReconcileError::PlanInvalid(error.to_string()))?,
        })
    }
}

impl MigrationReconcilePlan {
    pub fn migration_type_str(&self) -> &'static str {
        match self.migration_type {
            MigrationType::Pgroll => "pgroll",
            MigrationType::GhOst => "gh-ost",
        }
    }

    pub fn conflict_action_str(&self) -> &'static str {
        match self.conflict_action {
            MigrationConflictAction::Fail => "fail",
            MigrationConflictAction::Skip => "skip",
            MigrationConflictAction::Replace => "replace",
            MigrationConflictAction::ManualReview => "manual_review",
        }
    }

    pub fn target_state_str(&self) -> &'static str {
        self.target_state.as_canonical()
    }

    pub fn invariant_preflight_sql(&self) -> &str {
        &self.invariant_check_sql
    }

    pub fn data_invariant_preflight_sql(&self) -> &str {
        &self.data_invariant_check_sql
    }

    pub fn apply_plan(&self) -> MigrationApplyPlan {
        let mut steps = vec![
            MigrationApplyStep::new(
                "verify_two_version_invariant_preflight",
                verify_two_version_invariant_preflight_sql(),
                true,
            ),
            MigrationApplyStep::new(
                "ensure_ai_blaise_citus_extension",
                "CREATE EXTENSION IF NOT EXISTS ai_blaise_citus;",
                true,
            ),
            MigrationApplyStep::new(
                "start_schema_job",
                start_schema_job_sql(&self.schema_job),
                true,
            ),
        ];

        steps.extend(
            self.schema_job
                .operations
                .iter()
                .enumerate()
                .map(|(index, operation)| {
                    MigrationApplyStep::new(
                        format!("record_schema_job_operation_{:02}", index + 1),
                        add_operation_sql(&self.schema_job.name, operation),
                        false,
                    )
                }),
        );

        steps.push(MigrationApplyStep::new(
            format!("advance_to_{}", self.target_state.as_canonical()),
            advance_sql(&self.schema_job.name, self.target_state),
            true,
        ));
        steps.push(MigrationApplyStep::new(
            "verify_two_version_invariant",
            self.invariant_check_sql.clone(),
            true,
        ));
        steps.push(MigrationApplyStep::new(
            "verify_migration_data_invariants",
            self.data_invariant_check_sql.clone(),
            true,
        ));

        MigrationApplyPlan { steps }
    }

    pub fn apply_sql_script(&self) -> String {
        self.apply_plan().sql_script()
    }

    pub fn status_sql(&self) -> String {
        format!(
            "SELECT job_name, table_name, state, lease_expires_at FROM {view} WHERE job_name = {job};",
            view = SCHEMA_JOB_STATUS_VIEW,
            job = sql_literal(&self.schema_job.name),
        )
    }

    pub fn teardown_sql(&self, action: MigrationTeardownAction) -> String {
        let state = match action {
            MigrationTeardownAction::Pause => SchemaJobState::Paused,
            MigrationTeardownAction::Cancel => SchemaJobState::Canceled,
        };
        advance_sql(&self.schema_job.name, state)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MigrationTeardownAction {
    Pause,
    Cancel,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MigrationApplyStep {
    pub name: String,
    pub sql: String,
    pub idempotent: bool,
}

impl MigrationApplyStep {
    fn new(name: impl Into<String>, sql: impl Into<String>, idempotent: bool) -> Self {
        Self {
            name: name.into(),
            sql: sql.into(),
            idempotent,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MigrationApplyPlan {
    pub steps: Vec<MigrationApplyStep>,
}

impl MigrationApplyPlan {
    pub fn sql_script(&self) -> String {
        self.steps
            .iter()
            .map(|step| step.sql.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MigrationCommand {
    pub spec: MigrationSpec,
    pub job_name: String,
    pub table: String,
    pub current_state: SchemaJobState,
    pub operations: Vec<SchemaJobOperation>,
    pub lease_seconds: u32,
    pub workers: Vec<String>,
    pub data_invariants_verified: bool,
}

fn next_state_after(current: SchemaJobState) -> Option<SchemaJobState> {
    match current {
        SchemaJobState::DeleteOnly => Some(SchemaJobState::WriteOnly),
        SchemaJobState::WriteOnly => Some(SchemaJobState::Backfill),
        SchemaJobState::Backfill => Some(SchemaJobState::Public),
        SchemaJobState::Public | SchemaJobState::Paused | SchemaJobState::Canceled => None,
    }
}

fn gate_from_conflict(action: MigrationConflictAction) -> TransitionGate {
    match action {
        MigrationConflictAction::Fail => TransitionGate::RollbackOnTimeout,
        MigrationConflictAction::Skip => TransitionGate::SkipMissing,
        MigrationConflictAction::Replace | MigrationConflictAction::ManualReview => {
            TransitionGate::WaitForever
        }
    }
}

fn start_schema_job_sql(plan: &SchemaJobPlan) -> String {
    format!(
        "SELECT {function}({job}, {table}, {lease_seconds});",
        function = SCHEMA_JOB_START_FUNCTION,
        job = sql_literal(&plan.name),
        table = sql_literal(&plan.table),
        lease_seconds = plan.lease_seconds,
    )
}

fn add_operation_sql(job_name: &str, operation: &SchemaJobOperation) -> String {
    match operation {
        SchemaJobOperation::AddColumn { column, sql_type } => format!(
            "SELECT {function}({job}, 'add_column', {column}, {sql_type}, NULL, NULL);",
            function = SCHEMA_JOB_ADD_OPERATION_FUNCTION,
            job = sql_literal(job_name),
            column = sql_literal(column),
            sql_type = sql_literal(sql_type),
        ),
        SchemaJobOperation::Backfill { statement } => format!(
            "SELECT {function}({job}, 'backfill', NULL, NULL, {statement}, NULL);",
            function = SCHEMA_JOB_ADD_OPERATION_FUNCTION,
            job = sql_literal(job_name),
            statement = sql_literal(statement),
        ),
        SchemaJobOperation::SwapColumn {
            old_column,
            new_column,
        } => format!(
            "SELECT {function}({job}, 'swap_column', {old_column}, NULL, NULL, {new_column});",
            function = SCHEMA_JOB_ADD_OPERATION_FUNCTION,
            job = sql_literal(job_name),
            old_column = sql_literal(old_column),
            new_column = sql_literal(new_column),
        ),
        SchemaJobOperation::DropColumn { column } => format!(
            "SELECT {function}({job}, 'drop_column', {column}, NULL, NULL, NULL);",
            function = SCHEMA_JOB_ADD_OPERATION_FUNCTION,
            job = sql_literal(job_name),
            column = sql_literal(column),
        ),
    }
}

fn advance_sql(job_name: &str, state: SchemaJobState) -> String {
    format!(
        "SELECT {function}({job}, {state});",
        function = SCHEMA_JOB_ADVANCE_FUNCTION,
        job = sql_literal(job_name),
        state = sql_literal(state.as_canonical()),
    )
}

fn verify_two_version_invariant_preflight_sql() -> String {
    format!("SELECT {MIGRATION_2VI_PRECHECK_SQL};")
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MigrationReconcileError {
    SpecInvalid(MigrationSpecError),
    PlanInvalid(String),
    IllegalTransition {
        from: SchemaJobState,
        to: SchemaJobState,
    },
    NoForwardTransition(SchemaJobState),
    NoWorkers,
    DataInvariantsNotVerified(String),
}

impl fmt::Display for MigrationReconcileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SpecInvalid(err) => write!(formatter, "migration spec invalid: {err}"),
            Self::PlanInvalid(message) => write!(formatter, "{message}"),
            Self::IllegalTransition { from, to } => write!(
                formatter,
                "illegal transition: {} -> {}",
                from.as_canonical(),
                to.as_canonical()
            ),
            Self::NoForwardTransition(state) => write!(
                formatter,
                "no forward transition from {}",
                state.as_canonical()
            ),
            Self::NoWorkers => write!(formatter, "no workers attached to migration"),
            Self::DataInvariantsNotVerified(job_name) => write!(
                formatter,
                "data invariants are not verified for migration {job_name}"
            ),
        }
    }
}

impl Error for MigrationReconcileError {}

impl From<MigrationSpecError> for MigrationReconcileError {
    fn from(error: MigrationSpecError) -> Self {
        Self::SpecInvalid(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(state: SchemaJobState, conflict: MigrationConflictAction) -> MigrationCommand {
        MigrationCommand {
            spec: MigrationSpec {
                migration_type: MigrationType::Pgroll,
                yaml: valid_yaml(),
                on_conflict: conflict,
            },
            job_name: "users-display-name".to_string(),
            table: "public.users".to_string(),
            current_state: state,
            operations: vec![
                SchemaJobOperation::AddColumn {
                    column: "display_name".to_string(),
                    sql_type: "text".to_string(),
                },
                SchemaJobOperation::Backfill {
                    statement: "UPDATE public.users SET display_name = email".to_string(),
                },
            ],
            lease_seconds: 60,
            workers: vec!["worker-a".to_string(), "worker-b".to_string()],
            data_invariants_verified: true,
        }
    }

    fn valid_yaml() -> String {
        "twoVersionInvariantPrecheck: companion_internal.verify_two_version_invariant()\nrollback:\n  operation: companion_internal.schema_job_rollback_to\n  targetPhase: write_only\noperations:\n  - addColumn:\n      table: public.users\n      column: display_name\n      sqlType: text\n  - backfill:\n      statement: UPDATE public.users SET display_name = email"
            .to_string()
    }

    #[test]
    fn pgroll_migration_in_delete_only_advances_to_write_only() {
        let plan = MigrationReconcilePlan::try_from(&command(
            SchemaJobState::DeleteOnly,
            MigrationConflictAction::Replace,
        ))
        .unwrap();
        assert_eq!(plan.target_state, SchemaJobState::WriteOnly);
        assert_eq!(plan.gate, TransitionGate::WaitForever);
        assert_eq!(plan.migration_type_str(), "pgroll");
        assert_eq!(plan.conflict_action_str(), "replace");
        assert!(plan
            .invariant_preflight_sql()
            .contains("verify_two_version_invariant"));
        assert!(plan
            .data_invariant_preflight_sql()
            .contains("migration_assert_invariants"));
    }

    #[test]
    fn fail_on_conflict_uses_rollback_gate() {
        let plan = MigrationReconcilePlan::try_from(&command(
            SchemaJobState::Backfill,
            MigrationConflictAction::Fail,
        ))
        .unwrap();
        assert_eq!(plan.target_state, SchemaJobState::Public);
        assert_eq!(plan.gate, TransitionGate::RollbackOnTimeout);
    }

    #[test]
    fn skip_on_conflict_uses_skip_missing_gate() {
        let plan = MigrationReconcilePlan::try_from(&command(
            SchemaJobState::WriteOnly,
            MigrationConflictAction::Skip,
        ))
        .unwrap();
        assert_eq!(plan.gate, TransitionGate::SkipMissing);
    }

    #[test]
    fn apply_plan_renders_schema_job_sql_and_2vi_guard() {
        let plan = MigrationReconcilePlan::try_from(&command(
            SchemaJobState::DeleteOnly,
            MigrationConflictAction::ManualReview,
        ))
        .unwrap();
        let apply = plan.apply_plan();
        assert_eq!(apply.steps.len(), 8);
        assert!(apply.sql_script().contains(SCHEMA_JOB_START_FUNCTION));
        assert!(apply
            .sql_script()
            .contains(SCHEMA_JOB_ADD_OPERATION_FUNCTION));
        assert!(apply.sql_script().contains(SCHEMA_JOB_ADVANCE_FUNCTION));
        assert!(apply.sql_script().contains("verify_two_version_invariant"));
        assert_eq!(
            apply.steps[0].name,
            "verify_two_version_invariant_preflight"
        );
        assert!(apply.sql_script().contains("migration_assert_invariants"));
        assert!(plan.status_sql().contains(SCHEMA_JOB_STATUS_VIEW));
        assert!(plan
            .teardown_sql(MigrationTeardownAction::Pause)
            .contains("'paused'"));
    }

    #[test]
    fn publish_requires_verified_data_invariants() {
        let mut cmd = command(SchemaJobState::Backfill, MigrationConflictAction::Replace);
        cmd.data_invariants_verified = false;

        assert_eq!(
            MigrationReconcilePlan::try_from(&cmd),
            Err(MigrationReconcileError::DataInvariantsNotVerified(
                "users-display-name".to_string()
            ))
        );
    }

    #[test]
    fn public_state_has_no_forward_transition() {
        let result = MigrationReconcilePlan::try_from(&command(
            SchemaJobState::Public,
            MigrationConflictAction::Replace,
        ));
        assert_eq!(
            result,
            Err(MigrationReconcileError::NoForwardTransition(
                SchemaJobState::Public
            ))
        );
    }

    #[test]
    fn missing_workers_rejected() {
        let mut cmd = command(SchemaJobState::DeleteOnly, MigrationConflictAction::Replace);
        cmd.workers.clear();
        assert_eq!(
            MigrationReconcilePlan::try_from(&cmd),
            Err(MigrationReconcileError::NoWorkers)
        );
    }
}
