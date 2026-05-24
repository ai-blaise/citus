//! Raft shard-group sidecar contracts.

// FEATURE: S5

pub mod runtime;

pub use runtime::{
    run_durable_log_snapshot_round_trip, run_raft_round_trip, AppendEntries, AppendResponse,
    LogIndex, NodeId, RaftDurableLogReport, RaftDurableStore, RaftEntry, RaftMessage, RaftNode,
    RaftRole, RaftRoundTripReport, RaftRuntimeError, RaftSnapshot, Term, VoteRequest, VoteResponse,
};

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
        let mut member_ids = BTreeSet::new();
        let mut voter_count = 0;
        for member in &self.members {
            member.validate()?;
            if !member_ids.insert(member.node_id.as_str()) {
                return Err(RaftSidecarError::DuplicateMember(member.node_id.clone()));
            }
            if member.voter {
                voter_count += 1;
            }
        }
        if voter_count == 0 {
            return Err(RaftSidecarError::NoVotingMembers);
        }

        self.lease.validate()?;
        if self.member_by_id(&self.lease.holder).is_none() {
            return Err(RaftSidecarError::UnknownMemberReference {
                field: "lease.holder",
                value: self.lease.holder.clone(),
            });
        }

        if let Some(leader) = &self.leader {
            let leader_member = self.member_by_id(leader).ok_or_else(|| {
                RaftSidecarError::UnknownMemberReference {
                    field: "leader",
                    value: leader.clone(),
                }
            })?;
            if !leader_member.voter {
                return Err(RaftSidecarError::LeaderMustBeVoter(leader.clone()));
            }
            if self.lease.holder != *leader {
                return Err(RaftSidecarError::LeaseHolderMismatch {
                    leader: leader.clone(),
                    holder: self.lease.holder.clone(),
                });
            }
        }

        for intent in &self.placement_intents {
            intent.validate()?;
            let target = self.member_by_id(&intent.target_node).ok_or_else(|| {
                RaftSidecarError::UnknownMemberReference {
                    field: "placement.target_node",
                    value: intent.target_node.clone(),
                }
            })?;
            if intent.placement_generation < target.placement_generation {
                return Err(RaftSidecarError::InvalidPlacementGeneration);
            }
        }
        Ok(())
    }

    fn member_by_id(&self, node_id: &str) -> Option<&RaftMember> {
        self.members.iter().find(|member| member.node_id == node_id)
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
        for node in live_nodes {
            if self.member_by_id(node).is_none() {
                return Err(RaftSidecarError::UnknownLiveNode(node.clone()));
            }
        }
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
    DuplicateMember(String),
    InvalidPlacementGeneration,
    InvalidShardId,
    InvalidTerm,
    LeaderMustBeVoter(String),
    LeaseHolderMismatch { leader: String, holder: String },
    MissingRequiredField(&'static str),
    NoPromotionCandidate,
    NoVotingMembers,
    UnknownLiveNode(String),
    UnknownMemberReference { field: &'static str, value: String },
}

impl fmt::Display for RaftSidecarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateMember(node) => write!(formatter, "duplicate raft member: {node}"),
            Self::InvalidPlacementGeneration => {
                write!(
                    formatter,
                    "placement_generation must be greater than zero and not move backwards"
                )
            }
            Self::InvalidShardId => write!(formatter, "shard_id must be greater than zero"),
            Self::InvalidTerm => write!(formatter, "term must be greater than zero"),
            Self::LeaderMustBeVoter(node) => {
                write!(formatter, "leader must be a voting raft member: {node}")
            }
            Self::LeaseHolderMismatch { leader, holder } => write!(
                formatter,
                "lease holder {holder} does not match current leader {leader}"
            ),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::NoPromotionCandidate => write!(formatter, "no live voter can be promoted"),
            Self::NoVotingMembers => {
                write!(formatter, "raft group must include at least one voter")
            }
            Self::UnknownLiveNode(node) => {
                write!(formatter, "live node is not a raft member: {node}")
            }
            Self::UnknownMemberReference { field, value } => {
                write!(formatter, "{field} references unknown raft member {value}")
            }
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

/// Deterministic 3-node Raft round-trip used by the runtime canonical runner
/// and the sidecar-raft smoke test.
pub fn canonical_raft_runtime_report() -> Result<RaftRoundTripReport, RaftRuntimeError> {
    let members = vec![
        "worker-a".to_string(),
        "worker-b".to_string(),
        "worker-c".to_string(),
    ];
    run_raft_round_trip(members, "worker-a", b"shard-placement-canonical".to_vec())
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
    fn duplicate_members_are_rejected() {
        let mut plan = valid_plan();
        plan.members[1].node_id = "worker-a".to_string();

        assert_eq!(
            plan.validate(),
            Err(RaftSidecarError::DuplicateMember("worker-a".to_string()))
        );
    }

    #[test]
    fn leader_must_be_known_voter() {
        let mut plan = valid_plan();
        plan.members[0].voter = false;

        assert_eq!(
            plan.validate(),
            Err(RaftSidecarError::LeaderMustBeVoter("worker-a".to_string()))
        );
    }

    #[test]
    fn lease_holder_must_match_leader() {
        let mut plan = valid_plan();
        plan.lease.holder = "worker-b".to_string();

        assert_eq!(
            plan.validate(),
            Err(RaftSidecarError::LeaseHolderMismatch {
                leader: "worker-a".to_string(),
                holder: "worker-b".to_string(),
            })
        );
    }

    #[test]
    fn placement_target_must_be_known_member() {
        let mut plan = valid_plan();
        plan.placement_intents[0].target_node = "worker-z".to_string();

        assert_eq!(
            plan.validate(),
            Err(RaftSidecarError::UnknownMemberReference {
                field: "placement.target_node",
                value: "worker-z".to_string(),
            })
        );
    }

    #[test]
    fn unknown_live_node_is_rejected() {
        let error = valid_plan()
            .failover_decision(
                &["worker-b".to_string(), "worker-z".to_string()],
                timestamp(1_700_010_000),
            )
            .unwrap_err();

        assert_eq!(
            error,
            RaftSidecarError::UnknownLiveNode("worker-z".to_string())
        );
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
