//! HLC sidecar runtime: peer clock exchange and closed-timestamp tracking.
//!
//! This is the production runtime that backs closed-timestamp follower reads
//! (`FEATURE: S9`, `FEATURE: MR6`). It runs entirely in std-only Rust to
//! stay aligned with the rest of the sidecar workspace, so it can be embedded
//! by `txn_status` and exercised by smoke tests without bringing in tokio,
//! tonic, or extra async runtimes.
//!
//! Closed-timestamp invariant: `closed_at = max_observed_wall_clock_ms -
//! max_offset_ms`. Anything strictly less than `closed_at` is safe to serve
//! from any follower replica.

// FEATURE: S9
// FEATURE: MR6

use crate::{ClosedTimestampPlan, HlcClock, HlcError, HlcTimestamp};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

/// Errors raised by the HLC runtime.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum HlcRuntimeError {
    MissingShardGroup,
    UnknownPeer(String),
    ZeroStalenessBudget,
    Hlc(HlcError),
}

impl fmt::Display for HlcRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingShardGroup => write!(formatter, "shard_group must be set"),
            Self::UnknownPeer(peer) => write!(formatter, "unknown HLC peer: {peer}"),
            Self::ZeroStalenessBudget => {
                write!(formatter, "max_offset_ms must be greater than zero")
            }
            Self::Hlc(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for HlcRuntimeError {}

impl From<HlcError> for HlcRuntimeError {
    fn from(error: HlcError) -> Self {
        Self::Hlc(error)
    }
}

/// Peer clock-exchange message broadcast between HLC sidecars.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PeerClockExchange {
    pub from: String,
    pub timestamp: HlcTimestamp,
}

/// Per-shard-group HLC runtime. Tracks the local clock, the highest
/// timestamp observed from each peer, and the closed timestamp derived from
/// `max_observed - max_offset_ms`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HlcRuntime {
    shard_group: String,
    clock: HlcClock,
    peers: BTreeMap<String, HlcTimestamp>,
    closed_at: HlcTimestamp,
    replica_count: u32,
    max_staleness_ms: u64,
}

impl HlcRuntime {
    pub fn new(
        shard_group: impl Into<String>,
        clock: HlcClock,
        peers: Vec<String>,
        replica_count: u32,
        max_staleness_ms: u64,
    ) -> Result<Self, HlcRuntimeError> {
        let shard_group = shard_group.into();
        if shard_group.trim().is_empty() {
            return Err(HlcRuntimeError::MissingShardGroup);
        }
        if clock.max_offset_ms == 0 {
            return Err(HlcRuntimeError::ZeroStalenessBudget);
        }
        if max_staleness_ms == 0 {
            return Err(HlcRuntimeError::ZeroStalenessBudget);
        }
        let mut peer_map = BTreeMap::new();
        for peer in peers {
            if peer.trim().is_empty() {
                return Err(HlcRuntimeError::UnknownPeer(peer));
            }
            peer_map.insert(
                peer,
                HlcTimestamp {
                    physical_ms: 0,
                    logical: 0,
                },
            );
        }
        let closed_at = compute_closed_timestamp(&clock.timestamp, &peer_map, clock.max_offset_ms);
        Ok(Self {
            shard_group,
            clock,
            peers: peer_map,
            closed_at,
            replica_count,
            max_staleness_ms,
        })
    }

    pub fn shard_group(&self) -> &str {
        &self.shard_group
    }

    pub fn clock(&self) -> &HlcClock {
        &self.clock
    }

    pub fn closed_timestamp(&self) -> HlcTimestamp {
        self.closed_at
    }

    pub fn peers(&self) -> &BTreeMap<String, HlcTimestamp> {
        &self.peers
    }

    pub fn replica_count(&self) -> u32 {
        self.replica_count
    }

    pub fn max_staleness_ms(&self) -> u64 {
        self.max_staleness_ms
    }

