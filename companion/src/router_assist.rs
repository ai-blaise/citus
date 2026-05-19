// FEATURE: S6
// FEATURE: S13

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlacementGenerationQuery {
    pub shard_id: u64,
}

impl PlacementGenerationQuery {
    pub fn validate(&self) -> Result<(), RouterAssistError> {
        validate_shard_id(self.shard_id)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ShardForValuePlan {
    pub table: String,
    pub distribution_column: String,
    pub value_hash: i64,
    pub shard_count: u32,
    pub strategy: ShardRoutingStrategy,
}

impl ShardForValuePlan {
    pub fn validate(&self) -> Result<(), RouterAssistError> {
        validate_required("table", &self.table)?;
        validate_required("distribution_column", &self.distribution_column)?;
        if self.shard_count == 0 {
            return Err(RouterAssistError::InvalidShardCount);
        }
        self.strategy.validate()
    }

    pub fn target_shard_index(&self) -> Result<u32, RouterAssistError> {
        self.validate()?;
        Ok(self.value_hash.unsigned_abs() as u32 % self.shard_count)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ShardRoutingStrategy {
    Hash,
    Range {
        lower_bound: String,
        upper_bound: String,
    },
}

impl ShardRoutingStrategy {
    fn validate(&self) -> Result<(), RouterAssistError> {
        match self {
            Self::Hash => Ok(()),
            Self::Range {
                lower_bound,
                upper_bound,
            } => {
                validate_required("range.lower_bound", lower_bound)?;
                validate_required("range.upper_bound", upper_bound)
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LocalPlacementCheck {
    pub shard_id: u64,
    pub worker_name: String,
}

impl LocalPlacementCheck {
    pub fn validate(&self) -> Result<(), RouterAssistError> {
        validate_shard_id(self.shard_id)?;
        validate_required("worker_name", &self.worker_name)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RouterAssistError {
    InvalidShardCount,
    InvalidShardId,
    MissingRequiredField(&'static str),
}

impl fmt::Display for RouterAssistError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidShardCount => write!(formatter, "shard_count must be greater than zero"),
            Self::InvalidShardId => write!(formatter, "shard_id must be greater than zero"),
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for RouterAssistError {}

fn validate_shard_id(shard_id: u64) -> Result<(), RouterAssistError> {
    if shard_id == 0 {
        return Err(RouterAssistError::InvalidShardId);
    }
    Ok(())
}

fn validate_required(field: &'static str, value: &str) -> Result<(), RouterAssistError> {
    if value.trim().is_empty() {
        return Err(RouterAssistError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_routing_computes_target_index() {
        let plan = ShardForValuePlan {
            table: "public.metrics".to_string(),
            distribution_column: "tenant_id".to_string(),
            value_hash: 42,
            shard_count: 8,
            strategy: ShardRoutingStrategy::Hash,
        };

        assert_eq!(plan.target_shard_index(), Ok(2));
    }

    #[test]
    fn range_routing_requires_bounds() {
        let plan = ShardForValuePlan {
            table: "public.events".to_string(),
            distribution_column: "created_at".to_string(),
            value_hash: 0,
            shard_count: 16,
            strategy: ShardRoutingStrategy::Range {
                lower_bound: String::new(),
                upper_bound: "2026-01-01".to_string(),
            },
        };

        assert_eq!(
            plan.validate(),
            Err(RouterAssistError::MissingRequiredField("range.lower_bound"))
        );
    }

    #[test]
    fn local_placement_requires_worker_name() {
        let check = LocalPlacementCheck {
            shard_id: 1,
            worker_name: " ".to_string(),
        };

        assert_eq!(
            check.validate(),
            Err(RouterAssistError::MissingRequiredField("worker_name"))
        );
    }
}
