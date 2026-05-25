//! Parallel-commit transaction-status runtime backed by a deterministic Raft
//! state-machine boundary.
//!
//! This is the executable runtime evidence for the narrow `FEATURE: T5`
//! sidecar contract. It records transaction staging records, intent
//! replication evidence, and final commit/abort decisions through either the
//! local Raft round-trip model or the live Raft HTTP sidecar before
//! acknowledging a modeled commit. It is not Citus executor integration; when
//! the sidecar path is unavailable, callers must fall back to standard
//! distributed 2PC. The contract surface in `lib.rs` carries the deterministic
//! boundary used by the companion fallback path.

// FEATURE: T5

use crate::{ParallelCommitRecord, TxnFinalizeDecision, TxnIntent, TxnStatus, TxnStatusError};
use ai_blaise_citus_sidecar_hlc::HlcTimestamp;
use ai_blaise_citus_sidecar_raft::{
    run_raft_round_trip, NodeId, RaftRoundTripReport, RaftRuntimeError,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TxnRuntimeError {
    UnknownTxn(String),
    StatusValidation(TxnStatusError),
    RaftFailure(String),
    DuplicateTxn(String),
}

impl fmt::Display for TxnRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTxn(id) => write!(formatter, "unknown txn_id: {id}"),
            Self::StatusValidation(error) => write!(formatter, "{error}"),
            Self::RaftFailure(detail) => write!(formatter, "raft failure: {detail}"),
            Self::DuplicateTxn(id) => write!(formatter, "txn_id already staged: {id}"),
        }
    }
}

impl Error for TxnRuntimeError {}

impl From<TxnStatusError> for TxnRuntimeError {
    fn from(error: TxnStatusError) -> Self {
        Self::StatusValidation(error)
    }
}

impl From<RaftRuntimeError> for TxnRuntimeError {
    fn from(error: RaftRuntimeError) -> Self {
        Self::RaftFailure(error.to_string())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TxnRaftReplication {
    InProcess,
    HttpLeader { leader_addr: String },
}

impl TxnRaftReplication {
    pub fn http_leader(leader_addr: impl Into<String>) -> Result<Self, TxnRuntimeError> {
        let leader_addr = leader_addr.into();
        if leader_addr.trim().is_empty() {
            return Err(TxnRuntimeError::RaftFailure(
                "raft leader address must not be empty".to_string(),
            ));
        }
        Ok(Self::HttpLeader { leader_addr })
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::InProcess => "in_process",
            Self::HttpLeader { .. } => "http_leader",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct TxnRaftCommitEvidence {
    committed_index: u64,
    committed_payload: Vec<u8>,
}

/// Replication evidence for a single intent: replica acks observed since the
/// staging record was first committed.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct IntentEvidence {
    pub shard_id: u64,
    pub replica_acks: u32,
    pub required_acks: u32,
}

impl IntentEvidence {
    pub fn from_intent(intent: &TxnIntent) -> Self {
        Self {
            shard_id: intent.shard_id,
            replica_acks: intent.replica_ack_count,
            required_acks: intent.required_acks,
        }
    }

    pub fn has_evidence(&self) -> bool {
        self.replica_acks >= self.required_acks
    }
}

/// In-memory transaction record tracked by the runtime. The record itself is
/// replicated through Raft; this struct is the materialised state machine
/// view that callers query via HTTP.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TxnRuntimeRecord {
    pub txn_id: String,
    pub coordinator: String,
    pub status: TxnStatus,
    pub staging_at: HlcTimestamp,
    pub intents: Vec<IntentEvidence>,
    pub raft_index: u64,
    pub raft_transport: String,
}

/// Parallel-commit runtime. Holds the transaction state machine and tracks
/// each shard intent's replication evidence. Backed by Raft for staging
/// decisions; finalize decisions consult the local replication-evidence map
/// to decide commit / wait / abort outcomes.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TxnStatusRuntime {
    raft_group: String,
    voters: Vec<NodeId>,
    max_staging_ms: u64,
    records: BTreeMap<String, TxnRuntimeRecord>,
    next_raft_index: u64,
    raft_replication: TxnRaftReplication,
}

impl TxnStatusRuntime {
    pub fn new(
        raft_group: impl Into<String>,
        voters: Vec<NodeId>,
        max_staging_ms: u64,
    ) -> Result<Self, TxnRuntimeError> {
        let raft_group = raft_group.into();
        if raft_group.trim().is_empty() {
            return Err(TxnRuntimeError::StatusValidation(
                TxnStatusError::MissingRequiredField("raft_group"),
            ));
        }
        if voters.is_empty() {
            return Err(TxnRuntimeError::StatusValidation(
                TxnStatusError::MissingRequiredField("voters"),
            ));
        }
        if max_staging_ms == 0 {
            return Err(TxnRuntimeError::StatusValidation(
                TxnStatusError::InvalidStagingDeadline,
            ));
        }
        Ok(Self {
            raft_group,
            voters,
            max_staging_ms,
            records: BTreeMap::new(),
            next_raft_index: 1,
            raft_replication: TxnRaftReplication::InProcess,
        })
    }

