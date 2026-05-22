// FEATURE: C10
// FEATURE: M2
// FEATURE: M14

//! Rollback planner for F1-style schema changes.
//!
//! Each forward phase is reversible *until* `Backfill -> Public` lands; that
//! cutover commit is the F1 reorganization point. The rollback planner emits
//! an ordered sequence of [`RollbackStep`]s that the sidecar applies in
//! reverse to restore the prior phase's semantics, including a cleanup step
//! for any backfill rows the worker wrote before the abort.

use super::{SchemaJobError, SchemaJobPlan, SchemaJobState, COMPANION_INTERNAL_SCHEMA};
use std::error::Error;
use std::fmt;

/// One step in the rollback sequence.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RollbackStep {
    /// Walk the schema job state back to the target state.
    RevertState {
        from: SchemaJobState,
        to: SchemaJobState,
    },
    /// Remove rows the backfill worker partially populated.
    CleanupBackfillRows { table: String, column: String },
    /// Drop a column that was added during the rolled-back rollout.
    DropAddedColumn { table: String, column: String },
    /// Record the rollback decision in the phase log.
    RecordRollback {
        job_name: String,
        from_state: SchemaJobState,
        to_state: SchemaJobState,
        recorded_at: String,
    },
}

impl RollbackStep {
    /// Render this step as a single SQL statement.
    pub fn to_sql(&self) -> String {
        match self {
            Self::RevertState { from: _from, to } => format!(
                "SELECT {schema}.schema_job_rollback_to({state});",
                schema = COMPANION_INTERNAL_SCHEMA,
                state = sql_literal(to.as_canonical())
            ),
            Self::CleanupBackfillRows { table, column } => format!(
                "SELECT {schema}.schema_job_cleanup_backfill({table}, {column});",
                schema = COMPANION_INTERNAL_SCHEMA,
                table = sql_literal(table),
                column = sql_literal(column)
            ),
            Self::DropAddedColumn { table, column } => format!(
                "SELECT {schema}.schema_job_drop_added_column({table}, {column});",
                schema = COMPANION_INTERNAL_SCHEMA,
                table = sql_literal(table),
                column = sql_literal(column)
            ),
            Self::RecordRollback {
                job_name,
                from_state,
                to_state,
                recorded_at,
            } => format!(
                "SELECT {schema}.schema_job_phase_log_rollback({job}, {from}, {to}, {recorded});",
                schema = COMPANION_INTERNAL_SCHEMA,
                job = sql_literal(job_name),
                from = sql_literal(from_state.as_canonical()),
                to = sql_literal(to_state.as_canonical()),
                recorded = sql_literal(recorded_at)
            ),
        }
    }
}

/// Ordered set of rollback steps for one schema job.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RollbackPlan {
    pub job_name: String,
    pub steps: Vec<RollbackStep>,
}

impl RollbackPlan {
    /// Build a rollback plan for `plan` walking back to `to_state`. The
    /// caller supplies `recorded_at` (RFC3339 UTC) for the audit log entry.
    pub fn new(
        plan: &SchemaJobPlan,
        to_state: SchemaJobState,
        recorded_at: impl Into<String>,
    ) -> Result<Self, RollbackError> {
        plan.validate().map_err(RollbackError::from)?;
        if !plan.can_rollback_to(to_state) {
            return Err(RollbackError::IllegalRollback {
                from: plan.state,
                to: to_state,
            });
        }

        let mut steps = vec![RollbackStep::RevertState {
            from: plan.state,
            to: to_state,
        }];

        if plan.state == SchemaJobState::Backfill {
            for column in added_columns(plan) {
                steps.push(RollbackStep::CleanupBackfillRows {
                    table: plan.table.clone(),
                    column: column.clone(),
                });
            }
        }

        if matches!(to_state, SchemaJobState::DeleteOnly) {
            // When rolling all the way back to DeleteOnly, drop any column we
            // had added. The added column never made it to PUBLIC, so this
            // is safe.
            for column in added_columns(plan) {
                steps.push(RollbackStep::DropAddedColumn {
                    table: plan.table.clone(),
                    column: column.clone(),
                });
            }
        }

        steps.push(RollbackStep::RecordRollback {
            job_name: plan.name.clone(),
            from_state: plan.state,
            to_state,
            recorded_at: recorded_at.into(),
        });

        Ok(Self {
            job_name: plan.name.clone(),
            steps,
        })
    }

