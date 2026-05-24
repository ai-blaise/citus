// FEATURE: T12

//! HTAP routing classifier.
//!
//! This module owns the conservative query-feature contract and a fail-closed
//! parser for the compact feature reports that the pool hot path can emit after
//! SQL parsing. It does not claim full SQL parser integration or live
//! analytical-sidecar execution.

use crate::HtapRoutingPolicy;
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

/// Per-statement feature vector produced by the proxy after parsing.
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct QueryFeatures {
    pub is_read_only: bool,
    pub has_group_by: bool,
    pub has_aggregate: bool,
    pub references_analytical_table: bool,
    pub limit: Option<u64>,
}

impl QueryFeatures {
    pub fn from_contract_flags(input: &str) -> Result<Self, HtapClassifierError> {
        if input.trim().is_empty() {
            return Err(HtapClassifierError::EmptyFeatureReport);
        }
        let mut seen = BTreeSet::new();
        let mut features = Self::default();
        for item in input.split(',') {
            let (key, value) = item
                .split_once('=')
                .ok_or_else(|| HtapClassifierError::InvalidFeature(item.trim().to_string()))?;
            let key = key.trim();
            let value = value.trim();
            if !seen.insert(key.to_string()) {
                return Err(HtapClassifierError::DuplicateFeature(key.to_string()));
            }
            match key {
                "read_only" => features.is_read_only = parse_bool(key, value)?,
                "group_by" => features.has_group_by = parse_bool(key, value)?,
                "aggregate" => features.has_aggregate = parse_bool(key, value)?,
                "analytical_table" => {
                    features.references_analytical_table = parse_bool(key, value)?;
                }
                "limit" => {
                    features.limit = if value.eq_ignore_ascii_case("none") {
                        None
                    } else {
                        Some(value.parse::<u64>().map_err(|_| {
                            HtapClassifierError::InvalidFeatureValue {
                                key: key.to_string(),
                                value: value.to_string(),
                            }
                        })?)
                    };
                }
                other => return Err(HtapClassifierError::UnknownFeature(other.to_string())),
            }
        }
        Ok(features)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum HtapDecision {
    Analytical { reasons: Vec<&'static str> },
    Transactional { reasons: Vec<&'static str> },
}

impl HtapDecision {
    pub fn is_analytical(&self) -> bool {
        matches!(self, Self::Analytical { .. })
    }

    pub fn reasons(&self) -> &[&'static str] {
        match self {
            Self::Analytical { reasons } | Self::Transactional { reasons } => reasons,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HtapRouteReport {
    pub decision: HtapDecision,
    pub target_host: String,
    pub target_port: u16,
    pub max_staleness_ms: u64,
    pub predicate_hint_count: usize,
}

pub fn route_report(
    policy: &HtapRoutingPolicy,
    features: &QueryFeatures,
) -> Result<HtapRouteReport, HtapClassifierError> {
    let decision = classify(policy, features)?;
    Ok(HtapRouteReport {
        decision,
        target_host: policy.analytical_target.host.clone(),
        target_port: policy.analytical_target.port,
        max_staleness_ms: policy.max_staleness_ms,
        predicate_hint_count: policy.predicate_hints.len(),
    })
}

/// Decide whether to route `features` to the analytical sidecar.
pub fn classify(
    policy: &HtapRoutingPolicy,
    features: &QueryFeatures,
) -> Result<HtapDecision, HtapClassifierError> {
    policy
        .analytical_target
        .host
        .trim()
        .is_empty()
        .then_some(())
        .map(|_| Err::<(), HtapClassifierError>(HtapClassifierError::MissingAnalyticalTarget))
        .transpose()?;

    if !features.is_read_only {
        return Ok(HtapDecision::Transactional {
            reasons: vec!["write-statement"],
        });
    }
    if let Some(limit) = features.limit {
        if limit < 1_000 {
            return Ok(HtapDecision::Transactional {
                reasons: vec!["small-limit"],
            });
        }
    }
    if !features.has_group_by {
        return Ok(HtapDecision::Transactional {
            reasons: vec!["no-group-by"],
        });
    }
    if !features.has_aggregate {
        return Ok(HtapDecision::Transactional {
            reasons: vec!["no-aggregate"],
        });
    }
    if !features.references_analytical_table {
        return Ok(HtapDecision::Transactional {
            reasons: vec!["small-table"],
        });
    }

    let mut reasons = vec!["read-only", "group-by", "aggregate", "analytical-table"];
    if features.limit.is_none() {
        reasons.push("unbounded-limit");
    }
    Ok(HtapDecision::Analytical { reasons })
}

fn parse_bool(key: &str, value: &str) -> Result<bool, HtapClassifierError> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(HtapClassifierError::InvalidFeatureValue {
            key: key.to_string(),
            value: value.to_string(),
        }),
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum HtapClassifierError {
    DuplicateFeature(String),
    EmptyFeatureReport,
    InvalidFeature(String),
    InvalidFeatureValue { key: String, value: String },
    MissingAnalyticalTarget,
    UnknownFeature(String),
}

impl fmt::Display for HtapClassifierError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateFeature(key) => write!(formatter, "duplicate query feature {key}"),
            Self::EmptyFeatureReport => write!(formatter, "query feature report must not be empty"),
            Self::InvalidFeature(value) => write!(formatter, "invalid query feature {value}"),
            Self::InvalidFeatureValue { key, value } => {
                write!(formatter, "invalid query feature value {key}={value}")
            }
            Self::MissingAnalyticalTarget => {
                write!(formatter, "analytical_target.host must not be empty")
            }
            Self::UnknownFeature(key) => write!(formatter, "unknown query feature {key}"),
        }
    }
}

impl Error for HtapClassifierError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RouteTarget;

    fn policy() -> HtapRoutingPolicy {
        HtapRoutingPolicy {
            analytical_target: RouteTarget {
                host: "analytical-sidecar".to_string(),
                port: 5332,
            },
            max_staleness_ms: 2_000,
            predicate_hints: vec![],
        }
    }

    #[test]
    fn write_statement_routes_to_oltp() {
        let features = QueryFeatures::default();
        let decision = classify(&policy(), &features).expect("decision");
        assert!(matches!(decision, HtapDecision::Transactional { .. }));
    }

    #[test]
    fn small_limit_routes_to_oltp() {
        let features = QueryFeatures {
            is_read_only: true,
            has_group_by: true,
            has_aggregate: true,
            references_analytical_table: true,
            limit: Some(100),
        };
        let decision = classify(&policy(), &features).expect("decision");
        assert!(matches!(decision, HtapDecision::Transactional { .. }));
    }

    #[test]
    fn full_analytical_features_route_to_olap() {
        let features = QueryFeatures {
            is_read_only: true,
            has_group_by: true,
            has_aggregate: true,
            references_analytical_table: true,
            limit: None,
        };
        let decision = classify(&policy(), &features).expect("decision");
        if let HtapDecision::Analytical { reasons } = decision {
            assert!(reasons.contains(&"group-by"));
            assert!(reasons.contains(&"aggregate"));
            assert!(reasons.contains(&"analytical-table"));
            assert!(reasons.contains(&"unbounded-limit"));
        } else {
            panic!("expected analytical decision");
        }
    }

    #[test]
    fn limit_above_threshold_routes_to_olap() {
        let features = QueryFeatures {
            is_read_only: true,
            has_group_by: true,
            has_aggregate: true,
            references_analytical_table: true,
            limit: Some(5_000),
        };
        let decision = classify(&policy(), &features).expect("decision");
        assert!(decision.is_analytical());
    }

    #[test]
    fn missing_group_by_routes_to_oltp() {
        let features = QueryFeatures {
            is_read_only: true,
            has_group_by: false,
            has_aggregate: true,
            references_analytical_table: true,
            limit: None,
        };
        let decision = classify(&policy(), &features).expect("decision");
        assert!(matches!(decision, HtapDecision::Transactional { .. }));
    }

    #[test]
    fn feature_report_parser_fails_closed() {
        let features = QueryFeatures::from_contract_flags(
            "read_only=true,group_by=true,aggregate=true,analytical_table=true,limit=none",
        )
        .expect("features");
        assert_eq!(features.limit, None);
        assert!(features.is_read_only);
        assert!(matches!(
            QueryFeatures::from_contract_flags("read_only=maybe"),
            Err(HtapClassifierError::InvalidFeatureValue { .. })
        ));
        assert!(matches!(
            QueryFeatures::from_contract_flags("read_only=true,unknown=false"),
            Err(HtapClassifierError::UnknownFeature(_))
        ));
        assert!(matches!(
            QueryFeatures::from_contract_flags("read_only=true,read_only=false"),
            Err(HtapClassifierError::DuplicateFeature(_))
        ));
    }

    #[test]
    fn route_report_exposes_bounded_decision() {
        let features = QueryFeatures::from_contract_flags(
            "read_only=true,group_by=true,aggregate=true,analytical_table=true,limit=5000",
        )
        .expect("features");
        let report = route_report(&policy(), &features).expect("report");
        assert!(report.decision.is_analytical());
        assert_eq!(report.target_host, "analytical-sidecar");
        assert_eq!(report.max_staleness_ms, 2_000);
    }
}
