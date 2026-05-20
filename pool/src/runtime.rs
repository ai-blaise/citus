// FEATURE: Auth3
// FEATURE: MR5
// FEATURE: R10
// FEATURE: Sec12
// FEATURE: T1
// FEATURE: T3
// FEATURE: T7
// FEATURE: T9
// FEATURE: T12
// FEATURE: T15

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PoolRuntimeContract {
    pub settings_bucket: SettingsBucketPolicy,
    pub fast_path_router: FastPathRouterPolicy,
    pub mirror: MirrorTrafficPolicy,
    pub htap: HtapRoutingPolicy,
    pub pipeline: ProtocolPipelinePolicy,
    pub tls: TlsSessionTicketPolicy,
    pub tenant_quota: TenantAdmissionPolicy,
    pub geo_router: GeoRoutingPolicy,
    pub token_cache: TokenIntrospectionCachePolicy,
}

impl PoolRuntimeContract {
    pub fn validate(&self) -> Result<(), PoolRuntimeError> {
        self.settings_bucket.validate()?;
        self.fast_path_router.validate()?;
        self.mirror.validate()?;
        self.htap.validate()?;
        self.pipeline.validate()?;
        self.tls.validate()?;
        self.tenant_quota.validate()?;
        self.geo_router.validate()?;
        self.token_cache.validate()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SettingsBucketPolicy {
    pub bucket_name: String,
    pub tracked_gucs: Vec<String>,
    pub max_connections: u32,
}

impl SettingsBucketPolicy {
    fn validate(&self) -> Result<(), PoolRuntimeError> {
        validate_required("settings_bucket.bucket_name", &self.bucket_name)?;
        validate_required_list("settings_bucket.tracked_gucs", &self.tracked_gucs)?;
        if self.max_connections == 0 {
            return Err(PoolRuntimeError::InvalidConnectionLimit);
        }
        Ok(())
    }

    pub fn fingerprint(&self, settings: &[SessionSetting]) -> Result<String, PoolRuntimeError> {
        self.validate()?;
        for setting in settings {
            setting.validate()?;
        }

        let mut values = self
            .tracked_gucs
            .iter()
            .map(|tracked_guc| {
                let value = settings
                    .iter()
                    .find(|setting| setting.name.eq_ignore_ascii_case(tracked_guc))
                    .map(|setting| setting.value.as_str())
                    .unwrap_or("<unset>");
                format!(
                    "{}={}",
                    tracked_guc.to_ascii_lowercase(),
                    escape_fingerprint(value)
                )
            })
            .collect::<Vec<_>>();
        values.sort();

        Ok(format!("{}:{}", self.bucket_name, values.join(";")))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SessionSetting {
    pub name: String,
    pub value: String,
}

impl SessionSetting {
    fn validate(&self) -> Result<(), PoolRuntimeError> {
        validate_required("session_setting.name", &self.name)?;
        validate_required("session_setting.value", &self.value)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FastPathRouterPolicy {
    pub enabled: bool,
    pub single_shard_only: bool,
    pub fallback_target: RouteTarget,
}

impl FastPathRouterPolicy {
    fn validate(&self) -> Result<(), PoolRuntimeError> {
        self.fallback_target.validate()
    }

    pub fn decide(
        &self,
        single_shard_target: Option<RouteTarget>,
    ) -> Result<RouteDecision, PoolRuntimeError> {
        self.validate()?;
        if !self.enabled {
            return Ok(RouteDecision::Fallback(self.fallback_target.clone()));
        }

        match single_shard_target {
            Some(target) if self.single_shard_only => {
                target.validate()?;
                Ok(RouteDecision::FastPath(target))
            }
            _ => Ok(RouteDecision::Fallback(self.fallback_target.clone())),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RouteDecision {
    FastPath(RouteTarget),
    Fallback(RouteTarget),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RouteTarget {
    pub host: String,
    pub port: u16,
}

impl RouteTarget {
    fn validate(&self) -> Result<(), PoolRuntimeError> {
        validate_required("route_target.host", &self.host)?;
        if self.port == 0 {
            return Err(PoolRuntimeError::InvalidPort);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MirrorTrafficPolicy {
    pub enabled: bool,
    pub target: Option<RouteTarget>,
    pub sample_percent: u8,
}

impl MirrorTrafficPolicy {
    fn validate(&self) -> Result<(), PoolRuntimeError> {
        if self.sample_percent > 100 {
            return Err(PoolRuntimeError::InvalidPercent);
        }
        if self.enabled {
            return self
                .target
                .as_ref()
                .ok_or(PoolRuntimeError::MissingRequiredField("mirror.target"))?
                .validate();
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HtapRoutingPolicy {
    pub analytical_target: RouteTarget,
    pub max_staleness_ms: u64,
    pub predicate_hints: Vec<String>,
}

impl HtapRoutingPolicy {
    fn validate(&self) -> Result<(), PoolRuntimeError> {
        self.analytical_target.validate()?;
        if self.max_staleness_ms == 0 {
            return Err(PoolRuntimeError::InvalidStalenessBudget);
        }
        validate_optional_list("htap.predicate_hints", &self.predicate_hints)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProtocolPipelinePolicy {
    pub max_in_flight: u32,
    pub transaction_pipelining: bool,
}

impl ProtocolPipelinePolicy {
    fn validate(&self) -> Result<(), PoolRuntimeError> {
        if self.max_in_flight == 0 {
            return Err(PoolRuntimeError::InvalidPipelineDepth);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TlsSessionTicketPolicy {
    pub enabled: bool,
    pub rotation_seconds: u32,
}

impl TlsSessionTicketPolicy {
    fn validate(&self) -> Result<(), PoolRuntimeError> {
        if self.enabled && self.rotation_seconds == 0 {
            return Err(PoolRuntimeError::InvalidRotation);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantAdmissionPolicy {
    pub tenant_id: String,
    pub burst: u32,
    pub refill_per_second: u32,
}

impl TenantAdmissionPolicy {
    fn validate(&self) -> Result<(), PoolRuntimeError> {
        validate_required("tenant_quota.tenant_id", &self.tenant_id)?;
        if self.burst == 0 {
            return Err(PoolRuntimeError::InvalidQuota("burst"));
        }
        if self.refill_per_second == 0 {
            return Err(PoolRuntimeError::InvalidQuota("refill_per_second"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GeoRoutingPolicy {
    pub default_region: String,
    pub rules: Vec<GeoRoutingRule>,
}

impl GeoRoutingPolicy {
    fn validate(&self) -> Result<(), PoolRuntimeError> {
        validate_required("geo.default_region", &self.default_region)?;
        if self.rules.is_empty() {
            return Err(PoolRuntimeError::MissingRequiredField("geo.rules"));
        }
        for rule in &self.rules {
            rule.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GeoRoutingRule {
    pub cidr: String,
    pub region: String,
}

impl GeoRoutingRule {
    fn validate(&self) -> Result<(), PoolRuntimeError> {
        validate_required("geo.rules.cidr", &self.cidr)?;
        validate_required("geo.rules.region", &self.region)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TokenIntrospectionCachePolicy {
    pub max_entries: u32,
    pub ttl_seconds: u32,
}

impl TokenIntrospectionCachePolicy {
    fn validate(&self) -> Result<(), PoolRuntimeError> {
        if self.max_entries == 0 {
            return Err(PoolRuntimeError::InvalidCacheSize);
        }
        if self.ttl_seconds == 0 {
            return Err(PoolRuntimeError::InvalidCacheTtl);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PoolRuntimeError {
    InvalidCacheSize,
    InvalidCacheTtl,
    InvalidConnectionLimit,
    InvalidPercent,
    InvalidPipelineDepth,
    InvalidPort,
    InvalidQuota(&'static str),
    InvalidRotation,
    InvalidStalenessBudget,
    MissingRequiredField(&'static str),
}

impl fmt::Display for PoolRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCacheSize => write!(formatter, "max_entries must be greater than zero"),
            Self::InvalidCacheTtl => write!(formatter, "ttl_seconds must be greater than zero"),
            Self::InvalidConnectionLimit => {
                write!(formatter, "max_connections must be greater than zero")
            }
            Self::InvalidPercent => write!(formatter, "sample_percent must be between 0 and 100"),
            Self::InvalidPipelineDepth => {
                write!(formatter, "max_in_flight must be greater than zero")
            }
            Self::InvalidPort => write!(formatter, "port must be greater than zero"),
            Self::InvalidQuota(field) => write!(formatter, "{field} must be greater than zero"),
            Self::InvalidRotation => {
                write!(formatter, "rotation_seconds must be greater than zero")
            }
            Self::InvalidStalenessBudget => {
                write!(formatter, "max_staleness_ms must be greater than zero")
            }
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for PoolRuntimeError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), PoolRuntimeError> {
    if value.trim().is_empty() {
        return Err(PoolRuntimeError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_required_list(field: &'static str, values: &[String]) -> Result<(), PoolRuntimeError> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(PoolRuntimeError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_optional_list(field: &'static str, values: &[String]) -> Result<(), PoolRuntimeError> {
    if values.iter().any(|value| value.trim().is_empty()) {
        return Err(PoolRuntimeError::MissingRequiredField(field));
    }
    Ok(())
}

fn escape_fingerprint(value: &str) -> String {
    value.replace('\\', "\\\\").replace(';', "\\;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_pool_runtime_contract_passes() {
        assert_eq!(valid_contract().validate(), Ok(()));
    }

    #[test]
    fn settings_bucket_fingerprint_is_stable_and_sorted() {
        let policy = SettingsBucketPolicy {
            bucket_name: "default".to_string(),
            tracked_gucs: vec![
                "citus.enable_repartition_joins".to_string(),
                "search_path".to_string(),
            ],
            max_connections: 128,
        };

        let fingerprint = policy
            .fingerprint(&[
                SessionSetting {
                    name: "search_path".to_string(),
                    value: "tenant_a,public".to_string(),
                },
                SessionSetting {
                    name: "citus.enable_repartition_joins".to_string(),
                    value: "off".to_string(),
                },
            ])
            .expect("fingerprint");

        assert_eq!(
            fingerprint,
            "default:citus.enable_repartition_joins=off;search_path=tenant_a,public"
        );
    }

    #[test]
    fn fast_path_policy_routes_single_shard_target() {
        let policy = FastPathRouterPolicy {
            enabled: true,
            single_shard_only: true,
            fallback_target: RouteTarget {
                host: "coordinator".to_string(),
                port: 5432,
            },
        };
        let worker = RouteTarget {
            host: "worker-a".to_string(),
            port: 5432,
        };

        assert_eq!(
            policy.decide(Some(worker.clone())),
            Ok(RouteDecision::FastPath(worker))
        );
    }

    #[test]
    fn mirror_requires_target_when_enabled() {
        let mut contract = valid_contract();
        contract.mirror.enabled = true;
        contract.mirror.target = None;

        assert_eq!(
            contract.validate(),
            Err(PoolRuntimeError::MissingRequiredField("mirror.target"))
        );
    }

    #[test]
    fn tenant_admission_requires_refill() {
        let mut contract = valid_contract();
        contract.tenant_quota.refill_per_second = 0;

        assert_eq!(
            contract.validate(),
            Err(PoolRuntimeError::InvalidQuota("refill_per_second"))
        );
    }

    fn valid_contract() -> PoolRuntimeContract {
        PoolRuntimeContract {
            settings_bucket: SettingsBucketPolicy {
                bucket_name: "default".to_string(),
                tracked_gucs: vec!["citus.enable_repartition_joins".to_string()],
                max_connections: 1_000,
            },
            fast_path_router: FastPathRouterPolicy {
                enabled: true,
                single_shard_only: true,
                fallback_target: RouteTarget {
                    host: "coordinator".to_string(),
                    port: 5432,
                },
            },
            mirror: MirrorTrafficPolicy {
                enabled: true,
                target: Some(RouteTarget {
                    host: "canary".to_string(),
                    port: 5432,
                }),
                sample_percent: 5,
            },
            htap: HtapRoutingPolicy {
                analytical_target: RouteTarget {
                    host: "analytical-sidecar".to_string(),
                    port: 7432,
                },
                max_staleness_ms: 2_000,
                predicate_hints: vec!["/*+ analytical */".to_string()],
            },
            pipeline: ProtocolPipelinePolicy {
                max_in_flight: 32,
                transaction_pipelining: true,
            },
            tls: TlsSessionTicketPolicy {
                enabled: true,
                rotation_seconds: 3_600,
            },
            tenant_quota: TenantAdmissionPolicy {
                tenant_id: "tenant-a".to_string(),
                burst: 1_000,
                refill_per_second: 100,
            },
            geo_router: GeoRoutingPolicy {
                default_region: "us-east-1".to_string(),
                rules: vec![GeoRoutingRule {
                    cidr: "10.0.0.0/8".to_string(),
                    region: "us-east-1".to_string(),
                }],
            },
            token_cache: TokenIntrospectionCachePolicy {
                max_entries: 10_000,
                ttl_seconds: 60,
            },
        }
    }
}
