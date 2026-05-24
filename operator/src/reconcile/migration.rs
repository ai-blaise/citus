// FEATURE: C9
// FEATURE: M3
// FEATURE: M14

//! MigrationReconciler: drives `Migration` CRs through the F1-style
//! schema-job state machine and the two-version invariant (2VI).
//!
//! The reconciler does not own the controller's tokio loop. Instead, it
//! plans the next sidecar invocation: which schema-job to drive, which
//! phase to target, and which gate to use. The companion crate hosts the
//! controller logic; the sidecar/schema_job daemon hosts the runtime. This
//! module is the operator's view of "what do we want to happen next".

use ai_blaise_citus_companion::{
    assert_migration_data_invariants_sql, verify_two_version_invariant_sql, SchemaJobOperation,
    SchemaJobPlan, SchemaJobState, TransitionGate,
};
use std::error::Error;
use std::fmt;

use crate::crds::migration::{
    MigrationConflictAction, MigrationSpec, MigrationSpecError, MigrationType,
};

/// The next thing the operator wants the schema-job sidecar to do for a
/// Migration CR. The operator emits one of these per Migration during each
/// reconcile sweep.
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
            data_invariants_verified: command.data_invariants_verified,
            invariant_check_sql: verify_two_version_invariant_sql().to_string(),
            data_invariant_check_sql: assert_migration_data_invariants_sql(&command.job_name)
                .map_err(|error| MigrationReconcileError::PlanInvalid(error.to_string()))?,
        })
    }
}

impl MigrationReconcilePlan {
    /// SQL that the operator includes in the sidecar invocation envelope so
    /// the sidecar always verifies the 2VI before acting.
    pub fn invariant_preflight_sql(&self) -> &str {
        &self.invariant_check_sql
    }

    /// SQL that must pass before the sidecar can publish a data-preserving
    /// migration through the final BACKFILL -> PUBLIC boundary.
    pub fn data_invariant_preflight_sql(&self) -> &str {
        &self.data_invariant_check_sql
    }
}

/// Input the operator constructs from a Migration CR plus live cluster
/// state. The operator's outer reconcile loop owns the actual K8s client.
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
                yaml: "operations:\n  - add_column:\n      table: users".to_string(),
                on_conflict: conflict,
            },
            job_name: "users-display-name".to_string(),
            table: "public.users".to_string(),
            current_state: state,
            operations: vec![SchemaJobOperation::AddColumn {
                column: "display_name".to_string(),
                sql_type: "text".to_string(),
            }],
            lease_seconds: 60,
            workers: vec!["worker-a".to_string(), "worker-b".to_string()],
            data_invariants_verified: true,
        }
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
        assert!(plan
            .invariant_preflight_sql()
            .contains("verify_two_version_invariant"));
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
        assert!(plan
            .data_invariant_preflight_sql()
            .contains("migration_assert_invariants"));
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
}
