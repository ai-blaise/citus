// FEATURE: Sec12
// FEATURE: T15

//! Data-plane admission control for the pool proxy.
//!
//! The proxy stays byte-transparent once a connection is admitted, but the
//! production data plane still needs bounded admission before it opens or
//! routes backend connections. This module owns those fail-closed gates:
//! concurrent connection slots, bounded startup reads, and per-tenant
//! connection token buckets derived from the PostgreSQL startup envelope.

use crate::trace_tap::{StartupTraceTap, STARTUP_TAP_MIN_TIMEOUT};
use std::error::Error;
use std::fmt;
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

pub const DEFAULT_STARTUP_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PoolAdmissionConfig {
    pub max_active_connections: Option<u64>,
    pub admission_timeout: Duration,
    pub startup_timeout: Duration,
    pub tenant_quota: Option<TenantQuotaConfig>,
}

impl PoolAdmissionConfig {
    pub fn from_env() -> Result<Self, PoolAdmissionError> {
        let max_active_connections =
            parse_optional_u64_env("AI_BLAISE_POOL_MAX_ACTIVE_CONNECTIONS", "positive integer")?;
        let admission_timeout = parse_duration_ms_env(
            "AI_BLAISE_POOL_ADMISSION_TIMEOUT_MS",
            Duration::ZERO,
            "non-negative integer milliseconds",
        )?;
        let startup_timeout = parse_duration_ms_env(
            "AI_BLAISE_POOL_STARTUP_TIMEOUT_MS",
            DEFAULT_STARTUP_TIMEOUT,
            "integer milliseconds >= 500",
        )?;
        let tenant_quota = TenantQuotaConfig::from_env()?;

        let config = Self {
            max_active_connections,
            admission_timeout,
            startup_timeout,
            tenant_quota,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), PoolAdmissionError> {
        if self.max_active_connections == Some(0) {
            return Err(PoolAdmissionError::InvalidMaxActiveConnections);
        }
        if self.startup_timeout < STARTUP_TAP_MIN_TIMEOUT {
            return Err(PoolAdmissionError::InvalidStartupTimeout {
                minimum_ms: duration_ms(STARTUP_TAP_MIN_TIMEOUT),
            });
        }
        if let Some(quota) = &self.tenant_quota {
            quota.validate()?;
        }
        Ok(())
    }
}

impl Default for PoolAdmissionConfig {
    fn default() -> Self {
        Self {
            max_active_connections: None,
            admission_timeout: Duration::ZERO,
            startup_timeout: DEFAULT_STARTUP_TIMEOUT,
            tenant_quota: None,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantQuotaConfig {
    pub tenant_id: String,
    pub burst: u32,
    pub refill_per_second: u32,
}

impl TenantQuotaConfig {
    fn from_env() -> Result<Option<Self>, PoolAdmissionError> {
        let tenant_id = std::env::var("AI_BLAISE_POOL_QUOTA_TENANT_ID").ok();
        let burst = std::env::var("AI_BLAISE_POOL_QUOTA_BURST").ok();
        let refill = std::env::var("AI_BLAISE_POOL_QUOTA_REFILL_PER_SECOND").ok();

        if tenant_id.is_none() && burst.is_none() && refill.is_none() {
            return Ok(None);
        }

        let tenant_id = tenant_id.ok_or(PoolAdmissionError::MissingEnv(
            "AI_BLAISE_POOL_QUOTA_TENANT_ID",
        ))?;
        let burst = parse_u32_env_value(
            "AI_BLAISE_POOL_QUOTA_BURST",
            burst.as_deref(),
            "positive integer",
        )?;
        let refill_per_second = parse_u32_env_value(
            "AI_BLAISE_POOL_QUOTA_REFILL_PER_SECOND",
            refill.as_deref(),
            "positive integer",
        )?;

        let config = Self {
            tenant_id,
            burst,
            refill_per_second,
        };
        config.validate()?;
        Ok(Some(config))
    }

    pub fn validate(&self) -> Result<(), PoolAdmissionError> {
        if self.tenant_id.trim().is_empty() {
            return Err(PoolAdmissionError::InvalidTenantQuota("tenant_id"));
        }
        if self.burst == 0 {
            return Err(PoolAdmissionError::InvalidTenantQuota("burst"));
        }
        if self.refill_per_second == 0 {
            return Err(PoolAdmissionError::InvalidTenantQuota("refill_per_second"));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct PoolAdmissionController {
    config: PoolAdmissionConfig,
    active_connections: Mutex<u64>,
    active_changed: Condvar,
    tenant_quota: Mutex<Option<TenantQuotaBucket>>,
}

impl PoolAdmissionController {
    pub fn new(config: PoolAdmissionConfig) -> Result<Self, PoolAdmissionError> {
        config.validate()?;
        let tenant_quota = config
            .tenant_quota
            .clone()
            .map(|quota| TenantQuotaBucket::new(quota, Instant::now()))
            .transpose()?;
        Ok(Self {
            config,
            active_connections: Mutex::new(0),
            active_changed: Condvar::new(),
            tenant_quota: Mutex::new(tenant_quota),
        })
    }

    pub fn config(&self) -> &PoolAdmissionConfig {
        &self.config
    }

    pub fn active_slots(&self) -> Result<u64, PoolAdmissionError> {
        self.active_connections
            .lock()
            .map(|active| *active)
            .map_err(|_| PoolAdmissionError::StatePoisoned("active_connections"))
    }

    pub fn acquire_connection(&self) -> Result<PoolConnectionPermit<'_>, PoolAdmissionError> {
        let Some(max_active_connections) = self.config.max_active_connections else {
            return Ok(PoolConnectionPermit {
                controller: self,
                limited: false,
            });
        };

        let mut active = self
            .active_connections
            .lock()
            .map_err(|_| PoolAdmissionError::StatePoisoned("active_connections"))?;
        if self.config.admission_timeout.is_zero() {
            if *active >= max_active_connections {
                return Err(PoolAdmissionError::Overloaded {
                    max_active_connections,
                    timeout_ms: 0,
                });
            }
        } else {
            let deadline = Instant::now() + self.config.admission_timeout;
            while *active >= max_active_connections {
                let now = Instant::now();
                if now >= deadline {
                    return Err(PoolAdmissionError::Overloaded {
                        max_active_connections,
                        timeout_ms: duration_ms(self.config.admission_timeout),
                    });
                }
                let wait_for = deadline.saturating_duration_since(now);
                let wait = self
                    .active_changed
                    .wait_timeout(active, wait_for)
                    .map_err(|_| PoolAdmissionError::StatePoisoned("active_connections"))?;
                active = wait.0;
                if wait.1.timed_out() && *active >= max_active_connections {
                    return Err(PoolAdmissionError::Overloaded {
                        max_active_connections,
                        timeout_ms: duration_ms(self.config.admission_timeout),
                    });
                }
            }
        }

        *active += 1;
        Ok(PoolConnectionPermit {
            controller: self,
            limited: true,
        })
    }

    pub fn admit_startup(&self, tap: &StartupTraceTap) -> Result<(), PoolAdmissionError> {
        if self.config.tenant_quota.is_none() {
            return Ok(());
        }

        let tenant_id = tenant_id_from_startup(tap).ok_or(PoolAdmissionError::MissingTenantId)?;
        let mut quota = self
            .tenant_quota
            .lock()
            .map_err(|_| PoolAdmissionError::StatePoisoned("tenant_quota"))?;
        let quota = quota
            .as_mut()
            .ok_or(PoolAdmissionError::StatePoisoned("tenant_quota"))?;
        quota.try_admit(&tenant_id, Instant::now()).map(|_| ())
    }
}

#[derive(Debug)]
pub struct PoolConnectionPermit<'a> {
    controller: &'a PoolAdmissionController,
    limited: bool,
}

impl Drop for PoolConnectionPermit<'_> {
    fn drop(&mut self) {
        if !self.limited {
            return;
        }
        if let Ok(mut active) = self.controller.active_connections.lock() {
            *active = active.saturating_sub(1);
            self.controller.active_changed.notify_one();
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantQuotaSnapshot {
    pub tokens: u32,
    pub admitted_total: u64,
    pub rejected_total: u64,
}

#[derive(Debug, Clone)]
pub struct TenantQuotaBucket {
    config: TenantQuotaConfig,
    tokens: u32,
    last_refill: Instant,
    admitted_total: u64,
    rejected_total: u64,
}

impl TenantQuotaBucket {
    pub fn new(config: TenantQuotaConfig, now: Instant) -> Result<Self, PoolAdmissionError> {
        config.validate()?;
        Ok(Self {
            tokens: config.burst,
            last_refill: now,
            admitted_total: 0,
            rejected_total: 0,
            config,
        })
    }

    pub fn try_admit(
        &mut self,
        tenant_id: &str,
        now: Instant,
    ) -> Result<TenantQuotaAdmission, PoolAdmissionError> {
        let tenant_id = tenant_id.trim();
        if tenant_id.is_empty() {
            self.rejected_total += 1;
            return Err(PoolAdmissionError::MissingTenantId);
        }
        if tenant_id != self.config.tenant_id {
            self.rejected_total += 1;
            return Err(PoolAdmissionError::UnknownTenant {
                tenant_id: tenant_id.to_string(),
                expected_tenant_id: self.config.tenant_id.clone(),
            });
        }

        self.refill(now);
        if self.tokens == 0 {
            self.rejected_total += 1;
            return Err(PoolAdmissionError::TenantQuotaExceeded {
                tenant_id: tenant_id.to_string(),
                available_tokens: 0,
            });
        }

        self.tokens -= 1;
        self.admitted_total += 1;
        Ok(TenantQuotaAdmission::Admitted {
            remaining_tokens: self.tokens,
        })
    }

    pub fn snapshot(&self) -> TenantQuotaSnapshot {
        TenantQuotaSnapshot {
            tokens: self.tokens,
            admitted_total: self.admitted_total,
            rejected_total: self.rejected_total,
        }
    }

    fn refill(&mut self, now: Instant) {
        if now <= self.last_refill {
            return;
        }
        let elapsed_ms = now.duration_since(self.last_refill).as_millis();
        let refill = elapsed_ms.saturating_mul(self.config.refill_per_second as u128) / 1_000;
        if refill == 0 {
            return;
        }
        let refill = refill.min(u32::MAX as u128) as u32;
        self.tokens = self.config.burst.min(self.tokens.saturating_add(refill));
        self.last_refill = now;
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TenantQuotaAdmission {
    Admitted { remaining_tokens: u32 },
}

pub fn tenant_id_from_startup(tap: &StartupTraceTap) -> Option<String> {
    for key in ["ai_blaise.tenant_id", "tenant_id", "tenant"] {
        if let Some(value) = tap.startup_parameter(key).and_then(non_empty_value) {
            return Some(value.to_string());
        }
    }

    if let Some(options) = tap.startup_parameter("options") {
        for key in ["ai_blaise.tenant_id", "tenant_id", "tenant"] {
            if let Some(value) = extract_options_assignment(options, key).and_then(non_empty_value)
            {
                return Some(value.to_string());
            }
        }
    }

    tap.startup_parameter("application_name")
        .and_then(|application_name| {
            extract_application_assignment(application_name, "tenant_id")
                .or_else(|| extract_application_assignment(application_name, "tenant"))
        })
        .and_then(non_empty_value)
        .map(str::to_string)
}

fn extract_options_assignment<'a>(options: &'a str, key: &str) -> Option<&'a str> {
    let tokens = options.split_whitespace().collect::<Vec<_>>();
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index];
        if let Some(remainder) = token.strip_prefix("-c") {
            let assignment = if remainder.is_empty() {
                index += 1;
                if index >= tokens.len() {
                    break;
                }
                tokens[index]
            } else {
                remainder
            };
            if let Some((assignment_key, assignment_value)) = assignment.split_once('=') {
                if assignment_key == key {
                    return Some(assignment_value);
                }
            }
        }
        index += 1;
    }
    None
}

fn extract_application_assignment<'a>(application_name: &'a str, key: &str) -> Option<&'a str> {
    for pair in application_name.split(';') {
        let Some((field, value)) = pair.trim().split_once('=') else {
            continue;
        };
        if field.trim() == key {
            return Some(value.trim());
        }
    }
    None
}

fn non_empty_value(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn parse_optional_u64_env(
    name: &'static str,
    expected: &'static str,
) -> Result<Option<u64>, PoolAdmissionError> {
    let Ok(raw) = std::env::var(name) else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let value = trimmed
        .parse::<u64>()
        .map_err(|_| PoolAdmissionError::InvalidEnv {
            name,
            value: raw.clone(),
            expected,
        })?;
    if value == 0 {
        return Err(PoolAdmissionError::InvalidEnv {
            name,
            value: raw,
            expected,
        });
    }
    Ok(Some(value))
}

fn parse_duration_ms_env(
    name: &'static str,
    default: Duration,
    expected: &'static str,
) -> Result<Duration, PoolAdmissionError> {
    let Ok(raw) = std::env::var(name) else {
        return Ok(default);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(default);
    }
    let value = trimmed
        .parse::<u64>()
        .map_err(|_| PoolAdmissionError::InvalidEnv {
            name,
            value: raw,
            expected,
        })?;
    Ok(Duration::from_millis(value))
}

fn parse_u32_env_value(
    name: &'static str,
    value: Option<&str>,
    expected: &'static str,
) -> Result<u32, PoolAdmissionError> {
    let raw = value.ok_or(PoolAdmissionError::MissingEnv(name))?;
    let parsed = raw
        .trim()
        .parse::<u32>()
        .map_err(|_| PoolAdmissionError::InvalidEnv {
            name,
            value: raw.to_string(),
            expected,
        })?;
    if parsed == 0 {
        return Err(PoolAdmissionError::InvalidEnv {
            name,
            value: raw.to_string(),
            expected,
        });
    }
    Ok(parsed)
}

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u64::MAX as u128) as u64
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PoolAdmissionError {
    InvalidEnv {
        name: &'static str,
        value: String,
        expected: &'static str,
    },
    InvalidMaxActiveConnections,
    InvalidStartupTimeout {
        minimum_ms: u64,
    },
    InvalidTenantQuota(&'static str),
    MissingEnv(&'static str),
    MissingTenantId,
    Overloaded {
        max_active_connections: u64,
        timeout_ms: u64,
    },
    StatePoisoned(&'static str),
    TenantQuotaExceeded {
        tenant_id: String,
        available_tokens: u32,
    },
    UnknownTenant {
        tenant_id: String,
        expected_tenant_id: String,
    },
}

impl fmt::Display for PoolAdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEnv {
                name,
                value,
                expected,
            } => write!(formatter, "{name}={value:?} must be {expected}"),
            Self::InvalidMaxActiveConnections => {
                write!(formatter, "max_active_connections must be greater than zero")
            }
            Self::InvalidStartupTimeout { minimum_ms } => {
                write!(formatter, "startup_timeout_ms must be at least {minimum_ms}")
            }
            Self::InvalidTenantQuota(field) => {
                write!(formatter, "tenant quota {field} must be non-empty and positive")
            }
            Self::MissingEnv(name) => {
                write!(formatter, "{name} is required when tenant quota is enabled")
            }
            Self::MissingTenantId => write!(formatter, "tenant_id is required for pool admission"),
            Self::Overloaded {
                max_active_connections,
                timeout_ms,
            } => write!(
                formatter,
                "pool admission overloaded after {timeout_ms}ms with max_active_connections={max_active_connections}",
            ),
            Self::StatePoisoned(field) => write!(formatter, "pool admission state poisoned: {field}"),
            Self::TenantQuotaExceeded {
                tenant_id,
                available_tokens,
            } => write!(
                formatter,
                "tenant {tenant_id} exceeded pool admission quota with {available_tokens} tokens available",
            ),
            Self::UnknownTenant {
                tenant_id,
                expected_tenant_id,
            } => write!(
                formatter,
                "tenant {tenant_id} is not configured for this pool quota; expected {expected_tenant_id}",
            ),
        }
    }
}

impl Error for PoolAdmissionError {}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_blaise_citus_sidecar_shared::ApplicationNameFields;

    fn quota() -> TenantQuotaConfig {
        TenantQuotaConfig {
            tenant_id: "tenant-a".to_string(),
            burst: 1,
            refill_per_second: 1,
        }
    }

    fn startup(parameters: Vec<(&str, &str)>) -> StartupTraceTap {
        StartupTraceTap {
            fields: ApplicationNameFields::default(),
            buffered_bytes: Vec::new(),
            parameters: parameters
                .into_iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
            special_envelope: false,
        }
    }

    #[test]
    fn connection_gate_times_out_when_pool_is_full() {
        let controller = PoolAdmissionController::new(PoolAdmissionConfig {
            max_active_connections: Some(1),
            admission_timeout: Duration::from_millis(5),
            startup_timeout: DEFAULT_STARTUP_TIMEOUT,
            tenant_quota: None,
        })
        .expect("controller");

        let first = controller.acquire_connection().expect("first permit");
        assert!(matches!(
            controller.acquire_connection(),
            Err(PoolAdmissionError::Overloaded {
                max_active_connections: 1,
                timeout_ms: 5,
            })
        ));
        drop(first);
        let second = controller.acquire_connection().expect("second permit");
        drop(second);
        assert_eq!(controller.active_slots(), Ok(0));
    }

    #[test]
    fn tenant_quota_rejects_over_budget_and_refills() {
        let now = Instant::now();
        let mut bucket = TenantQuotaBucket::new(quota(), now).expect("bucket");

        assert_eq!(
            bucket.try_admit("tenant-a", now),
            Ok(TenantQuotaAdmission::Admitted {
                remaining_tokens: 0,
            })
        );
        assert!(matches!(
            bucket.try_admit("tenant-a", now),
            Err(PoolAdmissionError::TenantQuotaExceeded { .. })
        ));
        assert_eq!(bucket.snapshot().rejected_total, 1);

        assert_eq!(
            bucket.try_admit("tenant-a", now + Duration::from_secs(1)),
            Ok(TenantQuotaAdmission::Admitted {
                remaining_tokens: 0,
            })
        );
    }

    #[test]
    fn tenant_quota_fails_closed_for_missing_or_unknown_tenant() {
        let now = Instant::now();
        let mut bucket = TenantQuotaBucket::new(quota(), now).expect("bucket");

        assert_eq!(
            bucket.try_admit("", now),
            Err(PoolAdmissionError::MissingTenantId)
        );
        assert_eq!(
            bucket.try_admit("tenant-b", now),
            Err(PoolAdmissionError::UnknownTenant {
                tenant_id: "tenant-b".to_string(),
                expected_tenant_id: "tenant-a".to_string(),
            })
        );
        assert_eq!(bucket.snapshot().rejected_total, 2);
    }

    #[test]
    fn extracts_tenant_from_application_name_or_options() {
        assert_eq!(
            tenant_id_from_startup(&startup(vec![(
                "application_name",
                "application=psql;tenant_id=tenant-a"
            ),])),
            Some("tenant-a".to_string())
        );
        assert_eq!(
            tenant_id_from_startup(&startup(vec![(
                "options",
                "-c ai_blaise.tenant_id=tenant-b -c search_path=public"
            ),])),
            Some("tenant-b".to_string())
        );
    }
}