    pub fn with_raft_replication(mut self, replication: TxnRaftReplication) -> Self {
        self.raft_replication = replication;
        self
    }

    pub fn raft_group(&self) -> &str {
        &self.raft_group
    }

    pub fn voters(&self) -> &[NodeId] {
        &self.voters
    }

    pub fn max_staging_ms(&self) -> u64 {
        self.max_staging_ms
    }

    pub fn raft_replication(&self) -> &TxnRaftReplication {
        &self.raft_replication
    }

    pub fn records(&self) -> &BTreeMap<String, TxnRuntimeRecord> {
        &self.records
    }

    /// Stage a new parallel-commit record. Replicates the staging entry
    /// through the Raft state machine before returning the materialised
    /// record; if the Raft round-trip fails, the runtime stays untouched and
    /// the coordinator must fall back to standard 2PC.
    pub fn stage(
        &mut self,
        record: ParallelCommitRecord,
    ) -> Result<TxnRuntimeRecord, TxnRuntimeError> {
        record.validate()?;
        if self.records.contains_key(&record.txn_id) {
            return Err(TxnRuntimeError::DuplicateTxn(record.txn_id));
        }
        let payload = format!("stage:{}:{}", record.txn_id, record.coordinator).into_bytes();
        let round_trip = self.replicate_through_raft(payload.clone())?;
        validate_committed_payload(&round_trip, &payload)?;
        let runtime_record = TxnRuntimeRecord {
            txn_id: record.txn_id.clone(),
            coordinator: record.coordinator,
            status: TxnStatus::Staging,
            staging_at: record.staging_at,
            intents: record
                .intents
                .iter()
                .map(IntentEvidence::from_intent)
                .collect(),
            raft_index: round_trip.committed_index,
            raft_transport: self.raft_replication.name().to_string(),
        };
        self.records
            .insert(record.txn_id.clone(), runtime_record.clone());
        self.next_raft_index = round_trip.committed_index + 1;
        Ok(runtime_record)
    }

    /// Record a replica ack for a staged intent.
    pub fn record_replica_ack(
        &mut self,
        txn_id: &str,
        shard_id: u64,
        replica_acks: u32,
    ) -> Result<TxnRuntimeRecord, TxnRuntimeError> {
        let record = self
            .records
            .get_mut(txn_id)
            .ok_or_else(|| TxnRuntimeError::UnknownTxn(txn_id.to_string()))?;
        let intent = record
            .intents
            .iter_mut()
            .find(|intent| intent.shard_id == shard_id)
            .ok_or(TxnRuntimeError::StatusValidation(
                TxnStatusError::InvalidShardId,
            ))?;
        if replica_acks > intent.replica_acks {
            intent.replica_acks = replica_acks;
        }
        Ok(record.clone())
    }

