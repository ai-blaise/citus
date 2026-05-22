// FEATURE: C10
// FEATURE: M2
// FEATURE: M14

//! Worker-side schema-version lease management for the F1-style controller.
//!
//! Each Citus worker records the schema version it currently believes is in
//! force via `companion.worker_schema_lease(worker_id, schema_version_id,
//! expires_at)`. The F1 controller waits until every live worker
//! acknowledges the *current* phase before driving the *next* one; the
//! `WorkerLeaseRegistry` collects those acknowledgements in process and
//! renders the SQL that the sidecar uses to update them in the database.

use super::{SchemaJobError, SchemaJobState, COMPANION_INTERNAL_SCHEMA};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

/// In-memory record of a worker's currently held schema lease.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WorkerLease {
    pub worker_id: String,
    pub job_name: String,
    pub schema_version_id: String,
    pub phase: SchemaJobState,
    pub expires_at: String,
}

impl WorkerLease {
    /// Construct a validated lease record.
    pub fn new(
        worker_id: impl Into<String>,
        job_name: impl Into<String>,
        schema_version_id: impl Into<String>,
        phase: SchemaJobState,
        expires_at: impl Into<String>,
    ) -> Result<Self, WorkerLeaseError> {
        let lease = Self {
            worker_id: worker_id.into(),
            job_name: job_name.into(),
            schema_version_id: schema_version_id.into(),
            phase,
            expires_at: expires_at.into(),
        };
        lease.validate()?;
        Ok(lease)
    }

    /// Validate this lease in isolation. Mirrors the CHECK constraints on the
    /// SQL table.
    pub fn validate(&self) -> Result<(), WorkerLeaseError> {
        require_field("worker_id", &self.worker_id)?;
        require_field("job_name", &self.job_name)?;
        require_field("schema_version_id", &self.schema_version_id)?;
        require_field("expires_at", &self.expires_at)?;
        validate_rfc3339_utc("expires_at", &self.expires_at)?;
        Ok(())
    }

    /// True if `now` (an RFC3339 UTC timestamp) is strictly before the
    /// lease's expiry. Lexicographic comparison is safe because both
    /// timestamps are RFC3339 with the same zone suffix (`Z`).
    pub fn is_live_at(&self, now: &str) -> bool {
        now < self.expires_at.as_str()
    }
}

/// Status of a worker after the registry collects all acknowledgements.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum WorkerLeaseStatus {
    /// Worker acknowledged the expected phase before its lease expired.
    Acknowledged,
    /// Worker is alive but acknowledged a stale schema version. The
    /// controller must not advance.
    StalePhase { observed: SchemaJobState },
    /// Worker's lease expired before the controller polled.
    Expired,
    /// Worker never reported a lease for this job.
    Missing,
}

impl WorkerLeaseStatus {
    pub fn as_canonical(&self) -> &'static str {
        match self {
            Self::Acknowledged => "acknowledged",
            Self::StalePhase { .. } => "stale_phase",
            Self::Expired => "expired",
            Self::Missing => "missing",
        }
    }
}

/// Tracks per-worker lease state for one schema job.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct WorkerLeaseRegistry {
    job_name: String,
    expected_phase: Option<SchemaJobState>,
    leases: BTreeMap<String, WorkerLease>,
}

impl WorkerLeaseRegistry {
    /// Build a registry for `job_name`. The expected phase is set by the
    /// controller before sweeping worker acknowledgements.
    pub fn new(job_name: impl Into<String>) -> Result<Self, WorkerLeaseError> {
        let job_name = job_name.into();
        require_field("job_name", &job_name)?;
        Ok(Self {
            job_name,
            expected_phase: None,
            leases: BTreeMap::new(),
        })
    }

    /// Mark `phase` as the phase that the controller is currently driving.
    pub fn expect_phase(&mut self, phase: SchemaJobState) {
        self.expected_phase = Some(phase);
    }

    pub fn expected_phase(&self) -> Option<SchemaJobState> {
        self.expected_phase
    }

    pub fn job_name(&self) -> &str {
        &self.job_name
    }

    /// Add or replace a lease record for one worker.
    pub fn record(&mut self, lease: WorkerLease) -> Result<(), WorkerLeaseError> {
        if lease.job_name != self.job_name {
            return Err(WorkerLeaseError::JobNameMismatch {
                expected: self.job_name.clone(),
                observed: lease.job_name.clone(),
            });
        }
        lease.validate()?;
        self.leases.insert(lease.worker_id.clone(), lease);
        Ok(())
    }

