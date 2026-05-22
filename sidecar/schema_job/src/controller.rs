// FEATURE: C10
// FEATURE: M2
// FEATURE: M14

//! F1 controller loop hosted by the schema-job sidecar.
//!
//! The sidecar persists the controller cursor in the companion catalog and
//! drives one tick per polling interval. Each tick:
//!
//! 1. Loads the live worker leases from `companion.worker_schema_lease`.
//! 2. Asks `SchemaJobController::transition` whether the next phase can land.
//! 3. Emits a deterministic [`ControllerTickReport`] for the sidecar's
//!    polling loop, including the SQL statements that should be applied
//!    against the coordinator.
//!
//! The companion crate owns the *logic*; this module owns the *transport
//! contract* between sidecar -> companion. The tokio executor itself lives
//! in `sidecar/schema_job/src/main.rs`. Keeping the controller pure makes
//! it deterministic in tests.

use crate::SchemaJobSidecarError;
use ai_blaise_citus_companion::{
    verify_two_version_invariant_sql, PhaseCheckpoint, PhaseTransitionDecision,
    PhaseTransitionPlan, RollbackPlan, RollbackStep, SchemaJobController, SchemaJobControllerError,
    SchemaJobPlan, SchemaJobState, TransitionGate, WorkerLease, WorkerLeaseRegistry,
    WorkerLeaseStatus,
};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

/// Input to one controller tick.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ControllerTickInput {
    pub plan: SchemaJobPlan,
    pub target_state: SchemaJobState,
    pub expected_workers: Vec<String>,
    pub leases: Vec<WorkerLease>,
    pub now: String,
    pub started_at: String,
    pub gate: TransitionGate,
}

/// Deterministic output of one controller tick.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ControllerTickReport {
    pub job_name: String,
    pub from_state: SchemaJobState,
    pub target_state: SchemaJobState,
    pub gate: TransitionGate,
    pub decision: ControllerTickDecision,
    pub sql_statements: Vec<String>,
    pub two_version_invariant_check_sql: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ControllerTickDecision {
    /// Advance to `target_state` and log a checkpoint.
    Advance(PhaseCheckpoint),
    /// Wait — at least one expected worker is missing or stale.
    Wait { delinquent_workers: Vec<String> },
    /// Rollback was triggered: emits the rollback plan that the sidecar
    /// must apply before the next forward tick.
    Rollback {
        expired_workers: Vec<String>,
        rollback: RollbackPlan,
    },
}

impl ControllerTickDecision {
    pub fn as_canonical(&self) -> &'static str {
        match self {
            Self::Advance(_) => "advance",
            Self::Wait { .. } => "wait",
            Self::Rollback { .. } => "rollback",
        }
    }
}

/// Runs the F1 controller for one tick. Pure function: no I/O.
pub fn tick(input: &ControllerTickInput) -> Result<ControllerTickReport, ControllerError> {
    input
        .plan
        .validate()
        .map_err(ControllerError::PlanInvalid)?;
    if input.expected_workers.is_empty() {
        return Err(ControllerError::NoExpectedWorkers);
    }

    let unique_workers: BTreeSet<&String> = input.expected_workers.iter().collect();
    if unique_workers.len() != input.expected_workers.len() {
        return Err(ControllerError::DuplicateWorker);
    }

    let mut registry = WorkerLeaseRegistry::new(&input.plan.name)
        .map_err(|err| ControllerError::Companion(err.to_string()))?;
    registry.expect_phase(input.plan.state);
    for lease in &input.leases {
        registry
            .record(lease.clone())
            .map_err(|err| ControllerError::Companion(err.to_string()))?;
    }

    let request = PhaseTransitionPlan {
        plan: input.plan.clone(),
        target_state: input.target_state,
        now: input.now.clone(),
        started_at: input.started_at.clone(),
        expected_workers: input.expected_workers.clone(),
        registry,
        gate: input.gate,
    };

    let controller = SchemaJobController::new();
    let decision = controller.transition(&request)?;

    let mut sql = Vec::new();
    let tick_decision = match decision {
        PhaseTransitionDecision::Advance(checkpoint) => {
            sql.push(checkpoint.to_sql());
            sql.push(format!(
                "SELECT companion_internal.schema_job_advance('{}', '{}');",
                escape_single_quote(&input.plan.name),
                input.target_state.as_canonical()
            ));
            ControllerTickDecision::Advance(checkpoint)
        }
        PhaseTransitionDecision::WaitForAcknowledgement {
            delinquent_workers,
            gate: _,
        } => ControllerTickDecision::Wait { delinquent_workers },
        PhaseTransitionDecision::AbortOnTimeout {
            expired_workers,
            gate: _,
        } => {
            let rollback_target = rollback_target_for(input.plan.state)?;
            let rollback = RollbackPlan::new(&input.plan, rollback_target, input.now.clone())
                .map_err(|err| ControllerError::Companion(err.to_string()))?;
            sql.extend(rollback.steps.iter().map(RollbackStep::to_sql));
            ControllerTickDecision::Rollback {
                expired_workers,
                rollback,
            }
        }
    };

    Ok(ControllerTickReport {
        job_name: input.plan.name.clone(),
        from_state: input.plan.state,
        target_state: input.target_state,
        gate: input.gate,
        decision: tick_decision,
        sql_statements: sql,
        two_version_invariant_check_sql: verify_two_version_invariant_sql().to_string(),
    })
}