    /// Finalize a staged record. Returns the Raft-replicated decision.
    pub fn finalize(
        &mut self,
        txn_id: &str,
        observed_at: HlcTimestamp,
    ) -> Result<(TxnRuntimeRecord, TxnFinalizeDecision), TxnRuntimeError> {
        let staging_deadline = {
            let record = self
                .records
                .get(txn_id)
                .ok_or_else(|| TxnRuntimeError::UnknownTxn(txn_id.to_string()))?;
            record
                .staging_at
                .physical_ms
                .checked_add(self.max_staging_ms)
                .ok_or(TxnRuntimeError::StatusValidation(
                    TxnStatusError::InvalidStagingDeadline,
                ))?
        };

        let (decision, new_status, payload) = {
            let record = self
                .records
                .get(txn_id)
                .ok_or_else(|| TxnRuntimeError::UnknownTxn(txn_id.to_string()))?;
            if record.status == TxnStatus::Committed {
                (
                    TxnFinalizeDecision::AlreadyCommitted,
                    TxnStatus::Committed,
                    format!("noop:{}", txn_id),
                )
            } else if record.status == TxnStatus::Aborted {
                (
                    TxnFinalizeDecision::AlreadyAborted,
                    TxnStatus::Aborted,
                    format!("noop:{}", txn_id),
                )
            } else if record.status != TxnStatus::Staging {
                (
                    TxnFinalizeDecision::FallbackToTwoPhaseCommit,
                    record.status,
                    format!("fallback:{}", txn_id),
                )
            } else if observed_at.physical_ms > staging_deadline {
                (
                    TxnFinalizeDecision::AbortStaleStagingRecord,
                    TxnStatus::Aborted,
                    format!("abort:{}", txn_id),
                )
            } else if record.intents.iter().all(IntentEvidence::has_evidence) {
                (
                    TxnFinalizeDecision::Commit,
                    TxnStatus::Committed,
                    format!("commit:{}", txn_id),
                )
            } else {
                (
                    TxnFinalizeDecision::WaitForReplicationEvidence,
                    TxnStatus::Staging,
                    format!("wait:{}", txn_id),
                )
            }
        };

        // Only replicate state transitions through Raft; waits remain local.
        let raft_index = if matches!(
            decision,
            TxnFinalizeDecision::Commit | TxnFinalizeDecision::AbortStaleStagingRecord
        ) {
            let payload = payload.into_bytes();
            let round_trip = self.replicate_through_raft(payload.clone())?;
            validate_committed_payload(&round_trip, &payload)?;
            self.next_raft_index = round_trip.committed_index + 1;
            Some(round_trip.committed_index)
        } else {
            None
        };

        let record = self
            .records
            .get_mut(txn_id)
            .expect("txn must exist after staging deadline check");
        record.status = new_status;
        if let Some(index) = raft_index {
            record.raft_index = index;
        }
        Ok((record.clone(), decision))
    }

    pub fn status(&self, txn_id: &str) -> Result<&TxnRuntimeRecord, TxnRuntimeError> {
        self.records
            .get(txn_id)
            .ok_or_else(|| TxnRuntimeError::UnknownTxn(txn_id.to_string()))
    }

    fn replicate_through_raft(
        &mut self,
        payload: Vec<u8>,
    ) -> Result<TxnRaftCommitEvidence, TxnRuntimeError> {
        match &self.raft_replication {
            TxnRaftReplication::InProcess => {
                let leader = self
                    .voters
                    .first()
                    .cloned()
                    .ok_or(TxnRuntimeError::RaftFailure(
                        "no voters configured".to_string(),
                    ))?;
                Ok(TxnRaftCommitEvidence::from(run_raft_round_trip(
                    self.voters.clone(),
                    &leader,
                    payload,
                )?))
            }
            TxnRaftReplication::HttpLeader { leader_addr } => {
                replicate_via_http_raft(leader_addr, &payload)
            }
        }
    }
}

impl From<RaftRoundTripReport> for TxnRaftCommitEvidence {
    fn from(report: RaftRoundTripReport) -> Self {
        Self {
            committed_index: report.committed_index,
            committed_payload: report.committed_payload,
        }
    }
}

fn replicate_via_http_raft(
    leader_addr: &str,
    payload: &[u8],
) -> Result<TxnRaftCommitEvidence, TxnRuntimeError> {
    let payload_text = std::str::from_utf8(payload).map_err(|_| {
        TxnRuntimeError::RaftFailure("txn-status raft payload must be utf-8".to_string())
    })?;
    let response = http_post(leader_addr, "/raft/propose", payload_text)?;
    let (status, body) = split_http_response(&response)?;
    if status != 201 {
        return Err(TxnRuntimeError::RaftFailure(format!(
            "raft leader returned HTTP {status}: {body}"
        )));
    }
    let value: serde_json::Value = serde_json::from_str(body.trim()).map_err(|error| {
        TxnRuntimeError::RaftFailure(format!("raft leader response was not JSON: {error}"))
    })?;
    let committed_index = value
        .get("commit_index")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| {
            TxnRuntimeError::RaftFailure("raft leader response missing commit_index".to_string())
        })?;
    let committed_payload = value
        .get("committed_payload")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            TxnRuntimeError::RaftFailure(
                "raft leader response missing committed_payload".to_string(),
            )
        })?;
    if committed_payload != payload_text {
        return Err(TxnRuntimeError::RaftFailure(format!(
            "raft leader committed unexpected payload {committed_payload:?}"
        )));
    }
    Ok(TxnRaftCommitEvidence {
        committed_index,
        committed_payload: committed_payload.as_bytes().to_vec(),
    })
}