    pub fn script(&self) -> String {
        self.steps
            .iter()
            .map(RollbackStep::to_sql)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn added_columns(plan: &SchemaJobPlan) -> Vec<String> {
    plan.operations
        .iter()
        .filter_map(|op| match op {
            super::SchemaJobOperation::AddColumn { column, .. } => Some(column.clone()),
            _ => None,
        })
        .collect()
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RollbackError {
    PlanInvalid(SchemaJobError),
    IllegalRollback {
        from: SchemaJobState,
        to: SchemaJobState,
    },
}

impl fmt::Display for RollbackError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlanInvalid(error) => write!(formatter, "schema job plan invalid: {error}"),
            Self::IllegalRollback { from, to } => write!(
                formatter,
                "illegal rollback transition: {} -> {}",
                from.as_canonical(),
                to.as_canonical()
            ),
        }
    }
}

impl Error for RollbackError {}

impl From<SchemaJobError> for RollbackError {
    fn from(error: SchemaJobError) -> Self {
        Self::PlanInvalid(error)
    }
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::super::SchemaJobOperation;
    use super::*;

    fn plan_with_state(state: SchemaJobState) -> SchemaJobPlan {
        SchemaJobPlan {
            name: "orders-totals-v2".to_string(),
            table: "public.orders".to_string(),
            state,
            operations: vec![SchemaJobOperation::AddColumn {
                column: "totals_v2".to_string(),
                sql_type: "bigint".to_string(),
            }],
            lease_seconds: 30,
        }
    }

    #[test]
    fn backfill_rollback_to_delete_only_cleans_and_drops() {
        let plan = RollbackPlan::new(
            &plan_with_state(SchemaJobState::Backfill),
            SchemaJobState::DeleteOnly,
            "2026-05-22T14:00:00Z",
        )
        .unwrap();
        let kinds: Vec<_> = plan
            .steps
            .iter()
            .map(|step| std::mem::discriminant(step))
            .collect();
        assert_eq!(kinds.len(), 4); // revert + cleanup + drop + record
        let script = plan.script();
        assert!(script.contains("schema_job_rollback_to"));
        assert!(script.contains("schema_job_cleanup_backfill"));
        assert!(script.contains("schema_job_drop_added_column"));
        assert!(script.contains("schema_job_phase_log_rollback"));
    }

    #[test]
    fn write_only_rollback_to_delete_only_drops_added_column() {
        let plan = RollbackPlan::new(
            &plan_with_state(SchemaJobState::WriteOnly),
            SchemaJobState::DeleteOnly,
            "2026-05-22T14:00:00Z",
        )
        .unwrap();
        assert!(plan.script().contains("schema_job_drop_added_column"));
        assert!(!plan.script().contains("schema_job_cleanup_backfill"));
    }

    #[test]
    fn public_phase_cannot_rollback() {
        let result = RollbackPlan::new(
            &plan_with_state(SchemaJobState::Public),
            SchemaJobState::Backfill,
            "2026-05-22T14:00:00Z",
        );
        assert_eq!(
            result,
            Err(RollbackError::IllegalRollback {
                from: SchemaJobState::Public,
                to: SchemaJobState::Backfill,
            })
        );
    }

    #[test]
    fn rollback_records_audit_entry_last() {
        let plan = RollbackPlan::new(
            &plan_with_state(SchemaJobState::Backfill),
            SchemaJobState::WriteOnly,
            "2026-05-22T14:00:00Z",
        )
        .unwrap();
        assert!(matches!(
            plan.steps.last(),
            Some(RollbackStep::RecordRollback { .. })
        ));
    }
}
