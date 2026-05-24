//! Schema-job sidecar contracts.

// FEATURE: C10
// FEATURE: M2

pub mod controller;

pub use controller::{
    canonical_controller_tick_reports, tick, worker_status_snapshot, ControllerError,
    ControllerTickDecision, ControllerTickInput, ControllerTickReport, WorkerStatusSnapshot,
};

use ai_blaise_citus_companion::{
    SchemaJobError, SchemaJobOperation, SchemaJobPlan, SchemaJobState,
};
use serde::Deserialize;
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
        validate_job_apply_boundary(&self.job)?;
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
            SchemaJobState::Public => {
                if self.safety.require_data_invariants && !self.safety.data_invariants_verified {
                    return Err(SchemaJobSidecarError::DataInvariantsNotVerified {
                        job_name: self.job.name.clone(),
                    });
                }
                SchemaJobAction::Publish
            }
            SchemaJobState::Paused => SchemaJobAction::StopPaused,
            SchemaJobState::Canceled => SchemaJobAction::StopCanceled,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OnlineDdlSafetyPlan {
    pub max_replication_lag_bytes: u64,
    pub max_lock_ms: u32,
    pub allow_blocking_cutover: bool,
    pub require_data_invariants: bool,
    pub data_invariants_verified: bool,
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

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaJobManifest {
    pub job: ManifestJob,
    pub worker_id: String,
    pub lease: SchemaJobLease,
    pub backfill: BackfillPlan,
    pub safety: OnlineDdlSafetyPlan,
    pub shadow: Option<GhOstShadowPlan>,
}

impl SchemaJobManifest {
    pub fn into_worker_plan(self) -> Result<SchemaJobWorkerPlan, SchemaJobSidecarError> {
        let job = self.job.into_plan()?;
        let worker = SchemaJobWorkerPlan {
            job,
            worker_id: self.worker_id,
            lease: self.lease,
            backfill: self.backfill,
            safety: self.safety,
            shadow: self.shadow,
        };
        worker.validate()?;
        Ok(worker)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestJob {
    pub name: String,
    pub table: String,
    pub state: String,
    pub operations: Vec<ManifestSchemaJobOperation>,
    pub lease_seconds: u32,
}

impl ManifestJob {
    fn into_plan(self) -> Result<SchemaJobPlan, SchemaJobSidecarError> {
        Ok(SchemaJobPlan {
            name: self.name,
            table: self.table,
            state: SchemaJobState::from_canonical(&self.state)?,
            operations: self
                .operations
                .into_iter()
                .map(ManifestSchemaJobOperation::into_operation)
                .collect(),
            lease_seconds: self.lease_seconds,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ManifestSchemaJobOperation {
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

impl ManifestSchemaJobOperation {
    fn into_operation(self) -> SchemaJobOperation {
        match self {
            Self::AddColumn { column, sql_type } => {
                SchemaJobOperation::AddColumn { column, sql_type }
            }
            Self::Backfill { statement } => SchemaJobOperation::Backfill { statement },
            Self::SwapColumn {
                old_column,
                new_column,
            } => SchemaJobOperation::SwapColumn {
                old_column,
                new_column,
            },
            Self::DropColumn { column } => SchemaJobOperation::DropColumn { column },
        }
    }
}

pub fn parse_worker_plan_manifest(raw: &str) -> Result<SchemaJobWorkerPlan, SchemaJobSidecarError> {
    let manifest: SchemaJobManifest = serde_json::from_str(raw)
        .map_err(|error| SchemaJobSidecarError::ManifestJson(error.to_string()))?;
    manifest.into_worker_plan()
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
    InvalidSqlBoundary(&'static str),
    ManifestJson(String),
    MissingRequiredField(&'static str),
    DataInvariantsNotVerified { job_name: String },
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
            Self::InvalidSqlBoundary(field) => write!(
                formatter,
                "{field} contains SQL that is outside the schema-job apply boundary"
            ),
            Self::ManifestJson(error) => write!(formatter, "manifest JSON invalid: {error}"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::DataInvariantsNotVerified { job_name } => write!(
                formatter,
                "data invariants are not verified for schema job {job_name}"
            ),
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
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(SchemaJobSidecarError::MissingRequiredField(field));
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(SchemaJobSidecarError::InvalidIdentifier(field));
    }
    if chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
        Ok(())
    } else {
        Err(SchemaJobSidecarError::InvalidIdentifier(field))
    }
}

fn validate_job_apply_boundary(job: &SchemaJobPlan) -> Result<(), SchemaJobSidecarError> {
    validate_qualified_name("job.table", &job.table)?;
    for operation in &job.operations {
        match operation {
            SchemaJobOperation::AddColumn { column, sql_type } => {
                validate_identifier("operations.column", column)?;
                validate_safe_sql_fragment("operations.sql_type", sql_type)?;
            }
            SchemaJobOperation::Backfill { statement } => {
                validate_backfill_statement(statement)?;
            }
            SchemaJobOperation::SwapColumn {
                old_column,
                new_column,
            } => {
                validate_identifier("operations.old_column", old_column)?;
                validate_identifier("operations.new_column", new_column)?;
            }
            SchemaJobOperation::DropColumn { column } => {
                validate_identifier("operations.column", column)?;
            }
        }
    }
    Ok(())
}

fn validate_safe_sql_fragment(
    field: &'static str,
    value: &str,
) -> Result<(), SchemaJobSidecarError> {
    validate_required(field, value)?;
    let lower = value.to_ascii_lowercase();
    if [";", "--", "/*", "*/", "'", "\"", "$$"]
        .iter()
        .any(|token| lower.contains(token))
    {
        return Err(SchemaJobSidecarError::InvalidSqlBoundary(field));
    }
    if value.chars().all(|ch| {
        ch.is_ascii_alphanumeric()
            || matches!(
                ch,
                '_' | ' ' | '(' | ')' | ',' | '[' | ']' | '.' | '+' | '-' | '=' | '<' | '>'
            )
    }) {
        Ok(())
    } else {
        Err(SchemaJobSidecarError::InvalidSqlBoundary(field))
    }
}

fn validate_backfill_statement(statement: &str) -> Result<(), SchemaJobSidecarError> {
    validate_safe_sql_fragment("operations.statement", statement)?;
    if statement
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("update ")
    {
        Ok(())
    } else {
        Err(SchemaJobSidecarError::InvalidSqlBoundary(
            "operations.statement",
        ))
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
            require_data_invariants: true,
            data_invariants_verified: true,
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
    fn manifest_parser_accepts_valid_manifest() {
        let raw = r#"{
          "job": {
            "name": "users-display-name",
            "table": "public.users",
            "state": "backfill",
            "lease_seconds": 30,
            "operations": [
              {"kind":"add_column","column":"display_name","sql_type":"text"},
              {"kind":"backfill","statement":"UPDATE public.users SET display_name = name WHERE display_name IS NULL"}
            ]
          },
          "worker_id": "schema-worker-a",
          "lease": {"holder":"schema-worker-a","epoch":1,"expires_at":"2026-05-19T12:00:00Z"},
          "backfill": {"batch_size":1000,"max_parallel_shards":4,"throttle_ms":50},
          "safety": {"max_replication_lag_bytes":16777216,"max_lock_ms":500,"allow_blocking_cutover":false,"require_data_invariants":true,"data_invariants_verified":true},
          "shadow": {"source_table":"public.users","shadow_table":"public._users_new","changelog_table":"public._users_changelog","cutover_lock_timeout_ms":500}
        }"#;

        let plan = parse_worker_plan_manifest(raw).expect("manifest");
        assert_eq!(plan.job.state, SchemaJobState::Backfill);
        assert_eq!(
            plan.next_action().expect("action"),
            SchemaJobAction::RunBackfill {
                batch_size: 1000,
                max_parallel_shards: 4
            }
        );
    }

    #[test]
    fn manifest_parser_fails_closed_on_unverified_public_cutover() {
        let raw = r#"{
          "job": {"name":"users-display-name","table":"public.users","state":"public","lease_seconds":30,"operations":[{"kind":"add_column","column":"display_name","sql_type":"text"}]},
          "worker_id": "schema-worker-a",
          "lease": {"holder":"schema-worker-a","epoch":1,"expires_at":"2026-05-19T12:00:00Z"},
          "backfill": {"batch_size":1000,"max_parallel_shards":4,"throttle_ms":50},
          "safety": {"max_replication_lag_bytes":16777216,"max_lock_ms":500,"allow_blocking_cutover":false,"require_data_invariants":true,"data_invariants_verified":false},
          "shadow": null
        }"#;

        assert_eq!(
            parse_worker_plan_manifest(raw).and_then(|plan| plan.next_action()),
            Err(SchemaJobSidecarError::DataInvariantsNotVerified {
                job_name: "users-display-name".to_string()
            })
        );
    }

    #[test]
    fn apply_boundary_rejects_unsafe_sql_fragments() {
        let mut plan = valid_worker_plan();
        plan.job.operations = vec![SchemaJobOperation::AddColumn {
            column: "display_name".to_string(),
            sql_type: "text; drop table public.users".to_string(),
        }];

        assert_eq!(
            plan.validate(),
            Err(SchemaJobSidecarError::InvalidSqlBoundary(
                "operations.sql_type"
            ))
        );
    }

    #[test]
    fn identifiers_must_start_with_letter_or_underscore() {
        assert_eq!(
            validate_identifier("operations.column", "1bad"),
            Err(SchemaJobSidecarError::InvalidIdentifier(
                "operations.column"
            ))
        );
    }

    #[test]
    fn canonical_report_is_deterministic() {
        let report = canonical_schema_job_report().expect("canonical report");

        assert_eq!(report.worker.job.name, "users-add-display-name");
        assert_eq!(report.worker.worker_id, "schema-worker-a");
        assert_eq!(report.action, SchemaJobAction::ApplyDeleteOnly);
    }

    #[test]
    fn publish_requires_verified_data_invariants() {
        let mut plan = valid_worker_plan();
        plan.job.state = SchemaJobState::Public;
        plan.safety.require_data_invariants = true;
        plan.safety.data_invariants_verified = false;

        assert_eq!(
            plan.next_action(),
            Err(SchemaJobSidecarError::DataInvariantsNotVerified {
                job_name: "users-add-display-name".to_string()
            })
        );
    }

    fn valid_worker_plan() -> SchemaJobWorkerPlan {
        canonical_schema_job_worker_plan()
    }
}
