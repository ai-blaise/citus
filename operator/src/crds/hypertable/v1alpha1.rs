// FEATURE: TS7

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HypertableSpec {
    pub table: String,
    pub time_column: String,
    pub distribution_column: String,
    pub chunk_time_interval: String,
    pub num_shards: u32,
    pub compression: Option<CompressionPolicy>,
    pub retention: Option<RetentionPolicy>,
    pub continuous_aggregates: Vec<ContinuousAggregateSpec>,
}

impl HypertableSpec {
    pub fn validate(&self) -> Result<(), HypertableSpecError> {
        validate_required("table", &self.table)?;
        validate_required("time_column", &self.time_column)?;
        validate_required("distribution_column", &self.distribution_column)?;
        validate_required("chunk_time_interval", &self.chunk_time_interval)?;

        if self.num_shards == 0 {
            return Err(HypertableSpecError::InvalidShardCount);
        }

        if let Some(compression) = &self.compression {
            compression.validate()?;
        }

        if let Some(retention) = &self.retention {
            retention.validate()?;
        }

        for continuous_aggregate in &self.continuous_aggregates {
            continuous_aggregate.validate()?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CompressionPolicy {
    pub older_than: String,
    pub segment_by: Vec<String>,
    pub order_by: Vec<String>,
    pub bloom_filters: Vec<String>,
}

impl CompressionPolicy {
    fn validate(&self) -> Result<(), HypertableSpecError> {
        validate_required("compression.older_than", &self.older_than)?;
        validate_required_list("compression.segment_by", &self.segment_by)?;
        validate_required_list("compression.order_by", &self.order_by)?;
        validate_optional_list("compression.bloom_filters", &self.bloom_filters)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RetentionPolicy {
    pub drop_after: String,
}

impl RetentionPolicy {
    fn validate(&self) -> Result<(), HypertableSpecError> {
        validate_required("retention.drop_after", &self.drop_after)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ContinuousAggregateSpec {
    pub name: String,
    pub query: String,
    pub refresh_start: Option<String>,
    pub refresh_end: Option<String>,
    pub schedule: Option<String>,
    pub hierarchical_parent: Option<String>,
}

impl ContinuousAggregateSpec {
    fn validate(&self) -> Result<(), HypertableSpecError> {
        validate_required("continuous_aggregates.name", &self.name)?;
        validate_required("continuous_aggregates.query", &self.query)?;
        validate_optional("continuous_aggregates.refresh_start", &self.refresh_start)?;
        validate_optional("continuous_aggregates.refresh_end", &self.refresh_end)?;
        validate_optional("continuous_aggregates.schedule", &self.schedule)?;
        validate_optional(
            "continuous_aggregates.hierarchical_parent",
            &self.hierarchical_parent,
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum HypertableSpecError {
    InvalidShardCount,
    MissingRequiredField(&'static str),
}

impl fmt::Display for HypertableSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidShardCount => write!(formatter, "num_shards must be greater than zero"),
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for HypertableSpecError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), HypertableSpecError> {
    if value.trim().is_empty() {
        return Err(HypertableSpecError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_optional(
    field: &'static str,
    value: &Option<String>,
) -> Result<(), HypertableSpecError> {
    if matches!(value, Some(value) if value.trim().is_empty()) {
        return Err(HypertableSpecError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_required_list(
    field: &'static str,
    values: &[String],
) -> Result<(), HypertableSpecError> {
    if values.is_empty() {
        return Err(HypertableSpecError::MissingRequiredField(field));
    }
    validate_optional_list(field, values)
}

fn validate_optional_list(
    field: &'static str,
    values: &[String],
) -> Result<(), HypertableSpecError> {
    if values.iter().any(|value| value.trim().is_empty()) {
        return Err(HypertableSpecError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_hypertable_spec_passes() {
        let spec = HypertableSpec {
            table: "metrics".to_string(),
            time_column: "ts".to_string(),
            distribution_column: "tenant_id".to_string(),
            chunk_time_interval: "1 day".to_string(),
            num_shards: 32,
            compression: Some(CompressionPolicy {
                older_than: "7 days".to_string(),
                segment_by: vec!["tenant_id".to_string()],
                order_by: vec!["ts DESC".to_string()],
                bloom_filters: vec!["region".to_string()],
            }),
            retention: Some(RetentionPolicy {
                drop_after: "90 days".to_string(),
            }),
            continuous_aggregates: vec![ContinuousAggregateSpec {
                name: "metrics_hourly".to_string(),
                query: "SELECT tenant_id, time_bucket('1 hour', ts), count(*) FROM metrics GROUP BY 1, 2".to_string(),
                refresh_start: Some("7 days".to_string()),
                refresh_end: Some("1 hour".to_string()),
                schedule: Some("15 minutes".to_string()),
                hierarchical_parent: None,
            }],
        };

        assert_eq!(spec.validate(), Ok(()));
    }

    #[test]
    fn hypertable_requires_positive_shards() {
        let mut spec = minimal_spec();
        spec.num_shards = 0;

        assert_eq!(spec.validate(), Err(HypertableSpecError::InvalidShardCount));
    }

    #[test]
    fn compression_requires_segment_by() {
        let mut spec = minimal_spec();
        spec.compression = Some(CompressionPolicy {
            older_than: "7 days".to_string(),
            segment_by: Vec::new(),
            order_by: vec!["ts DESC".to_string()],
            bloom_filters: Vec::new(),
        });

        assert_eq!(
            spec.validate(),
            Err(HypertableSpecError::MissingRequiredField(
                "compression.segment_by"
            ))
        );
    }

    #[test]
    fn continuous_aggregate_rejects_empty_schedule() {
        let mut spec = minimal_spec();
        spec.continuous_aggregates = vec![ContinuousAggregateSpec {
            name: "metrics_hourly".to_string(),
            query: "SELECT 1".to_string(),
            refresh_start: None,
            refresh_end: None,
            schedule: Some(String::new()),
            hierarchical_parent: None,
        }];

        assert_eq!(
            spec.validate(),
            Err(HypertableSpecError::MissingRequiredField(
                "continuous_aggregates.schedule"
            ))
        );
    }

    fn minimal_spec() -> HypertableSpec {
        HypertableSpec {
            table: "metrics".to_string(),
            time_column: "ts".to_string(),
            distribution_column: "tenant_id".to_string(),
            chunk_time_interval: "1 day".to_string(),
            num_shards: 32,
            compression: None,
            retention: None,
            continuous_aggregates: Vec::new(),
        }
    }
}