    /// Inspect the lease for one worker.
    pub fn lease_for(&self, worker_id: &str) -> Option<&WorkerLease> {
        self.leases.get(worker_id)
    }

    pub fn workers(&self) -> impl Iterator<Item = &str> {
        self.leases.keys().map(String::as_str)
    }

    /// Status of one worker against the expected phase as of `now`.
    pub fn status_for(&self, worker_id: &str, now: &str) -> WorkerLeaseStatus {
        let Some(lease) = self.leases.get(worker_id) else {
            return WorkerLeaseStatus::Missing;
        };
        if !lease.is_live_at(now) {
            return WorkerLeaseStatus::Expired;
        }
        match self.expected_phase {
            Some(expected) if lease.phase == expected => WorkerLeaseStatus::Acknowledged,
            Some(_) => WorkerLeaseStatus::StalePhase {
                observed: lease.phase,
            },
            None => WorkerLeaseStatus::Acknowledged,
        }
    }

    /// Collect statuses for a set of expected worker IDs.
    pub fn summarize(
        &self,
        expected_workers: &[&str],
        now: &str,
    ) -> BTreeMap<String, WorkerLeaseStatus> {
        expected_workers
            .iter()
            .map(|worker_id| ((*worker_id).to_string(), self.status_for(worker_id, now)))
            .collect()
    }

    /// True if every expected worker reports `Acknowledged`.
    pub fn all_acknowledged(&self, expected_workers: &[&str], now: &str) -> bool {
        expected_workers.iter().all(|worker_id| {
            matches!(
                self.status_for(worker_id, now),
                WorkerLeaseStatus::Acknowledged
            )
        })
    }

    /// Worker IDs that have not yet acknowledged the expected phase.
    pub fn delinquent(&self, expected_workers: &[&str], now: &str) -> Vec<String> {
        expected_workers
            .iter()
            .filter(|worker_id| {
                !matches!(
                    self.status_for(worker_id, now),
                    WorkerLeaseStatus::Acknowledged
                )
            })
            .map(|worker_id| (*worker_id).to_string())
            .collect()
    }
}

/// SQL renderer for the lease registry. The companion crate emits the SQL
/// that the sidecar issues against the coordinator.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WorkerLeaseSqlPlan {
    pub feature_id: &'static str,
    pub commands: Vec<String>,
}

impl WorkerLeaseSqlPlan {
    pub fn upsert(lease: &WorkerLease) -> Result<Self, WorkerLeaseError> {
        lease.validate()?;
        let command = format!(
            "SELECT {schema}.worker_schema_lease_upsert({worker}, {job}, {version}, {phase}, {expires});",
            schema = COMPANION_INTERNAL_SCHEMA,
            worker = sql_literal(&lease.worker_id),
            job = sql_literal(&lease.job_name),
            version = sql_literal(&lease.schema_version_id),
            phase = sql_literal(lease.phase.as_canonical()),
            expires = sql_literal(&lease.expires_at)
        );
        Ok(Self {
            feature_id: "C10",
            commands: vec![command],
        })
    }

    pub fn revoke(job_name: &str, worker_id: &str) -> Result<Self, WorkerLeaseError> {
        require_field("job_name", job_name)?;
        require_field("worker_id", worker_id)?;
        let command = format!(
            "SELECT {schema}.worker_schema_lease_revoke({worker}, {job});",
            schema = COMPANION_INTERNAL_SCHEMA,
            worker = sql_literal(worker_id),
            job = sql_literal(job_name)
        );
        Ok(Self {
            feature_id: "C10",
            commands: vec![command],
        })
    }

