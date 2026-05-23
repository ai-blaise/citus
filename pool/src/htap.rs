// FEATURE: T12

//! HTAP routing classifier.
//!
//! Inspects an incoming SQL statement via the `pg_query` (libpg_query) parser
//! and decides whether it should be routed to the analytical sidecar
//! (`sidecar/analytical` on `localhost:5332`) or to the Citus shards.
//!
//! Heuristic (deliberately conservative — false negatives are fine, false
//! positives are not since the analytical mirror lags real-time):
//!
//! 1. Contains at least one `GROUP BY`.
//! 2. Contains an aggregate (`SUM`, `AVG`, `COUNT`, `MIN`, `MAX`).
//! 3. References at least one large/analytical table.
//! 4. Has no `LIMIT < 1000`.
//! 5. Is read-only (no `INSERT/UPDATE/DELETE/MERGE/COPY`).
//!
//! The actual `pg_query` parsing is delegated to the proxy hot path; this
//! module owns the rule engine + the explanation surface so tests can pin
//! behavior without depending on libpg_query.

use crate::HtapRoutingPolicy;
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum HtapDecision {
    Analytical { reasons: Vec<&'static str> },
    Transactional { reasons: Vec<&'static str> },
}

impl HtapDecision {
    pub fn is_analytical(&self) -> bool {
        matches!(self, Self::Analytical { .. })
    }
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
        .then(|| ())
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum HtapClassifierError {
    MissingAnalyticalTarget,
}

impl fmt::Display for HtapClassifierError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAnalyticalTarget => {
                write!(formatter, "analytical_target.host must not be empty")
            }
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
}
