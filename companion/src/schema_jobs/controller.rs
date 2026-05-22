// FEATURE: C10
// FEATURE: M2
// FEATURE: M14

//! F1 controller: phase-transition pre-conditions, checkpoint records, and
//! the SQL renderer for `companion.schema_job_phase_log`.
//!
//! The controller is purely deterministic. It does not block on the network
//! or hold a tokio runtime — instead it consumes a snapshot of worker
//! acknowledgements and emits a [`PhaseTransitionDecision`]. The sidecar
//! daemon owns the I/O, polls the controller every tick, and applies the
//! resulting SQL plan against the coordinator.

use super::worker_lease::{WorkerLeaseRegistry, WorkerLeaseStatus};
use super::{SchemaJobError, SchemaJobPlan, SchemaJobState, COMPANION_INTERNAL_SCHEMA};
use std::error::Error;
use std::fmt;

/// Recorded acknowledgement that the controller has observed for one worker.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PhaseAcknowledgement {
    pub worker_id: String,
    pub status: WorkerLeaseStatus,
}

/// Decision returned by the controller after one tick.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PhaseTransitionDecision {
    /// All preconditions met — apply this checkpoint and advance the SQL
    /// state machine.
    Advance(PhaseCheckpoint),
    /// At least one worker has not yet acknowledged the current phase. The
    /// controller should keep waiting; the sidecar can re-poll after the
    /// gate's configured interval.
    WaitForAcknowledgement {
        delinquent_workers: Vec<String>,
        gate: TransitionGate,
    },
    /// At least one worker timed out. The controller chooses Wait, Skip, or
    /// Rollback according to the configured gate.
    AbortOnTimeout {
        expired_workers: Vec<String>,
        gate: TransitionGate,
    },
}

/// Phase-transition gate configuration. Mirrors the F1 paper's notion of
/// waiting for all replicas to acknowledge a schema version before advancing,
/// with three operational escape hatches.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TransitionGate {
    /// Wait indefinitely. Default behavior — matches strict 2VI.
    WaitForever,
    /// Allow the controller to ignore workers that fail to acknowledge in
    /// time. Used when a worker is permanently lost.
    SkipMissing,
    /// Trigger an automatic rollback if any worker times out. Used during
    /// canary phases.
    RollbackOnTimeout,
}

impl TransitionGate {
    pub fn as_canonical(&self) -> &'static str {
        match self {
            Self::WaitForever => "wait_forever",
            Self::SkipMissing => "skip_missing",
            Self::RollbackOnTimeout => "rollback_on_timeout",
        }
    }
}

/// Checkpoint record persisted to `companion.schema_job_phase_log` whenever a
/// phase boundary lands.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PhaseCheckpoint {
    pub job_name: String,
    pub from_state: SchemaJobState,
    pub to_state: SchemaJobState,
    pub started_at: String,
    pub completed_at: String,
    pub workers_acknowledged: Vec<String>,
    pub gate: TransitionGate,
}

impl PhaseCheckpoint {
    /// Render the SQL command that records this checkpoint. The companion
    /// SQL extension hosts the matching function definition.
    pub fn to_sql(&self) -> String {
        let workers_array = format!(
            "ARRAY[{}]::text[]",
            self.workers_acknowledged
                .iter()
                .map(|worker| sql_literal(worker))
                .collect::<Vec<_>>()
                .join(", ")
        );
        format!(
            "SELECT {schema}.schema_job_phase_log_insert({job}, {from}, {to}, {started}, {completed}, {workers}, {gate});",
            schema = COMPANION_INTERNAL_SCHEMA,
            job = sql_literal(&self.job_name),
            from = sql_literal(self.from_state.as_canonical()),
            to = sql_literal(self.to_state.as_canonical()),
            started = sql_literal(&self.started_at),
            completed = sql_literal(&self.completed_at),
            workers = workers_array,
            gate = sql_literal(self.gate.as_canonical())
        )
    }
}

