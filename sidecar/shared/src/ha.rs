// FEATURE: SC7

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::str::FromStr;

const DEFAULT_PRIORITY: u16 = 100;
const DEFAULT_WEIGHT: u16 = 1;
const DEFAULT_FAILOVER_AFTER: u16 = 1;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RetargetConfig {
    endpoints: Vec<EndpointConfig>,
}

impl RetargetConfig {
    pub fn parse(input: &str) -> Result<Self, RetargetError> {
        input.parse()
    }

    pub fn endpoints(&self) -> &[EndpointConfig] {
        &self.endpoints
    }

    pub fn endpoint(&self, id: &str) -> Option<&EndpointConfig> {
        self.endpoints.iter().find(|endpoint| endpoint.id == id)
    }

    fn sorted(mut endpoints: Vec<EndpointConfig>) -> Self {
        endpoints.sort_by(|left, right| left.id.cmp(&right.id));
        Self { endpoints }
    }
}

impl FromStr for RetargetConfig {
    type Err = RetargetError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut endpoints = Vec::new();
        let mut seen_ids = BTreeSet::new();

        for raw_entry in input.split([';', '\n']) {
            let entry = raw_entry.trim();
            if entry.is_empty() || entry.starts_with('#') {
                continue;
            }

            let endpoint = parse_endpoint(entry)?;
            if !seen_ids.insert(endpoint.id.clone()) {
                return Err(RetargetError::DuplicateEndpoint(endpoint.id));
            }
            endpoints.push(endpoint);
        }

        if endpoints.is_empty() {
            return Err(RetargetError::EmptyConfig);
        }

        Ok(Self::sorted(endpoints))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EndpointConfig {
    pub id: String,
    pub target: String,
    pub priority: u16,
    pub weight: u16,
    pub failover_after: u16,
    pub zone: Option<String>,
}

impl EndpointConfig {
    pub fn new(id: impl Into<String>, target: impl Into<String>) -> Result<Self, RetargetError> {
        let endpoint = Self {
            id: id.into(),
            target: target.into(),
            priority: DEFAULT_PRIORITY,
            weight: DEFAULT_WEIGHT,
            failover_after: DEFAULT_FAILOVER_AFTER,
            zone: None,
        };
        endpoint.validate()?;
        Ok(endpoint)
    }

    pub fn with_priority(mut self, priority: u16) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_weight(mut self, weight: u16) -> Self {
        self.weight = weight;
        self
    }

    pub fn with_failover_after(mut self, failover_after: u16) -> Self {
        self.failover_after = failover_after;
        self
    }

    pub fn with_zone(mut self, zone: impl Into<String>) -> Self {
        self.zone = Some(zone.into());
        self
    }

