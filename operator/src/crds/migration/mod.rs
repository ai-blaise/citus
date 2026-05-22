// FEATURE: C9
// FEATURE: M3

pub mod state_machine;

use std::error::Error;
use std::fmt;

pub use state_machine::{transition, PhaseEvidence, StateMachineError};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MigrationSpec {
    pub migration_type: MigrationType,
    pub yaml: String,
    pub on_conflict: MigrationConflictAction,
}

impl MigrationSpec {
    pub fn validate(&self) -> Result<(), MigrationSpecError> {
        validate_required("yaml", &self.yaml)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MigrationType {
    Pgroll,
    GhOst,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MigrationConflictAction {
    Fail,
    Skip,
    Replace,
    ManualReview,
}

/// gh-ost-style life-cycle phases driven by [`state_machine::transition`].
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum MigrationPhase {
    DeleteOnly,
    WriteOnly,
    Backfill,
    Public,
    Complete,
}

impl MigrationPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DeleteOnly => "DeleteOnly",
            Self::WriteOnly => "WriteOnly",
            Self::Backfill => "Backfill",
            Self::Public => "Public",
            Self::Complete => "Complete",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MigrationSpecError {
    MissingRequiredField(&'static str),
}

impl fmt::Display for MigrationSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for MigrationSpecError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), MigrationSpecError> {
    if value.trim().is_empty() {
        return Err(MigrationSpecError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_pgroll_migration_passes() {
        let spec = MigrationSpec {
            migration_type: MigrationType::Pgroll,
            yaml: "operations:\n  - add_column:\n      table: users".to_string(),
            on_conflict: MigrationConflictAction::ManualReview,
        };

        assert_eq!(spec.validate(), Ok(()));
    }

    #[test]
    fn migration_rejects_empty_yaml() {
        let spec = MigrationSpec {
            migration_type: MigrationType::GhOst,
            yaml: String::new(),
            on_conflict: MigrationConflictAction::Fail,
        };

        assert_eq!(
            spec.validate(),
            Err(MigrationSpecError::MissingRequiredField("yaml"))
        );
    }

    #[test]
    fn migration_phase_as_str_round_trip() {
        for phase in [
            MigrationPhase::DeleteOnly,
            MigrationPhase::WriteOnly,
            MigrationPhase::Backfill,
            MigrationPhase::Public,
            MigrationPhase::Complete,
        ] {
            assert!(!phase.as_str().is_empty());
        }
    }
}
