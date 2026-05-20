// FEATURE: O1
// FEATURE: O2
// FEATURE: O3
// FEATURE: R4

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OperationsGuardrailPlan {
    pub query_percentiles: QueryPercentileViewPlan,
    pub local_activity_stats: LocalActivityStatPlan,
    pub replication_lag: ReplicationLagPlan,
    pub idle_transaction_detector: IdleTransactionDetectorPlan,
}

impl OperationsGuardrailPlan {
    pub fn validate(&self) -> Result<(), ObservabilityError> {
        self.query_percentiles.validate()?;
        self.local_activity_stats.validate()?;
        self.replication_lag.validate()?;
        self.idle_transaction_detector.validate()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct QueryPercentileViewPlan {
    pub view_name: String,
    pub source_view: String,
    pub percentiles: Vec<LatencyPercentile>,
}

impl QueryPercentileViewPlan {
    pub fn validate(&self) -> Result<(), ObservabilityError> {
        validate_required("query_percentiles.view_name", &self.view_name)?;
        validate_required("query_percentiles.source_view", &self.source_view)?;
        if self.percentiles.is_empty() {
            return Err(ObservabilityError::MissingRequiredField(
                "query_percentiles.percentiles",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LatencyPercentile {
    P95,
    P99,
    P999,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LocalActivityStatPlan {
    pub view_name: String,
    pub sample_interval_seconds: u32,
}

impl LocalActivityStatPlan {
    pub fn validate(&self) -> Result<(), ObservabilityError> {
        validate_required("local_activity_stats.view_name", &self.view_name)?;
        if self.sample_interval_seconds == 0 {
            return Err(ObservabilityError::InvalidSampleInterval);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReplicationLagPlan {
    pub view_name: String,
    pub regions: Vec<String>,
    pub max_lag_ms: u64,
}

impl ReplicationLagPlan {
    pub fn validate(&self) -> Result<(), ObservabilityError> {
        validate_required("replication_lag.view_name", &self.view_name)?;
        validate_required_list("replication_lag.regions", &self.regions)?;
        if self.max_lag_ms == 0 {
            return Err(ObservabilityError::InvalidLagBudget);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct IdleTransactionDetectorPlan {
    pub max_idle_seconds: u32,
}

impl IdleTransactionDetectorPlan {
    pub fn validate(&self) -> Result<(), ObservabilityError> {
        if self.max_idle_seconds == 0 {
            return Err(ObservabilityError::InvalidIdleLimit);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ObservabilityError {
    InvalidIdleLimit,
    InvalidLagBudget,
    InvalidSampleInterval,
    MissingRequiredField(&'static str),
}

impl fmt::Display for ObservabilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdleLimit => {
                write!(formatter, "max_idle_seconds must be greater than zero")
            }
            Self::InvalidLagBudget => write!(formatter, "max_lag_ms must be greater than zero"),
            Self::InvalidSampleInterval => {
                write!(
                    formatter,
                    "sample_interval_seconds must be greater than zero"
                )
            }
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for ObservabilityError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), ObservabilityError> {
    if value.trim().is_empty() {
        return Err(ObservabilityError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_required_list(
    field: &'static str,
    values: &[String],
) -> Result<(), ObservabilityError> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(ObservabilityError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_operations_guardrail_plan_passes() {
        let plan = OperationsGuardrailPlan {
            query_percentiles: QueryPercentileViewPlan {
                view_name: "companion.pg_stat_statements_p95".to_string(),
                source_view: "pg_stat_statements".to_string(),
                percentiles: vec![LatencyPercentile::P95, LatencyPercentile::P99],
            },
            local_activity_stats: LocalActivityStatPlan {
                view_name: "companion.pg_stat_local_activity".to_string(),
                sample_interval_seconds: 15,
            },
            replication_lag: ReplicationLagPlan {
                view_name: "companion.pg_dist_replication_lag".to_string(),
                regions: vec!["us-east-1".to_string(), "us-west-2".to_string()],
                max_lag_ms: 5_000,
            },
            idle_transaction_detector: IdleTransactionDetectorPlan {
                max_idle_seconds: 60,
            },
        };

        assert_eq!(plan.validate(), Ok(()));
    }

    #[test]
    fn query_percentiles_require_targets() {
        let plan = QueryPercentileViewPlan {
            view_name: "companion.pg_stat_statements_p95".to_string(),
            source_view: "pg_stat_statements".to_string(),
            percentiles: Vec::new(),
        };

        assert_eq!(
            plan.validate(),
            Err(ObservabilityError::MissingRequiredField(
                "query_percentiles.percentiles"
            ))
        );
    }

    #[test]
    fn idle_detector_requires_positive_limit() {
        let plan = IdleTransactionDetectorPlan {
            max_idle_seconds: 0,
        };

        assert_eq!(plan.validate(), Err(ObservabilityError::InvalidIdleLimit));
    }
}
