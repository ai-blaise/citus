// FEATURE: MR2
// FEATURE: S11

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use crate::crds::region::RegionSpec;
use crate::crds::shard_group::{PlacementPolicy, ShardGroupSpec};
use crate::crds::survival_goal::{SurvivalGoalSpec, SurvivalGoalSpecError, SurvivalGoalType};

const ZONE_TOPOLOGY_KEY: &str = "topology.kubernetes.io/zone";
const REGION_TOPOLOGY_KEY: &str = "topology.kubernetes.io/region";
const PGACTIVE_EXTENSION: &str = "pgactive";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SurvivalGoalReconcilePlan {
    pub spec: SurvivalGoalSpec,
    pub required_topology_key: String,
    pub steps: Vec<SurvivalGoalApplyStep>,
}

impl TryFrom<&SurvivalGoalSpec> for SurvivalGoalReconcilePlan {
    type Error = SurvivalGoalReconcileError;

    fn try_from(spec: &SurvivalGoalSpec) -> Result<Self, Self::Error> {
        Self::from_spec(spec)
    }
}

impl SurvivalGoalReconcilePlan {
    pub fn from_spec(spec: &SurvivalGoalSpec) -> Result<Self, SurvivalGoalReconcileError> {
        spec.validate()?;
        validate_distinct_regions(spec)?;
        let required_topology_key = required_topology_key(spec.goal).to_string();
        let steps = build_survival_apply_steps(spec, &required_topology_key);

        Ok(Self {
            spec: spec.clone(),
            required_topology_key,
            steps,
        })
    }

    pub fn new(
        spec: &SurvivalGoalSpec,
        shard_groups: &[ShardGroupSpec],
        regions: &[RegionSpec],
    ) -> Result<Self, SurvivalGoalReconcileError> {
        let plan = Self::from_spec(spec)?;
        for shard_group in shard_groups {
            shard_group.validate().map_err(|error| {
                SurvivalGoalReconcileError::InvalidShardGroup(error.to_string())
            })?;
        }
        for region in regions {
            region
                .validate()
                .map_err(|error| SurvivalGoalReconcileError::InvalidRegion(error.to_string()))?;
        }

        validate_regions_declared(spec, regions)?;
        validate_placements_satisfy_goal(spec, shard_groups)?;

        Ok(plan)
    }

    pub fn required_topology_key(&self) -> &str {
        &self.required_topology_key
    }
}

fn validate_distinct_regions(spec: &SurvivalGoalSpec) -> Result<(), SurvivalGoalReconcileError> {
    let mut seen = BTreeSet::new();
    for region in &spec.regions {
        if !seen.insert(region) {
            return Err(SurvivalGoalReconcileError::DuplicateRegion(region.clone()));
        }
    }
    Ok(())
}