    pub fn script(&self) -> String {
        self.commands.join("\n")
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum WorkerLeaseError {
    JobNameMismatch { expected: String, observed: String },
    InvalidTimestamp(&'static str),
    MissingRequiredField(&'static str),
}

impl fmt::Display for WorkerLeaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JobNameMismatch { expected, observed } => write!(
                formatter,
                "lease job name mismatch: registry holds {expected:?}, received {observed:?}"
            ),
            Self::InvalidTimestamp(field) => {
                write!(
                    formatter,
                    "{field} must be an RFC3339 UTC timestamp ending in Z"
                )
            }
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for WorkerLeaseError {}

impl From<SchemaJobError> for WorkerLeaseError {
    fn from(error: SchemaJobError) -> Self {
        match error {
            SchemaJobError::MissingRequiredField(field) => {
                WorkerLeaseError::MissingRequiredField(field)
            }
            SchemaJobError::InvalidLease => WorkerLeaseError::MissingRequiredField("lease"),
            SchemaJobError::UnknownState(_) => {
                WorkerLeaseError::MissingRequiredField("schema_version_id")
            }
        }
    }
}

fn require_field(field: &'static str, value: &str) -> Result<(), WorkerLeaseError> {
    if value.trim().is_empty() {
        return Err(WorkerLeaseError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_rfc3339_utc(field: &'static str, value: &str) -> Result<(), WorkerLeaseError> {
    if value.len() >= 20 && value.contains('T') && value.ends_with('Z') {
        Ok(())
    } else {
        Err(WorkerLeaseError::InvalidTimestamp(field))
    }
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lease(
        worker_id: &str,
        job_name: &str,
        version: &str,
        phase: SchemaJobState,
        expires: &str,
    ) -> WorkerLease {
        WorkerLease::new(worker_id, job_name, version, phase, expires).expect("valid lease")
    }

    #[test]
    fn lease_rejects_non_rfc3339_timestamps() {
        let invalid = WorkerLease::new(
            "worker-a",
            "users-display-name",
            "schema-v2",
            SchemaJobState::WriteOnly,
            "2026-05-22 14:00:00+00",
        );
        assert_eq!(
            invalid,
            Err(WorkerLeaseError::InvalidTimestamp("expires_at"))
        );
    }

    #[test]
    fn registry_summarizes_three_workers() {
        let mut registry = WorkerLeaseRegistry::new("users-display-name").expect("registry");
        registry.expect_phase(SchemaJobState::WriteOnly);
        registry
            .record(lease(
                "worker-a",
                "users-display-name",
                "schema-v2",
                SchemaJobState::WriteOnly,
                "2026-05-22T14:00:00Z",
            ))
            .expect("worker-a recorded");
        registry
            .record(lease(
                "worker-b",
                "users-display-name",
                "schema-v2",
                SchemaJobState::DeleteOnly,
                "2026-05-22T14:00:00Z",
            ))
            .expect("worker-b recorded");

        let now = "2026-05-22T13:50:00Z";
        let summary = registry.summarize(&["worker-a", "worker-b", "worker-c"], now);

        assert_eq!(
            summary.get("worker-a"),
            Some(&WorkerLeaseStatus::Acknowledged)
        );
        assert_eq!(
            summary.get("worker-b"),
            Some(&WorkerLeaseStatus::StalePhase {
                observed: SchemaJobState::DeleteOnly,
            })
        );
        assert_eq!(summary.get("worker-c"), Some(&WorkerLeaseStatus::Missing));
        assert!(!registry.all_acknowledged(&["worker-a", "worker-b", "worker-c"], now));
        assert_eq!(
            registry.delinquent(&["worker-a", "worker-b", "worker-c"], now),
            vec!["worker-b".to_string(), "worker-c".to_string()]
        );
    }

    #[test]
    fn lease_upsert_renders_qualified_sql() {
        let plan = WorkerLeaseSqlPlan::upsert(&lease(
            "worker-a",
            "users-display-name",
            "schema-v2",
            SchemaJobState::WriteOnly,
            "2026-05-22T14:00:00Z",
        ))
        .expect("plan");
        assert_eq!(plan.feature_id, "C10");
        assert!(plan
            .script()
            .contains("companion_internal.worker_schema_lease_upsert"));
        assert!(plan.script().contains("'worker-a'"));
        assert!(plan.script().contains("'write_only'"));
    }

    #[test]
    fn job_name_mismatch_rejected() {
        let mut registry = WorkerLeaseRegistry::new("job-a").expect("registry");
        let other = lease(
            "worker-a",
            "job-b",
            "schema-v2",
            SchemaJobState::WriteOnly,
            "2026-05-22T14:00:00Z",
        );
        assert_eq!(
            registry.record(other),
            Err(WorkerLeaseError::JobNameMismatch {
                expected: "job-a".to_string(),
                observed: "job-b".to_string(),
            })
        );
    }

    #[test]
    fn expired_lease_reported_after_expiry() {
        let mut registry = WorkerLeaseRegistry::new("job-a").expect("registry");
        registry.expect_phase(SchemaJobState::WriteOnly);
        registry
            .record(lease(
                "worker-a",
                "job-a",
                "schema-v2",
                SchemaJobState::WriteOnly,
                "2026-05-22T14:00:00Z",
            ))
            .expect("lease recorded");
        let now = "2026-05-22T15:30:00Z";
        assert_eq!(
            registry.status_for("worker-a", now),
            WorkerLeaseStatus::Expired
        );
    }
}