/// Input to one controller tick.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PhaseTransitionPlan {
    pub plan: SchemaJobPlan,
    pub target_state: SchemaJobState,
    pub now: String,
    pub started_at: String,
    pub expected_workers: Vec<String>,
    pub registry: WorkerLeaseRegistry,
    pub gate: TransitionGate,
}

/// Pure F1-style transition controller. Construct with `new`, call
/// [`SchemaJobController::transition`] to evaluate one tick.
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct SchemaJobController;

impl SchemaJobController {
    pub fn new() -> Self {
        Self
    }

    /// Evaluate a forward-phase transition. Returns
    /// [`PhaseTransitionDecision::Advance`] if every expected worker has
    /// acknowledged the current phase under the configured gate.
    pub fn transition(
        &self,
        request: &PhaseTransitionPlan,
    ) -> Result<PhaseTransitionDecision, SchemaJobControllerError> {
        request.plan.validate()?;
        if request.expected_workers.is_empty() {
            return Err(SchemaJobControllerError::NoExpectedWorkers);
        }
        if !request.plan.can_advance_to(request.target_state) {
            return Err(SchemaJobControllerError::IllegalTransition {
                from: request.plan.state,
                to: request.target_state,
            });
        }

        let expected_phase = request.plan.state;
        if request.registry.expected_phase() != Some(expected_phase) {
            return Err(SchemaJobControllerError::RegistryPhaseMismatch {
                expected: expected_phase,
                observed: request.registry.expected_phase(),
            });
        }

        let workers: Vec<&str> = request
            .expected_workers
            .iter()
            .map(String::as_str)
            .collect();

        let mut expired = Vec::new();
        let mut delinquent = Vec::new();
        let mut acknowledged = Vec::new();
        for worker_id in &workers {
            match request.registry.status_for(worker_id, &request.now) {
                WorkerLeaseStatus::Acknowledged => acknowledged.push((*worker_id).to_string()),
                WorkerLeaseStatus::Expired => expired.push((*worker_id).to_string()),
                WorkerLeaseStatus::StalePhase { .. } | WorkerLeaseStatus::Missing => {
                    delinquent.push((*worker_id).to_string())
                }
            }
        }

        if !expired.is_empty() {
            match request.gate {
                TransitionGate::SkipMissing => {
                    // Continue: treat expired workers as if they had
                    // acknowledged.
                }
                TransitionGate::WaitForever => {
                    return Ok(PhaseTransitionDecision::WaitForAcknowledgement {
                        delinquent_workers: expired,
                        gate: request.gate,
                    });
                }
                TransitionGate::RollbackOnTimeout => {
                    return Ok(PhaseTransitionDecision::AbortOnTimeout {
                        expired_workers: expired,
                        gate: request.gate,
                    });
                }
            }
        }

        if !delinquent.is_empty() {
            match request.gate {
                TransitionGate::SkipMissing => {
                    // Continue: drop the delinquent workers from the
                    // acknowledged-set but proceed.
                }
                TransitionGate::WaitForever => {
                    return Ok(PhaseTransitionDecision::WaitForAcknowledgement {
                        delinquent_workers: delinquent,
                        gate: request.gate,
                    });
                }
                TransitionGate::RollbackOnTimeout => {
                    return Ok(PhaseTransitionDecision::AbortOnTimeout {
                        expired_workers: delinquent,
                        gate: request.gate,
                    });
                }
            }
        }

        let checkpoint = PhaseCheckpoint {
            job_name: request.plan.name.clone(),
            from_state: request.plan.state,
            to_state: request.target_state,
            started_at: request.started_at.clone(),
            completed_at: request.now.clone(),
            workers_acknowledged: acknowledged,
            gate: request.gate,
        };

        Ok(PhaseTransitionDecision::Advance(checkpoint))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SchemaJobControllerError {
    PlanInvalid(SchemaJobError),
    IllegalTransition {
        from: SchemaJobState,
        to: SchemaJobState,
    },
    RegistryPhaseMismatch {
        expected: SchemaJobState,
        observed: Option<SchemaJobState>,
    },
    NoExpectedWorkers,
}

impl fmt::Display for SchemaJobControllerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlanInvalid(error) => write!(formatter, "schema job plan invalid: {error}"),
            Self::IllegalTransition { from, to } => write!(
                formatter,
                "illegal phase transition: {} -> {}",
                from.as_canonical(),
                to.as_canonical()
            ),
            Self::RegistryPhaseMismatch { expected, observed } => write!(
                formatter,
                "registry expected_phase {observed:?} does not match plan state {expected:?}"
            ),
            Self::NoExpectedWorkers => {
                write!(
                    formatter,
                    "controller requires a non-empty expected_workers list"
                )
            }
        }
    }
}

