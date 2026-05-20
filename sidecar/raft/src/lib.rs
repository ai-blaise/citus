//! Raft shard-group sidecar contracts.

// FEATURE: S5

use ai_blaise_citus_sidecar_hlc::HlcTimestamp;
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RaftShardGroupPlan {
    pub shard_group: String,
    pub term: u64,
    pub leader: Option<String>,
    pub members: Vec<RaftMember>,
    pub lease: PlacementLeasePlan,
    pub placement_intents: Vec<ShardPlacementIntent>,
}

impl RaftShardGroupPlan {
    pub fn validate(&self) -> Result<(), RaftSidecarError> {
        validate_required("shard_group", &self.shard_group)?;
        if self.term == 0 {
            return Err(RaftSidecarError::InvalidTerm);
        }
        if self.members.is_empty() {
            return Err(RaftSidecarError::MissingRequiredField("members"));
        }
        for member in &self.members {
            member.validate()?;
        }
        self.lease.validate()?;
        for intent in &self.placement_intents {
            intent.validate()?;
        }
        Ok(())
    }

    pub fn quorum_size(&self) -> usize {
        let voters = self.members.iter().filter(|member| member.voter).count();
        voters / 2 + 1
    }

    pub fn failover_decision(
        &self,
        live_nodes: &[String],
        observed_at: HlcTimestamp,
    ) -> Result<FailoverDecision, RaftSidecarError> {
        self.validate()?;
        let live: BTreeSet<_> = live_nodes.iter().map(String::as_str).collect();
        let live_voters = self
            .members
            .iter()
            .filter(|member| member.voter && live.contains(member.node_id.as_str()))
            .count();
        if live_voters < self.quorum_size() {
            return Ok(FailoverDecision::WaitForQuorum);
        }
        if let Some(leader) = &self.leader {
            if live.contains(leader.as_str()) && observed_at <= self.lease.expires_at {
                return Ok(FailoverDecision::KeepLeader {
                    node_id: leader.clone(),
                });
            }
        }

        let candidate = self
            .members
            .iter()
            .filter(|member| member.voter && live.contains(member.node_id.as_str()))
            .max_by_key(|member| member.placement_generation)
            .ok_or(RaftSidecarError::NoPromotionCandidate)?;

        Ok(FailoverDecision::Promote {
            node_id: candidate.node_id.clone(),
            cnpg_pod: candidate.cnpg_pod.clone(),
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RaftMember {
    pub node_id: String,
    pub cnpg_pod: String,
    pub voter: bool,
    pub placement_generation: u64,
}

impl RaftMember {
    fn validate(&self) -> Result<(), RaftSidecarError> {
        validate_required("member.node_id", &self.node_id)?;
        validate_required("member.cnpg_pod", &self.cnpg_pod)?;
        if self.placement_generation == 0 {
            return Err(RaftSidecarError::InvalidPlacementGeneration);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlacementLeasePlan {
    pub holder: String,
    pub expires_at: HlcTimestamp,
}

impl PlacementLeasePlan {
    fn validate(&self) -> Result<(), RaftSidecarError> {
        validate_required("lease.holder", &self.holder)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ShardPlacementIntent {
    pub shard_id: u64,
    pub target_node: String,
    pub placement_generation: u64,
}

impl ShardPlacementIntent {
    fn validate(&self) -> Result<(), RaftSidecarError> {
        if self.shard_id == 0 {
            return Err(RaftSidecarError::InvalidShardId);
        }
        validate_required("placement.target_node", &self.target_node)?;
        if self.placement_generation == 0 {
            return Err(RaftSidecarError::InvalidPlacementGeneration);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FailoverDecision {
    KeepLeader { node_id: String },
    Promote { node_id: String, cnpg_pod: String },
    WaitForQuorum,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RaftSidecarError {
    InvalidPlacementGeneration,
    InvalidShardId,
    InvalidTerm,
    MissingRequiredField(&'static str),
    NoPromotionCandidate,
}

impl fmt::Display for RaftSidecarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPlacementGeneration => {
                write!(formatter, "placement_generation must be greater than zero")
            }
            Self::InvalidShardId => write!(formatter, "shard_id must be greater than zero"),
            Self::InvalidTerm => write!(formatter, "term must be greater than zero"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::NoPromotionCandidate => write!(formatter, "no live voter can be promoted"),
        }
    }
}

impl Error for RaftSidecarError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), RaftSidecarError> {
    if value.trim().is_empty() {
        return Err(RaftSidecarError::MissingRequiredField(field));
    }
    Ok(())
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RaftCanonicalReport {
    pub plan: RaftShardGroupPlan,
    pub observed_at: HlcTimestamp,
    pub live_nodes: Vec<String>,
    pub decision: FailoverDecision,
}

pub fn canonical_raft_plan() -> RaftShardGroupPlan {
    RaftShardGroupPlan {
        shard_group: "orders-sg".to_string(),
        term: 7,
        leader: Some("worker-a".to_string()),
        members: vec![
            RaftMember {
                node_id: "worker-a".to_string(),
                cnpg_pod: "orders-0".to_string(),
                voter: true,
                placement_generation: 3,
            },
            RaftMember {
                node_id: "worker-b".to_string(),
                cnpg_pod: "orders-1".to_string(),
                voter: true,
                placement_generation: 4,
            },
            RaftMember {
                node_id: "worker-c".to_string(),
                cnpg_pod: "orders-2".to_string(),
                voter: true,
                placement_generation: 5,
            },
        ],
        lease: PlacementLeasePlan {
            holder: "worker-a".to_string(),
            expires_at: timestamp(1_700_005_000),
        },
        placement_intents: vec![ShardPlacementIntent {
            shard_id: 10,
            target_node: "worker-c".to_string(),
            placement_generation: 5,
        }],
    }
}

pub fn canonical_raft_report() -> Result<RaftCanonicalReport, RaftSidecarError> {
    let plan = canonical_raft_plan();
    let observed_at = timestamp(1_700_010_000);
    let live_nodes = vec!["worker-b".to_string(), "worker-c".to_string()];
    let decision = plan.failover_decision(&live_nodes, observed_at)?;

    Ok(RaftCanonicalReport {
        plan,
        observed_at,
        live_nodes,
        decision,
    })
}

pub fn timestamp(physical_ms: u64) -> HlcTimestamp {
    HlcTimestamp {
        physical_ms,
        logical: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_leader_with_valid_lease_is_kept() {
        let decision = valid_plan()
            .failover_decision(
                &["worker-a".to_string(), "worker-b".to_string()],
                timestamp(1_700_000_000),
            )
            .expect("decision");

        assert_eq!(
            decision,
            FailoverDecision::KeepLeader {
                node_id: "worker-a".to_string()
            }
        );
    }

    #[test]
    fn expired_leader_promotes_live_voter_with_highest_generation() {
        let decision = valid_plan()
            .failover_decision(
                &["worker-b".to_string(), "worker-c".to_string()],
                timestamp(1_700_010_000),
            )
            .expect("decision");

        assert_eq!(
            decision,
            FailoverDecision::Promote {
                node_id: "worker-c".to_string(),
                cnpg_pod: "orders-2".to_string(),
            }
        );
    }

    #[test]
    fn missing_quorum_waits() {
        let decision = valid_plan()
            .failover_decision(&["worker-b".to_string()], timestamp(1_700_010_000))
            .expect("decision");

        assert_eq!(decision, FailoverDecision::WaitForQuorum);
    }

    #[test]
    fn placement_intent_requires_shard_id() {
        let mut plan = valid_plan();
        plan.placement_intents[0].shard_id = 0;

        assert_eq!(plan.validate(), Err(RaftSidecarError::InvalidShardId));
    }

    #[test]
    fn canonical_report_promotes_highest_generation_live_voter() {
        let report = canonical_raft_report().expect("canonical report");

        assert_eq!(report.plan.quorum_size(), 2);
        assert_eq!(
            report.decision,
            FailoverDecision::Promote {
                node_id: "worker-c".to_string(),
                cnpg_pod: "orders-2".to_string(),
            }
        );
    }

    fn valid_plan() -> RaftShardGroupPlan {
        canonical_raft_plan()
    }
}