    /// Advance the local clock from a wall-clock reading.
    pub fn tick(&mut self, physical_ms: u64) -> Result<HlcTimestamp, HlcRuntimeError> {
        let timestamp = self.clock.tick(physical_ms)?;
        self.refresh_closed_at();
        Ok(timestamp)
    }

    /// Merge an inbound peer-clock-exchange and advance the closed timestamp.
    pub fn observe_peer(
        &mut self,
        exchange: PeerClockExchange,
        physical_ms: u64,
    ) -> Result<HlcTimestamp, HlcRuntimeError> {
        if !self.peers.contains_key(&exchange.from) {
            return Err(HlcRuntimeError::UnknownPeer(exchange.from));
        }
        let merged = self.clock.observe(exchange.timestamp, physical_ms)?;
        let entry = self
            .peers
            .entry(exchange.from.clone())
            .or_insert(HlcTimestamp {
                physical_ms: 0,
                logical: 0,
            });
        if exchange.timestamp > *entry {
            *entry = exchange.timestamp;
        }
        self.refresh_closed_at();
        Ok(merged)
    }

    /// Build the closed-timestamp plan this runtime currently serves.
    pub fn closed_timestamp_plan(&self) -> Result<ClosedTimestampPlan, HlcError> {
        let plan = ClosedTimestampPlan {
            shard_group: self.shard_group.clone(),
            closed_at: self.closed_at,
            max_staleness_ms: self.max_staleness_ms,
            replica_count: self.replica_count,
        };
        plan.validate()?;
        Ok(plan)
    }

    fn refresh_closed_at(&mut self) {
        self.closed_at =
            compute_closed_timestamp(&self.clock.timestamp, &self.peers, self.clock.max_offset_ms);
    }
}

fn compute_closed_timestamp(
    local: &HlcTimestamp,
    peers: &BTreeMap<String, HlcTimestamp>,
    max_offset_ms: u64,
) -> HlcTimestamp {
    let max_observed = peers
        .values()
        .fold(*local, |acc, peer| if peer > &acc { *peer } else { acc });
    HlcTimestamp {
        physical_ms: max_observed.physical_ms.saturating_sub(max_offset_ms),
        logical: 0,
    }
}

/// Deterministic three-node clock exchange report.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HlcRuntimeReport {
    pub shard_group: String,
    pub local_clock: HlcTimestamp,
    pub closed_at: HlcTimestamp,
    pub max_offset_ms: u64,
    pub max_staleness_ms: u64,
    pub replica_count: u32,
    pub peers: BTreeMap<String, HlcTimestamp>,
}

pub fn canonical_hlc_runtime_report() -> Result<HlcRuntimeReport, HlcRuntimeError> {
    let local_clock = HlcClock {
        node_id: "worker-a".to_string(),
        timestamp: HlcTimestamp {
            physical_ms: 1_700_000_000,
            logical: 0,
        },
        max_offset_ms: 500,
    };
    let mut runtime = HlcRuntime::new(
        "orders-sg",
        local_clock,
        vec!["worker-b".to_string(), "worker-c".to_string()],
        3,
        5_000,
    )?;

    runtime.tick(1_700_000_010)?;
    runtime.observe_peer(
        PeerClockExchange {
            from: "worker-b".to_string(),
            timestamp: HlcTimestamp {
                physical_ms: 1_700_000_005,
                logical: 3,
            },
        },
        1_700_000_011,
    )?;
    runtime.observe_peer(
        PeerClockExchange {
            from: "worker-c".to_string(),
            timestamp: HlcTimestamp {
                physical_ms: 1_700_000_020,
                logical: 1,
            },
        },
        1_700_000_021,
    )?;

    Ok(HlcRuntimeReport {
        shard_group: runtime.shard_group().to_string(),
        local_clock: runtime.clock().timestamp,
        closed_at: runtime.closed_timestamp(),
        max_offset_ms: runtime.clock().max_offset_ms,
        max_staleness_ms: runtime.max_staleness_ms(),
        replica_count: runtime.replica_count(),
        peers: runtime.peers().clone(),
    })
}

