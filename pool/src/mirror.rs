// FEATURE: T9

//! Per-tenant + per-query-class canary mirroring.
//!
//! Mirrors a fraction of production traffic to a canary backend without
//! blocking the primary path. Mirrored traffic is fire-and-forget; the
//! canary's response is discarded after structural shape checks.

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
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantMirrorRule {
    pub tenant_id: String,
    pub query_class: QueryClass,
    pub sample_percent: u8,
}

impl TenantMirrorRule {
    fn validate(&self) -> Result<(), MirrorPolicyError> {
        if self.tenant_id.trim().is_empty() {
            return Err(MirrorPolicyError::MissingField("tenant_id"));
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
    /// mirrored. `hash` is used as a deterministic sampler — caller supplies a
    /// stable per-query hash so the same query consistently mirrors or skips.
    pub fn should_mirror(
        &self,
        tenant_id: &str,
        class: &QueryClass,
        hash: u64,
    ) -> Result<MirrorDecision, MirrorPolicyError> {
        self.validate()?;
        if !self.base.enabled {
            return Ok(MirrorDecision::Skip);
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

        if sample == 0 {
            return Ok(MirrorDecision::Skip);
        }
        let bucket = (hash % 100) as u8;
        if bucket < sample {
            Ok(MirrorDecision::Mirror(target))
        } else {
            Ok(MirrorDecision::Skip)
        }
    }
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
            Self::InvalidPercent(value) => {
                write!(
                    formatter,
                    "sample_percent {value} must be between 0 and 100"
                )
            }
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
        // tenant-a writes always mirror; tenant-b writes only at base 10%.
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
    fn invalid_percent_is_rejected() {
        let rule = TenantMirrorRule {
            tenant_id: "tenant".to_string(),
            query_class: QueryClass::Write,
            sample_percent: 101,
        };
        assert_eq!(rule.validate(), Err(MirrorPolicyError::InvalidPercent(101)));
    }
}
