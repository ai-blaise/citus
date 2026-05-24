// FEATURE: MR5

//! GeoIP routing.
//!
//! This module owns fail-closed CIDR/replica-table parsing and deterministic
//! route reports for the pool-side GeoIP boundary. Managed GeoIP database
//! loading, Region-CR synchronization, hot swapping, and live read routing are
//! outside this bounded contract until proven by separate data-plane evidence.

use crate::{GeoRoutingPolicy, GeoRoutingRule, PoolRuntimeError, RouteTarget};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Replica metadata for a single region, ordered by latency rank.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RegionReplica {
    pub region: String,
    pub latency_rank: u32,
    pub target: RouteTarget,
}

impl RegionReplica {
    fn validate(&self) -> Result<(), GeoIpError> {
        if self.region.trim().is_empty() {
            return Err(GeoIpError::MissingField("region"));
        }
        validate_region(&self.region)?;
        if self.target.host.trim().is_empty() {
            return Err(GeoIpError::MissingField("target.host"));
        }
        if self.target.host.chars().any(char::is_whitespace) {
            return Err(GeoIpError::InvalidHost(self.target.host.clone()));
        }
        if self.target.port == 0 {
            return Err(GeoIpError::InvalidPort);
        }
        Ok(())
    }
}

/// Closest-replica lookup table. Keyed on the resolved client region; values
/// are ordered lists of replicas sorted by `latency_rank` (lower is closer).
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct ClosestReplicaTable {
    replicas: BTreeMap<String, Vec<RegionReplica>>,
}

impl ClosestReplicaTable {
    pub fn from_specs(specs: &[&str]) -> Result<Self, GeoIpError> {
        let mut table = Self::default();
        for spec in specs {
            table.insert(parse_replica_spec(spec)?)?;
        }
        Ok(table)
    }

    pub fn insert(&mut self, replica: RegionReplica) -> Result<(), GeoIpError> {
        replica.validate()?;
        let entries = self.replicas.entry(replica.region.clone()).or_default();
        entries.push(replica);
        entries.sort_by_key(|entry| entry.latency_rank);
        Ok(())
    }

    pub fn closest_for_region(&self, region: &str) -> Option<&RegionReplica> {
        self.replicas
            .get(region)
            .and_then(|entries| entries.first())
    }

    pub fn region_count(&self) -> usize {
        self.replicas.len()
    }
}

/// Decide the route for `client_ip` given the policy + lookup table.
pub fn route_for_client(
    policy: &GeoRoutingPolicy,
    table: &ClosestReplicaTable,
    client_ip: IpAddr,
    resolved_region: Option<&str>,
) -> Result<RegionReplica, GeoIpError> {
    route_report_for_client(policy, table, client_ip, resolved_region).map(|report| report.replica)
}

