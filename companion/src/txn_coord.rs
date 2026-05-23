//! Parallel-commit coordinator companion module.
//!
//! Renders the SQL-callable contract for `companion.txn_stage` and
//! `companion.txn_finalize`, plus the deterministic fallback decision that
//! routes coordinators back to standard distributed 2PC when the
//! `txn_status` sidecar is unreachable. The actual HTTP call to the sidecar
//! lives in the SQL runtime (`images/citus-pg-overlay/extensions/...`); the
//! Rust side just owns the shape and the failure semantics.

// FEATURE: T5
// FEATURE: T5

use std::error::Error;
use std::fmt;

/// Stage request that a coordinator posts to `companion.txn_stage(...)`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TxnStageRequest {
    pub txn_id: String,
    pub coordinator: String,
    pub staging_physical_ms: u64,
    pub intents: Vec<TxnStageIntent>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TxnStageIntent {
    pub shard_id: u64,
    pub key_range: String,
    pub required_acks: u32,
}

impl TxnStageRequest {
    pub fn validate(&self) -> Result<(), TxnCoordError> {
        if self.txn_id.trim().is_empty() {
            return Err(TxnCoordError::MissingRequiredField("txn_id"));
        }
        if self.coordinator.trim().is_empty() {
            return Err(TxnCoordError::MissingRequiredField("coordinator"));
        }
        if self.staging_physical_ms == 0 {
            return Err(TxnCoordError::InvalidStagingTime);
        }
        if self.intents.is_empty() {
            return Err(TxnCoordError::MissingRequiredField("intents"));
        }
        for intent in &self.intents {
            intent.validate()?;
        }
        Ok(())
    }

    /// Render the SQL plan a SQL-runtime function executes when called as
    /// `select companion.txn_stage(<txn_id>, <jsonb_intents>)`. This is a
    /// stable shape, NOT an end-to-end execution promise.
    pub fn to_sql_plan(&self) -> Result<TxnCoordSqlPlan, TxnCoordError> {
        self.validate()?;
        let intents_json = render_intents_json(&self.intents);
        Ok(TxnCoordSqlPlan {
            feature_id: "T5",
            commands: vec![format!(
                "SELECT companion.txn_stage('{}', '{}', {}, '{}'::jsonb);",
                escape_sql(&self.txn_id),
                escape_sql(&self.coordinator),
                self.staging_physical_ms,
                escape_sql(&intents_json),
            )],
        })
    }
}

impl TxnStageIntent {
    pub fn validate(&self) -> Result<(), TxnCoordError> {
        if self.shard_id == 0 {
            return Err(TxnCoordError::InvalidShardId);
        }
        if self.key_range.trim().is_empty() {
            return Err(TxnCoordError::MissingRequiredField("intent.key_range"));
        }
        if self.required_acks == 0 {
            return Err(TxnCoordError::InvalidRequiredAcks);
        }
        Ok(())
    }
}

/// Finalize request that a coordinator posts to `companion.txn_finalize(...)`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TxnFinalizeRequest {
    pub txn_id: String,
    pub observed_physical_ms: u64,
}

impl TxnFinalizeRequest {
    pub fn validate(&self) -> Result<(), TxnCoordError> {
        if self.txn_id.trim().is_empty() {
            return Err(TxnCoordError::MissingRequiredField("txn_id"));
        }
        if self.observed_physical_ms == 0 {
            return Err(TxnCoordError::InvalidObservedTime);
        }
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<TxnCoordSqlPlan, TxnCoordError> {
        self.validate()?;
        Ok(TxnCoordSqlPlan {
            feature_id: "T5",
            commands: vec![format!(
                "SELECT companion.txn_finalize('{}', {});",
                escape_sql(&self.txn_id),
                self.observed_physical_ms,
            )],
        })
    }
}

/// Decision the coordinator records after consulting `txn_status`. When the
/// sidecar is unreachable, the coordinator falls back to standard 2PC.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TxnCoordDecision {
    UseParallelCommitFastPath {
        sidecar_address: String,
        request: TxnStageRequest,
    },
    FallbackToTwoPhaseCommit {
        reason: String,
        request: TxnStageRequest,
    },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TxnCoordRoutingPlan {
    pub sidecar_address: String,
    pub sidecar_reachable: bool,
    pub request: TxnStageRequest,
}

impl TxnCoordRoutingPlan {
    pub fn validate(&self) -> Result<(), TxnCoordError> {
        if self.sidecar_address.trim().is_empty() {
            return Err(TxnCoordError::MissingRequiredField("sidecar_address"));
        }
        self.request.validate()
    }