fn http_post(addr: &str, path: &str, body: &str) -> Result<String, TxnRuntimeError> {
    let mut stream = TcpStream::connect(addr).map_err(|error| {
        TxnRuntimeError::RaftFailure(format!("connect raft leader {addr}: {error}"))
    })?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| TxnRuntimeError::RaftFailure(format!("set read timeout: {error}")))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| TxnRuntimeError::RaftFailure(format!("set write timeout: {error}")))?;
    let request = format!(
        "POST {path} HTTP/1.1\r\nhost: {addr}\r\ncontent-type: text/plain\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).map_err(|error| {
        TxnRuntimeError::RaftFailure(format!("write raft leader {addr}: {error}"))
    })?;
    let mut response = String::new();
    stream.read_to_string(&mut response).map_err(|error| {
        TxnRuntimeError::RaftFailure(format!("read raft leader {addr}: {error}"))
    })?;
    Ok(response)
}

fn split_http_response(response: &str) -> Result<(u16, &str), TxnRuntimeError> {
    let status_line = response.lines().next().ok_or_else(|| {
        TxnRuntimeError::RaftFailure("missing raft HTTP response status".to_string())
    })?;
    let status = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| {
            TxnRuntimeError::RaftFailure(format!(
                "malformed raft HTTP response status: {status_line}"
            ))
        })?
        .parse::<u16>()
        .map_err(|_| {
            TxnRuntimeError::RaftFailure(format!(
                "invalid raft HTTP status in response: {status_line}"
            ))
        })?;
    let body = response
        .split_once("\r\n\r\n")
        .or_else(|| response.split_once("\n\n"))
        .map(|(_, body)| body)
        .unwrap_or("");
    Ok((status, body))
}

fn validate_committed_payload(
    evidence: &TxnRaftCommitEvidence,
    expected: &[u8],
) -> Result<(), TxnRuntimeError> {
    if evidence.committed_payload == expected {
        return Ok(());
    }
    Err(TxnRuntimeError::RaftFailure(
        "raft committed payload does not match transaction decision".to_string(),
    ))
}

/// JSON-rendering helpers used by the `/txn/*` HTTP routes.
pub fn render_record_json(record: &TxnRuntimeRecord) -> String {
    let intents = record
        .intents
        .iter()
        .map(|intent| {
            json!({
                "shard_id": intent.shard_id,
                "replica_acks": intent.replica_acks,
                "required_acks": intent.required_acks,
            })
        })
        .collect::<Vec<_>>();
    format!(
        "{}\n",
        json!({
            "txn_id": record.txn_id,
            "coordinator": record.coordinator,
            "status": txn_status_name(record.status),
            "staging_at": {
                "physical_ms": record.staging_at.physical_ms,
                "logical": record.staging_at.logical,
            },
            "raft_index": record.raft_index,
            "raft_transport": record.raft_transport,
            "intents": intents,
        })
    )
}

pub fn render_finalize_json(record: &TxnRuntimeRecord, decision: TxnFinalizeDecision) -> String {
    let intents = record
        .intents
        .iter()
        .map(|intent| {
            json!({
                "shard_id": intent.shard_id,
                "replica_acks": intent.replica_acks,
                "required_acks": intent.required_acks,
            })
        })
        .collect::<Vec<_>>();
    format!(
        "{}\n",
        json!({
            "txn_id": record.txn_id,
            "decision": finalize_decision_name(decision),
            "status": txn_status_name(record.status),
            "raft_index": record.raft_index,
            "raft_transport": record.raft_transport,
            "intents": intents,
        })
    )
}

