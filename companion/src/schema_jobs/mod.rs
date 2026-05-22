// FEATURE: C10
// FEATURE: M2
// FEATURE: M14
// FEATURE: M15

//! Online schema-change state machine for the F1-style two-version invariant.
//!
//! The companion crate hosts the data-plane DSL (`SchemaJobPlan`) and the
//! controller-side reasoning needed to drive a Google F1 style "Online,
//! Asynchronous Schema Change" rollout. The runtime keeps at most two schema
//! versions in flight across the cluster at any moment ("two-version
//! invariant", 2VI) by walking each migration through the canonical
//! phases:
//!
//! ```text
//! Absent -> DeleteOnly -> WriteOnly -> Backfill -> Public -> Absent'
//! ```
//!
//! A migration is reversible until it crosses the `Backfill -> Public` cutover
//! commit, which is treated as the F1 reorganization point. The controller
//! exposes:
//!
//! * `SchemaJobController::transition` to drive the next phase only when every
//!   live Citus worker has acknowledged the current one.
//! * `SchemaJobController::rollback` to walk back through prior phases when a
//!   transition pre-condition or worker drift trips the invariant.
//! * `verify_two_version_invariant_sql` to render the SQL probe that the
//!   continuous monitor uses to assert at most two schema versions exist.
//!
//! The submodules layer on the matching primitives:
//!
//! * [`worker_lease`] — per-worker schema-version lease records and TTL math.
//! * [`controller`] — the F1 controller loop that the sidecar embeds.
//! * [`rollback`] — phase reversal helper and partial-backfill cleanup planner.

pub mod controller;
pub mod rollback;
pub mod worker_lease;

pub use controller::{
    PhaseAcknowledgement, PhaseCheckpoint, PhaseTransitionDecision, PhaseTransitionPlan,
    SchemaJobController, SchemaJobControllerError, TransitionGate,
};
pub use rollback::{RollbackPlan, RollbackStep};
pub use worker_lease::{
    WorkerLease, WorkerLeaseError, WorkerLeaseRegistry, WorkerLeaseSqlPlan, WorkerLeaseStatus,
};

use std::error::Error;
use std::fmt;

/// Canonical pgrx schema namespace used by the F1 controller surface.
pub const COMPANION_INTERNAL_SCHEMA: &str = "companion_internal";

/// Maximum number of distinct schema versions that may co-exist according to
/// the F1 two-version invariant.
pub const TWO_VERSION_INVARIANT_MAX_VERSIONS: u32 = 2;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SchemaJobPlan {
    pub name: String,
    pub table: String,
    pub state: SchemaJobState,
    pub operations: Vec<SchemaJobOperation>,
    pub lease_seconds: u32,
}

impl SchemaJobPlan {
    pub fn validate(&self) -> Result<(), SchemaJobError> {
        validate_required("name", &self.name)?;
        validate_required("table", &self.table)?;
        if self.operations.is_empty() {
            return Err(SchemaJobError::MissingRequiredField("operations"));
        }
        for operation in &self.operations {
            operation.validate()?;
        }
        if self.lease_seconds == 0 {
            return Err(SchemaJobError::InvalidLease);
        }
        Ok(())
    }

    /// True if the F1 controller is allowed to drive a forward transition
    /// from this plan's current state into `next`.
    pub fn can_advance_to(&self, next: SchemaJobState) -> bool {
        matches!(
            (self.state, next),
            (SchemaJobState::DeleteOnly, SchemaJobState::WriteOnly)
                | (SchemaJobState::WriteOnly, SchemaJobState::Backfill)
                | (SchemaJobState::Backfill, SchemaJobState::Public)
        )
    }

    /// True if the F1 controller is allowed to walk this plan back to `prev`.
    /// Public is the F1 reorganization point and intentionally one-way.
    pub fn can_rollback_to(&self, prev: SchemaJobState) -> bool {
        matches!(
            (self.state, prev),
            (SchemaJobState::WriteOnly, SchemaJobState::DeleteOnly)
                | (
                    SchemaJobState::Backfill,
                    SchemaJobState::WriteOnly | SchemaJobState::DeleteOnly,
                )
                | (
                    SchemaJobState::Paused,
                    SchemaJobState::DeleteOnly
                        | SchemaJobState::WriteOnly
                        | SchemaJobState::Backfill,
                )
        )
    }

