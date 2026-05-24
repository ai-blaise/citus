// FEATURE: T9

//! Per-tenant + per-query-class canary mirroring.
//!
//! This module owns the deterministic policy parser, validator, and decision
//! report used before any request is eligible for fire-and-forget canary
//! mirroring. It intentionally does not claim live backend fan-out or response
//! comparison; those stay outside the bounded pool contract until a data-plane
//! canary smoke proves them.

use crate::{MirrorTrafficPolicy, PoolRuntimeError, RouteTarget};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

/// Query class used to scope mirror sampling. Matched literally against the
/// classifier output (see `proxy::classify_query_class`).
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum QueryClass {
    ReadShortHash,
    ReadScatterGather,
    Analytical,
    Write,
    Ddl,
    Other,
}

impl QueryClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReadShortHash => "read.short.hash",
            Self::ReadScatterGather => "read.scatter",
            Self::Analytical => "analytical",
            Self::Write => "write",
            Self::Ddl => "ddl",
            Self::Other => "other",
        }
    }

    pub fn parse(value: &str) -> Result<Self, MirrorPolicyError> {
        match value.trim() {
            "read.short.hash" => Ok(Self::ReadShortHash),
            "read.scatter" => Ok(Self::ReadScatterGather),
            "analytical" => Ok(Self::Analytical),
            "write" => Ok(Self::Write),
            "ddl" => Ok(Self::Ddl),
            "other" => Ok(Self::Other),
            other => Err(MirrorPolicyError::InvalidQueryClass(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantMirrorRule {
    pub tenant_id: String,
    pub query_class: QueryClass,
    pub sample_percent: u8,
}

impl TenantMirrorRule {
    pub fn parse(spec: &str) -> Result<Self, MirrorPolicyError> {
        let parts = spec.split(':').collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err(MirrorPolicyError::InvalidRuleSpec(spec.to_string()));
        }
        let sample_percent = parts[2]
            .parse::<u8>()
            .map_err(|_| MirrorPolicyError::InvalidRuleSpec(spec.to_string()))?;
        let rule = Self {
            tenant_id: parts[0].trim().to_string(),
            query_class: QueryClass::parse(parts[1])?,
            sample_percent,
        };
        rule.validate()?;
        Ok(rule)
    }

    fn validate(&self) -> Result<(), MirrorPolicyError> {
        if self.tenant_id.trim().is_empty() {
            return Err(MirrorPolicyError::MissingField("tenant_id"));
        }
        if self.tenant_id.chars().any(char::is_whitespace) {
            return Err(MirrorPolicyError::InvalidTenantId(self.tenant_id.clone()));
        }
        if self.sample_percent > 100 {
            return Err(MirrorPolicyError::InvalidPercent(self.sample_percent));
        }
        Ok(())
    }
}

/// Extended mirror policy with per-tenant + per-query-class overrides on top
/// of the base `MirrorTrafficPolicy`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantMirrorPolicy {
    pub base: MirrorTrafficPolicy,
    pub rules: Vec<TenantMirrorRule>,
}

impl TenantMirrorPolicy {
    pub fn from_rule_specs(
        base: MirrorTrafficPolicy,
        specs: &[&str],
    ) -> Result<Self, MirrorPolicyError> {
        let rules = specs
            .iter()
            .map(|spec| TenantMirrorRule::parse(spec))
            .collect::<Result<Vec<_>, _>>()?;
        let policy = Self { base, rules };
        policy.validate()?;
        Ok(policy)
    }

    pub fn validate(&self) -> Result<(), MirrorPolicyError> {
        self.base.validate().map_err(MirrorPolicyError::Runtime)?;
        let mut seen: BTreeMap<(String, QueryClass), ()> = BTreeMap::new();
        for rule in &self.rules {
            rule.validate()?;
            if seen
                .insert((rule.tenant_id.clone(), rule.query_class.clone()), ())
                .is_some()
            {
                return Err(MirrorPolicyError::DuplicateRule {
                    tenant_id: rule.tenant_id.clone(),
                    query_class: rule.query_class.clone(),
                });
            }
        }
        Ok(())
    }

    /// Decide whether a request keyed on `(tenant_id, class, hash)` should be
    /// mirrored. `hash` is used as a deterministic sampler so the same query
    /// consistently mirrors or skips.
    pub fn should_mirror(
        &self,
        tenant_id: &str,
        class: &QueryClass,
        hash: u64,
    ) -> Result<MirrorDecision, MirrorPolicyError> {
        Ok(self.decision_report(tenant_id, class, hash)?.decision)
    }

    pub fn decision_report(
        &self,
        tenant_id: &str,
        class: &QueryClass,
        hash: u64,
    ) -> Result<MirrorDecisionReport, MirrorPolicyError> {
        self.validate()?;
        if tenant_id.trim().is_empty() {
            return Err(MirrorPolicyError::MissingField("tenant_id"));
        }
        if tenant_id.chars().any(char::is_whitespace) {
            return Err(MirrorPolicyError::InvalidTenantId(tenant_id.to_string()));
        }
        if !self.base.enabled {
            return Ok(MirrorDecisionReport {
                tenant_id: tenant_id.to_string(),
                query_class: class.as_str(),
                sample_percent: 0,
                hash_bucket: (hash % 100) as u8,
                decision: MirrorDecision::Skip,
                target: None,
            });
        }
        let target = self.base.target.clone().ok_or(MirrorPolicyError::Runtime(
            PoolRuntimeError::MissingRequiredField("mirror.target"),
        ))?;

        let sample = self
            .rules
            .iter()
            .find(|rule| rule.tenant_id == tenant_id && &rule.query_class == class)
            .map(|rule| rule.sample_percent)
            .unwrap_or(self.base.sample_percent);

        let bucket = (hash % 100) as u8;
        let decision = if sample > 0 && bucket < sample {
            MirrorDecision::Mirror(target.clone())
        } else {
            MirrorDecision::Skip
        };
        Ok(MirrorDecisionReport {
            tenant_id: tenant_id.to_string(),
            query_class: class.as_str(),
            sample_percent: sample,
            hash_bucket: bucket,
            target: matches!(decision, MirrorDecision::Mirror(_)).then_some(target),
            decision,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MirrorDecisionReport {
    pub tenant_id: String,
    pub query_class: &'static str,
    pub sample_percent: u8,
    pub hash_bucket: u8,
    pub decision: MirrorDecision,
    pub target: Option<RouteTarget>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MirrorDecision {
    Mirror(RouteTarget),
    Skip,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MirrorPolicyError {
    DuplicateRule {
        tenant_id: String,
        query_class: QueryClass,
    },
    InvalidPercent(u8),
    InvalidQueryClass(String),
    InvalidRuleSpec(String),
    InvalidTenantId(String),
    MissingField(&'static str),
    Runtime(PoolRuntimeError),
}

impl fmt::Display for MirrorPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateRule {
                tenant_id,
                query_class,
            } => write!(
                formatter,
                "duplicate mirror rule for tenant {tenant_id} class {}",
                query_class.as_str()
            ),
            Self::InvalidPercent(value) => write!(
                formatter,
                "sample_percent {value} must be between 0 and 100"
            ),
            Self::InvalidQueryClass(value) => write!(formatter, "unknown query class {value}"),
            Self::InvalidRuleSpec(value) => write!(
                formatter,
                "mirror rule {value} must use tenant:query-class:sample-percent"
            ),
            Self::InvalidTenantId(value) => write!(formatter, "invalid mirror tenant id {value}"),
            Self::MissingField(field) => write!(formatter, "{field} must not be empty"),
            Self::Runtime(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for MirrorPolicyError {}

impl From<PoolRuntimeError> for MirrorPolicyError {
    fn from(error: PoolRuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> MirrorTrafficPolicy {
        MirrorTrafficPolicy {
            enabled: true,
            target: Some(RouteTarget {
                host: "canary".to_string(),
                port: 5432,
            }),
            sample_percent: 10,
        }
    }

    #[test]
    fn skip_when_disabled() {
        let policy = TenantMirrorPolicy {
            base: MirrorTrafficPolicy {
                enabled: false,
                target: None,
                sample_percent: 0,
            },
            rules: vec![],
        };
        assert_eq!(
            policy.should_mirror("tenant-a", &QueryClass::Write, 1),
            Ok(MirrorDecision::Skip),
        );
    }

    #[test]
    fn tenant_rule_overrides_base_sample() {
        let policy = TenantMirrorPolicy {
            base: base(),
            rules: vec![TenantMirrorRule {
                tenant_id: "tenant-a".to_string(),
                query_class: QueryClass::Write,
                sample_percent: 100,
            }],
        };
        assert!(matches!(
            policy.should_mirror("tenant-a", &QueryClass::Write, 99),
            Ok(MirrorDecision::Mirror(_))
        ));
        assert_eq!(
            policy.should_mirror("tenant-b", &QueryClass::Write, 50),
            Ok(MirrorDecision::Skip),
        );
        assert!(matches!(
            policy.should_mirror("tenant-b", &QueryClass::Write, 5),
            Ok(MirrorDecision::Mirror(_))
        ));
    }

    #[test]
    fn zero_percent_skips_all() {
        let policy = TenantMirrorPolicy {
            base: base(),
            rules: vec![TenantMirrorRule {
                tenant_id: "tenant-a".to_string(),
                query_class: QueryClass::Write,
                sample_percent: 0,
            }],
        };
        assert_eq!(
            policy.should_mirror("tenant-a", &QueryClass::Write, 0),
            Ok(MirrorDecision::Skip),
        );
    }

    #[test]
    fn duplicate_rule_is_rejected() {
        let policy = TenantMirrorPolicy {
            base: base(),
            rules: vec![
                TenantMirrorRule {
                    tenant_id: "tenant-a".to_string(),
                    query_class: QueryClass::Write,
                    sample_percent: 50,
                },
                TenantMirrorRule {
                    tenant_id: "tenant-a".to_string(),
                    query_class: QueryClass::Write,
                    sample_percent: 100,
                },
            ],
        };
        assert!(matches!(
            policy.validate(),
            Err(MirrorPolicyError::DuplicateRule { .. })
        ));
    }

    #[test]
    fn parses_rules_fail_closed() {
        let policy = TenantMirrorPolicy::from_rule_specs(
            base(),
            &["tenant-a:analytical:100", "tenant-b:write:0"],
        )
        .expect("policy");
        assert_eq!(policy.rules.len(), 2);
        assert!(matches!(
            TenantMirrorPolicy::from_rule_specs(base(), &["tenant-a:unknown:10"]),
            Err(MirrorPolicyError::InvalidQueryClass(_))
        ));
        assert!(matches!(
            TenantMirrorPolicy::from_rule_specs(base(), &["tenant a:write:10"]),
            Err(MirrorPolicyError::InvalidTenantId(_))
        ));
        assert!(matches!(
            TenantMirrorPolicy::from_rule_specs(base(), &["tenant-a:write:101"]),
            Err(MirrorPolicyError::InvalidPercent(101))
        ));
    }

    #[test]
    fn decision_report_is_deterministic_and_non_secret() {
        let policy = TenantMirrorPolicy::from_rule_specs(base(), &["tenant-a:analytical:100"])
            .expect("policy");
        let report = policy
            .decision_report("tenant-a", &QueryClass::Analytical, 142)
            .expect("report");
        assert_eq!(report.hash_bucket, 42);
        assert_eq!(report.sample_percent, 100);
        assert_eq!(report.query_class, "analytical");
        assert!(matches!(report.decision, MirrorDecision::Mirror(_)));
        assert_eq!(report.target.expect("target").host, "canary");
    }
}