impl Error for SchemaJobControllerError {}

impl From<SchemaJobError> for SchemaJobControllerError {
    fn from(error: SchemaJobError) -> Self {
        Self::PlanInvalid(error)
    }
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::super::worker_lease::WorkerLease;
    use super::super::SchemaJobOperation;
    use super::*;

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
            .expect("valid lease")
    }

    #[test]
    fn advance_emits_checkpoint_when_all_workers_acknowledge() {
        let mut registry = WorkerLeaseRegistry::new("users-display-name").expect("registry");
        registry.expect_phase(SchemaJobState::DeleteOnly);
        registry
            .record(lease(
                "worker-a",
                SchemaJobState::DeleteOnly,
                "2026-05-22T14:00:00Z",
            ))
            .unwrap();
        registry
            .record(lease(
                "worker-b",
                SchemaJobState::DeleteOnly,
                "2026-05-22T14:00:00Z",
            ))
            .unwrap();
        let request = PhaseTransitionPlan {
            plan: plan(SchemaJobState::DeleteOnly),
            target_state: SchemaJobState::WriteOnly,
            now: "2026-05-22T13:50:00Z".to_string(),
            started_at: "2026-05-22T13:49:30Z".to_string(),
            expected_workers: vec!["worker-a".to_string(), "worker-b".to_string()],
            registry,
            gate: TransitionGate::WaitForever,
        };
        let decision = SchemaJobController::new().transition(&request).unwrap();
        match decision {
            PhaseTransitionDecision::Advance(checkpoint) => {
                assert_eq!(checkpoint.from_state, SchemaJobState::DeleteOnly);
                assert_eq!(checkpoint.to_state, SchemaJobState::WriteOnly);
                assert_eq!(
                    checkpoint.workers_acknowledged,
                    vec!["worker-a".to_string(), "worker-b".to_string()]
                );
                assert!(checkpoint.to_sql().contains("schema_job_phase_log_insert"));
                assert!(checkpoint.to_sql().contains("'worker-a'"));
            }
            other => panic!("expected Advance, got {other:?}"),
        }
    }

    #[test]
    fn wait_when_one_worker_is_missing() {
        let mut registry = WorkerLeaseRegistry::new("users-display-name").expect("registry");
        registry.expect_phase(SchemaJobState::DeleteOnly);
        registry
            .record(lease(
                "worker-a",
                SchemaJobState::DeleteOnly,
                "2026-05-22T14:00:00Z",
            ))
            .unwrap();
        let request = PhaseTransitionPlan {
            plan: plan(SchemaJobState::DeleteOnly),
            target_state: SchemaJobState::WriteOnly,
            now: "2026-05-22T13:50:00Z".to_string(),
            started_at: "2026-05-22T13:49:30Z".to_string(),
            expected_workers: vec!["worker-a".to_string(), "worker-b".to_string()],
            registry,
            gate: TransitionGate::WaitForever,
        };
        let decision = SchemaJobController::new().transition(&request).unwrap();
        assert_eq!(
            decision,
            PhaseTransitionDecision::WaitForAcknowledgement {
                delinquent_workers: vec!["worker-b".to_string()],
                gate: TransitionGate::WaitForever,
            }
        );
    }

    #[test]
    fn skip_missing_lets_controller_advance() {
        let mut registry = WorkerLeaseRegistry::new("users-display-name").expect("registry");
        registry.expect_phase(SchemaJobState::DeleteOnly);
        registry
            .record(lease(
                "worker-a",
                SchemaJobState::DeleteOnly,
                "2026-05-22T14:00:00Z",
            ))
            .unwrap();
        let request = PhaseTransitionPlan {
            plan: plan(SchemaJobState::DeleteOnly),
            target_state: SchemaJobState::WriteOnly,
            now: "2026-05-22T13:50:00Z".to_string(),
            started_at: "2026-05-22T13:49:30Z".to_string(),
            expected_workers: vec!["worker-a".to_string(), "worker-b".to_string()],
            registry,
            gate: TransitionGate::SkipMissing,
        };
        let decision = SchemaJobController::new().transition(&request).unwrap();
        assert!(matches!(decision, PhaseTransitionDecision::Advance(_)));
    }

    #[test]
    fn rollback_on_timeout_aborts_when_lease_expired() {
        let mut registry = WorkerLeaseRegistry::new("users-display-name").expect("registry");
        registry.expect_phase(SchemaJobState::WriteOnly);
        registry
            .record(lease(
                "worker-a",
                SchemaJobState::WriteOnly,
                "2026-05-22T13:00:00Z",
            ))
            .unwrap();
        let request = PhaseTransitionPlan {
            plan: plan(SchemaJobState::WriteOnly),
            target_state: SchemaJobState::Backfill,
            now: "2026-05-22T14:30:00Z".to_string(),
            started_at: "2026-05-22T14:29:00Z".to_string(),
            expected_workers: vec!["worker-a".to_string()],
            registry,
            gate: TransitionGate::RollbackOnTimeout,
        };
        let decision = SchemaJobController::new().transition(&request).unwrap();
        match decision {
            PhaseTransitionDecision::AbortOnTimeout {
                expired_workers,
                gate,
            } => {
                assert_eq!(expired_workers, vec!["worker-a".to_string()]);
                assert_eq!(gate, TransitionGate::RollbackOnTimeout);
            }
            other => panic!("expected AbortOnTimeout, got {other:?}"),
        }
    }

    #[test]
    fn illegal_forward_transition_rejected() {
        let mut registry = WorkerLeaseRegistry::new("users-display-name").expect("registry");
        registry.expect_phase(SchemaJobState::DeleteOnly);
        registry
            .record(lease(
                "worker-a",
                SchemaJobState::DeleteOnly,
                "2026-05-22T14:00:00Z",
            ))
            .unwrap();
        let request = PhaseTransitionPlan {
            plan: plan(SchemaJobState::DeleteOnly),
            target_state: SchemaJobState::Public,
            now: "2026-05-22T13:50:00Z".to_string(),
            started_at: "2026-05-22T13:49:30Z".to_string(),
            expected_workers: vec!["worker-a".to_string()],
            registry,
            gate: TransitionGate::WaitForever,
        };
        let result = SchemaJobController::new().transition(&request);
        assert_eq!(
            result,
            Err(SchemaJobControllerError::IllegalTransition {
                from: SchemaJobState::DeleteOnly,
                to: SchemaJobState::Public,
            })
        );
    }

    #[test]
    fn checkpoint_sql_includes_phase_and_gate() {
        let checkpoint = PhaseCheckpoint {
            job_name: "users-display-name".to_string(),
            from_state: SchemaJobState::WriteOnly,
            to_state: SchemaJobState::Backfill,
            started_at: "2026-05-22T13:00:00Z".to_string(),
            completed_at: "2026-05-22T13:01:00Z".to_string(),
            workers_acknowledged: vec!["worker-a".to_string()],
            gate: TransitionGate::WaitForever,
        };
        let sql = checkpoint.to_sql();
        assert!(sql.contains("'write_only'"));
        assert!(sql.contains("'backfill'"));
        assert!(sql.contains("'wait_forever'"));
        assert!(sql.contains("schema_job_phase_log_insert"));
    }
}
