//! Canonical report for companion runtime depth A.

use crate::{
    canonical_migration_runtime_report, canonical_queue_runtime_report,
    canonical_replication_conflict_report, MigrationError, QueueRuntimeError,
    ReplicationConflictError,
};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CompanionRuntimeDepthAReport {
    pub feature_ids: Vec<&'static str>,
    pub migration_phases: usize,
    pub migration_sql_batches: usize,
    pub migration_commands: usize,
    pub queue_commands: usize,
    pub queue_leased_messages: usize,
    pub queue_dead_lettered_messages: usize,
    pub conflict_classes: usize,
    pub conflict_resolutions: usize,
    pub conflict_rejections: usize,
    pub fail_closed_guards: usize,
}

pub fn canonical_companion_runtime_depth_a_report(
) -> Result<CompanionRuntimeDepthAReport, CompanionRuntimeDepthAError> {
    let migration = canonical_migration_runtime_report()?;
    let queue = canonical_queue_runtime_report()?;
    let conflicts = canonical_replication_conflict_report()?;

    Ok(CompanionRuntimeDepthAReport {
        feature_ids: vec!["M1", "M11", "R6", "C4", "C5"],
        migration_phases: migration.phase_count,
        migration_sql_batches: migration.sql_batch_count,
        migration_commands: migration.command_count,
        queue_commands: queue.command_count,
        queue_leased_messages: queue.leased_messages,
        queue_dead_lettered_messages: queue.dead_lettered_messages,
        conflict_classes: conflicts.class_count,
        conflict_resolutions: conflicts.resolution_count,
        conflict_rejections: conflicts.rejected_count,
        fail_closed_guards: migration.safety_guard_count
            + queue.safety_guard_count
            + conflicts.fail_closed_guard_count,
    })
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CompanionRuntimeDepthAError {
    Migration(String),
    Queue(String),
    ReplicationConflict(String),
}

impl fmt::Display for CompanionRuntimeDepthAError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Migration(error) => write!(formatter, "migration runtime report failed: {error}"),
            Self::Queue(error) => write!(formatter, "queue runtime report failed: {error}"),
            Self::ReplicationConflict(error) => {
                write!(formatter, "replication conflict report failed: {error}")
            }
        }
    }
}

impl Error for CompanionRuntimeDepthAError {}

impl From<MigrationError> for CompanionRuntimeDepthAError {
    fn from(error: MigrationError) -> Self {
        Self::Migration(error.to_string())
    }
}

impl From<QueueRuntimeError> for CompanionRuntimeDepthAError {
    fn from(error: QueueRuntimeError) -> Self {
        Self::Queue(error.to_string())
    }
}

impl From<ReplicationConflictError> for CompanionRuntimeDepthAError {
    fn from(error: ReplicationConflictError) -> Self {
        Self::ReplicationConflict(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_report_covers_depth_a_runtime_surfaces() {
        let report = canonical_companion_runtime_depth_a_report().expect("report");

        assert_eq!(report.feature_ids, vec!["M1", "M11", "R6", "C4", "C5"]);
        assert_eq!(report.migration_phases, 6);
        assert_eq!(report.migration_sql_batches, 4);
        assert_eq!(report.queue_dead_lettered_messages, 1);
        assert_eq!(report.conflict_classes, 7);
        assert_eq!(report.conflict_resolutions, 7);
        assert_eq!(report.conflict_rejections, 2);
        assert_eq!(report.fail_closed_guards, 14);
    }
}