fn rollback_target_for(state: SchemaJobState) -> Result<SchemaJobState, ControllerError> {
    match state {
        SchemaJobState::WriteOnly => Ok(SchemaJobState::DeleteOnly),
        SchemaJobState::Backfill => Ok(SchemaJobState::WriteOnly),
        SchemaJobState::Public | SchemaJobState::DeleteOnly => {
            Err(ControllerError::NoRollbackTarget(state))
        }
        SchemaJobState::Paused | SchemaJobState::Canceled => {
            Err(ControllerError::NoRollbackTarget(state))
        }
    }
}

fn escape_single_quote(value: &str) -> String {
    value.replace('\'', "''")
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ControllerError {
    PlanInvalid(ai_blaise_citus_companion::SchemaJobError),
    Companion(String),
    Transition(SchemaJobControllerError),
    NoExpectedWorkers,
    DuplicateWorker,
    NoRollbackTarget(SchemaJobState),
}

impl fmt::Display for ControllerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlanInvalid(err) => write!(formatter, "plan invalid: {err}"),
            Self::Companion(message) => write!(formatter, "{message}"),
            Self::Transition(err) => write!(formatter, "{err}"),
            Self::NoExpectedWorkers => write!(formatter, "expected_workers must not be empty"),
            Self::DuplicateWorker => write!(formatter, "expected_workers contains duplicates"),
            Self::NoRollbackTarget(state) => write!(
                formatter,
                "no rollback target for state {}",
                state.as_canonical()
            ),
        }
    }
}

impl Error for ControllerError {}

impl From<SchemaJobControllerError> for ControllerError {
    fn from(error: SchemaJobControllerError) -> Self {
        Self::Transition(error)
    }
}

impl From<ControllerError> for SchemaJobSidecarError {
    fn from(error: ControllerError) -> Self {
        Self::Companion(error.to_string())
    }
}

/// Snapshot of one expected worker as the controller sees it. Useful for
/// monitoring dashboards.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WorkerStatusSnapshot {
    pub worker_id: String,
    pub status: WorkerLeaseStatus,
}

