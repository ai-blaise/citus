//! Schema-job sidecar contracts.

// FEATURE: C10
// FEATURE: M2

use ai_blaise_citus_companion::{SchemaJobError, SchemaJobPlan, SchemaJobState};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SchemaJobWorkerPlan {
    pub job: SchemaJobPlan,
    pub worker_id: String,
    pub lease: SchemaJobLease,
    pub backfill: BackfillPlan,
    pub safety: OnlineDdlSafetyPlan,
    pub shadow: Option<GhOstShadowPlan>,
}

impl SchemaJobWorkerPlan {
    pub fn validate(&self) -> Result<(), SchemaJobSidecarError> {
        self.job.validate()?;
        validate_required("worker_id", &self.worker_id)?;
        self.lease.validate()?;
        self.backfill.validate()?;
        self.safety.validate()?;
        if let Some(shadow) = &self.shadow {
            shadow.validate()?;
        }
        Ok(())
    }

    pub fn next_action(&self) -> Result<SchemaJobAction, SchemaJobSidecarError> {
        self.validate()?;
        if self.lease.holder != self.worker_id {
            return Ok(SchemaJobAction::AcquireLease);
        }

        Ok(match self.job.state {
            SchemaJobState::DeleteOnly => SchemaJobAction::ApplyDeleteOnly,
            SchemaJobState::WriteOnly => SchemaJobAction::ApplyWriteOnly,
            SchemaJobState::Backfill => SchemaJobAction::RunBackfill {
                batch_size: self.backfill.batch_size,
                max_parallel_shards: self.backfill.max_parallel_shards,
            },
            SchemaJobState::Public => SchemaJobAction::Publish,
            SchemaJobState::Paused => SchemaJobAction::StopPaused,
            SchemaJobState::Canceled => SchemaJobAction::StopCanceled,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SchemaJobLease {
    pub holder: String,
    pub epoch: u64,
    pub expires_at: String,
}

impl SchemaJobLease {
    fn validate(&self) -> Result<(), SchemaJobSidecarError> {
        validate_required("lease.holder", &self.holder)?;
        if self.epoch == 0 {
            return Err(SchemaJobSidecarError::InvalidLeaseEpoch);
        }
        validate_timestamp("lease.expires_at", &self.expires_at)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackfillPlan {
    pub batch_size: u32,
    pub max_parallel_shards: u32,
    pub throttle_ms: u32,
}

impl BackfillPlan {
    fn validate(&self) -> Result<(), SchemaJobSidecarError> {
        if self.batch_size == 0 {
            return Err(SchemaJobSidecarError::InvalidBatchSize);
        }
        if self.max_parallel_shards == 0 {
            return Err(SchemaJobSidecarError::InvalidParallelism);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OnlineDdlSafetyPlan {
    pub max_replication_lag_bytes: u64,
    pub max_lock_ms: u32,
    pub allow_blocking_cutover: bool,
}

impl OnlineDdlSafetyPlan {
    fn validate(&self) -> Result<(), SchemaJobSidecarError> {
        if self.max_replication_lag_bytes == 0 {
            return Err(SchemaJobSidecarError::InvalidReplicationLagBudget);
        }
        if self.max_lock_ms == 0 {
            return Err(SchemaJobSidecarError::InvalidLockTimeout);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GhOstShadowPlan {
    pub source_table: String,
    pub shadow_table: String,
    pub changelog_table: String,
    pub cutover_lock_timeout_ms: u32,
}

impl GhOstShadowPlan {
    fn validate(&self) -> Result<(), SchemaJobSidecarError> {
        validate_qualified_name("shadow.source_table", &self.source_table)?;
        validate_qualified_name("shadow.shadow_table", &self.shadow_table)?;
        validate_qualified_name("shadow.changelog_table", &self.changelog_table)?;
        if self.cutover_lock_timeout_ms == 0 {
            return Err(SchemaJobSidecarError::InvalidLockTimeout);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SchemaJobAction {
    AcquireLease,
    ApplyDeleteOnly,
    ApplyWriteOnly,
    RunBackfill {
        batch_size: u32,
        max_parallel_shards: u32,
    },
    Publish,
    StopPaused,
    StopCanceled,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SchemaJobSidecarError {
    Companion(String),
    InvalidBatchSize,
    InvalidIdentifier(&'static str),
    InvalidLeaseEpoch,
    InvalidLockTimeout,
    InvalidParallelism,
    InvalidReplicationLagBudget,
    InvalidTimestamp(&'static str),
    MissingRequiredField(&'static str),
}

impl fmt::Display for SchemaJobSidecarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Companion(error) => write!(formatter, "{error}"),
            Self::InvalidBatchSize => write!(formatter, "batch_size must be greater than zero"),
            Self::InvalidIdentifier(field) => write!(formatter, "{field} must be a SQL identifier"),
            Self::InvalidLeaseEpoch => write!(formatter, "lease epoch must be greater than zero"),
            Self::InvalidLockTimeout => write!(formatter, "lock timeout must be greater than zero"),
            Self::InvalidParallelism => {
                write!(formatter, "max_parallel_shards must be greater than zero")
            }
            Self::InvalidReplicationLagBudget => {
                write!(
                    formatter,
                    "replication lag budget must be greater than zero"
                )
            }
            Self::InvalidTimestamp(field) => {
                write!(formatter, "{field} must be an RFC3339 UTC timestamp")
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
        }
    }
}

impl Error for SchemaJobSidecarError {}

impl From<SchemaJobError> for SchemaJobSidecarError {
    fn from(error: SchemaJobError) -> Self {
        Self::Companion(error.to_string())
    }
}

fn validate_required(field: &'static str, value: &str) -> Result<(), SchemaJobSidecarError> {
    if value.trim().is_empty() {
        return Err(SchemaJobSidecarError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_timestamp(field: &'static str, value: &str) -> Result<(), SchemaJobSidecarError> {
    validate_required(field, value)?;
    if value.len() >= 20 && value.contains('T') && value.ends_with('Z') {
        Ok(())
    } else {
        Err(SchemaJobSidecarError::InvalidTimestamp(field))
    }
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), SchemaJobSidecarError> {
    validate_required(field, value)?;
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        Ok(())
    } else {
        Err(SchemaJobSidecarError::InvalidIdentifier(field))
    }
}

fn validate_qualified_name(field: &'static str, value: &str) -> Result<(), SchemaJobSidecarError> {
    validate_required(field, value)?;
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() == 2
        && parts
            .iter()
            .all(|part| validate_identifier(field, part).is_ok())
    {
        Ok(())
    } else {
        Err(SchemaJobSidecarError::InvalidIdentifier(field))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SchemaJobCanonicalReport {
    pub worker: SchemaJobWorkerPlan,
    pub action: SchemaJobAction,
}

pub fn canonical_schema_job_worker_plan() -> SchemaJobWorkerPlan {
    use ai_blaise_citus_companion::SchemaJobOperation;

    SchemaJobWorkerPlan {
        job: SchemaJobPlan {
            name: "users-add-display-name".to_string(),
            table: "public.users".to_string(),
            state: SchemaJobState::DeleteOnly,
            operations: vec![SchemaJobOperation::AddColumn {
                column: "display_name".to_string(),
                sql_type: "text".to_string(),
            }],
            lease_seconds: 30,
        },
        worker_id: "schema-worker-a".to_string(),
        lease: SchemaJobLease {
            holder: "schema-worker-a".to_string(),
            epoch: 1,
            expires_at: "2026-05-19T12:00:00Z".to_string(),
        },
        backfill: BackfillPlan {
            batch_size: 1_000,
            max_parallel_shards: 4,
            throttle_ms: 50,
        },
        safety: OnlineDdlSafetyPlan {
            max_replication_lag_bytes: 16_777_216,
            max_lock_ms: 500,
            allow_blocking_cutover: false,
        },
        shadow: Some(GhOstShadowPlan {
            source_table: "public.users".to_string(),
            shadow_table: "public._users_new".to_string(),
            changelog_table: "public._users_changelog".to_string(),
            cutover_lock_timeout_ms: 500,
        }),
    }
}

pub fn canonical_schema_job_report() -> Result<SchemaJobCanonicalReport, SchemaJobSidecarError> {
    let worker = canonical_schema_job_worker_plan();
    let action = worker.next_action()?;

    Ok(SchemaJobCanonicalReport { worker, action })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_plan_maps_backfill_state_to_backfill_action() {
        let mut plan = valid_worker_plan();
        plan.job.state = SchemaJobState::Backfill;

        assert_eq!(
            plan.next_action(),
            Ok(SchemaJobAction::RunBackfill {
                batch_size: 1_000,
                max_parallel_shards: 4
            })
        );
    }

    #[test]
    fn worker_without_lease_acquires_before_work() {
        let mut plan = valid_worker_plan();
        plan.lease.holder = "other-worker".to_string();

        assert_eq!(plan.next_action(), Ok(SchemaJobAction::AcquireLease));
    }

    #[test]
    fn shadow_plan_requires_qualified_tables() {
        let shadow = GhOstShadowPlan {
            source_table: "users".to_string(),
            shadow_table: "public._users_new".to_string(),
            changelog_table: "public._users_changelog".to_string(),
            cutover_lock_timeout_ms: 500,
        };

        assert_eq!(
            shadow.validate(),
            Err(SchemaJobSidecarError::InvalidIdentifier(
                "shadow.source_table"
            ))
        );
    }

    #[test]
    fn safety_plan_requires_replication_lag_budget() {
        let mut plan = valid_worker_plan();
        plan.safety.max_replication_lag_bytes = 0;

        assert_eq!(
            plan.validate(),
            Err(SchemaJobSidecarError::InvalidReplicationLagBudget)
        );
    }

    #[test]
    fn canonical_report_is_deterministic() {
        let report = canonical_schema_job_report().expect("canonical report");

        assert_eq!(report.worker.job.name, "users-add-display-name");
        assert_eq!(report.worker.worker_id, "schema-worker-a");
        assert_eq!(report.action, SchemaJobAction::ApplyDeleteOnly);
    }

    fn valid_worker_plan() -> SchemaJobWorkerPlan {
        canonical_schema_job_worker_plan()
    }
}