    pub fn decide(&self) -> Result<TxnCoordDecision, TxnCoordError> {
        self.validate()?;
        if self.sidecar_reachable {
            Ok(TxnCoordDecision::UseParallelCommitFastPath {
                sidecar_address: self.sidecar_address.clone(),
                request: self.request.clone(),
            })
        } else {
            Ok(TxnCoordDecision::FallbackToTwoPhaseCommit {
                reason: "txn_status sidecar unreachable".to_string(),
                request: self.request.clone(),
            })
        }
    }
}

/// SQL command envelope for the txn coordinator runtime, mirroring the
/// router_assist::RouterAssistSqlPlan shape.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TxnCoordSqlPlan {
    pub feature_id: &'static str,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TxnCoordError {
    InvalidObservedTime,
    InvalidRequiredAcks,
    InvalidShardId,
    InvalidStagingTime,
    MissingRequiredField(&'static str),
}

impl fmt::Display for TxnCoordError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidObservedTime => {
                write!(formatter, "observed_physical_ms must be greater than zero")
            }
            Self::InvalidRequiredAcks => {
                write!(formatter, "required_acks must be greater than zero")
            }
            Self::InvalidShardId => write!(formatter, "shard_id must be greater than zero"),
            Self::InvalidStagingTime => {
                write!(formatter, "staging_physical_ms must be greater than zero")
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
        }
    }
}

impl Error for TxnCoordError {}

pub fn canonical_txn_coord_request() -> TxnStageRequest {
    TxnStageRequest {
        txn_id: "txn-companion-1".to_string(),
        coordinator: "worker-a".to_string(),
        staging_physical_ms: 1_700_000_000,
        intents: vec![
            TxnStageIntent {
                shard_id: 10,
                key_range: "[a,m)".to_string(),
                required_acks: 2,
            },
            TxnStageIntent {
                shard_id: 11,
                key_range: "[m,z)".to_string(),
                required_acks: 2,
            },
        ],
    }
}

pub fn canonical_txn_coord_routing_plan() -> TxnCoordRoutingPlan {
    TxnCoordRoutingPlan {
        sidecar_address: "txn-status.ai-blaise.svc.cluster.local:8080".to_string(),
        sidecar_reachable: true,
        request: canonical_txn_coord_request(),
    }
}

fn render_intents_json(intents: &[TxnStageIntent]) -> String {
    let parts = intents
        .iter()
        .map(|intent| {
            format!(
                "{{\"shard_id\":{},\"key_range\":\"{}\",\"required_acks\":{}}}",
                intent.shard_id,
                escape_json(&intent.key_range),
                intent.required_acks
            )
        })
        .collect::<Vec<_>>();
    format!("[{}]", parts.join(","))
}

fn escape_sql(value: &str) -> String {
    value.replace('\'', "''")
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_request_renders_sql_plan() {
        let plan = canonical_txn_coord_request().to_sql_plan().expect("plan");
        assert_eq!(plan.feature_id, "T5");
        assert_eq!(plan.commands.len(), 1);
        let command = &plan.commands[0];
        assert!(command.contains("companion.txn_stage"));
        assert!(command.contains("txn-companion-1"));
        assert!(command.contains("\"shard_id\":10"));
        assert!(command.contains("\"key_range\":\"[a,m)\""));
    }

    #[test]
    fn stage_request_validates_intents() {
        let mut request = canonical_txn_coord_request();
        request.intents[0].shard_id = 0;
        assert_eq!(request.validate(), Err(TxnCoordError::InvalidShardId));
    }

    #[test]
    fn finalize_request_renders_sql_plan() {
        let plan = TxnFinalizeRequest {
            txn_id: "txn-companion-1".to_string(),
            observed_physical_ms: 1_700_000_010,
        }
        .to_sql_plan()
        .expect("plan");
        assert!(plan.commands[0].contains("companion.txn_finalize"));
        assert!(plan.commands[0].contains("1700000010"));
    }

    #[test]
    fn unreachable_sidecar_falls_back_to_two_phase_commit() {
        let plan = TxnCoordRoutingPlan {
            sidecar_address: "txn-status.ai-blaise.svc:8080".to_string(),
            sidecar_reachable: false,
            request: canonical_txn_coord_request(),
        };
        let decision = plan.decide().expect("decision");
        match decision {
            TxnCoordDecision::FallbackToTwoPhaseCommit { reason, .. } => {
                assert!(reason.contains("txn_status sidecar unreachable"));
            }
            other => panic!("expected fallback, got {other:?}"),
        }
    }

    #[test]
    fn reachable_sidecar_uses_parallel_commit_fast_path() {
        let plan = canonical_txn_coord_routing_plan();
        let decision = plan.decide().expect("decision");
        assert!(matches!(
            decision,
            TxnCoordDecision::UseParallelCommitFastPath { .. }
        ));
    }
}