pub fn route_report_for_client(
    policy: &GeoRoutingPolicy,
    table: &ClosestReplicaTable,
    client_ip: IpAddr,
    resolved_region: Option<&str>,
) -> Result<GeoRouteReport, GeoIpError> {
    policy.validate().map_err(GeoIpError::Runtime)?;
    if let Some(region) = resolved_region {
        if region.trim().is_empty() {
            return Err(GeoIpError::MissingField("resolved_region"));
        }
        validate_region(region)?;
    }

    let requested_region = resolved_region
        .map(str::to_string)
        .unwrap_or_else(|| resolve_region_via_rules(policy, client_ip));
    let (replica, fallback_used) = match table.closest_for_region(&requested_region) {
        Some(replica) => (replica.clone(), false),
        None => match table.closest_for_region(&policy.default_region) {
            Some(replica) => (replica.clone(), true),
            None => return Err(GeoIpError::NoReplicaForRegion(requested_region)),
        },
    };

    Ok(GeoRouteReport {
        client_ip,
        requested_region,
        selected_region: replica.region.clone(),
        fallback_used,
        replica,
    })
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GeoRouteReport {
    pub client_ip: IpAddr,
    pub requested_region: String,
    pub selected_region: String,
    pub fallback_used: bool,
    pub replica: RegionReplica,
}

fn resolve_region_via_rules(policy: &GeoRoutingPolicy, client_ip: IpAddr) -> String {
    for rule in &policy.rules {
        if let Some((cidr, prefix)) = parse_cidr(&rule.cidr) {
            if matches!(
                (cidr, client_ip),
                (IpAddr::V4(_), IpAddr::V4(_)) | (IpAddr::V6(_), IpAddr::V6(_))
            ) && cidr_contains(cidr, client_ip, prefix)
            {
                return rule.region.clone();
            }
        }
    }
    policy.default_region.clone()
}

fn parse_replica_spec(spec: &str) -> Result<RegionReplica, GeoIpError> {
    let parts = spec.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 4 {
        return Err(GeoIpError::InvalidReplicaSpec(spec.to_string()));
    }
    let latency_rank = parts[1]
        .parse::<u32>()
        .map_err(|_| GeoIpError::InvalidReplicaSpec(spec.to_string()))?;
    let port = parts[3]
        .parse::<u16>()
        .map_err(|_| GeoIpError::InvalidReplicaSpec(spec.to_string()))?;
    let replica = RegionReplica {
        region: parts[0].to_string(),
        latency_rank,
        target: RouteTarget {
            host: parts[2].to_string(),
            port,
        },
    };
    replica.validate()?;
    Ok(replica)
}

fn parse_cidr(value: &str) -> Option<(IpAddr, u8)> {
    let (host, prefix) = value.split_once('/')?;
    if host.contains('/') || prefix.contains('/') {
        return None;
    }
    let ip: IpAddr = host.parse().ok()?;
    let prefix: u8 = prefix.parse().ok()?;
    match ip {
        IpAddr::V4(_) if prefix <= 32 => Some((ip, prefix)),
        IpAddr::V6(_) if prefix <= 128 => Some((ip, prefix)),
        _ => None,
    }
}

fn cidr_contains(network: IpAddr, candidate: IpAddr, prefix: u8) -> bool {
    match (network, candidate) {
        (IpAddr::V4(network), IpAddr::V4(candidate)) => v4_contains(network, candidate, prefix),
        (IpAddr::V6(network), IpAddr::V6(candidate)) => v6_contains(network, candidate, prefix),
        _ => false,
    }
}

fn v4_contains(network: Ipv4Addr, candidate: Ipv4Addr, prefix: u8) -> bool {
    if prefix > 32 {
        return false;
    }
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    (u32::from(network) & mask) == (u32::from(candidate) & mask)
}

fn v6_contains(network: Ipv6Addr, candidate: Ipv6Addr, prefix: u8) -> bool {
    if prefix > 128 {
        return false;
    }
    let mask = if prefix == 0 {
        0
    } else {
        u128::MAX << (128 - prefix)
    };
    (u128::from(network) & mask) == (u128::from(candidate) & mask)
}

fn validate_region(region: &str) -> Result<(), GeoIpError> {
    if region
        .chars()
        .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.')))
    {
        return Err(GeoIpError::InvalidRegion(region.to_string()));
    }
    Ok(())
}

/// Helper to build a single-rule policy in tests.
pub fn policy_with_default(
    default_region: impl Into<String>,
    rules: Vec<GeoRoutingRule>,
) -> GeoRoutingPolicy {
    GeoRoutingPolicy {
        default_region: default_region.into(),
        rules,
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum GeoIpError {
    InvalidHost(String),
    InvalidPort,
    InvalidRegion(String),
    InvalidReplicaSpec(String),
    MissingField(&'static str),
    NoReplicaForRegion(String),
    Runtime(PoolRuntimeError),
}

impl fmt::Display for GeoIpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHost(host) => write!(formatter, "invalid replica host {host}"),
            Self::InvalidPort => write!(formatter, "target.port must be greater than zero"),
            Self::InvalidRegion(region) => write!(formatter, "invalid region {region}"),
            Self::InvalidReplicaSpec(spec) => write!(
                formatter,
                "replica spec {spec} must use region,latency_rank,host,port"
            ),
            Self::MissingField(field) => write!(formatter, "{field} must not be empty"),
            Self::NoReplicaForRegion(region) => {
                write!(formatter, "no replica for region {region}")
            }
            Self::Runtime(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for GeoIpError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn replica(region: &str, host: &str, rank: u32) -> RegionReplica {
        RegionReplica {
            region: region.to_string(),
            latency_rank: rank,
            target: RouteTarget {
                host: host.to_string(),
                port: 5432,
            },
        }
    }

    #[test]
    fn closest_for_region_returns_lowest_rank() {
        let mut table = ClosestReplicaTable::default();
        table
            .insert(replica("us-east-1", "replica-2", 2))
            .expect("a");
        table
            .insert(replica("us-east-1", "replica-1", 1))
            .expect("b");
        let closest = table.closest_for_region("us-east-1").expect("closest");
        assert_eq!(closest.target.host, "replica-1");
    }

    #[test]
    fn route_uses_resolved_region_when_available() {
        let mut table = ClosestReplicaTable::default();
        table
            .insert(replica("eu-west-1", "eu-replica", 1))
            .expect("eu");
        table
            .insert(replica("us-east-1", "us-replica", 1))
            .expect("us");
        let policy = policy_with_default(
            "us-east-1",
            vec![GeoRoutingRule {
                cidr: "10.0.0.0/8".to_string(),
                region: "us-east-1".to_string(),
            }],
        );
        let replica = route_for_client(
            &policy,
            &table,
            "10.0.0.1".parse().unwrap(),
            Some("eu-west-1"),
        )
        .expect("route");
        assert_eq!(replica.target.host, "eu-replica");
    }

    #[test]
    fn cidr_rules_pick_region_when_resolver_silent() {
        let mut table = ClosestReplicaTable::default();
        table
            .insert(replica("ap-south-1", "ap-replica", 1))
            .expect("ap");
        table
            .insert(replica("us-east-1", "us-replica", 1))
            .expect("us");
        let policy = policy_with_default(
            "us-east-1",
            vec![GeoRoutingRule {
                cidr: "203.0.113.0/24".to_string(),
                region: "ap-south-1".to_string(),
            }],
        );
        let replica = route_for_client(&policy, &table, "203.0.113.10".parse().unwrap(), None)
            .expect("route");
        assert_eq!(replica.target.host, "ap-replica");
    }

    #[test]
    fn falls_back_to_default_region() {
        let mut table = ClosestReplicaTable::default();
        table
            .insert(replica("us-east-1", "us-replica", 1))
            .expect("us");
        let policy = policy_with_default(
            "us-east-1",
            vec![GeoRoutingRule {
                cidr: "10.0.0.0/8".to_string(),
                region: "us-east-1".to_string(),
            }],
        );
        let report = route_report_for_client(&policy, &table, "8.8.8.8".parse().unwrap(), None)
            .expect("route");
        assert_eq!(report.replica.target.host, "us-replica");
        assert!(!report.fallback_used);
    }

    #[test]
    fn resolved_unknown_region_falls_back_to_default() {
        let table =
            ClosestReplicaTable::from_specs(&["us-east-1,1,us-replica,5432"]).expect("table");
        let policy = policy_with_default(
            "us-east-1",
            vec![GeoRoutingRule {
                cidr: "10.0.0.0/8".to_string(),
                region: "us-east-1".to_string(),
            }],
        );
        let report = route_report_for_client(
            &policy,
            &table,
            "198.51.100.9".parse().unwrap(),
            Some("ap-south-1"),
        )
        .expect("route");
        assert!(report.fallback_used);
        assert_eq!(report.selected_region, "us-east-1");
    }

    #[test]
    fn no_replica_returns_error() {
        let table = ClosestReplicaTable::default();
        let policy = policy_with_default(
            "us-east-1",
            vec![GeoRoutingRule {
                cidr: "10.0.0.0/8".to_string(),
                region: "us-east-1".to_string(),
            }],
        );
        let err = route_for_client(&policy, &table, "10.0.0.1".parse().unwrap(), None)
            .expect_err("route");
        assert!(matches!(err, GeoIpError::NoReplicaForRegion(_)));
    }

    #[test]
    fn replica_specs_fail_closed() {
        let table = ClosestReplicaTable::from_specs(&[
            "us-east-1,10,us-replica,5432",
            "eu-west-1,5,eu-replica,5432",
        ])
        .expect("table");
        assert_eq!(table.region_count(), 2);
        assert!(matches!(
            ClosestReplicaTable::from_specs(&["us-east-1,bad,host,5432"]),
            Err(GeoIpError::InvalidReplicaSpec(_))
        ));
        assert!(matches!(
            ClosestReplicaTable::from_specs(&["us east,1,host,5432"]),
            Err(GeoIpError::InvalidRegion(_))
        ));
    }
}
