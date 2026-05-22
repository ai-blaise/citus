//! gh-ost-style state machine driving Migration phases.
//!
//! Phase guards (mirror of `gh-ost` cut-over):
//!
//! | from       | to        | required evidence                                       |
//! |------------|-----------|---------------------------------------------------------|
//! | DeleteOnly | WriteOnly | shadow table built on every shard                       |
//! | WriteOnly  | Backfill  | write triggers installed on every shard                 |
//! | Backfill   | Public    | backfill complete AND row-diff verification passed      |
//! | Public     | Complete  | terminal transition (always allowed)                    |
//!
//! Any other transition is illegal and returns [`StateMachineError`].

use super::MigrationPhase;
use std::error::Error;
use std::fmt;

/// Reconciler-supplied evidence about per-shard progress.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct PhaseEvidence {
    pub shadow_table_built: bool,
    pub write_triggers_installed: bool,
    pub backfill_complete: bool,
    pub row_diff_verified: bool,
}

/// Compute the next phase given the current phase and live evidence. The
/// returned phase is the most-advanced phase the evidence permits; callers
/// invoke `transition` again on the next reconcile iteration until `Complete`
/// is reached.
pub fn transition(
    current: MigrationPhase,
    evidence: &PhaseEvidence,
) -> Result<MigrationPhase, StateMachineError> {
    match current {
        MigrationPhase::DeleteOnly => {
            if evidence.shadow_table_built {
                Ok(MigrationPhase::WriteOnly)
            } else {
                Ok(MigrationPhase::DeleteOnly)
            }
        }
        MigrationPhase::WriteOnly => {
            if !evidence.shadow_table_built {
                Err(StateMachineError::EvidenceRegressed("shadow_table_built"))
            } else if evidence.write_triggers_installed {
                Ok(MigrationPhase::Backfill)
            } else {
                Ok(MigrationPhase::WriteOnly)
            }
        }
        MigrationPhase::Backfill => {
            if !evidence.shadow_table_built {
                Err(StateMachineError::EvidenceRegressed("shadow_table_built"))
            } else if !evidence.write_triggers_installed {
                Err(StateMachineError::EvidenceRegressed(
                    "write_triggers_installed",
                ))
            } else if evidence.backfill_complete && evidence.row_diff_verified {
                Ok(MigrationPhase::Public)
            } else {
                Ok(MigrationPhase::Backfill)
            }
        }
        MigrationPhase::Public => Ok(MigrationPhase::Complete),
        MigrationPhase::Complete => Ok(MigrationPhase::Complete),
    }
}

/// State-machine errors.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum StateMachineError {
    /// Evidence previously asserted at an earlier phase is now missing.
    EvidenceRegressed(&'static str),
}

impl fmt::Display for StateMachineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EvidenceRegressed(field) => {
                write!(
                    formatter,
                    "evidence regressed at {field}; cannot continue advancing"
                )
            }
        }
    }
}

impl Error for StateMachineError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delete_only_waits_for_shadow_table() {
        let evidence = PhaseEvidence::default();
        assert_eq!(
            transition(MigrationPhase::DeleteOnly, &evidence),
            Ok(MigrationPhase::DeleteOnly)
        );
    }

    #[test]
    fn delete_only_advances_to_write_only_when_shadow_table_built() {
        let evidence = PhaseEvidence {
            shadow_table_built: true,
            ..Default::default()
        };
        assert_eq!(
            transition(MigrationPhase::DeleteOnly, &evidence),
            Ok(MigrationPhase::WriteOnly)
        );
    }

    #[test]
    fn write_only_advances_when_triggers_installed() {
        let evidence = PhaseEvidence {
            shadow_table_built: true,
            write_triggers_installed: true,
            ..Default::default()
        };
        assert_eq!(
            transition(MigrationPhase::WriteOnly, &evidence),
            Ok(MigrationPhase::Backfill)
        );
    }

    #[test]
    fn backfill_requires_both_completion_and_row_diff() {
        let mut evidence = PhaseEvidence {
            shadow_table_built: true,
            write_triggers_installed: true,
            backfill_complete: true,
            ..Default::default()
        };
        assert_eq!(
            transition(MigrationPhase::Backfill, &evidence),
            Ok(MigrationPhase::Backfill),
            "row diff not verified — must not advance"
        );
        evidence.row_diff_verified = true;
        assert_eq!(
            transition(MigrationPhase::Backfill, &evidence),
            Ok(MigrationPhase::Public)
        );
    }

    #[test]
    fn public_advances_to_complete() {
        assert_eq!(
            transition(MigrationPhase::Public, &PhaseEvidence::default()),
            Ok(MigrationPhase::Complete)
        );
    }

    #[test]
    fn backfill_rejects_regressed_evidence() {
        let evidence = PhaseEvidence {
            shadow_table_built: true,
            write_triggers_installed: false,
            backfill_complete: true,
            row_diff_verified: true,
        };
        assert_eq!(
            transition(MigrationPhase::Backfill, &evidence),
            Err(StateMachineError::EvidenceRegressed(
                "write_triggers_installed"
            ))
        );
    }
}