/// Build the snapshot vector for one tick.
pub fn worker_status_snapshot(input: &ControllerTickInput) -> Vec<WorkerStatusSnapshot> {
    let Ok(mut registry) = WorkerLeaseRegistry::new(&input.plan.name) else {
        return Vec::new();
    };
    registry.expect_phase(input.plan.state);
    for lease in &input.leases {
        let _ = registry.record(lease.clone());
    }
    input
        .expected_workers
        .iter()
        .map(|worker| WorkerStatusSnapshot {
            worker_id: worker.clone(),
            status: registry.status_for(worker, &input.now),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_blaise_citus_companion::SchemaJobOperation;

    fn plan(state: SchemaJobState) -> SchemaJobPlan {
        SchemaJobPlan {
            name: "users-display-name".to_string(),
            table: "public.users".to_string(),
            state,
            operations: vec![SchemaJobOperation::AddColumn {
                column: "display_name".to_string(),
                sql_type: "text".to_string(),
            }],
            lease_seconds: 30,
        }
    }

    fn lease(worker_id: &str, phase: SchemaJobState, expires: &str) -> WorkerLease {
        WorkerLease::new(worker_id, "users-display-name", "schema-v2", phase, expires)
            .expect("lease")
    }

    #[test]
    fn tick_advances_when_all_workers_acknowledge() {
        let input = ControllerTickInput {
            plan: plan(SchemaJobState::DeleteOnly),
            target_state: SchemaJobState::WriteOnly,
            expected_workers: vec!["worker-a".to_string(), "worker-b".to_string()],
            leases: vec![
                lease(
                    "worker-a",
                    SchemaJobState::DeleteOnly,
                    "2026-05-22T14:00:00Z",
                ),
                lease(
                    "worker-b",
                    SchemaJobState::DeleteOnly,
                    "2026-05-22T14:00:00Z",
                ),
            ],
            now: "2026-05-22T13:50:00Z".to_string(),
            started_at: "2026-05-22T13:49:30Z".to_string(),
            gate: TransitionGate::WaitForever,
        };
        let report = tick(&input).unwrap();
        assert!(matches!(
            report.decision,
            ControllerTickDecision::Advance(_)
        ));
        assert!(report
            .sql_statements
            .iter()
            .any(|stmt| stmt.contains("schema_job_advance")));
        assert!(report
            .sql_statements
            .iter()
            .any(|stmt| stmt.contains("schema_job_phase_log_insert")));
        assert!(report
            .two_version_invariant_check_sql
            .contains("verify_two_version_invariant"));
    }

    #[test]
    fn tick_waits_when_worker_missing() {
        let input = ControllerTickInput {
            plan: plan(SchemaJobState::DeleteOnly),
            target_state: SchemaJobState::WriteOnly,
            expected_workers: vec!["worker-a".to_string(), "worker-b".to_string()],
            leases: vec![lease(
                "worker-a",
                SchemaJobState::DeleteOnly,
                "2026-05-22T14:00:00Z",
            )],
            now: "2026-05-22T13:50:00Z".to_string(),
            started_at: "2026-05-22T13:49:30Z".to_string(),
            gate: TransitionGate::WaitForever,
        };
        let report = tick(&input).unwrap();
        match &report.decision {
            ControllerTickDecision::Wait { delinquent_workers } => {
                assert_eq!(delinquent_workers, &vec!["worker-b".to_string()]);
            }
            other => panic!("expected Wait, got {other:?}"),
        }
        assert!(report.sql_statements.is_empty());
    }

    #[test]
    fn tick_rolls_back_on_expiry() {
        let input = ControllerTickInput {
            plan: plan(SchemaJobState::Backfill),
            target_state: SchemaJobState::Public,
            expected_workers: vec!["worker-a".to_string()],
            leases: vec![lease(
                "worker-a",
                SchemaJobState::Backfill,
                "2026-05-22T13:00:00Z",
            )],
            now: "2026-05-22T15:00:00Z".to_string(),
            started_at: "2026-05-22T14:30:00Z".to_string(),
            gate: TransitionGate::RollbackOnTimeout,
        };
        let report = tick(&input).unwrap();
        match &report.decision {
            ControllerTickDecision::Rollback {
                expired_workers,
                rollback,
            } => {
                assert_eq!(expired_workers, &vec!["worker-a".to_string()]);
                assert!(!rollback.steps.is_empty());
            }
            other => panic!("expected Rollback, got {other:?}"),
        }
        assert!(report
            .sql_statements
            .iter()
            .any(|stmt| stmt.contains("schema_job_rollback_to")));
    }

    #[test]
    fn duplicate_worker_rejected() {
        let input = ControllerTickInput {
            plan: plan(SchemaJobState::DeleteOnly),
            target_state: SchemaJobState::WriteOnly,
            expected_workers: vec!["worker-a".to_string(), "worker-a".to_string()],
            leases: vec![],
            now: "2026-05-22T13:50:00Z".to_string(),
            started_at: "2026-05-22T13:49:30Z".to_string(),
            gate: TransitionGate::WaitForever,
        };
        assert_eq!(tick(&input), Err(ControllerError::DuplicateWorker));
    }

    #[test]
    fn snapshot_reports_missing_workers() {
        let input = ControllerTickInput {
            plan: plan(SchemaJobState::WriteOnly),
            target_state: SchemaJobState::Backfill,
            expected_workers: vec!["worker-a".to_string(), "worker-b".to_string()],
            leases: vec![lease(
                "worker-a",
                SchemaJobState::WriteOnly,
                "2026-05-22T14:00:00Z",
            )],
            now: "2026-05-22T13:50:00Z".to_string(),
            started_at: "2026-05-22T13:49:30Z".to_string(),
            gate: TransitionGate::WaitForever,
        };
        let snapshot = worker_status_snapshot(&input);
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[0].status, WorkerLeaseStatus::Acknowledged);
        assert_eq!(snapshot[1].status, WorkerLeaseStatus::Missing);
    }
}