    fn validate(&self) -> Result<(), RetargetError> {
        validate_identifier("id", &self.id)?;
        validate_target(&self.target)?;
        if self.weight == 0 {
            return Err(RetargetError::InvalidNumber {
                field: "weight",
                value: self.weight.to_string(),
            });
        }
        if self.failover_after == 0 {
            return Err(RetargetError::InvalidNumber {
                field: "failover_after",
                value: self.failover_after.to_string(),
            });
        }
        if let Some(zone) = &self.zone {
            validate_identifier("zone", zone)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum EndpointHealth {
    Ready,
    Degraded,
    Draining,
    Unhealthy,
}

impl EndpointHealth {
    fn can_accept_new_work(self) -> bool {
        matches!(self, Self::Ready | Self::Degraded)
    }

    fn rank(self) -> u8 {
        match self {
            Self::Ready => 0,
            Self::Degraded => 1,
            Self::Draining => 2,
            Self::Unhealthy => 3,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Degraded => "degraded",
            Self::Draining => "draining",
            Self::Unhealthy => "unhealthy",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EndpointStatus {
    pub health: EndpointHealth,
    pub consecutive_failures: u16,
    pub in_flight_work: u64,
    pub last_error: Option<String>,
}

impl EndpointStatus {
    pub fn ready() -> Self {
        Self {
            health: EndpointHealth::Ready,
            consecutive_failures: 0,
            in_flight_work: 0,
            last_error: None,
        }
    }

    pub fn degraded(consecutive_failures: u16, last_error: impl Into<String>) -> Self {
        Self {
            health: EndpointHealth::Degraded,
            consecutive_failures,
            in_flight_work: 0,
            last_error: Some(last_error.into()),
        }
    }

    pub fn unhealthy(consecutive_failures: u16, last_error: impl Into<String>) -> Self {
        Self {
            health: EndpointHealth::Unhealthy,
            consecutive_failures,
            in_flight_work: 0,
            last_error: Some(last_error.into()),
        }
    }

    pub fn with_in_flight_work(mut self, in_flight_work: u64) -> Self {
        self.in_flight_work = in_flight_work;
        self
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EndpointSelection {
    pub id: String,
    pub target: String,
    pub priority: u16,
    pub weight: u16,
    pub health: EndpointHealth,
    pub generation: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RetargetDecision {
    pub selection: Option<EndpointSelection>,
    pub reason: String,
    pub generation: u64,
}

impl RetargetDecision {
    pub fn selected_id(&self) -> Option<&str> {
        self.selection
            .as_ref()
            .map(|selection| selection.id.as_str())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EndpointReload {
    pub changed: bool,
    pub previous_generation: u64,
    pub generation: u64,
    pub endpoint_count: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EndpointRegistry {
    config: RetargetConfig,
    statuses: BTreeMap<String, EndpointStatus>,
    generation: u64,
}

impl EndpointRegistry {
    pub fn new(config: RetargetConfig) -> Self {
        let statuses = config
            .endpoints()
            .iter()
            .map(|endpoint| (endpoint.id.clone(), EndpointStatus::ready()))
            .collect();
        Self {
            config,
            statuses,
            generation: 1,
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn config(&self) -> &RetargetConfig {
        &self.config
    }

    pub fn status(&self, id: &str) -> Option<&EndpointStatus> {
        self.statuses.get(id)
    }

    pub fn set_status(&mut self, id: &str, status: EndpointStatus) -> Result<(), RetargetError> {
        if !self.statuses.contains_key(id) {
            return Err(RetargetError::UnknownEndpoint(id.to_string()));
        }
        self.statuses.insert(id.to_string(), status);
        Ok(())
    }

    pub fn record_success(&mut self, id: &str) -> Result<(), RetargetError> {
        self.set_status(id, EndpointStatus::ready())
    }

    pub fn record_failure(
        &mut self,
        id: &str,
        error: impl Into<String>,
    ) -> Result<(), RetargetError> {
        let endpoint = self
            .config
            .endpoint(id)
            .ok_or_else(|| RetargetError::UnknownEndpoint(id.to_string()))?;
        let error = error.into();
        let previous = self
            .statuses
            .get(id)
            .cloned()
            .unwrap_or_else(EndpointStatus::ready);
        let failures = previous.consecutive_failures.saturating_add(1);
        let status = if failures >= endpoint.failover_after {
            EndpointStatus::unhealthy(failures, error)
        } else {
            EndpointStatus::degraded(failures, error).with_in_flight_work(previous.in_flight_work)
        };
        self.statuses.insert(id.to_string(), status);
        Ok(())
    }

    pub fn begin_drain(&mut self, id: &str, in_flight_work: u64) -> Result<(), RetargetError> {
        self.set_status(
            id,
            EndpointStatus {
                health: EndpointHealth::Draining,
                consecutive_failures: 0,
                in_flight_work,
                last_error: None,
            },
        )
    }

    pub fn select(&self) -> RetargetDecision {
        let mut best: Option<(&EndpointConfig, &EndpointStatus)> = None;

        for endpoint in self.config.endpoints() {
            let Some(status) = self.statuses.get(&endpoint.id) else {
                continue;
            };
            if !status.health.can_accept_new_work()
                || status.consecutive_failures >= endpoint.failover_after
            {
                continue;
            }

            if let Some((best_endpoint, best_status)) = best {
                if candidate_key(endpoint, status) < candidate_key(best_endpoint, best_status) {
                    best = Some((endpoint, status));
                }
            } else {
                best = Some((endpoint, status));
            }
        }

        match best {
            Some((endpoint, status)) => RetargetDecision {
                selection: Some(EndpointSelection {
                    id: endpoint.id.clone(),
                    target: endpoint.target.clone(),
                    priority: endpoint.priority,
                    weight: endpoint.weight,
                    health: status.health,
                    generation: self.generation,
                }),
                reason: format!("selected {}", endpoint.id),
                generation: self.generation,
            },
            None => RetargetDecision {
                selection: None,
                reason: "no endpoint accepts new work".to_string(),
                generation: self.generation,
            },
        }
    }

    pub fn reload(&mut self, config: RetargetConfig) -> EndpointReload {
        let previous_generation = self.generation;
        let changed = self.config != config;
        if changed {
            let old_statuses = std::mem::take(&mut self.statuses);
            self.statuses = config
                .endpoints()
                .iter()
                .map(|endpoint| {
                    let status = old_statuses
                        .get(&endpoint.id)
                        .cloned()
                        .unwrap_or_else(EndpointStatus::ready);
                    (endpoint.id.clone(), status)
                })
                .collect();
            self.config = config;
            self.generation = self.generation.saturating_add(1);
        }
        EndpointReload {
            changed,
            previous_generation,
            generation: self.generation,
            endpoint_count: self.config.endpoints().len(),
        }
    }

    pub fn canonical_rows(&self) -> Vec<String> {
        self.config
            .endpoints()
            .iter()
            .map(|endpoint| {
                let status = self
                    .statuses
                    .get(&endpoint.id)
                    .cloned()
                    .unwrap_or_else(EndpointStatus::ready);
                format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    endpoint.id,
                    endpoint.target,
                    endpoint.priority,
                    endpoint.weight,
                    endpoint.failover_after,
                    status.health.as_str(),
                    status.consecutive_failures,
                )
            })
            .collect()
    }
}

type CandidateKey<'a> = (u16, u8, u16, u64, std::cmp::Reverse<u16>, &'a str);

fn candidate_key<'a>(endpoint: &'a EndpointConfig, status: &'a EndpointStatus) -> CandidateKey<'a> {
    (
        endpoint.priority,
        status.health.rank(),
        status.consecutive_failures,
        status.in_flight_work,
        std::cmp::Reverse(endpoint.weight),
        endpoint.id.as_str(),
    )
}

fn parse_endpoint(entry: &str) -> Result<EndpointConfig, RetargetError> {
    let mut id = None;
    let mut target = None;
    let mut priority = DEFAULT_PRIORITY;
    let mut weight = DEFAULT_WEIGHT;
    let mut failover_after = DEFAULT_FAILOVER_AFTER;
    let mut zone = None;

    if let Some((left, right)) = entry.split_once('=') {
        if !left.contains(',') && !right.contains(',') && !right.contains('=') {
            id = Some(left.trim().to_string());
            target = Some(right.trim().to_string());
        }
    }

    if id.is_none() || target.is_none() {
        for raw_field in entry.split(',') {
            let field = raw_field.trim();
            if field.is_empty() {
                continue;
            }
            let (key, value) = field
                .split_once('=')
                .ok_or_else(|| RetargetError::MalformedField(field.to_string()))?;
            let key = key.trim();
            let value = value.trim();
            match key {
                "id" | "name" => id = Some(value.to_string()),
                "target" | "addr" | "address" | "url" => target = Some(value.to_string()),
                "priority" => priority = parse_u16("priority", value)?,
                "weight" => weight = parse_u16("weight", value)?,
                "failover_after" | "failure_threshold" => {
                    failover_after = parse_u16("failover_after", value)?;
                }
                "zone" => zone = Some(value.to_string()),
                other => return Err(RetargetError::UnknownField(other.to_string())),
            }
        }
    }

    let mut endpoint = EndpointConfig {
        id: id.ok_or(RetargetError::MissingField("id"))?,
        target: target.ok_or(RetargetError::MissingField("target"))?,
        priority,
        weight,
        failover_after,
        zone,
    };
    endpoint.id = endpoint.id.trim().to_string();
    endpoint.target = endpoint.target.trim().to_string();
    endpoint.validate()?;
    Ok(endpoint)
}

fn parse_u16(field: &'static str, value: &str) -> Result<u16, RetargetError> {
    value
        .parse::<u16>()
        .map_err(|_| RetargetError::InvalidNumber {
            field,
            value: value.to_string(),
        })
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), RetargetError> {
    if value.trim().is_empty()
        || value
            .chars()
            .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.')))
    {
        return Err(RetargetError::InvalidIdentifier {
            field,
            value: value.to_string(),
        });
    }
    Ok(())
}

fn validate_target(target: &str) -> Result<(), RetargetError> {
    if target.trim().is_empty()
        || target.chars().any(char::is_whitespace)
        || !(target.starts_with("http://")
            || target.starts_with("https://")
            || target.starts_with("tcp://")
            || target.starts_with("unix://"))
    {
        return Err(RetargetError::InvalidTarget(target.to_string()));
    }
    Ok(())
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RetargetError {
    EmptyConfig,
    MissingField(&'static str),
    MalformedField(String),
    UnknownField(String),
    DuplicateEndpoint(String),
    UnknownEndpoint(String),
    InvalidIdentifier { field: &'static str, value: String },
    InvalidTarget(String),
    InvalidNumber { field: &'static str, value: String },
}

impl fmt::Display for RetargetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyConfig => write!(formatter, "retarget config contains no endpoints"),
            Self::MissingField(field) => write!(formatter, "retarget endpoint missing {field}"),
            Self::MalformedField(field) => write!(formatter, "malformed retarget field: {field}"),
            Self::UnknownField(field) => write!(formatter, "unknown retarget field: {field}"),
            Self::DuplicateEndpoint(id) => write!(formatter, "duplicate retarget endpoint: {id}"),
            Self::UnknownEndpoint(id) => write!(formatter, "unknown retarget endpoint: {id}"),
            Self::InvalidIdentifier { field, value } => {
                write!(formatter, "invalid retarget {field}: {value}")
            }
            Self::InvalidTarget(target) => write!(formatter, "invalid retarget target: {target}"),
            Self::InvalidNumber { field, value } => {
                write!(formatter, "invalid retarget {field}: {value}")
            }
        }
    }
}

impl Error for RetargetError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_config_and_sorts_by_id() {
        let config = RetargetConfig::parse(
            "id=west,target=http://10.0.0.2:8080,priority=20,weight=3,zone=us-west;\
             id=east,target=http://10.0.0.1:8080,priority=10,weight=5,failover_after=2",
        )
        .unwrap();

        assert_eq!(config.endpoints()[0].id, "east");
        assert_eq!(config.endpoints()[0].failover_after, 2);
        assert_eq!(config.endpoints()[1].zone.as_deref(), Some("us-west"));
    }

    #[test]
    fn rejects_duplicate_and_empty_configs() {
        assert_eq!(
            RetargetConfig::parse("primary=http://127.0.0.1:8080; primary=http://127.0.0.2:8080")
                .unwrap_err(),
            RetargetError::DuplicateEndpoint("primary".to_string())
        );
        assert_eq!(
            RetargetConfig::parse(" ;\n# comment").unwrap_err(),
            RetargetError::EmptyConfig
        );
    }

    #[test]
    fn selects_lowest_priority_then_ready_then_load_then_weight() {
        let config = RetargetConfig::parse(
            "id=a,target=http://a:8080,priority=10,weight=1;\
             id=b,target=http://b:8080,priority=10,weight=5;\
             id=c,target=http://c:8080,priority=20,weight=100",
        )
        .unwrap();
        let mut registry = EndpointRegistry::new(config);
        registry
            .set_status("b", EndpointStatus::ready().with_in_flight_work(2))
            .unwrap();

        assert_eq!(registry.select().selected_id(), Some("a"));

        registry
            .set_status("a", EndpointStatus::degraded(0, "slow"))
            .unwrap();
        assert_eq!(registry.select().selected_id(), Some("b"));
    }

    #[test]
    fn failure_retargets_and_fail_closed_when_all_endpoints_are_down() {
        let config = RetargetConfig::parse(
            "id=primary,target=http://primary:8080,priority=1,failover_after=1;\
             id=standby,target=http://standby:8080,priority=2,failover_after=1",
        )
        .unwrap();
        let mut registry = EndpointRegistry::new(config);

        assert_eq!(registry.select().selected_id(), Some("primary"));
        registry
            .record_failure("primary", "connection refused")
            .unwrap();
        assert_eq!(registry.select().selected_id(), Some("standby"));
        registry.record_failure("standby", "timeout").unwrap();

        let decision = registry.select();
        assert_eq!(decision.selected_id(), None);
        assert_eq!(decision.reason, "no endpoint accepts new work");
    }

    #[test]
    fn draining_endpoint_is_not_selected_for_new_work() {
        let config = RetargetConfig::parse(
            "id=primary,target=http://primary:8080,priority=1;\
             id=standby,target=http://standby:8080,priority=2",
        )
        .unwrap();
        let mut registry = EndpointRegistry::new(config);
        registry.begin_drain("primary", 7).unwrap();

        assert_eq!(registry.select().selected_id(), Some("standby"));
        assert_eq!(
            registry.status("primary").unwrap().health,
            EndpointHealth::Draining
        );
    }

    #[test]
    fn reload_preserves_status_by_id_and_increments_generation_only_on_change() {
        let first = RetargetConfig::parse(
            "id=primary,target=http://primary:8080,priority=1,failover_after=2;\
             id=standby,target=http://standby:8080,priority=2",
        )
        .unwrap();
        let mut registry = EndpointRegistry::new(first.clone());
        registry.record_failure("primary", "one miss").unwrap();

        let unchanged = registry.reload(first);
        assert!(!unchanged.changed);
        assert_eq!(unchanged.generation, 1);

        let second = RetargetConfig::parse(
            "id=primary,target=http://primary:8080,priority=1,failover_after=2;\
             id=standby,target=http://standby:8080,priority=2;\
             id=third,target=http://third:8080,priority=3",
        )
        .unwrap();
        let changed = registry.reload(second);

        assert!(changed.changed);
        assert_eq!(changed.previous_generation, 1);
        assert_eq!(changed.generation, 2);
        assert_eq!(registry.status("primary").unwrap().consecutive_failures, 1);
        assert_eq!(
            registry.status("third").unwrap().health,
            EndpointHealth::Ready
        );
    }
}