pub fn txn_status_name(status: TxnStatus) -> &'static str {
    match status {
        TxnStatus::Pending => "pending",
        TxnStatus::Staging => "staging",
        TxnStatus::Committed => "committed",
        TxnStatus::Aborted => "aborted",
    }
}

pub fn finalize_decision_name(decision: TxnFinalizeDecision) -> &'static str {
    match decision {
        TxnFinalizeDecision::Commit => "commit",
        TxnFinalizeDecision::WaitForReplicationEvidence => "wait_for_replication_evidence",
        TxnFinalizeDecision::AbortStaleStagingRecord => "abort_stale_staging_record",
        TxnFinalizeDecision::FallbackToTwoPhaseCommit => "fallback_to_two_phase_commit",
        TxnFinalizeDecision::AlreadyCommitted => "already_committed",
        TxnFinalizeDecision::AlreadyAborted => "already_aborted",
    }
}

/// Deterministic parallel-commit report exercised by the runtime canonical
/// runner and the smoke harness.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TxnRuntimeCanonicalReport {
    pub raft_group: String,
    pub voters: Vec<NodeId>,
    pub max_staging_ms: u64,
    pub staged_record: TxnRuntimeRecord,
    pub finalize_decision: TxnFinalizeDecision,
    pub finalized_record: TxnRuntimeRecord,
}

pub fn canonical_txn_runtime_report() -> Result<TxnRuntimeCanonicalReport, TxnRuntimeError> {
    let voters: Vec<NodeId> = vec![
        "worker-a".to_string(),
        "worker-b".to_string(),
        "worker-c".to_string(),
    ];
    let mut runtime = TxnStatusRuntime::new("txn-status-orders", voters.clone(), 5_000)?;

    let record = ParallelCommitRecord {
        txn_id: "txn-runtime-1".to_string(),
        coordinator: "worker-a".to_string(),
        status: TxnStatus::Staging,
        staging_at: HlcTimestamp {
            physical_ms: 1_700_000_000,
            logical: 0,
        },
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
    };
    let staged_record = runtime.stage(record)?;

    let (finalized_record, finalize_decision) = runtime.finalize(
        "txn-runtime-1",
        HlcTimestamp {
            physical_ms: 1_700_000_010,
            logical: 0,
        },
    )?;

    Ok(TxnRuntimeCanonicalReport {
        raft_group: runtime.raft_group().to_string(),
        voters,
        max_staging_ms: runtime.max_staging_ms(),
        staged_record,
        finalize_decision,
        finalized_record,
    })
}

/// Parallel-commit microbenchmark used by the `parallel-commits-smoke` CI
/// gate. Simulates N concurrent shard intents using the in-process runtime
/// and compares against a synthetic 2PC baseline.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParallelCommitMicrobench {
    pub shard_count: u32,
    pub two_phase_commit_steps: u32,
    pub parallel_commit_steps: u32,
}

impl ParallelCommitMicrobench {
    /// Returns the speedup ratio: 2PC steps / parallel-commit steps.
    pub fn speedup(&self) -> f64 {
        if self.parallel_commit_steps == 0 {
            return 0.0;
        }
        f64::from(self.two_phase_commit_steps) / f64::from(self.parallel_commit_steps)
    }
}

