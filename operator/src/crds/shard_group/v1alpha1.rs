// FEATURE: S2

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ShardGroupSpec {
    pub parent_table: String,
    pub distribution_column: String,
    pub num_shards: u32,
    pub colocation_group: Option<String>,
    pub replication_factor: u32,
    pub placement_policy: Vec<PlacementPolicy>,
}

impl ShardGroupSpec {
    pub fn validate(&self) -> Result<(), ShardGroupSpecError> {
        validate_required("parent_table", &self.parent_table)?;
        validate_required("distribution_column", &self.distribution_column)?;
        validate_optional("colocation_group", &self.colocation_group)?;

        if self.num_shards == 0 {
            return Err(ShardGroupSpecError::InvalidShardCount);
        }
        if self.replication_factor == 0 {
            return Err(ShardGroupSpecError::InvalidReplicationFactor);
        }

        for policy in &self.placement_policy {
            policy.validate()?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlacementPolicy {
    pub topology_key: String,
    pub max_skew: u32,
    pub when_unsatisfiable: UnsatisfiablePlacementAction,
}

impl PlacementPolicy {
    fn validate(&self) -> Result<(), ShardGroupSpecError> {
        validate_required("placement_policy.topology_key", &self.topology_key)?;
        if self.max_skew == 0 {
            return Err(ShardGroupSpecError::InvalidPlacementSkew);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum UnsatisfiablePlacementAction {
    DoNotSchedule,
    ScheduleAnyway,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ShardGroupSpecError {
    InvalidShardCount,
    InvalidReplicationFactor,
    InvalidPlacementSkew,
    MissingRequiredField(&'static str),
}

impl fmt::Display for ShardGroupSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidShardCount => write!(formatter, "num_shards must be greater than zero"),
            Self::InvalidReplicationFactor => {
                write!(formatter, "replication_factor must be greater than zero")
            }
            Self::InvalidPlacementSkew => write!(formatter, "max_skew must be greater than zero"),
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for ShardGroupSpecError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), ShardGroupSpecError> {
    if value.trim().is_empty() {
        return Err(ShardGroupSpecError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_optional(
    field: &'static str,
    value: &Option<String>,
) -> Result<(), ShardGroupSpecError> {
    if matches!(value, Some(value) if value.trim().is_empty()) {
        return Err(ShardGroupSpecError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_shard_group_with_zone_spread_passes() {
        let spec = ShardGroupSpec {
            parent_table: "public.metrics".to_string(),
            distribution_column: "tenant_id".to_string(),
            num_shards: 32,
            colocation_group: Some("metrics".to_string()),
            replication_factor: 3,
            placement_policy: vec![PlacementPolicy {
                topology_key: "topology.kubernetes.io/zone".to_string(),
                max_skew: 1,
                when_unsatisfiable: UnsatisfiablePlacementAction::DoNotSchedule,
            }],
        };

        assert_eq!(spec.validate(), Ok(()));
    }

    #[test]
    fn shard_group_requires_positive_shards() {
        let mut spec = minimal_spec();
        spec.num_shards = 0;

        assert_eq!(spec.validate(), Err(ShardGroupSpecError::InvalidShardCount));
    }

    #[test]
    fn placement_policy_requires_topology_key() {
        let mut spec = minimal_spec();
        spec.placement_policy = vec![PlacementPolicy {
            topology_key: String::new(),
            max_skew: 1,
            when_unsatisfiable: UnsatisfiablePlacementAction::ScheduleAnyway,
        }];

        assert_eq!(
            spec.validate(),
            Err(ShardGroupSpecError::MissingRequiredField(
                "placement_policy.topology_key"
            ))
        );
    }

    fn minimal_spec() -> ShardGroupSpec {
        ShardGroupSpec {
            parent_table: "public.metrics".to_string(),
            distribution_column: "tenant_id".to_string(),
            num_shards: 32,
            colocation_group: None,
            replication_factor: 1,
            placement_policy: Vec::new(),
        }
    }
}
