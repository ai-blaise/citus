// FEATURE: S11
// FEATURE: MR2

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SurvivalGoalSpec {
    pub goal: SurvivalGoalType,
    pub regions: Vec<String>,
    pub min_replicas: u32,
}

impl SurvivalGoalSpec {
    pub fn validate(&self) -> Result<(), SurvivalGoalSpecError> {
        validate_required_list("regions", &self.regions)?;

        if self.min_replicas == 0 {
            return Err(SurvivalGoalSpecError::InvalidReplicaCount);
        }
        if self.min_replicas as usize > self.regions.len() {
            return Err(SurvivalGoalSpecError::ReplicaCountExceedsRegions);
        }
        if self.goal == SurvivalGoalType::RegionFailure && self.regions.len() < 2 {
            return Err(SurvivalGoalSpecError::InvalidRegionCount);
        }
        for region in &self.regions {
            validate_region_name("regions", region)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SurvivalGoalType {
    ZoneFailure,
    RegionFailure,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SurvivalGoalSpecError {
    InvalidRegionCount,
    InvalidRegionName(&'static str),
    InvalidReplicaCount,
    ReplicaCountExceedsRegions,
    MissingRequiredField(&'static str),
}

impl fmt::Display for SurvivalGoalSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRegionCount => {
                write!(
                    formatter,
                    "region failure goals require at least two regions"
                )
            }
            Self::InvalidRegionName(field) => {
                write!(
                    formatter,
                    "{field} must contain lowercase region labels such as us-east-1"
                )
            }
            Self::InvalidReplicaCount => {
                write!(formatter, "min_replicas must be greater than zero")
            }
            Self::ReplicaCountExceedsRegions => {
                write!(
                    formatter,
                    "min_replicas cannot exceed the number of regions"
                )
            }
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for SurvivalGoalSpecError {}

fn validate_required_list(
    field: &'static str,
    values: &[String],
) -> Result<(), SurvivalGoalSpecError> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(SurvivalGoalSpecError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_region_name(field: &'static str, value: &str) -> Result<(), SurvivalGoalSpecError> {
    let bytes = value.as_bytes();
    let parts = value.split('-').collect::<Vec<_>>();
    if parts.len() < 3
        || parts.iter().any(|part| part.is_empty())
        || !bytes[0].is_ascii_lowercase()
        || !bytes[bytes.len() - 1].is_ascii_digit()
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
    {
        return Err(SurvivalGoalSpecError::InvalidRegionName(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_region_survival_goal_passes() {
        let spec = SurvivalGoalSpec {
            goal: SurvivalGoalType::RegionFailure,
            regions: vec!["us-east-1".to_string(), "us-west-2".to_string()],
            min_replicas: 2,
        };

        assert_eq!(spec.validate(), Ok(()));
    }

    #[test]
    fn region_failure_requires_two_regions() {
        let spec = SurvivalGoalSpec {
            goal: SurvivalGoalType::RegionFailure,
            regions: vec!["us-east-1".to_string()],
            min_replicas: 1,
        };

        assert_eq!(
            spec.validate(),
            Err(SurvivalGoalSpecError::InvalidRegionCount)
        );
    }

    #[test]
    fn survival_goal_rejects_replica_count_above_region_count() {
        let mut spec = minimal_spec();
        spec.min_replicas = 3;

        assert_eq!(
            spec.validate(),
            Err(SurvivalGoalSpecError::ReplicaCountExceedsRegions)
        );
    }

    #[test]
    fn survival_goal_rejects_invalid_region_labels() {
        let mut spec = minimal_spec();
        spec.regions = vec!["us_east_1".to_string(), "us-west-2".to_string()];

        assert_eq!(
            spec.validate(),
            Err(SurvivalGoalSpecError::InvalidRegionName("regions"))
        );
    }

    fn minimal_spec() -> SurvivalGoalSpec {
        SurvivalGoalSpec {
            goal: SurvivalGoalType::ZoneFailure,
            regions: vec!["us-east-1".to_string(), "us-west-2".to_string()],
            min_replicas: 2,
        }
    }
}
