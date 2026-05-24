//! Transaction-status sidecar contracts.

// FEATURE: T5

pub mod runtime;

pub use runtime::{
    canonical_txn_runtime_report, finalize_decision_name, render_finalize_json, render_record_json,
    run_parallel_commit_microbench, txn_status_name, IntentEvidence, ParallelCommitMicrobench,
    TxnRuntimeCanonicalReport, TxnRuntimeError, TxnRuntimeRecord, TxnStatusRuntime,
};

use ai_blaise_citus_sidecar_hlc::HlcTimestamp;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TxnStatusServicePlan {
    pub raft_group: String,
    pub voters: Vec<String>,
    pub clock: HlcTimestamp,
    pub max_staging_ms: u64,
}

impl TxnStatusServicePlan {
    pub fn validate(&self) -> Result<(), TxnStatusError> {
        validate_required("raft_group", &self.raft_group)?;
        validate_required_list("voters", &self.voters)?;
        if self.max_staging_ms == 0 {
            return Err(TxnStatusError::InvalidStagingDeadline);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParallelCommitRecord {
    pub txn_id: String,
    pub coordinator: String,
    pub status: TxnStatus,
    pub staging_at: HlcTimestamp,
    pub intents: Vec<TxnIntent>,
}

impl ParallelCommitRecord {
    pub fn validate(&self) -> Result<(), TxnStatusError> {
        validate_required("txn_id", &self.txn_id)?;
        validate_required("coordinator", &self.coordinator)?;
        if self.intents.is_empty() {
            return Err(TxnStatusError::MissingRequiredField("intents"));
        }
        for intent in &self.intents {
            intent.validate()?;
        }
        Ok(())
    }

    pub fn finalize_decision(
        &self,
        service: &TxnStatusServicePlan,
        observed_at: HlcTimestamp,
    ) -> Result<TxnFinalizeDecision, TxnStatusError> {
        self.validate()?;
        service.validate()?;

        if self.status == TxnStatus::Committed {
            return Ok(TxnFinalizeDecision::AlreadyCommitted);
        }
        if self.status == TxnStatus::Aborted {
            return Ok(TxnFinalizeDecision::AlreadyAborted);
        }
        if self.status != TxnStatus::Staging {
            return Ok(TxnFinalizeDecision::FallbackToTwoPhaseCommit);
        }
        let staging_deadline = self
            .staging_at
            .physical_ms
            .checked_add(service.max_staging_ms)
            .ok_or(TxnStatusError::InvalidStagingDeadline)?;
        if observed_at.physical_ms > staging_deadline {
            return Ok(TxnFinalizeDecision::AbortStaleStagingRecord);
        }
        if self.intents.iter().all(TxnIntent::has_replication_evidence) {
            Ok(TxnFinalizeDecision::Commit)
        } else {
            Ok(TxnFinalizeDecision::WaitForReplicationEvidence)
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TxnStatus {
    Pending,
    Staging,
    Committed,
    Aborted,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TxnIntent {
    pub shard_id: u64,
    pub key_range: String,
    pub replica_ack_count: u32,
    pub required_acks: u32,
}

impl TxnIntent {
    fn validate(&self) -> Result<(), TxnStatusError> {
        if self.shard_id == 0 {
            return Err(TxnStatusError::InvalidShardId);
        }
        validate_required("intent.key_range", &self.key_range)?;
        if self.required_acks == 0 {
            return Err(TxnStatusError::InvalidRequiredAcks);
        }
        Ok(())
    }

    fn has_replication_evidence(&self) -> bool {
        self.replica_ack_count >= self.required_acks
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TxnFinalizeDecision {
    Commit,
    WaitForReplicationEvidence,
    AbortStaleStagingRecord,
    FallbackToTwoPhaseCommit,
    AlreadyCommitted,
    AlreadyAborted,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TxnStatusError {
    InvalidRequiredAcks,
    InvalidShardId,
    InvalidStagingDeadline,
    MissingRequiredField(&'static str),
}

impl fmt::Display for TxnStatusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequiredAcks => {
                write!(formatter, "required_acks must be greater than zero")
            }
            Self::InvalidShardId => write!(formatter, "shard_id must be greater than zero"),
            Self::InvalidStagingDeadline => {
                write!(formatter, "max_staging_ms must be greater than zero")
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
        }
    }
}

impl Error for TxnStatusError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), TxnStatusError> {
    if value.trim().is_empty() {
        return Err(TxnStatusError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_required_list(field: &'static str, values: &[String]) -> Result<(), TxnStatusError> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(TxnStatusError::MissingRequiredField(field));
    }
    Ok(())
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TxnStatusCanonicalReport {
    pub service: TxnStatusServicePlan,
    pub record: ParallelCommitRecord,
    pub observed_at: HlcTimestamp,
    pub decision: TxnFinalizeDecision,
}

pub fn canonical_txn_status_service() -> TxnStatusServicePlan {
    TxnStatusServicePlan {
        raft_group: "txn-status-orders".to_string(),
        voters: vec![
            "worker-a".to_string(),
            "worker-b".to_string(),
            "worker-c".to_string(),
        ],
        clock: observed_at(1_700_000_000),
        max_staging_ms: 5_000,
    }
}

pub fn canonical_parallel_commit_record() -> ParallelCommitRecord {
    ParallelCommitRecord {
        txn_id: "txn-42".to_string(),
        coordinator: "worker-a".to_string(),
        status: TxnStatus::Staging,
        staging_at: observed_at(1_700_000_000),
        intents: vec![
            TxnIntent {
                shard_id: 10,
                key_range: "[a,m)".to_string(),
                replica_ack_count: 2,
                required_acks: 2,
            },
            TxnIntent {
                shard_id: 11,
                key_range: "[m,z)".to_string(),
                replica_ack_count: 2,
                required_acks: 2,
            },
        ],
    }
}

pub fn canonical_txn_status_report() -> Result<TxnStatusCanonicalReport, TxnStatusError> {
    let service = canonical_txn_status_service();
    let record = canonical_parallel_commit_record();
    let observed_at = observed_at(1_700_000_010);
    let decision = record.finalize_decision(&service, observed_at)?;

    Ok(TxnStatusCanonicalReport {
        service,
        record,
        observed_at,
        decision,
    })
}

pub fn observed_at(physical_ms: u64) -> HlcTimestamp {
    HlcTimestamp {
        physical_ms,
        logical: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staging_record_commits_after_all_intents_have_evidence() {
        let decision = valid_record()
            .finalize_decision(&valid_service(), observed_at(1_700_000_010))
            .expect("decision");

        assert_eq!(decision, TxnFinalizeDecision::Commit);
    }

    #[test]
    fn staging_record_waits_for_missing_replication_evidence() {
        let mut record = valid_record();
        record.intents[1].replica_ack_count = 1;

        let decision = record
            .finalize_decision(&valid_service(), observed_at(1_700_000_010))
            .expect("decision");

        assert_eq!(decision, TxnFinalizeDecision::WaitForReplicationEvidence);
    }

    #[test]
    fn stale_staging_record_aborts() {
        let decision = valid_record()
            .finalize_decision(&valid_service(), observed_at(1_700_010_000))
            .expect("decision");

        assert_eq!(decision, TxnFinalizeDecision::AbortStaleStagingRecord);
    }

    #[test]
    fn pending_record_falls_back_to_two_phase_commit() {
        let mut record = valid_record();
        record.status = TxnStatus::Pending;

        let decision = record
            .finalize_decision(&valid_service(), observed_at(1_700_000_010))
            .expect("decision");

        assert_eq!(decision, TxnFinalizeDecision::FallbackToTwoPhaseCommit);
    }

    #[test]
    fn canonical_report_is_deterministic() {
        let report = canonical_txn_status_report().expect("canonical report");

        assert_eq!(report.service.raft_group, "txn-status-orders");
        assert_eq!(report.record.txn_id, "txn-42");
        assert_eq!(report.decision, TxnFinalizeDecision::Commit);
    }

    fn valid_service() -> TxnStatusServicePlan {
        canonical_txn_status_service()
    }

    fn valid_record() -> ParallelCommitRecord {
        canonical_parallel_commit_record()
    }
}