    /// Phase invariant SQL fragment that triggers must enforce while the job
    /// holds the supplied state. The companion SQL extension installs a check
    /// against this fragment when each transition lands.
    pub fn phase_invariant_sql(&self, state: SchemaJobState) -> &'static str {
        match state {
            SchemaJobState::DeleteOnly => {
                "INSERTs that target a delete_only column must fail closed"
            }
            SchemaJobState::WriteOnly => {
                "SELECTs against a write_only column must return NULL until backfill completes"
            }
            SchemaJobState::Backfill => {
                "backfill must run in bounded batches and must not block readers"
            }
            SchemaJobState::Public => "no invariant; column is fully published",
            SchemaJobState::Paused => "no transitions allowed while paused",
            SchemaJobState::Canceled => "no transitions allowed once canceled",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum SchemaJobState {
    DeleteOnly,
    WriteOnly,
    Backfill,
    Public,
    Paused,
    Canceled,
}

impl SchemaJobState {
    /// Stable lowercase identifier used by the SQL catalog and by the F1
    /// controller decision log.
    pub fn as_canonical(&self) -> &'static str {
        match self {
            Self::DeleteOnly => "delete_only",
            Self::WriteOnly => "write_only",
            Self::Backfill => "backfill",
            Self::Public => "public",
            Self::Paused => "paused",
            Self::Canceled => "canceled",
        }
    }

    /// Parse the canonical lowercase identifier back into a state.
    pub fn from_canonical(text: &str) -> Result<Self, SchemaJobError> {
        match text {
            "delete_only" => Ok(Self::DeleteOnly),
            "write_only" => Ok(Self::WriteOnly),
            "backfill" => Ok(Self::Backfill),
            "public" => Ok(Self::Public),
            "paused" => Ok(Self::Paused),
            "canceled" => Ok(Self::Canceled),
            _ => Err(SchemaJobError::UnknownState(text.to_string())),
        }
    }

    /// True when the state is one of the F1 forward-progress phases that
    /// counts against the two-version invariant.
    pub fn counts_against_two_version_invariant(&self) -> bool {
        matches!(
            self,
            Self::DeleteOnly | Self::WriteOnly | Self::Backfill | Self::Public
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SchemaJobOperation {
    AddColumn {
        column: String,
        sql_type: String,
    },
    Backfill {
        statement: String,
    },
    SwapColumn {
        old_column: String,
        new_column: String,
    },
    DropColumn {
        column: String,
    },
}

impl SchemaJobOperation {
    fn validate(&self) -> Result<(), SchemaJobError> {
        match self {
            Self::AddColumn { column, sql_type } => {
                validate_required("operations.column", column)?;
                validate_required("operations.sql_type", sql_type)
            }
            Self::Backfill { statement } => validate_required("operations.statement", statement),
            Self::SwapColumn {
                old_column,
                new_column,
            } => {
                validate_required("operations.old_column", old_column)?;
                validate_required("operations.new_column", new_column)
            }
            Self::DropColumn { column } => validate_required("operations.column", column),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SchemaJobError {
    InvalidLease,
    MissingRequiredField(&'static str),
    UnknownState(String),
}

impl fmt::Display for SchemaJobError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLease => write!(formatter, "lease_seconds must be greater than zero"),
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
            Self::UnknownState(text) => {
                write!(formatter, "unknown schema job state: {text}")
            }
        }
    }
}

impl Error for SchemaJobError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), SchemaJobError> {
    if value.trim().is_empty() {
        return Err(SchemaJobError::MissingRequiredField(field));
    }
    Ok(())
}

/// Render the SQL call that the continuous monitor uses to assert the F1
/// two-version invariant. The returned text is intentionally a single
/// statement so the caller can `EXECUTE` it from inside a pg_cron job.
pub fn verify_two_version_invariant_sql() -> &'static str {
    "SELECT companion_internal.verify_two_version_invariant();"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_schema_job_passes() {
        let plan = SchemaJobPlan {
            name: "users-add-display-name".to_string(),
            table: "public.users".to_string(),
            state: SchemaJobState::DeleteOnly,
            operations: vec![SchemaJobOperation::AddColumn {
                column: "display_name".to_string(),
                sql_type: "text".to_string(),
            }],
            lease_seconds: 30,
        };

        assert_eq!(plan.validate(), Ok(()));
        assert!(plan.can_advance_to(SchemaJobState::WriteOnly));
        assert!(!plan.can_advance_to(SchemaJobState::Public));
    }

    #[test]
    fn schema_job_requires_operation_list() {
        let plan = SchemaJobPlan {
            name: "empty".to_string(),
            table: "public.users".to_string(),
            state: SchemaJobState::DeleteOnly,
            operations: Vec::new(),
            lease_seconds: 30,
        };

        assert_eq!(
            plan.validate(),
            Err(SchemaJobError::MissingRequiredField("operations"))
        );
    }

    #[test]
    fn swap_column_requires_new_column() {
        let plan = SchemaJobPlan {
            name: "users-swap-name".to_string(),
            table: "public.users".to_string(),
            state: SchemaJobState::WriteOnly,
            operations: vec![SchemaJobOperation::SwapColumn {
                old_column: "name".to_string(),
                new_column: String::new(),
            }],
            lease_seconds: 30,
        };

        assert_eq!(
            plan.validate(),
            Err(SchemaJobError::MissingRequiredField(
                "operations.new_column"
            ))
        );
    }

    #[test]
    fn canonical_state_round_trips() {
        for state in [
            SchemaJobState::DeleteOnly,
            SchemaJobState::WriteOnly,
            SchemaJobState::Backfill,
            SchemaJobState::Public,
            SchemaJobState::Paused,
            SchemaJobState::Canceled,
        ] {
            assert_eq!(
                SchemaJobState::from_canonical(state.as_canonical()),
                Ok(state)
            );
        }
    }

    #[test]
    fn rollback_walks_backwards_only_before_public() {
        let mut plan = SchemaJobPlan {
            name: "rollback-test".to_string(),
            table: "public.orders".to_string(),
            state: SchemaJobState::Backfill,
            operations: vec![SchemaJobOperation::AddColumn {
                column: "totals_v2".to_string(),
                sql_type: "bigint".to_string(),
            }],
            lease_seconds: 30,
        };
        assert!(plan.can_rollback_to(SchemaJobState::DeleteOnly));
        assert!(plan.can_rollback_to(SchemaJobState::WriteOnly));

        plan.state = SchemaJobState::Public;
        assert!(!plan.can_rollback_to(SchemaJobState::Backfill));
    }

    #[test]
    fn invariant_sql_is_a_single_statement() {
        let stmt = verify_two_version_invariant_sql();
        assert!(stmt.ends_with(';'));
        assert_eq!(stmt.matches(';').count(), 1);
    }

    #[test]
    fn forward_progress_states_count_against_invariant() {
        assert!(SchemaJobState::DeleteOnly.counts_against_two_version_invariant());
        assert!(SchemaJobState::Public.counts_against_two_version_invariant());
        assert!(!SchemaJobState::Paused.counts_against_two_version_invariant());
        assert!(!SchemaJobState::Canceled.counts_against_two_version_invariant());
    }
}