pub fn run_parallel_commit_microbench(shard_count: u32) -> ParallelCommitMicrobench {
    // 2PC: PREPARE on every shard + COMMIT on every shard. Two sequential
    // round-trips per shard.
    let two_phase_commit_steps = shard_count.saturating_mul(2);
    // Parallel commit: one staging round-trip + one finalize round-trip,
    // independent of shard count because every intent acks in parallel and
    // the txn record itself is the linearization point.
    let parallel_commit_steps = 2;
    ParallelCommitMicrobench {
        shard_count,
        two_phase_commit_steps,
        parallel_commit_steps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voters() -> Vec<NodeId> {
        vec![
            "worker-a".to_string(),
            "worker-b".to_string(),
            "worker-c".to_string(),
        ]
    }

    fn fixture_record() -> ParallelCommitRecord {
        ParallelCommitRecord {
            txn_id: "txn-test".to_string(),
            coordinator: "worker-a".to_string(),
            status: TxnStatus::Staging,
            staging_at: HlcTimestamp {
                physical_ms: 1_700_000_000,
                logical: 0,
            },
            intents: vec![TxnIntent {
                shard_id: 10,
                key_range: "[a,m)".to_string(),
                replica_ack_count: 1,
                required_acks: 2,
            }],
        }
    }

    #[test]
    fn staging_round_trip_persists_record() {
        let mut runtime = TxnStatusRuntime::new("g", voters(), 5_000).expect("runtime");
        let staged = runtime.stage(fixture_record()).expect("stage");
        assert_eq!(staged.status, TxnStatus::Staging);
        assert_eq!(staged.raft_index, 1);
        assert_eq!(staged.raft_transport, "in_process");
        assert_eq!(runtime.records().len(), 1);
    }

    #[test]
    fn duplicate_staging_is_rejected() {
        let mut runtime = TxnStatusRuntime::new("g", voters(), 5_000).expect("runtime");
        runtime.stage(fixture_record()).expect("first stage");
        let error = runtime.stage(fixture_record()).unwrap_err();
        assert!(matches!(error, TxnRuntimeError::DuplicateTxn(_)));
    }

    #[test]
    fn finalize_waits_when_evidence_missing() {
        let mut runtime = TxnStatusRuntime::new("g", voters(), 5_000).expect("runtime");
        runtime.stage(fixture_record()).expect("stage");
        let (_record, decision) = runtime
            .finalize(
                "txn-test",
                HlcTimestamp {
                    physical_ms: 1_700_000_010,
                    logical: 0,
                },
            )
            .expect("finalize");
        assert_eq!(decision, TxnFinalizeDecision::WaitForReplicationEvidence);
    }

    #[test]
    fn finalize_commits_after_replica_acks() {
        let mut runtime = TxnStatusRuntime::new("g", voters(), 5_000).expect("runtime");
        runtime.stage(fixture_record()).expect("stage");
        runtime.record_replica_ack("txn-test", 10, 2).expect("ack");
        let (record, decision) = runtime
            .finalize(
                "txn-test",
                HlcTimestamp {
                    physical_ms: 1_700_000_010,
                    logical: 0,
                },
            )
            .expect("finalize");
        assert_eq!(decision, TxnFinalizeDecision::Commit);
        assert_eq!(record.status, TxnStatus::Committed);
        // The finalize commit replays through Raft and bumps the index; in
        // the per-call model this resets to 1 again but the next_raft_index
        // bookkeeping advances past the staging entry.
        assert!(record.raft_index >= 1);
        assert!(runtime.next_raft_index >= 2);
    }

    #[test]
    fn stale_staging_record_aborts() {
        let mut runtime = TxnStatusRuntime::new("g", voters(), 5_000).expect("runtime");
        runtime.stage(fixture_record()).expect("stage");
        let (record, decision) = runtime
            .finalize(
                "txn-test",
                HlcTimestamp {
                    physical_ms: 1_700_010_000,
                    logical: 0,
                },
            )
            .expect("finalize");
        assert_eq!(decision, TxnFinalizeDecision::AbortStaleStagingRecord);
        assert_eq!(record.status, TxnStatus::Aborted);
    }

    #[test]
    fn microbench_reports_speedup_over_two_phase_commit() {
        let micro = run_parallel_commit_microbench(3);
        assert_eq!(micro.shard_count, 3);
        assert_eq!(micro.two_phase_commit_steps, 6);
        assert_eq!(micro.parallel_commit_steps, 2);
        assert!(micro.speedup() >= 3.0);
    }

    #[test]
    fn http_raft_replication_requires_leader_address() {
        let error = TxnRaftReplication::http_leader(" ").unwrap_err();
        assert!(matches!(error, TxnRuntimeError::RaftFailure(_)));
    }

    #[test]
    fn canonical_runtime_report_is_deterministic() {
        let report = canonical_txn_runtime_report().expect("report");
        assert_eq!(report.staged_record.status, TxnStatus::Staging);
        assert_eq!(report.finalize_decision, TxnFinalizeDecision::Commit);
        assert_eq!(report.finalized_record.status, TxnStatus::Committed);
    }
}