/// Render the closed-timestamp record as the JSON body served by the
/// `/closed_ts` HTTP route. Stable shape so callers (txn_status, follower
/// readers) can parse it without pulling serde.
pub fn render_closed_ts_json(report: &HlcRuntimeReport) -> String {
    let peers_json = report
        .peers
        .iter()
        .map(|(id, ts)| {
            format!(
                "{{\"node\":\"{}\",\"physical_ms\":{},\"logical\":{}}}",
                id, ts.physical_ms, ts.logical
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"shard_group\":\"{}\",\"closed_at\":{{\"physical_ms\":{},\"logical\":{}}},\"max_offset_ms\":{},\"max_staleness_ms\":{},\"replica_count\":{},\"peers\":[{}]}}\n",
        report.shard_group,
        report.closed_at.physical_ms,
        report.closed_at.logical,
        report.max_offset_ms,
        report.max_staleness_ms,
        report.replica_count,
        peers_json,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_clock() -> HlcClock {
        HlcClock {
            node_id: "worker-a".to_string(),
            timestamp: HlcTimestamp {
                physical_ms: 1_700_000_000,
                logical: 0,
            },
            max_offset_ms: 500,
        }
    }

    #[test]
    fn missing_shard_group_is_rejected() {
        let result = HlcRuntime::new("  ", fixture_clock(), vec!["worker-b".to_string()], 2, 500);
        assert_eq!(result.err(), Some(HlcRuntimeError::MissingShardGroup));
    }

    #[test]
    fn observing_unknown_peer_is_rejected() {
        let mut runtime = HlcRuntime::new(
            "orders-sg",
            fixture_clock(),
            vec!["worker-b".to_string()],
            2,
            500,
        )
        .expect("runtime");

        let result = runtime.observe_peer(
            PeerClockExchange {
                from: "worker-z".to_string(),
                timestamp: HlcTimestamp {
                    physical_ms: 1_700_000_001,
                    logical: 0,
                },
            },
            1_700_000_001,
        );
        assert_eq!(
            result.err(),
            Some(HlcRuntimeError::UnknownPeer("worker-z".to_string()))
        );
    }

    #[test]
    fn closed_timestamp_advances_after_peer_exchange() {
        let mut runtime = HlcRuntime::new(
            "orders-sg",
            fixture_clock(),
            vec!["worker-b".to_string(), "worker-c".to_string()],
            3,
            5_000,
        )
        .expect("runtime");

        runtime.tick(1_700_000_100).expect("tick");
        runtime
            .observe_peer(
                PeerClockExchange {
                    from: "worker-c".to_string(),
                    timestamp: HlcTimestamp {
                        physical_ms: 1_700_000_200,
                        logical: 0,
                    },
                },
                1_700_000_201,
            )
            .expect("observe");

        // max_observed = max(local_clock, peer) after observe(); the peer
        // advanced both. closed_at = max_observed - max_offset_ms.
        let closed = runtime.closed_timestamp().physical_ms;
        let local = runtime.clock().timestamp.physical_ms;
        let max_offset = runtime.clock().max_offset_ms;
        assert!(closed >= local - max_offset);
        assert!(closed <= local);
        // Closed timestamp must be strictly greater than the initial closed
        // (which was 1_700_000_000 - 500 = 1_699_999_500).
        assert!(closed > 1_699_999_500);
    }

    #[test]
    fn canonical_runtime_is_deterministic() {
        let report = canonical_hlc_runtime_report().expect("report");
        assert_eq!(report.shard_group, "orders-sg");
        assert_eq!(report.peers.len(), 2);
        assert!(report.closed_at.physical_ms <= report.local_clock.physical_ms);
    }

    #[test]
    fn closed_ts_json_is_stable() {
        let report = canonical_hlc_runtime_report().expect("report");
        let json = render_closed_ts_json(&report);
        assert!(json.contains("\"shard_group\":\"orders-sg\""));
        assert!(json.contains("\"replica_count\":3"));
    }
}