fn validate_regions_declared(
    spec: &SurvivalGoalSpec,
    regions: &[RegionSpec],
) -> Result<(), SurvivalGoalReconcileError> {
    for required_region in &spec.regions {
        if !regions.iter().any(|region| &region.name == required_region) {
            return Err(SurvivalGoalReconcileError::UndeclaredRegion(
                required_region.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_placements_satisfy_goal(
    spec: &SurvivalGoalSpec,
    shard_groups: &[ShardGroupSpec],
) -> Result<(), SurvivalGoalReconcileError> {
    if shard_groups.is_empty() {
        return Err(SurvivalGoalReconcileError::NoShardGroups);
    }

    let required_topology_key = required_topology_key(spec.goal);
    let min_replicas = spec.min_replicas;

    for shard_group in shard_groups {
        if shard_group.replication_factor < min_replicas {
            return Err(SurvivalGoalReconcileError::InsufficientReplicationFactor {
                shard_group: shard_group.parent_table.clone(),
                replication_factor: shard_group.replication_factor,
                required: min_replicas,
            });
        }

        let satisfying_policy = shard_group
            .placement_policy
            .iter()
            .find(|policy| policy.topology_key == required_topology_key);
        let Some(policy) = satisfying_policy else {
            return Err(SurvivalGoalReconcileError::MissingTopologySpread {
                shard_group: shard_group.parent_table.clone(),
                topology_key: required_topology_key.to_string(),
            });
        };
        validate_policy_skew(spec.goal, shard_group, policy)?;
    }

    Ok(())
}

fn validate_policy_skew(
    goal: SurvivalGoalType,
    shard_group: &ShardGroupSpec,
    policy: &PlacementPolicy,
) -> Result<(), SurvivalGoalReconcileError> {
    if matches!(goal, SurvivalGoalType::RegionFailure) && policy.max_skew > 1 {
        return Err(SurvivalGoalReconcileError::PlacementSkewTooLoose {
            shard_group: shard_group.parent_table.clone(),
            max_skew: policy.max_skew,
        });
    }
    Ok(())
}

fn build_survival_apply_steps(
    spec: &SurvivalGoalSpec,
    required_topology_key: &str,
) -> Vec<SurvivalGoalApplyStep> {
    let mut steps = Vec::new();

    steps.push(SurvivalGoalApplyStep::policy_record(
        "record_survival_goal_status",
        "MR2",
        format!(
            "goal={};regions={};min_replicas={};topology_key={required_topology_key}",
            survival_goal_label(spec.goal),
            spec.regions.join(","),
            spec.min_replicas
        ),
        true,
    ));

    match spec.goal {
        SurvivalGoalType::ZoneFailure => {
            steps.push(SurvivalGoalApplyStep::topology_spread(
                "enforce_zone_topology_spread",
                "S11",
                format!("topologyKey={ZONE_TOPOLOGY_KEY},maxSkew=1"),
                true,
            ));
        }
        SurvivalGoalType::RegionFailure => {
            steps.push(SurvivalGoalApplyStep::topology_spread(
                "enforce_region_topology_spread",
                "MR2",
                format!("topologyKey={REGION_TOPOLOGY_KEY},maxSkew=1"),
                true,
            ));
            steps.push(SurvivalGoalApplyStep::pgactive(
                "require_pgactive_cross_region",
                "MR2",
                format!(
                    "extension={PGACTIVE_EXTENSION};regions={};min_replicas={}",
                    spec.regions.join(","),
                    spec.min_replicas
                ),
                true,
            ));
            steps.push(SurvivalGoalApplyStep::cnpg_cluster(
                "update_cnpg_replication_policy",
                "MR2",
                format!(
                    "replicas={};regions={}",
                    spec.min_replicas,
                    spec.regions.join(",")
                ),
                true,
            ));
        }
    }

    steps
}

fn required_topology_key(goal: SurvivalGoalType) -> &'static str {
    match goal {
        SurvivalGoalType::ZoneFailure => ZONE_TOPOLOGY_KEY,
        SurvivalGoalType::RegionFailure => REGION_TOPOLOGY_KEY,
    }
}

fn survival_goal_label(goal: SurvivalGoalType) -> &'static str {
    match goal {
        SurvivalGoalType::ZoneFailure => "zone-failure",
        SurvivalGoalType::RegionFailure => "region-failure",
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SurvivalGoalApplyStep {
    pub name: String,
    pub feature_id: String,
    pub kind: SurvivalGoalApplyStepKind,
    pub payload: String,
    pub idempotent: bool,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SurvivalGoalApplyStepKind {
    PolicyRecord,
    TopologySpread,
    Pgactive,
    CnpgCluster,
}

impl SurvivalGoalApplyStep {
    fn policy_record(
        name: impl Into<String>,
        feature_id: impl Into<String>,
        payload: impl Into<String>,
        idempotent: bool,
    ) -> Self {
        Self {
            name: name.into(),
            feature_id: feature_id.into(),
            kind: SurvivalGoalApplyStepKind::PolicyRecord,
            payload: payload.into(),
            idempotent,
        }
    }

    fn topology_spread(
        name: impl Into<String>,
        feature_id: impl Into<String>,
        constraint: impl Into<String>,
        idempotent: bool,
    ) -> Self {
        Self {
            name: name.into(),
            feature_id: feature_id.into(),
            kind: SurvivalGoalApplyStepKind::TopologySpread,
            payload: constraint.into(),
            idempotent,
        }
    }

    fn pgactive(
        name: impl Into<String>,
        feature_id: impl Into<String>,
        config: impl Into<String>,
        idempotent: bool,
    ) -> Self {
        Self {
            name: name.into(),
            feature_id: feature_id.into(),
            kind: SurvivalGoalApplyStepKind::Pgactive,
            payload: config.into(),
            idempotent,
        }
    }

    fn cnpg_cluster(
        name: impl Into<String>,
        feature_id: impl Into<String>,
        config: impl Into<String>,
        idempotent: bool,
    ) -> Self {
        Self {
            name: name.into(),
            feature_id: feature_id.into(),
            kind: SurvivalGoalApplyStepKind::CnpgCluster,
            payload: config.into(),
            idempotent,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SurvivalGoalReconcileError {
    InvalidSpec(SurvivalGoalSpecError),
    InvalidShardGroup(String),
    InvalidRegion(String),
    DuplicateRegion(String),
    UndeclaredRegion(String),
    NoShardGroups,
    MissingTopologySpread {
        shard_group: String,
        topology_key: String,
    },
    PlacementSkewTooLoose {
        shard_group: String,
        max_skew: u32,
    },
    InsufficientReplicationFactor {
        shard_group: String,
        replication_factor: u32,
        required: u32,
    },
}

impl fmt::Display for SurvivalGoalReconcileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSpec(error) => write!(formatter, "{error}"),
            Self::InvalidShardGroup(message) => {
                write!(formatter, "invalid shard group: {message}")
            }
            Self::InvalidRegion(message) => write!(formatter, "invalid region: {message}"),
            Self::DuplicateRegion(region) => write!(
                formatter,
                "survival goal references region {region} more than once"
            ),
            Self::UndeclaredRegion(region) => write!(
                formatter,
                "survival goal references region {region} which has no Region CR"
            ),
            Self::NoShardGroups => write!(
                formatter,
                "survival goal requires at least one ShardGroup placement inventory entry"
            ),
            Self::MissingTopologySpread {
                shard_group,
                topology_key,
            } => write!(
                formatter,
                "shard group {shard_group} lacks a placement policy for topology key {topology_key}"
            ),
            Self::PlacementSkewTooLoose {
                shard_group,
                max_skew,
            } => write!(
                formatter,
                "shard group {shard_group} max_skew={max_skew} cannot survive region failure (require max_skew=1)"
            ),
            Self::InsufficientReplicationFactor {
                shard_group,
                replication_factor,
                required,
            } => write!(
                formatter,
                "shard group {shard_group} replication_factor={replication_factor} is below survival goal min_replicas={required}"
            ),
        }
    }
}

impl Error for SurvivalGoalReconcileError {}

impl From<SurvivalGoalSpecError> for SurvivalGoalReconcileError {
    fn from(error: SurvivalGoalSpecError) -> Self {
        Self::InvalidSpec(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crds::shard_group::{PlacementPolicy, UnsatisfiablePlacementAction};

    fn region(name: &str, zone: &str) -> RegionSpec {
        RegionSpec {
            name: name.to_string(),
            kubernetes_zone: zone.to_string(),
            tablespace_name: format!("ts_{}", name.replace('-', "_")),
            leader_pinned: false,
        }
    }

    fn shard_group_with_policy(
        parent_table: &str,
        replication_factor: u32,
        policies: Vec<PlacementPolicy>,
    ) -> ShardGroupSpec {
        ShardGroupSpec {
            parent_table: parent_table.to_string(),
            distribution_column: "tenant_id".to_string(),
            num_shards: 32,
            colocation_group: None,
            replication_factor,
            placement_policy: policies,
        }
    }

    fn zone_policy() -> PlacementPolicy {
        PlacementPolicy {
            topology_key: ZONE_TOPOLOGY_KEY.to_string(),
            max_skew: 1,
            when_unsatisfiable: UnsatisfiablePlacementAction::DoNotSchedule,
        }
    }

    fn region_policy(max_skew: u32) -> PlacementPolicy {
        PlacementPolicy {
            topology_key: REGION_TOPOLOGY_KEY.to_string(),
            max_skew,
            when_unsatisfiable: UnsatisfiablePlacementAction::DoNotSchedule,
        }
    }

    #[test]
    fn zone_failure_plan_emits_topology_spread() {
        let spec = SurvivalGoalSpec {
            goal: SurvivalGoalType::ZoneFailure,
            regions: vec!["us-east-1".to_string(), "us-west-2".to_string()],
            min_replicas: 2,
        };
        let regions = vec![
            region("us-east-1", "us-east-1a"),
            region("us-west-2", "us-west-2a"),
        ];
        let shard_groups = vec![shard_group_with_policy(
            "public.metrics",
            3,
            vec![zone_policy()],
        )];

        let plan = SurvivalGoalReconcilePlan::new(&spec, &shard_groups, &regions)
            .expect("valid zone failure plan");

        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.required_topology_key(), ZONE_TOPOLOGY_KEY);
        assert_eq!(plan.steps[0].feature_id, "MR2");
        assert_eq!(
            plan.steps[1].kind,
            SurvivalGoalApplyStepKind::TopologySpread
        );
        assert!(plan.steps[1].payload.contains(ZONE_TOPOLOGY_KEY));
    }

    #[test]
    fn region_failure_plan_emits_pgactive_and_cnpg() {
        let spec = SurvivalGoalSpec {
            goal: SurvivalGoalType::RegionFailure,
            regions: vec!["us-east-1".to_string(), "us-west-2".to_string()],
            min_replicas: 2,
        };
        let regions = vec![
            region("us-east-1", "us-east-1a"),
            region("us-west-2", "us-west-2a"),
        ];
        let shard_groups = vec![shard_group_with_policy(
            "public.metrics",
            3,
            vec![region_policy(1)],
        )];

        let plan = SurvivalGoalReconcilePlan::new(&spec, &shard_groups, &regions)
            .expect("valid region failure plan");

        assert_eq!(plan.steps.len(), 4);
        assert_eq!(plan.required_topology_key(), REGION_TOPOLOGY_KEY);
        assert_eq!(plan.steps[2].kind, SurvivalGoalApplyStepKind::Pgactive);
        assert!(plan.steps[2].payload.contains("pgactive"));
        assert_eq!(plan.steps[3].kind, SurvivalGoalApplyStepKind::CnpgCluster);
        assert!(plan.steps[3].payload.contains("replicas=2"));
    }

    #[test]
    fn region_failure_rejects_missing_region_policy() {
        let spec = SurvivalGoalSpec {
            goal: SurvivalGoalType::RegionFailure,
            regions: vec!["us-east-1".to_string(), "us-west-2".to_string()],
            min_replicas: 2,
        };
        let regions = vec![
            region("us-east-1", "us-east-1a"),
            region("us-west-2", "us-west-2a"),
        ];
        let shard_groups = vec![shard_group_with_policy(
            "public.metrics",
            3,
            vec![zone_policy()],
        )];

        let result = SurvivalGoalReconcilePlan::new(&spec, &shard_groups, &regions);
        assert!(matches!(
            result,
            Err(SurvivalGoalReconcileError::MissingTopologySpread { .. })
        ));
    }

    #[test]
    fn region_failure_rejects_loose_skew() {
        let spec = SurvivalGoalSpec {
            goal: SurvivalGoalType::RegionFailure,
            regions: vec!["us-east-1".to_string(), "us-west-2".to_string()],
            min_replicas: 2,
        };
        let regions = vec![
            region("us-east-1", "us-east-1a"),
            region("us-west-2", "us-west-2a"),
        ];
        let shard_groups = vec![shard_group_with_policy(
            "public.metrics",
            3,
            vec![region_policy(3)],
        )];

        let result = SurvivalGoalReconcilePlan::new(&spec, &shard_groups, &regions);
        assert!(matches!(
            result,
            Err(SurvivalGoalReconcileError::PlacementSkewTooLoose { .. })
        ));
    }

    #[test]
    fn survival_goal_rejects_undeclared_region() {
        let spec = SurvivalGoalSpec {
            goal: SurvivalGoalType::RegionFailure,
            regions: vec!["us-east-1".to_string(), "us-west-2".to_string()],
            min_replicas: 2,
        };
        let regions = vec![region("us-east-1", "us-east-1a")];

        let result = SurvivalGoalReconcilePlan::new(&spec, &[], &regions);
        assert!(matches!(
            result,
            Err(SurvivalGoalReconcileError::UndeclaredRegion(name)) if name == "us-west-2"
        ));
    }

    #[test]
    fn survival_goal_rejects_low_replication_factor() {
        let spec = SurvivalGoalSpec {
            goal: SurvivalGoalType::ZoneFailure,
            regions: vec![
                "us-east-1".to_string(),
                "us-west-2".to_string(),
                "eu-central-1".to_string(),
            ],
            min_replicas: 3,
        };
        let regions = vec![
            region("us-east-1", "us-east-1a"),
            region("us-west-2", "us-west-2a"),
            region("eu-central-1", "eu-central-1a"),
        ];
        let shard_groups = vec![shard_group_with_policy(
            "public.metrics",
            2,
            vec![zone_policy()],
        )];

        let result = SurvivalGoalReconcilePlan::new(&spec, &shard_groups, &regions);
        assert!(matches!(
            result,
            Err(SurvivalGoalReconcileError::InsufficientReplicationFactor { .. })
        ));
    }

    #[test]
    fn survival_goal_rejects_duplicate_regions() {
        let spec = SurvivalGoalSpec {
            goal: SurvivalGoalType::ZoneFailure,
            regions: vec!["us-east-1".to_string(), "us-east-1".to_string()],
            min_replicas: 1,
        };

        assert!(matches!(
            SurvivalGoalReconcilePlan::from_spec(&spec),
            Err(SurvivalGoalReconcileError::DuplicateRegion(region)) if region == "us-east-1"
        ));
    }
}
